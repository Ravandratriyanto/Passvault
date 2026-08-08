mod crypto;
mod vault;
mod state;
mod settings;

use std::str::FromStr;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use rand::Rng;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, State, WindowEvent,
};
use tauri_plugin_autostart::{ManagerExt, MacosLauncher};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use uuid::Uuid;
use zeroize::Zeroizing;
use crypto::{
    decrypt, encrypt, random_salt, random_key,
    derive_key_legacy, derive_factor_key, Factor,
    shamir_split_2of, shamir_combine_2,
};
use vault::{Vault, VaultEntry};
use state::{AppState, UnlockedVault, VaultFormat};
use settings::Settings;
use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use flate2::{write::DeflateEncoder, read::DeflateDecoder, Compression};
use std::io::{Read, Write};

const FILE_VERSION_V3: u8 = 3;
const FLAG_KEYFILE_V2: u8 = 0x01;
const F_PIN: u8 = 0b001;
const F_PW:  u8 = 0b010;
const F_KF:  u8 = 0b100;
const AUTO_LOCK_SECS: u64 = 300;

fn vault_path(app: &AppHandle) -> std::path::PathBuf {
    app.path().app_data_dir().unwrap().join("vault.enc")
}
fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}

fn lockout_seconds(fails: u32) -> u64 {
    match fails {
        0..=2 => 0,
        3 => 15,
        4 => 30,
        5 => 60,
        6 => 300,
        _ => 900,
    }
}
fn check_lockout(state: &State<AppState>) -> Result<(), String> {
    let mut lockout = state.lockout_until.lock().unwrap();
    if let Some(until) = *lockout {
        let now = Instant::now();
        if now < until {
            return Err(format!("locked out — wait {}s", (until - now).as_secs()));
        }
        *lockout = None;
    }
    Ok(())
}
fn register_failure(state: &State<AppState>) -> u32 {
    let mut fails = state.failed_attempts.lock().unwrap();
    *fails += 1;
    let wait = lockout_seconds(*fails);
    if wait > 0 {
        *state.lockout_until.lock().unwrap() = Some(Instant::now() + Duration::from_secs(wait));
    }
    *fails
}
fn touch(guard: &mut Option<UnlockedVault>) -> Result<&mut UnlockedVault, String> {
    let should_lock = guard
        .as_ref()
        .map_or(false, |v| v.last_activity.elapsed().as_secs() > AUTO_LOCK_SECS);
    if should_lock {
        *guard = None;
    }
    let v = guard.as_mut().ok_or("vault locked")?;
    v.last_activity = Instant::now();
    Ok(v)
}

fn show_main(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.set_skip_taskbar(false);
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
    }
}
fn hide_main(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.hide();
        let _ = win.set_skip_taskbar(true);
    }
}

const BLOCKED_HOTKEYS: &[&str] = &[
    "Ctrl+C", "Ctrl+V", "Ctrl+X", "Ctrl+Z", "Ctrl+A", "Ctrl+S",
    "Ctrl+F", "Ctrl+N", "Ctrl+T", "Ctrl+W", "Ctrl+Y", "Ctrl+P",
    "Ctrl+Tab", "Alt+F4", "Alt+Tab", "Alt+Space",
];

fn validate_hotkey(s: &str) -> Result<(), String> {
    let has_mod = ["Ctrl", "Alt", "Shift"].iter().any(|m| s.contains(m));
    if !has_mod {
        return Err("hotkey must include a modifier (Ctrl, Alt, or Shift)".into());
    }
    if s.contains("Super") || s.contains("Meta") || s.contains("Win") {
        return Err("Windows-key shortcuts are reserved by the OS".into());
    }
    let normalized = s.replace(' ', "");
    if BLOCKED_HOTKEYS.iter().any(|b| b.eq_ignore_ascii_case(&normalized)) {
        return Err("that shortcut is commonly used by other apps — pick another".into());
    }
    Ok(())
}

