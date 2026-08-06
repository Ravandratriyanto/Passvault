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
use crypto::{decrypt, derive_key, encrypt, random_salt};
use vault::{Vault, VaultEntry};
use state::{AppState, UnlockedVault};
use settings::Settings;
use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use flate2::{write::DeflateEncoder, read::DeflateDecoder, Compression};
use std::io::{Read, Write};

const FILE_VERSION: u8 = 2;
const FLAG_KEYFILE: u8 = 0x01;
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

#[tauri::command]
fn vault_exists(app: AppHandle) -> bool {
    vault_path(&app).exists()
}

#[tauri::command]
fn vault_needs_keyfile(app: AppHandle) -> Result<bool, String> {
    let path = vault_path(&app);
    if !path.exists() {
        return Ok(false);
    }
    let data = std::fs::read(path).map_err(|e| e.to_string())?;
    if data.len() < 2 {
        return Err("corrupt vault".into());
    }
    if data[0] != 1 && data[0] != 2 {
        return Err("unsupported vault version".into());
    }
    Ok(data[1] & FLAG_KEYFILE != 0)
}

#[tauri::command]
fn setup(
    password: String,
    keyfile: Option<Vec<u8>>,
    autostart: Option<bool>,
    app: AppHandle,
    state: State<AppState>,
) -> Result<(), String> {
    if vault_path(&app).exists() {
        return Err("vault already exists".into());
    }
    let salt = random_salt();
    let key = derive_key(&password, keyfile.as_deref(), &salt);
    let vault = Vault::default();
    let json = serde_json::to_vec(&vault).unwrap();
    let encrypted = encrypt(&compress(&json), &key);

    let flags: u8 = if keyfile.is_some() { FLAG_KEYFILE } else { 0 };
    let mut file_data = Vec::with_capacity(18 + encrypted.len());
    file_data.push(FILE_VERSION);
    file_data.push(flags);
    file_data.extend_from_slice(&salt);
    file_data.extend_from_slice(&encrypted);

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
        key,
        salt,
        flags,
        last_activity: Instant::now(),
    });
    Ok(())
}

#[tauri::command]
fn unlock(
    password: String,
    keyfile: Option<Vec<u8>>,
    app: AppHandle,
    state: State<AppState>,
) -> Result<(), String> {
    check_lockout(&state)?;

    let file_data = std::fs::read(vault_path(&app)).map_err(|_| "vault not found".to_string())?;
    if file_data.len() < 18 {
        return Err("corrupt vault file".into());
    }
    let version = file_data[0];
    if version != 1 && version != 2 {
        return Err("unsupported vault version".into());
    }
    let flags = file_data[1];
    let salt: [u8; 16] = file_data[2..18].try_into().unwrap();
    let encrypted = &file_data[18..];

    if flags & FLAG_KEYFILE != 0 && keyfile.is_none() {
        return Err("keyfile required".into());
    }

    let key = derive_key(&password, keyfile.as_deref(), &salt);
    let decrypted = match decrypt(encrypted, &key) {
        Ok(j) => j,
        Err(_) => {
            let n = register_failure(&state);
            return Err(format!("wrong password (attempt {})", n));
        }
    };
    let json = if version == 2 { decompress(&decrypted)? } else { decrypted };
    let vault: Vault = serde_json::from_slice(&json).map_err(|e| e.to_string())?;

    *state.failed_attempts.lock().unwrap() = 0;
    *state.vault.lock().unwrap() = Some(UnlockedVault {
        entries: vault.entries,
        key,
        salt,
        flags,
        last_activity: Instant::now(),
    });
    Ok(())
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
    let encrypted = encrypt(&compress(&json), &v.key);
    let mut file_data = Vec::with_capacity(18 + encrypted.len());
    file_data.push(FILE_VERSION);
    file_data.push(v.flags);
    file_data.extend_from_slice(&v.salt);
    file_data.extend_from_slice(&encrypted);
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
    password: String,
    keyfile: Option<Vec<u8>>,
    app: AppHandle,
    state: State<AppState>,
) -> Result<(), String> {
    check_lockout(&state)?;

    let file_data = std::fs::read(vault_path(&app)).map_err(|_| "vault not found".to_string())?;
    if file_data.len() < 18 || (file_data[0] != 1 && file_data[0] != 2) {
        return Err("corrupt vault".into());
    }
    let flags = file_data[1];
    let salt: [u8; 16] = file_data[2..18].try_into().unwrap();
    let encrypted = &file_data[18..];

    if flags & FLAG_KEYFILE != 0 && keyfile.is_none() {
        return Err("keyfile required".into());
    }

    let key = derive_key(&password, keyfile.as_deref(), &salt);
    if decrypt(encrypted, &key).is_err() {
        let n = register_failure(&state);
        return Err(format!("wrong password (attempt {})", n));
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
    keyfile: Option<Vec<u8>>,
    password: String,
    app: AppHandle,
    state: State<AppState>,
) -> Result<(), String> {
    check_lockout(&state)?;
    if vault_path(&app).exists() {
        return Err("delete existing vault first".into());
    }

    let clean: String = blob_b64.chars().filter(|c| !c.is_whitespace()).collect();
    let raw = B64.decode(&clean).map_err(|e| format!("invalid backup data: {}", e))?;
    if raw.len() < 18 {
        return Err("backup too short".into());
    }
    let version = raw[0];
    if version != 1 && version != 2 {
        return Err("unsupported backup version".into());
    }
    let flags = raw[1];
    let salt: [u8; 16] = raw[2..18].try_into().unwrap();
    let encrypted = &raw[18..];

    if flags & FLAG_KEYFILE != 0 && keyfile.is_none() {
        return Err("keyfile required".into());
    }

    let key = derive_key(&password, keyfile.as_deref(), &salt);
    let decrypted = match decrypt(encrypted, &key) {
        Ok(d) => d,
        Err(_) => {
            let n = register_failure(&state);
            return Err(format!("wrong password (attempt {})", n));
        }
    };
    let json = if version == 2 { decompress(&decrypted)? } else { decrypted };
    let vault: Vault = serde_json::from_slice(&json).map_err(|e| e.to_string())?;

    let unlocked = UnlockedVault {
        entries: vault.entries,
        key,
        salt,
        flags,
        last_activity: Instant::now(),
    };
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
                .tooltip("Passvault")
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
            vault_needs_keyfile,
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

