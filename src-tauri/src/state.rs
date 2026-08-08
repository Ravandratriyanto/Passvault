use std::sync::Mutex;
use std::time::Instant;
use zeroize::Zeroizing;
use crate::vault::VaultEntry;

pub enum VaultFormat {
    LegacyV2 {
        salt: [u8; 16],
        flags: u8,
    },
    V3 {
        salt: [u8; 16],
        flags: u8,
        shares_blob: Vec<u8>,
    },
}

pub struct UnlockedVault {
    pub entries: Vec<VaultEntry>,
    pub master_key: Zeroizing<[u8; 32]>,
    pub format: VaultFormat,
    pub last_activity: Instant,
}

pub struct AppState {
    pub vault: Mutex<Option<UnlockedVault>>,
    pub failed_attempts: Mutex<u32>,
    pub lockout_until: Mutex<Option<Instant>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            vault: Mutex::new(None),
            failed_attempts: Mutex::new(0),
            lockout_until: Mutex::new(None),
        }
    }
}