fn apply_hotkey(app: &AppHandle, new: &str, previous: Option<&str>) -> Result<(), String> {
    validate_hotkey(new)?;
    let new_shortcut = Shortcut::from_str(new).map_err(|_| "unrecognized hotkey".to_string())?;
    let gs = app.global_shortcut();

    if let Some(prev) = previous {
        if let Ok(prev_shortcut) = Shortcut::from_str(prev) {
            let _ = gs.unregister(prev_shortcut);
        }
    }
    gs.register(new_shortcut)
        .map_err(|e| format!("could not register — already in use? ({})", e))
}

fn set_autostart_os(app: &AppHandle, enabled: bool) -> Result<(), String> {
    let manager = app.autolaunch();
    if enabled {
        manager.enable().map_err(|e| e.to_string())
    } else {
        manager.disable().map_err(|e| e.to_string())
    }
}

fn push_lp16(buf: &mut Vec<u8>, data: &[u8]) {
    buf.extend_from_slice(&(data.len() as u16).to_le_bytes());
    buf.extend_from_slice(data);
}

fn build_v3_file(flags: u8, salt: &[u8; 16], vault_ct: &[u8], shares_blob: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(22 + vault_ct.len() + shares_blob.len());
    out.push(FILE_VERSION_V3);
    out.push(flags);
    out.extend_from_slice(salt);
    out.extend_from_slice(&(vault_ct.len() as u32).to_le_bytes());
    out.extend_from_slice(vault_ct);
    out.extend_from_slice(shares_blob);
    out
}

#[tauri::command]
fn vault_exists(app: AppHandle) -> bool {
    vault_path(&app).exists()
}

#[derive(serde::Serialize)]
pub struct VaultInfo {
    pub version: u8,
    pub needs_pin: bool,
    pub needs_password: bool,
    pub needs_keyfile: bool,
}

#[tauri::command]
fn vault_info(app: AppHandle) -> Result<VaultInfo, String> {
    let path = vault_path(&app);
    if !path.exists() { return Err("no vault".into()); }
    let data = std::fs::read(path).map_err(|e| e.to_string())?;
    if data.len() < 2 { return Err("corrupt vault".into()); }
    match data[0] {
        1 | 2 => Ok(VaultInfo {
            version: data[0],
            needs_pin: true,
            needs_password: false,
            needs_keyfile: data[1] & FLAG_KEYFILE_V2 != 0,
        }),
        3 => {
            let f = data[1];
            Ok(VaultInfo {
                version: 3,
                needs_pin: f & F_PIN != 0,
                needs_password: f & F_PW != 0,
                needs_keyfile: f & F_KF != 0,
            })
        }
        _ => Err("unsupported vault version".into()),
    }
}

