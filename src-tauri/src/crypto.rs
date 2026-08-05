use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use aes_gcm::aead::rand_core::RngCore;
use argon2::{Argon2, Params};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

pub fn derive_key(password: &str, keyfile: Option<&[u8]>, salt: &[u8; 16]) -> Zeroizing<[u8; 32]> {
    let mut input = Zeroizing::new(Vec::with_capacity(password.len() + 32));
    input.extend_from_slice(password.as_bytes());
    if let Some(kf) = keyfile {
        let mut hasher = Sha256::new();
        hasher.update(kf);
        let kf_hash = hasher.finalize();
        input.extend_from_slice(&kf_hash);
    }

    let mut key = Zeroizing::new([0u8; 32]);
    let params = Params::new(131072, 4, 1, Some(32)).unwrap(); // 128 MB, 4 iters
    Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params)
        .hash_password_into(&input, salt, key.as_mut())
        .expect("argon2 failed");
    key
}

pub fn encrypt(plaintext: &[u8], key: &[u8; 32]) -> Vec<u8> {
    let cipher = Aes256Gcm::new_from_slice(key).unwrap();
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher.encrypt(nonce, plaintext).expect("encryption failed");

    let mut out = Vec::with_capacity(12 + ciphertext.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    out
}

pub fn decrypt(data: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, String> {
    if data.len() < 12 {
        return Err("data too short".into());
    }
    let (nonce_bytes, ciphertext) = data.split_at(12);
    let cipher = Aes256Gcm::new_from_slice(key).unwrap();
    let nonce = Nonce::from_slice(nonce_bytes);
    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| "decryption failed — wrong password".into())
}

pub fn random_salt() -> [u8; 16] {
    let mut salt = [0u8; 16];
    OsRng.fill_bytes(&mut salt);
    salt
}