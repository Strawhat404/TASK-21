use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};

type HmacSha256 = Hmac<Sha256>;

/// Hash a password with argon2 + random salt.
pub fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| e.to_string())
}

/// Verify a password against an argon2 hash.
pub fn verify_password(password: &str, hash: &str) -> bool {
    let parsed = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

/// Create a signed session token: base64(user_id|expires_epoch)|signature
pub fn create_session_token(user_id: &str, secret: &[u8], ttl_secs: u64) -> String {
    let expires = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + ttl_secs;
    let payload = format!("{}|{}", user_id, expires);
    let encoded = URL_SAFE_NO_PAD.encode(payload.as_bytes());

    let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC key");
    mac.update(encoded.as_bytes());
    let sig = hex::encode(mac.finalize().into_bytes());

    format!("{}.{}", encoded, sig)
}

/// Validate a signed session token, returning the user_id if valid.
pub fn validate_session_token(token: &str, secret: &[u8]) -> Option<String> {
    let parts: Vec<&str> = token.splitn(2, '.').collect();
    if parts.len() != 2 {
        return None;
    }
    let (encoded, sig) = (parts[0], parts[1]);

    // Verify signature
    let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC key");
    mac.update(encoded.as_bytes());
    let expected_sig = hex::encode(mac.finalize().into_bytes());
    if sig != expected_sig {
        return None;
    }

    // Decode payload
    let decoded = URL_SAFE_NO_PAD.decode(encoded).ok()?;
    let payload = String::from_utf8(decoded).ok()?;
    let parts: Vec<&str> = payload.splitn(2, '|').collect();
    if parts.len() != 2 {
        return None;
    }

    let expires: u64 = parts[1].parse().ok()?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    if now > expires {
        return None; // Token expired
    }

    Some(parts[0].to_string())
}

/// Generate a short-lived nonce (random hex string).
pub fn generate_nonce() -> String {
    use rand::Rng;
    let bytes: [u8; 16] = rand::thread_rng().gen();
    hex::encode(bytes)
}