#[tauri::command]
fn setup(
    pin: String,
    password: String,
    keyfile: Option<Vec<u8>>,
    autostart: Option<bool>,
    app: AppHandle,
    state: State<AppState>,
) -> Result<(), String> {
    if vault_path(&app).exists() {
        return Err("vault already exists".into());
    }
    if pin.is_empty() || password.is_empty() {
        return Err("PIN and password are both required".into());
    }

    let salt = random_salt();
    let master_key = random_key();

    let has_kf = keyfile.is_some();
    let flags = F_PIN | F_PW | if has_kf { F_KF } else { 0 };
    let n_shares = if has_kf { 3 } else { 2 };

    let shares = shamir_split_2of(&master_key, n_shares);

    let pin_key = derive_factor_key(Factor::Pin, pin.as_bytes(), &salt);
    let pw_key  = derive_factor_key(Factor::Password, password.as_bytes(), &salt);
    let pin_share_ct = encrypt(&shares[0], &pin_key);
    let pw_share_ct  = encrypt(&shares[1], &pw_key);
    let kf_share_ct = if let Some(kf) = keyfile.as_deref() {
        let kf_key = derive_factor_key(Factor::Keyfile, kf, &salt);
        Some(encrypt(&shares[2], &kf_key))
    } else { None };

    let vault = Vault::default();
    let json = serde_json::to_vec(&vault).unwrap();
    let vault_ct = encrypt(&compress(&json), &master_key);

    let mut shares_blob = Vec::new();
    push_lp16(&mut shares_blob, &pin_share_ct);
    push_lp16(&mut shares_blob, &pw_share_ct);
    if let Some(ref kf_ct) = kf_share_ct { push_lp16(&mut shares_blob, kf_ct); }

    let file_data = build_v3_file(flags, &salt, &vault_ct, &shares_blob);
    std::fs::create_dir_all(vault_path(&app).parent().unwrap()).ok();
    std::fs::write(vault_path(&app), &file_data).map_err(|e| e.to_string())?;

    if let Some(want) = autostart {
        let _ = set_autostart_os(&app, want);
        let mut cfg = settings::load(&app);
        cfg.autostart = Some(want);
        let _ = settings::save(&app, &cfg);
    }

    *state.vault.lock().unwrap() = Some(UnlockedVault {
        entries: vault.entries,
        master_key,
        format: VaultFormat::V3 { salt, flags, shares_blob },
        last_activity: Instant::now(),
    });
    Ok(())
}

fn unlock_legacy(
    file_data: &[u8],
    pin: Option<String>,
    keyfile: Option<Vec<u8>>,
) -> Result<UnlockedVault, String> {
    if file_data.len() < 18 { return Err("corrupt vault file".into()); }
    let version = file_data[0];
    let flags = file_data[1];
    let salt: [u8; 16] = file_data[2..18].try_into().unwrap();
    let encrypted = &file_data[18..];

    let pin = pin.ok_or("PIN required")?;
    if flags & FLAG_KEYFILE_V2 != 0 && keyfile.is_none() {
        return Err("keyfile required".into());
    }

    let key = derive_key_legacy(&pin, keyfile.as_deref(), &salt);
    let decrypted = decrypt(encrypted, &key)
        .map_err(|_| "decryption failed — wrong password".to_string())?;
    let json = if version == 2 { decompress(&decrypted)? } else { decrypted };
    let vault: Vault = serde_json::from_slice(&json).map_err(|e| e.to_string())?;

    Ok(UnlockedVault {
        entries: vault.entries,
        master_key: key,
        format: VaultFormat::LegacyV2 { salt, flags },
        last_activity: Instant::now(),
    })
}

