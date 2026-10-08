//! The auth server's RSA key. It signs skin/cape data ("textures") and
//! player chat certificates; game clients and servers learn the public half
//! from the Yggdrasil metadata via authlib-injector.

use anyhow::{Context, Result};
use base64::Engine;
use rsa::pkcs1v15::SigningKey;
use rsa::pkcs8::{DecodePrivateKey, EncodePrivateKey, EncodePublicKey, LineEnding};
use rsa::signature::{SignatureEncoding, Signer};
use rsa::{RsaPrivateKey, RsaPublicKey};
use sha1::Sha1;
use std::path::Path;

pub const KEY_BITS: usize = 4096;

pub struct Keys {
    signer: SigningKey<Sha1>,
    /// `-----BEGIN PUBLIC KEY-----` (X.509 SubjectPublicKeyInfo).
    pub public_pem: String,
    /// DER of the same, base64 — the format of Mojang's `/publickeys`.
    pub public_der_b64: String,
}

impl Keys {
    /// Load an existing key without generating material on a missing path.
    pub fn load(path: &Path) -> Result<Self> {
        Self::from_pem(&std::fs::read_to_string(path).context("reading checkpoint RSA signing key")?)
    }
    pub fn from_private(key: RsaPrivateKey) -> Result<Self> {
        let public = RsaPublicKey::from(&key);
        let public_pem = public.to_public_key_pem(LineEnding::LF)?;
        let public_der_b64 = base64::engine::general_purpose::STANDARD.encode(public.to_public_key_der()?.as_bytes());
        Ok(Self { signer: SigningKey::<Sha1>::new(key), public_pem, public_der_b64 })
    }

    /// Load `path`, or generate and save a new key if it doesn't exist.
    pub fn load_or_create(path: &Path) -> Result<Self> {
        match std::fs::read_to_string(path) {
            Ok(pem) => return Self::from_pem(&pem),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error).context("reading existing RSA signing key; automatic replacement is refused"),
        }
        tracing::info!("generating the auth server signing key ({KEY_BITS}-bit RSA, one-time)…");
        let key = RsaPrivateKey::new(&mut rand::thread_rng(), KEY_BITS)?;
        match crate::secrets::write_new(path, key.to_pkcs8_pem(LineEnding::LF)?.as_bytes()) {
            Ok(()) => Self::from_private(key),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                Self::from_pem(&std::fs::read_to_string(path).context("reading concurrently created RSA signing key")?)
            }
            Err(error) => Err(error).context("creating RSA signing key; inspect any incomplete file before retrying"),
        }
    }

    fn from_pem(pem: &str) -> Result<Self> {
        Self::from_private(
            RsaPrivateKey::from_pkcs8_pem(pem).context("parsing existing RSA signing key; restore it rather than rotating it")?,
        )
    }

    /// SHA1withRSA, base64 — what Mojang uses for every Yggdrasil signature.
    pub fn sign_b64(&self, data: &[u8]) -> String {
        base64::engine::general_purpose::STANDARD.encode(self.signer.sign(data).to_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsa::pkcs1v15::{Signature, VerifyingKey};
    use rsa::pkcs8::DecodePublicKey;
    use rsa::signature::Verifier;

    #[test]
    fn signs_verifiably() {
        let key = RsaPrivateKey::new(&mut rand::thread_rng(), 1024).unwrap();
        let keys = Keys::from_private(key).unwrap();
        let sig = base64::engine::general_purpose::STANDARD.decode(keys.sign_b64(b"hello")).unwrap();
        let public = RsaPublicKey::from_public_key_pem(&keys.public_pem).unwrap();
        VerifyingKey::<Sha1>::new(public).verify(b"hello", &Signature::try_from(sig.as_slice()).unwrap()).unwrap();
        assert!(keys.public_pem.starts_with("-----BEGIN PUBLIC KEY-----"));
    }
    #[test]
    fn persisted_rsa_key_bytes_and_signatures_survive_restart() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("signing.pem");
        let private = RsaPrivateKey::new(&mut rand::thread_rng(), 1024).unwrap();
        let original = private.to_pkcs8_pem(LineEnding::LF).unwrap();
        std::fs::write(&path, original.as_bytes()).unwrap();
        let first = Keys::load_or_create(&path).unwrap();
        let second = Keys::load_or_create(&path).unwrap();
        assert_eq!(first.public_pem, second.public_pem);
        assert_eq!(first.sign_b64(b"synthetic continuity"), second.sign_b64(b"synthetic continuity"));
        assert_eq!(std::fs::read(path).unwrap(), original.as_bytes());
    }
    #[test]
    fn invalid_or_unreadable_existing_rsa_material_is_never_replaced() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("signing.pem");
        for original in [b"synthetic-invalid-pem".as_slice(), b"\xff\xfe".as_slice()] {
            std::fs::write(&path, original).unwrap();
            assert!(Keys::load_or_create(&path).is_err());
            assert_eq!(std::fs::read(&path).unwrap(), original);
        }
        assert!(Keys::load_or_create(directory.path()).is_err());
        assert!(directory.path().is_dir());
    }
    #[test]
    fn checkpoint_loading_never_creates_a_missing_key() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("missing.pem");
        assert!(Keys::load(&path).is_err());
        assert!(!path.exists());
    }
}
