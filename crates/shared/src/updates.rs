use serde::{Deserialize, Serialize};

/// A published installer. Panel URLs are relative; repository URLs are absolute.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LauncherUpdate {
    pub version: String,
    pub notes: String,
    pub url: String,
    pub size: u64,
    #[serde(default)]
    pub sha256: Option<String>,
    /// Ed25519 signature over [`signed_message`] by the Velora release key (base64).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

/// Public half of the Velora release-signing key. The private half is the
/// VELORA_RELEASE_SIGNING_KEY secret used by the launcher release workflow.
pub const RELEASE_SIGNING_KEY: &str = include_str!("../release-signing.pub");

/// What the release workflow signs for each installer: its version and SHA-256,
/// so a validly signed installer can't be advertised as a different release.
pub fn signed_message(version: &str, sha256: &str) -> String {
    format!("velora-launcher-release:v1\n{}\n{}", version.trim().trim_start_matches('v'), sha256.to_ascii_lowercase())
}

/// What the Velora Core workflow signs for each jar. A different domain from [`signed_message`], so a launcher
/// signature can never be replayed as a mod release or the reverse.
pub fn core_signed_message(version: &str, sha256: &str) -> String {
    format!("velora-core-release:v1\n{}\n{}", version.trim().trim_start_matches('v'), sha256.to_ascii_lowercase())
}

/// Verifies an installer signature against the embedded release key.
pub fn verify_release_signature(version: &str, sha256: &str, signature: &str) -> bool {
    verify_with_key(RELEASE_SIGNING_KEY, version, sha256, signature)
}

/// Verifies a Velora Core jar signature against the embedded release key.
pub fn verify_core_signature(version: &str, sha256: &str, signature: &str) -> bool {
    verify_message(RELEASE_SIGNING_KEY, &core_signed_message(version, sha256), sha256, signature)
}

fn verify_with_key(public_key: &str, version: &str, sha256: &str, signature: &str) -> bool {
    verify_message(public_key, &signed_message(version, sha256), sha256, signature)
}

fn verify_message(public_key: &str, message: &str, sha256: &str, signature: &str) -> bool {
    use base64::Engine;
    let engine = base64::engine::general_purpose::STANDARD;
    let (Ok(key), Ok(sig)) = (engine.decode(public_key.trim()), engine.decode(signature.trim())) else { return false };
    if key.len() != 32 || sig.len() != 64 || sha256.len() != 64 || !sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
        return false;
    }
    ring::signature::UnparsedPublicKey::new(&ring::signature::ED25519, key).verify(message.as_bytes(), &sig).is_ok()
}

pub fn release_version(value: &str) -> Option<semver::Version> {
    if value.len() > 128 {
        return None;
    }
    semver::Version::parse(value.trim().trim_start_matches('v')).ok()
}

pub fn newer_release(candidate: &str, current: &str) -> bool {
    match (release_version(candidate), release_version(current)) {
        (Some(next), Some(installed)) => next.cmp_precedence(&installed).is_gt(),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compares_release_precedence_without_build_metadata() {
        assert!(newer_release("v0.10.0", "0.9.9"));
        assert!(newer_release("1.0.0", "1.0.0-beta.2"));
        assert!(newer_release("1.0.0-beta.10", "1.0.0-beta.2"));
        assert!(!newer_release("1.0.0-beta.2", "1.0.0"));
        assert!(!newer_release("1.0.0+build.2", "1.0.0+build.1"));
        assert!(!newer_release("not-a-version", "0.9.0"));
    }

    #[test]
    fn release_signatures_bind_version_and_checksum() {
        use base64::Engine;
        use ring::signature::KeyPair;
        let engine = base64::engine::general_purpose::STANDARD;
        let pkcs8 = ring::signature::Ed25519KeyPair::generate_pkcs8(&ring::rand::SystemRandom::new()).unwrap();
        let pair = ring::signature::Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).unwrap();
        let public = engine.encode(pair.public_key().as_ref());
        let digest = "ab".repeat(32);
        let signature = engine.encode(pair.sign(signed_message("1.3.0", &digest).as_bytes()).as_ref());
        assert!(verify_with_key(&public, "v1.3.0", &digest.to_uppercase(), &signature));
        assert!(!verify_with_key(&public, "1.3.1", &digest, &signature), "version is bound");
        assert!(!verify_with_key(&public, "1.3.0", &"cd".repeat(32), &signature), "checksum is bound");
        assert!(!verify_with_key(&public, "1.3.0", &digest, "not base64"));
        assert!(!verify_release_signature("1.3.0", &digest, &signature), "only the Velora key is trusted");
        assert_eq!(engine.decode(RELEASE_SIGNING_KEY.trim()).unwrap().len(), 32);
    }

    #[test]
    fn core_and_launcher_signatures_are_not_interchangeable() {
        use base64::Engine;
        use ring::signature::KeyPair;
        let engine = base64::engine::general_purpose::STANDARD;
        let pkcs8 = ring::signature::Ed25519KeyPair::generate_pkcs8(&ring::rand::SystemRandom::new()).unwrap();
        let pair = ring::signature::Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).unwrap();
        let public = engine.encode(pair.public_key().as_ref());
        let digest = "ab".repeat(32);
        let core = engine.encode(pair.sign(core_signed_message("0.6.0", &digest).as_bytes()).as_ref());
        let launcher = engine.encode(pair.sign(signed_message("0.6.0", &digest).as_bytes()).as_ref());
        assert!(verify_message(&public, &core_signed_message("v0.6.0", &digest.to_uppercase()), &digest, &core));
        assert!(
            !verify_message(&public, &core_signed_message("0.6.0", &digest), &digest, &launcher),
            "a launcher signature is not a mod signature"
        );
        assert!(!verify_with_key(&public, "0.6.0", &digest, &core), "a mod signature is not a launcher signature");
        assert!(!verify_core_signature("0.6.0", &digest, &core), "only the Velora key is trusted");
    }
}