fn unlock_v3(
    file_data: &[u8],
    pin: Option<String>,
    password: Option<String>,
    keyfile: Option<Vec<u8>>,
) -> Result<UnlockedVault, String> {
    if file_data.len() < 22 { return Err("corrupt vault file".into()); }
    let flags = file_data[1];
    let salt: [u8; 16] = file_data[2..18].try_into().unwrap();
    let vault_ct_len = u32::from_le_bytes(file_data[18..22].try_into().unwrap()) as usize;
    let vault_end = 22 + vault_ct_len;
    if file_data.len() < vault_end { return Err("corrupt vault file".into()); }
    let vault_ct = &file_data[22..vault_end];
    let shares_blob = &file_data[vault_end..];

    let mut cur = 0usize;
    let mut pin_ct: Option<&[u8]> = None;
    let mut pw_ct:  Option<&[u8]> = None;
    let mut kf_ct:  Option<&[u8]> = None;
    for (bit, slot) in [
        (F_PIN, &mut pin_ct),
        (F_PW,  &mut pw_ct),
        (F_KF,  &mut kf_ct),
    ] {
        if flags & bit != 0 {
            if cur + 2 > shares_blob.len() { return Err("truncated share".into()); }
            let n = u16::from_le_bytes(shares_blob[cur..cur+2].try_into().unwrap()) as usize;
            cur += 2;
            if cur + n > shares_blob.len() { return Err("truncated share".into()); }
            *slot = Some(&shares_blob[cur..cur+n]);
            cur += n;
        }
    }

    let mut recovered: Vec<[u8; 33]> = Vec::with_capacity(2);
    if let (Some(p), Some(ct)) = (pin.as_deref(), pin_ct) {
        let k = derive_factor_key(Factor::Pin, p.as_bytes(), &salt);
        if let Ok(pt) = decrypt(ct, &k) {
            if pt.len() == 33 {
                recovered.push(pt.as_slice().try_into().unwrap());
            }
        }
    }
    if recovered.len() < 2 {
        if let (Some(p), Some(ct)) = (password.as_deref(), pw_ct) {
            let k = derive_factor_key(Factor::Password, p.as_bytes(), &salt);
            if let Ok(pt) = decrypt(ct, &k) {
                if pt.len() == 33 {
                    recovered.push(pt.as_slice().try_into().unwrap());
                }
            }
        }
    }
    if recovered.len() < 2 {
        if let (Some(kf), Some(ct)) = (keyfile.as_deref(), kf_ct) {
            let k = derive_factor_key(Factor::Keyfile, kf, &salt);
            if let Ok(pt) = decrypt(ct, &k) {
                if pt.len() == 33 {
                    recovered.push(pt.as_slice().try_into().unwrap());
                }
            }
        }
    }

    if recovered.len() < 2 {
        return Err("decryption failed — need any two matching factors".into());
    }

    let combined = shamir_combine_2(&recovered[0], &recovered[1])?;
    let master_key = Zeroizing::new(combined);

    let decrypted = decrypt(vault_ct, &master_key)
        .map_err(|_| "decryption failed — reconstructed key is wrong".to_string())?;
    let json = decompress(&decrypted)?;
    let vault: Vault = serde_json::from_slice(&json).map_err(|e| e.to_string())?;

    Ok(UnlockedVault {
        entries: vault.entries,
        master_key,
        format: VaultFormat::V3 { salt, flags, shares_blob: shares_blob.to_vec() },
        last_activity: Instant::now(),
    })
}

#[tauri::command]
fn unlock(
    pin: Option<String>,
    password: Option<String>,
    keyfile: Option<Vec<u8>>,
    app: AppHandle,
    state: State<AppState>,
) -> Result<(), String> {
    check_lockout(&state)?;

    let file_data = std::fs::read(vault_path(&app)).map_err(|_| "vault not found".to_string())?;
    if file_data.is_empty() { return Err("corrupt vault".into()); }

    let result = match file_data[0] {
        1 | 2 => unlock_legacy(&file_data, pin, keyfile),
        3     => unlock_v3(&file_data, pin, password, keyfile),
        _     => Err("unsupported vault version".into()),
    };

    match result {
        Ok(unlocked) => {
            *state.failed_attempts.lock().unwrap() = 0;
            *state.vault.lock().unwrap() = Some(unlocked);
            Ok(())
        }
        Err(e) if e.contains("decryption failed") || e.contains("matching factors") => {
            let n = register_failure(&state);
            Err(format!("wrong credentials (attempt {})", n))
        }
        Err(e) => Err(e),
    }
}

#[tauri::command]
fn lock(state: State<AppState>) {
    *state.vault.lock().unwrap() = None;
}

#[tauri::command]
fn is_unlocked(state: State<AppState>) -> bool {
    let mut guard = state.vault.lock().unwrap();
    let should_lock = guard
        .as_ref()
        .map_or(false, |v| v.last_activity.elapsed().as_secs() > AUTO_LOCK_SECS);
    if should_lock {
        *guard = None;
    }
    guard.is_some()
}

#[tauri::command]
fn get_entries(state: State<AppState>) -> Result<Vec<VaultEntry>, String> {
    let mut guard = state.vault.lock().unwrap();
    let v = touch(&mut *guard)?;
    Ok(v.entries.clone())
}

