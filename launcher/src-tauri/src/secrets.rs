//! Encrypted storage for tokens (panel sessions and game sessions).
//!
//! Secrets live in `secrets.bin`, encrypted with ChaCha20-Poly1305. The key
//! is kept in the OS credential store (Windows Credential Manager / macOS
//! Keychain), so copying the data folder to another machine doesn't leak
//! sessions. Where no credential store exists we fall back to a key file.

use anyhow::{anyhow, Result};
use base64::Engine;
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Secret {
    /// Panel API session (manifest, account/skin API).
    pub panel_token: Option<String>,
    /// Game session from the panel's Yggdrasil server.
    pub ygg_access_token: Option<String>,
    pub ygg_client_token: Option<String>,
}

pub struct Secrets {
    path: PathBuf,
    key: Option<[u8; 32]>,
    data: Mutex<HashMap<String, Secret>>,
}

const SERVICE: &str = "net.scopenet.launcher";

fn decode_key(encoded: &str) -> Result<[u8; 32]> {
    let bytes = base64::engine::general_purpose::STANDARD.decode(encoded)?;
    bytes.as_slice().try_into().map_err(|_| anyhow!("saved encryption key has an invalid length"))
}

#[cfg(any(test, windows, target_os = "macos"))]
fn credential_key(read: Result<Option<String>>, write: impl FnOnce(&str) -> Result<()>) -> Result<[u8; 32]> {
    match read {
        Ok(Some(existing)) => decode_key(&existing),
        Ok(None) => {
            let mut key = [0u8; 32];
            rand::thread_rng().fill_bytes(&mut key);
            write(&base64::engine::general_purpose::STANDARD.encode(key))?;
            Ok(key)
        }
        // Denial/unavailability is not an absent entry: never overwrite its key.
        Err(error) => Err(error.into()),
    }
}

fn load_or_create_key(dir: &std::path::Path) -> Result<[u8; 32]> {
    let path = dir.join("secrets.key");
    // Reuse the existing fallback identity without repeatedly requesting a different OS key.
    if path.exists() {
        let key = decode_key(&std::fs::read_to_string(&path)?)?;
        let encrypted = dir.join("secrets.bin");
        if !encrypted.exists() || decrypt(&key, &std::fs::read(encrypted)?).is_ok() {
            return Ok(key);
        }
        // An older fallback can coexist with a newer OS-protected store.
        #[cfg(any(test, not(any(windows, target_os = "macos"))))]
        return Err(anyhow!("existing fallback cannot unlock saved sign-ins"));
    }
    // Unit tests use their own temporary key file; the OS keychain can prompt
    // for desktop access and block unattended checks.
    #[cfg(all(not(test), any(windows, target_os = "macos")))]
    {
        let entry = keyring::Entry::new(SERVICE, "secrets-key")?;
        let read = match entry.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(error.into()),
        };
        if matches!(read, Ok(None)) && dir.join("secrets.bin").exists() {
            return Err(anyhow!("saved credential key is missing; existing secrets were preserved"));
        }
        return credential_key(read, |value| entry.set_password(value).map_err(Into::into));
    }
    #[cfg(any(test, not(any(windows, target_os = "macos"))))]
    {
        let b64 = base64::engine::general_purpose::STANDARD;
        let _ = SERVICE;
        if dir.join("secrets.bin").exists() {
            return Err(anyhow!("saved credential key is missing; existing secrets were preserved"));
        }
        let mut key = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut key);
        use std::io::Write;
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        options.open(path)?.write_all(b64.encode(key).as_bytes())?;
        Ok(key)
    }
}

