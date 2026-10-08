//! Existing Argon2 password behavior, independent of HTTP and persistence.
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;

pub fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|error| format!("hashing failed: {error}"))
}
pub fn verify_password(password: &str, hash: &str) -> bool {
    PasswordHash::new(hash).map(|hash| Argon2::default().verify_password(password.as_bytes(), &hash).is_ok()).unwrap_or(false)
}
pub fn validate_password(password: &str) -> Result<(), &'static str> {
    if password.chars().count() < 8 {
        Err("passwords need at least 8 characters")
    } else {
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn verifies_existing_argon2_encoding_and_rejects_wrong_passwords() {
        let hash = hash_password("old-password-123").unwrap();
        assert!(hash.starts_with("$argon2id$v=19$m=19456,t=2,p=1$"));
        assert!(verify_password("old-password-123", &hash));
        assert!(!verify_password("wrong-password", &hash));
        assert!(!verify_password("old-password-123", "malformed"));
    }
    #[test]
    fn password_length_counts_unicode_characters() {
        assert!(validate_password("ééééééé").is_err());
        assert!(validate_password("éééééééé").is_ok());
    }
}
