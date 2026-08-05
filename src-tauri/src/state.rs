use std::sync::Mutex;
use std::time::Instant;
use zeroize::Zeroizing;
use crate::vault::VaultEntry;
pub struct UnlockedVault {
    pub entries: Vec<VaultEntry>,
    pub key: Zeroizing<[u8; 32]>,
    pub salt: [u8; 16],
    pub flags: u8,
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