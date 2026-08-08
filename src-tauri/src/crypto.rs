use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use aes_gcm::aead::rand_core::RngCore;
use argon2::{Argon2, Params};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

#[derive(Clone, Copy)]
pub enum Factor { Pin, Password, Keyfile }

impl Factor {
    fn domain(self) -> &'static [u8] {
        match self {
            Factor::Pin      => b"onyxlock-v3-pin\0",
            Factor::Password => b"onyxlock-v3-pw\0",
            Factor::Keyfile  => b"onyxlock-v3-kf\0",
        }
    }
}

pub fn derive_factor_key(factor: Factor, material: &[u8], salt: &[u8; 16]) -> Zeroizing<[u8; 32]> {
    let mut input = Zeroizing::new(Vec::with_capacity(factor.domain().len() + 32));
    input.extend_from_slice(factor.domain());

    if matches!(factor, Factor::Keyfile) {
        let mut h = Sha256::new();
        h.update(material);
        input.extend_from_slice(&h.finalize());
    } else {
        input.extend_from_slice(material);
    }

    let mut key = Zeroizing::new([0u8; 32]);
    let params = Params::new(131072, 4, 1, Some(32)).unwrap();
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
    if data.len() < 12 { return Err("data too short".into()); }
    let (nonce_bytes, ciphertext) = data.split_at(12);
    let cipher = Aes256Gcm::new_from_slice(key).unwrap();
    let nonce = Nonce::from_slice(nonce_bytes);
    cipher.decrypt(nonce, ciphertext)
        .map_err(|_| "decryption failed".into())
}

pub fn random_salt() -> [u8; 16] {
    let mut salt = [0u8; 16];
    OsRng.fill_bytes(&mut salt);
    salt
}

pub fn random_key() -> Zeroizing<[u8; 32]> {
    let mut k = Zeroizing::new([0u8; 32]);
    OsRng.fill_bytes(k.as_mut());
    k
}

pub fn derive_key_legacy(password: &str, keyfile: Option<&[u8]>, salt: &[u8; 16]) -> Zeroizing<[u8; 32]> {
    let mut input = Zeroizing::new(Vec::with_capacity(password.len() + 32));
    input.extend_from_slice(password.as_bytes());
    if let Some(kf) = keyfile {
        let mut h = Sha256::new();
        h.update(kf);
        input.extend_from_slice(&h.finalize());
    }
    let mut key = Zeroizing::new([0u8; 32]);
    let params = Params::new(131072, 4, 1, Some(32)).unwrap();
    Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params)
        .hash_password_into(&input, salt, key.as_mut())
        .expect("argon2 failed");
    key
}

mod gf {
    // Peasant multiplication in GF(2^8) with Rijndael's irreducible polynomial 0x11b.
    pub fn mul(mut a: u8, mut b: u8) -> u8 {
        let mut r = 0u8;
        for _ in 0..8 {
            if b & 1 != 0 { r ^= a; }
            let hi = a & 0x80;
            a <<= 1;
            if hi != 0 { a ^= 0x1b; }
            b >>= 1;
        }
        r
    }

    // a^-1 = a^254 in GF(256), computed via square-and-multiply.
    pub fn inv(a: u8) -> u8 {
        assert!(a != 0, "gf inverse of zero");
        let mut result = 1u8;
        let mut base = a;
        let mut exp = 254u8;
        while exp > 0 {
            if exp & 1 != 0 { result = mul(result, base); }
            base = mul(base, base);
            exp >>= 1;
        }
        result
    }
}

pub fn shamir_split_2of(secret: &[u8; 32], n: u8) -> Vec<[u8; 33]> {
    assert!(n >= 2);
    let mut coeff = [0u8; 32];
    OsRng.fill_bytes(&mut coeff);
    (1..=n).map(|x| {
        let mut share = [0u8; 33];
        share[0] = x;
        for i in 0..32 {
            share[1 + i] = secret[i] ^ gf::mul(coeff[i], x);
        }
        share
    }).collect()
}

pub fn shamir_combine_2(a: &[u8; 33], b: &[u8; 33]) -> Result<[u8; 32], String> {
    let (xa, xb) = (a[0], b[0]);
    if xa == 0 || xb == 0 || xa == xb {
        return Err("invalid shares".into());
    }
    let denom = xa ^ xb;
    let inv_denom = gf::inv(denom);
    let la = gf::mul(xb, inv_denom);
    let lb = gf::mul(xa, inv_denom);
    let mut out = [0u8; 32];
    for i in 0..32 {
        out[i] = gf::mul(a[1 + i], la) ^ gf::mul(b[1 + i], lb);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn split_and_combine_any_two() {
        let secret = [0x42u8; 32];
        let shares = shamir_split_2of(&secret, 3);
        for i in 0..3 {
            for j in (i+1)..3 {
                let out = shamir_combine_2(&shares[i], &shares[j]).unwrap();
                assert_eq!(out, secret);
            }
        }
    }

    #[test]
    fn split_and_combine_random_secret() {
        let mut secret = [0u8; 32];
        OsRng.fill_bytes(&mut secret);
        let shares = shamir_split_2of(&secret, 3);
        assert_eq!(shamir_combine_2(&shares[0], &shares[2]).unwrap(), secret);
    }
}
