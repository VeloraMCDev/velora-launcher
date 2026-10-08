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
    pub fn from_private(key: RsaPrivateKey) -> Result<Self> {
        let public = RsaPublicKey::from(&key);
        let public_pem = public.to_public_key_pem(LineEnding::LF)?;
        let public_der_b64 = base64::engine::general_purpose::STANDARD.encode(public.to_public_key_der()?.as_bytes());
        Ok(Self { signer: SigningKey::<Sha1>::new(key), public_pem, public_der_b64 })
    }

    /// Load `path`, or generate and save a new key if it doesn't exist.
    pub fn load_or_create(path: &Path) -> Result<Self> {
        if let Ok(pem) = std::fs::read_to_string(path) {
            let key = RsaPrivateKey::from_pkcs8_pem(&pem).with_context(|| format!("reading {}", path.display()))?;
            return Self::from_private(key);
        }
        tracing::info!("generating the auth server signing key ({KEY_BITS}-bit RSA, one-time)…");
        let key = RsaPrivateKey::new(&mut rand::thread_rng(), KEY_BITS)?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, key.to_pkcs8_pem(LineEnding::LF)?.as_bytes())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).ok();
        }
        Self::from_private(key)
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
}
