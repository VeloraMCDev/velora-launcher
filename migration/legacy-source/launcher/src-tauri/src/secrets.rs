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
    key: [u8; 32],
    data: Mutex<HashMap<String, Secret>>,
}

const SERVICE: &str = "net.scopenet.launcher";

fn load_or_create_key(dir: &std::path::Path) -> [u8; 32] {
    let b64 = base64::engine::general_purpose::STANDARD;
    // Unit tests use their own temporary key file; the OS keychain can prompt
    // for desktop access and block unattended checks.
    #[cfg(all(not(test), any(windows, target_os = "macos")))]
    {
        if let Ok(entry) = keyring::Entry::new(SERVICE, "secrets-key") {
            if let Ok(existing) = entry.get_password() {
                if let Ok(bytes) = b64.decode(existing) {
                    if let Ok(key) = <[u8; 32]>::try_from(bytes.as_slice()) {
                        return key;
                    }
                }
            }
            let mut key = [0u8; 32];
            rand::thread_rng().fill_bytes(&mut key);
            if entry.set_password(&b64.encode(key)).is_ok() {
                return key;
            }
            tracing::warn!("credential store unavailable, falling back to a key file");
        }
    }
    let _ = SERVICE;
    let path = dir.join("secrets.key");
    if let Ok(bytes) = std::fs::read(&path).map(|b| b64.decode(b).unwrap_or_default()) {
        if let Ok(key) = <[u8; 32]>::try_from(bytes.as_slice()) {
            return key;
        }
    }
    let mut key = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut key);
    std::fs::write(&path, b64.encode(key)).ok();
    key
}

impl Secrets {
    pub fn open(dir: &std::path::Path) -> Self {
        let key = load_or_create_key(dir);
        let path = dir.join("secrets.bin");
        let data = std::fs::read(&path).ok().and_then(|raw| decrypt(&key, &raw).ok()).unwrap_or_default();
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
        let cipher = ChaCha20Poly1305::new(Key::from_slice(&self.key));
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
    fn roundtrip() {
        let dir = std::env::temp_dir().join(format!("scopenet-secrets-{}", uuid::Uuid::new_v4()));
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