fn save_vault(app: &AppHandle, v: &UnlockedVault) -> Result<(), String> {
    let vault = Vault { entries: v.entries.clone() };
    let json = serde_json::to_vec(&vault).unwrap();
    let vault_ct = encrypt(&compress(&json), &v.master_key);

    let file_data = match &v.format {
        VaultFormat::LegacyV2 { salt, flags } => {
            let mut out = Vec::with_capacity(18 + vault_ct.len());
            out.push(2);
            out.push(*flags);
            out.extend_from_slice(salt);
            out.extend_from_slice(&vault_ct);
            out
        }
        VaultFormat::V3 { salt, flags, shares_blob } => {
            build_v3_file(*flags, salt, &vault_ct, shares_blob)
        }
    };
    std::fs::write(vault_path(app), &file_data).map_err(|e| e.to_string())
}

#[tauri::command]
fn add_entry(
    title: String, username: String, password: String,
    url: Option<String>, notes: Option<String>,
    category: Option<String>, totp_secret: Option<String>,
    app: AppHandle, state: State<AppState>,
) -> Result<VaultEntry, String> {
    let mut guard = state.vault.lock().unwrap();
    let v = touch(&mut *guard)?;
    let entry = VaultEntry {
        id: Uuid::new_v4().to_string(),
        title, username, password, url, notes, category, totp_secret,
        created_at: now_secs(),
        updated_at: now_secs(),
    };
    v.entries.push(entry.clone());
    save_vault(&app, v)?;
    Ok(entry)
}

#[tauri::command]
fn update_entry(entry: VaultEntry, app: AppHandle, state: State<AppState>) -> Result<(), String> {
    let mut guard = state.vault.lock().unwrap();
    let v = touch(&mut *guard)?;
    let existing = v.entries.iter_mut().find(|e| e.id == entry.id).ok_or("entry not found")?;
    *existing = VaultEntry { updated_at: now_secs(), ..entry };
    save_vault(&app, v)
}

#[tauri::command]
fn delete_entry(id: String, app: AppHandle, state: State<AppState>) -> Result<(), String> {
    let mut guard = state.vault.lock().unwrap();
    let v = touch(&mut *guard)?;
    v.entries.retain(|e| e.id != id);
    save_vault(&app, v)
}

#[tauri::command]
fn generate_password(length: usize, symbols: bool) -> String {
    let mut chars: Vec<char> = ('a'..='z').chain('A'..='Z').chain('0'..='9').collect();
    if symbols {
        chars.extend("!@#$%^&*()-_=+[]{}|;:,.<>?".chars());
    }
    let mut rng = rand::thread_rng();
    (0..length).map(|_| chars[rng.gen_range(0..chars.len())]).collect()
}

#[tauri::command]
fn delete_vault(
    pin: Option<String>,
    password: Option<String>,
    keyfile: Option<Vec<u8>>,
    app: AppHandle,
    state: State<AppState>,
) -> Result<(), String> {
    check_lockout(&state)?;
    let file_data = std::fs::read(vault_path(&app)).map_err(|_| "vault not found".to_string())?;
    let ok = match file_data.first().copied() {
        Some(1) | Some(2) => unlock_legacy(&file_data, pin, keyfile).is_ok(),
        Some(3)           => unlock_v3(&file_data, pin, password, keyfile).is_ok(),
        _                 => false,
    };
    if !ok {
        let n = register_failure(&state);
        return Err(format!("wrong credentials (attempt {})", n));
    }
    std::fs::remove_file(vault_path(&app)).map_err(|e| e.to_string())?;
    *state.vault.lock().unwrap() = None;
    *state.failed_attempts.lock().unwrap() = 0;
    Ok(())
}

#[tauri::command]
fn get_settings(app: AppHandle) -> Settings {
    settings::load(&app)
}

