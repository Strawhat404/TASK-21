use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use rand::Rng;
use base64::{engine::general_purpose::STANDARD, Engine};
use std::path::Path;

/// Load or generate the application encryption key (32 bytes for AES-256).
/// Key path is resolved from ENCRYPTION_KEY_PATH env var,
/// defaulting to /var/lib/fund_transparency/encryption.key (host-managed, outside repo).
///
/// Returns an error instead of panicking when the key file is malformed or
/// unreadable, allowing the caller to log the problem and exit gracefully.
pub fn load_or_create_key() -> Result<[u8; 32], String> {
    let key_path = std::env::var("ENCRYPTION_KEY_PATH")
        .unwrap_or_else(|_| "/var/lib/fund_transparency/encryption.key".to_string());
    load_or_create_key_at(&key_path)
}

pub fn load_or_create_key_at(key_path: &str) -> Result<[u8; 32], String> {
    let path = Path::new(key_path);
    if path.exists() {
        let data = std::fs::read(path).map_err(|e| {
            format!("Failed to read encryption key file '{}': {}", key_path, e)
        })?;
        if data.len() < 32 {
            return Err(format!(
                "Encryption key file '{}' is malformed: expected at least 32 bytes, got {}. \
                 Delete the file to auto-generate a new key, or provide a valid 32-byte key.",
                key_path, data.len()
            ));
        }
        let mut key = [0u8; 32];
        key.copy_from_slice(&data[..32]);
        Ok(key)
    } else {
        let key: [u8; 32] = rand::thread_rng().gen();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::write(path, key).map_err(|e| {
            format!("Failed to write encryption key file '{}': {}", key_path, e)
        })?;
        Ok(key)
    }
}

/// Encrypt plaintext using AES-256-GCM. Returns base64(nonce || ciphertext).
pub fn encrypt(plaintext: &str, key: &[u8; 32]) -> Result<String, String> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| e.to_string())?;
    let nonce_bytes: [u8; 12] = rand::thread_rng().gen();
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| e.to_string())?;

    let mut combined = nonce_bytes.to_vec();
    combined.extend_from_slice(&ciphertext);
    Ok(STANDARD.encode(&combined))
}

/// Encrypt raw bytes using AES-256-GCM. Returns nonce || ciphertext.
pub fn encrypt_bytes(plaintext: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, String> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| e.to_string())?;
    let nonce_bytes: [u8; 12] = rand::thread_rng().gen();
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| e.to_string())?;

    let mut combined = nonce_bytes.to_vec();
    combined.extend_from_slice(&ciphertext);
    Ok(combined)
}

/// Decrypt raw bytes (nonce || ciphertext) using AES-256-GCM.
pub fn decrypt_bytes(encrypted: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, String> {
    if encrypted.len() < 13 {
        return Err("Ciphertext too short".into());
    }
    let (nonce_bytes, ciphertext) = encrypted.split_at(12);
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| e.to_string())?;
    let nonce = Nonce::from_slice(nonce_bytes);

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| e.to_string())
}

/// Decrypt base64(nonce || ciphertext) using AES-256-GCM.
pub fn decrypt(encoded: &str, key: &[u8; 32]) -> Result<String, String> {
    let combined = STANDARD.decode(encoded).map_err(|e| e.to_string())?;
    if combined.len() < 13 {
        return Err("Ciphertext too short".into());
    }

    let (nonce_bytes, ciphertext) = combined.split_at(12);
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| e.to_string())?;
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| e.to_string())?;

    String::from_utf8(plaintext).map_err(|e| e.to_string())
}
