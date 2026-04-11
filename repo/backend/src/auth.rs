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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_and_verify_roundtrip() {
        let password = "correct-horse-battery-staple";
        let hash = hash_password(password).expect("hashing should succeed");
        assert!(verify_password(password, &hash));
    }

    #[test]
    fn test_wrong_password_fails() {
        let hash = hash_password("secret").expect("hashing should succeed");
        assert!(!verify_password("wrong", &hash));
    }

    #[test]
    fn test_invalid_hash_returns_false() {
        assert!(!verify_password("anything", "not-a-valid-hash"));
    }

    #[test]
    fn test_token_create_and_validate_roundtrip() {
        let secret = b"test-secret-key-1234";
        let token = create_session_token("user42", secret, 3600);
        let result = validate_session_token(&token, secret);
        assert_eq!(result, Some("user42".to_string()));
    }

    #[test]
    fn test_token_wrong_secret_fails() {
        let token = create_session_token("user42", b"secret-a", 3600);
        let result = validate_session_token(&token, b"secret-b");
        assert_eq!(result, None);
    }

    #[test]
    fn test_tampered_token_fails() {
        let secret = b"my-secret";
        let token = create_session_token("user1", secret, 3600);
        // Tamper with the signature portion
        let tampered = format!("{}x", token);
        assert_eq!(validate_session_token(&tampered, secret), None);
    }

    #[test]
    fn test_token_invalid_format_returns_none() {
        let secret = b"key";
        assert_eq!(validate_session_token("no-dot-here", secret), None);
        assert_eq!(validate_session_token("", secret), None);
        assert_eq!(validate_session_token("a.b.c", secret), None);
    }

    #[test]
    fn test_nonce_is_32_char_hex() {
        let nonce = generate_nonce();
        assert_eq!(nonce.len(), 32);
        assert!(nonce.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_two_nonces_differ() {
        let a = generate_nonce();
        let b = generate_nonce();
        assert_ne!(a, b);
    }
}