#[tauri::command]
fn set_hotkey(new_hotkey: String, app: AppHandle) -> Result<(), String> {
    let mut current = settings::load(&app);
    let previous = current.hotkey.clone();
    apply_hotkey(&app, &new_hotkey, Some(&previous))?;
    current.hotkey = new_hotkey;
    settings::save(&app, &current)
}

#[tauri::command]
fn set_autostart(enabled: bool, app: AppHandle) -> Result<(), String> {
    set_autostart_os(&app, enabled)?;
    let mut current = settings::load(&app);
    current.autostart = Some(enabled);
    settings::save(&app, &current)
}

#[tauri::command]
fn export_vault(app: AppHandle) -> Result<String, String> {
    let data = std::fs::read(vault_path(&app)).map_err(|_| "no vault to export".to_string())?;
    Ok(B64.encode(data))
}

#[tauri::command]
fn import_vault(
    blob_b64: String,
    pin: Option<String>,
    password: Option<String>,
    keyfile: Option<Vec<u8>>,
    app: AppHandle,
    state: State<AppState>,
) -> Result<(), String> {
    check_lockout(&state)?;
    if vault_path(&app).exists() {
        return Err("delete existing vault first".into());
    }

    let clean: String = blob_b64.chars().filter(|c| !c.is_whitespace()).collect();
    let raw = B64.decode(&clean).map_err(|e| format!("invalid backup data: {}", e))?;

    let unlocked = match raw.first().copied() {
        Some(1) | Some(2) => unlock_legacy(&raw, pin, keyfile),
        Some(3)           => unlock_v3(&raw, pin, password, keyfile),
        _                 => Err("unsupported backup version".into()),
    }?;

    std::fs::create_dir_all(vault_path(&app).parent().unwrap()).ok();
    save_vault(&app, &unlocked)?;

    *state.failed_attempts.lock().unwrap() = 0;
    *state.vault.lock().unwrap() = Some(unlocked);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main(app);
        }))
        .manage(AppState::new())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--hidden"]),
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        show_main(app);
                    }
                })
                .build(),
        )
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.app_handle().emit("close-requested", ());
                api.prevent_close();
            }
        })
        .setup(|app| {
            let handle = app.handle().clone();
            let cfg = settings::load(&handle);

            let hidden_launch = std::env::args().any(|a| a == "--hidden");
            if !hidden_launch {
                show_main(&handle);
            }

            if let Err(e) = apply_hotkey(&handle, &cfg.hotkey, None) {
                eprintln!("hotkey registration failed: {}", e);
            }

            if let Some(want) = cfg.autostart {
                let auto = handle.autolaunch();
                let currently = auto.is_enabled().unwrap_or(false);
                if want && !currently {
                    let _ = auto.enable();
                } else if !want && currently {
                    let _ = auto.disable();
                }
            }

            let show_i = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
            let lock_i = MenuItem::with_id(app, "lock", "Lock", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &lock_i, &quit_i])?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("Onyxlock")
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => show_main(app),
                    "lock" => {
                        if let Some(s) = app.try_state::<AppState>() {
                            *s.vault.lock().unwrap() = None;
                        }
                        hide_main(app);
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            vault_exists,
            vault_info,
            setup,
            unlock,
            lock,
            is_unlocked,
            get_entries,
            add_entry,
            update_entry,
            delete_entry,
            generate_password,
            delete_vault,
            get_settings,
            set_hotkey,
            set_autostart,
            export_vault,
            import_vault,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn compress(data: &[u8]) -> Vec<u8> {
    let mut enc = DeflateEncoder::new(Vec::new(), Compression::default());
    enc.write_all(data).unwrap();
    enc.finish().unwrap()
}
fn decompress(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    DeflateDecoder::new(data)
        .read_to_end(&mut out)
        .map_err(|e| format!("decompress failed: {}", e))?;
    Ok(out)
}