impl Secrets {
    pub fn open(dir: &std::path::Path) -> Self {
        let mut key = load_or_create_key(dir).map_err(|error| tracing::warn!("saved sign-ins unavailable: {error}")).ok();
        let path = dir.join("secrets.bin");
        let data = match std::fs::read(&path) {
            Ok(raw) => match key.as_ref().and_then(|key| decrypt(key, &raw).ok()) {
                Some(data) => data,
                None => {
                    key = None;
                    HashMap::new()
                }
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => HashMap::new(),
            Err(_) => {
                key = None;
                HashMap::new()
            }
        };
        Self { path, key, data: Mutex::new(data) }
    }

    pub fn get(&self, id: &str) -> Secret {
        self.data.lock().unwrap().get(id).cloned().unwrap_or_default()
    }

    pub fn set(&self, id: &str, secret: Secret) -> Result<()> {
        let mut data = self.data.lock().unwrap();
        data.insert(id.to_string(), secret);
        self.flush(&data)
    }

    pub fn remove(&self, id: &str) -> Result<()> {
        let mut data = self.data.lock().unwrap();
        data.remove(id);
        self.flush(&data)
    }

    fn flush(&self, data: &HashMap<String, Secret>) -> Result<()> {
        let plain = serde_json::to_vec(data)?;
        let key = self.key.as_ref().ok_or_else(|| {
            anyhow!("Saved sign-ins are locked. Allow Velora access in the system credential store and restart the launcher.")
        })?;
        let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
        let mut nonce = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce);
        let mut out = nonce.to_vec();
        out.extend(cipher.encrypt(Nonce::from_slice(&nonce), plain.as_slice()).map_err(|_| anyhow!("encryption failed"))?);
        let tmp = self.path.with_extension("bin.tmp");
        std::fs::write(&tmp, out)?;
        std::fs::rename(tmp, &self.path)?;
        Ok(())
    }
}

fn decrypt(key: &[u8; 32], raw: &[u8]) -> Result<HashMap<String, Secret>> {
    if raw.len() < 13 {
        return Err(anyhow!("secrets file too short"));
    }
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
    let plain = cipher.decrypt(Nonce::from_slice(&raw[..12]), &raw[12..]).map_err(|_| anyhow!("can't decrypt secrets"))?;
    Ok(serde_json::from_slice(&plain)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credential_denial_or_invalid_key_never_attempts_a_replacement() {
        for read in [Err(anyhow!("access denied")), Ok(Some("invalid key".into()))] {
            assert!(credential_key(read, |_| panic!("must not write after denial or corruption")).is_err());
        }
        let expected = [7; 32];
        let encoded = base64::engine::general_purpose::STANDARD.encode(expected);
        assert_eq!(credential_key(Ok(Some(encoded)), |_| panic!("existing identity must not be overwritten")).unwrap(), expected);
        let writes = std::cell::Cell::new(0);
        credential_key(Ok(None), |_| {
            writes.set(writes.get() + 1);
            Ok(())
        })
        .unwrap();
        assert_eq!(writes.get(), 1);
    }

    #[test]
    fn unavailable_or_wrong_key_preserves_encrypted_credentials() {
        let dir = std::env::temp_dir().join(format!("velora-locked-secrets-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("secrets.bin");
        let original = b"previous encrypted credentials";
        std::fs::write(&path, original).unwrap();
        let secrets = Secrets::open(&dir);
        assert!(secrets.set("new", Secret::default()).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), original);
        assert!(!dir.join("secrets.key").exists());
        std::fs::write(dir.join("secrets.key"), base64::engine::general_purpose::STANDARD.encode([8; 32])).unwrap();
        assert!(Secrets::open(&dir).set("new", Secret::default()).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), original);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn roundtrip() {
        let dir = std::env::temp_dir().join(format!("velora-secrets-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let s = Secrets::open(&dir);
        s.set("a", Secret { panel_token: Some("tok".into()), ..Default::default() }).unwrap();
        let raw = std::fs::read(dir.join("secrets.bin")).unwrap();
        assert!(!String::from_utf8_lossy(&raw).contains("tok"), "stored encrypted");
        let again = Secrets::open(&dir);
        assert_eq!(again.get("a").panel_token.as_deref(), Some("tok"));
        std::fs::remove_dir_all(dir).ok();
    }
}
