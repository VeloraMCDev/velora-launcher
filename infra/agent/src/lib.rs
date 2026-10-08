use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
pub mod probe_executor;
pub mod probe_job;
pub mod probe_transport;
use ed25519_dalek::{Signer, SigningKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};
#[cfg(any(unix, test))]
use zeroize::Zeroize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub schema: u32,
    pub control_origin: String,
    pub environment: String,
    pub identity_path: PathBuf,
    pub heartbeat_seconds: u64,
}

impl Config {
    pub fn load(path: &std::path::Path) -> Result<Self, &'static str> {
        if fs::metadata(path).map_err(|_| "Configuration unavailable")?.len() > 4096 {
            return Err("Configuration too large");
        }
        let contents = fs::read_to_string(path).map_err(|_| "Configuration unavailable")?;
        let config: Self = toml::from_str(&contents).map_err(|_| "Invalid configuration")?;
        config.validate()?;
        Ok(config)
    }
    pub fn validate(&self) -> Result<(), &'static str> {
        let origin = reqwest::Url::parse(&self.control_origin).map_err(|_| "Invalid control origin")?;
        if self.schema != 1
            || self.environment != "development"
            || origin.scheme() != "https"
            || origin.host_str().is_none()
            || !origin.username().is_empty()
            || origin.password().is_some()
            || origin.query().is_some()
            || origin.fragment().is_some()
            || origin.path() != "/"
            || !self.identity_path.is_absolute()
            || self.identity_path.components().any(|part| matches!(part, std::path::Component::ParentDir))
            || !(30..=60).contains(&self.heartbeat_seconds)
        {
            return Err("Configuration violates Development heartbeat policy");
        }
        Ok(())
    }
}

pub struct Identity {
    key: SigningKey,
}
impl Identity {
    #[cfg(any(unix, test))]
    fn from_seed(mut seed: [u8; 32]) -> Self {
        let key = SigningKey::from_bytes(&seed);
        seed.zeroize();
        Self { key }
    }
    pub fn public_key(&self) -> String {
        URL_SAFE_NO_PAD.encode(self.key.verifying_key().to_bytes())
    }
    pub fn key_id(&self) -> String {
        hex::encode(Sha256::digest(self.key.verifying_key().to_bytes()))
    }
    pub fn agent_id(&self) -> String {
        format!("agt_{}", self.key_id())
    }
    pub fn sign_request(&self, path: &str, timestamp: &str, nonce: &str, body: &[u8]) -> Result<SignedHeaders, &'static str> {
        let checksum = hex::encode(Sha256::digest(body));
        let canonical = canonical_request(path, timestamp, nonce, &checksum)?;
        Ok(SignedHeaders {
            agent_id: self.agent_id(),
            key_id: self.key_id(),
            timestamp: timestamp.into(),
            nonce: nonce.into(),
            body_sha256: checksum,
            signature: URL_SAFE_NO_PAD.encode(self.key.sign(canonical.as_bytes()).to_bytes()),
        })
    }
    #[cfg(unix)]
    pub fn load(path: &std::path::Path) -> Result<Self, &'static str> {
        use std::{
            io::Read,
            os::unix::fs::{OpenOptionsExt, PermissionsExt},
        };
        let mut file = fs::OpenOptions::new().read(true).custom_flags(libc::O_NOFOLLOW).open(path).map_err(|_| "Identity unavailable")?;
        let meta = file.metadata().map_err(|_| "Identity metadata unavailable")?;
        if !meta.is_file() || meta.len() != 32 || meta.permissions().mode() & 0o077 != 0 {
            return Err("Identity must be a private 32-byte regular file");
        }
        let mut seed = zeroize::Zeroizing::new([0u8; 32]);
        file.read_exact(seed.as_mut()).map_err(|_| "Identity unavailable")?;
        Ok(Self::from_seed(*seed))
    }
    #[cfg(not(unix))]
    pub fn load(_: &std::path::Path) -> Result<Self, &'static str> {
        Err("Native identity storage requires Linux")
    }
    #[cfg(unix)]
    pub fn create(path: &std::path::Path) -> Result<(), &'static str> {
        use std::{io::Write, os::unix::fs::OpenOptionsExt};
        let mut seed = [0u8; 32];
        getrandom::fill(&mut seed).map_err(|_| "Secure random source unavailable")?;
        let result = (|| {
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .custom_flags(libc::O_NOFOLLOW)
                .open(path)
                .map_err(|_| "Identity exists or protected directory unavailable")?;
            file.write_all(&seed).map_err(|_| "Identity write failed")?;
            file.sync_all().map_err(|_| "Identity sync failed")
        })();
        seed.zeroize();
        result
    }
    #[cfg(not(unix))]
    pub fn create(_: &std::path::Path) -> Result<(), &'static str> {
        Err("Native identity storage requires Linux")
    }
}

#[derive(Serialize)]
pub struct SignedHeaders {
    pub agent_id: String,
    pub key_id: String,
    pub timestamp: String,
    pub nonce: String,
    pub body_sha256: String,
    pub signature: String,
}

pub fn valid_nonce(nonce: &str) -> bool {
    let bytes = nonce.as_bytes();
    bytes.len() == 36
        && bytes.iter().enumerate().all(|(index, byte)| {
            if [8, 13, 18, 23].contains(&index) {
                *byte == b'-'
            } else {
                byte.is_ascii_digit() || (b'a'..=b'f').contains(byte)
            }
        })
        && bytes[14] == b'4'
        && b"89ab".contains(&bytes[19])
}
pub fn canonical_request(path: &str, timestamp: &str, nonce: &str, checksum: &str) -> Result<String, &'static str> {
    if !["/api/v1/agents/enroll", "/api/v1/agents/heartbeat", "/api/v1/agents/probe/poll", "/api/v1/agents/probe/result"].contains(&path)
        || !(10..=11).contains(&timestamp.len())
        || timestamp.starts_with('0')
        || !timestamp.bytes().all(|byte| byte.is_ascii_digit())
        || !valid_nonce(nonce)
        || checksum.len() != 64
        || !checksum.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err("Invalid signed request fields");
    }
    Ok(format!("POST\n{path}\n\n{timestamp}\n{nonce}\n{checksum}"))
}
pub fn new_nonce() -> Result<String, &'static str> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).map_err(|_| "Secure random source unavailable")?;
    bytes[6] = (bytes[6] & 15) | 64;
    bytes[8] = (bytes[8] & 63) | 128;
    let raw = hex::encode(bytes);
    Ok(format!("{}-{}-{}-{}-{}", &raw[..8], &raw[8..12], &raw[12..16], &raw[16..20], &raw[20..]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signature, Verifier};
    #[test]
    fn signature_binds_exact_wire_fields_and_tampering_fails() {
        let identity = Identity::from_seed([7u8; 32]);
        let body = br#"{"schema":1}"#;
        let headers =
            identity.sign_request("/api/v1/agents/heartbeat", "1791400000", "11111111-1111-4111-8111-111111111111", body).unwrap();
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../../deployment/contracts/agent-request.fixture.json")).unwrap();
        assert_eq!(headers.signature, fixture["signature"].as_str().unwrap());
        assert_eq!(identity.public_key(), fixture["public_key"].as_str().unwrap());
        assert_eq!(headers.key_id, fixture["key_id"].as_str().unwrap());
        let signature = Signature::from_slice(&URL_SAFE_NO_PAD.decode(&headers.signature).unwrap()).unwrap();
        let canonical = canonical_request("/api/v1/agents/heartbeat", &headers.timestamp, &headers.nonce, &headers.body_sha256).unwrap();
        identity.key.verifying_key().verify(canonical.as_bytes(), &signature).unwrap();
        assert!(identity.key.verifying_key().verify(canonical.replace("heartbeat", "enroll").as_bytes(), &signature).is_err());
        assert!(identity.sign_request("/api/v1/agents/shell", &headers.timestamp, &headers.nonce, body).is_err());
        assert!(!valid_nonce("11111111-1111-1111-8111-111111111111"));
        assert!(valid_nonce(&new_nonce().unwrap()));
    }
    #[test]
    fn config_rejects_credentials_query_other_environments_and_unbounded_polling() {
        let mut config = Config {
            schema: 1,
            control_origin: "https://agent.example.com".into(),
            environment: "development".into(),
            identity_path: std::env::temp_dir().join("synthetic-identity"),
            heartbeat_seconds: 45,
        };
        config.validate().unwrap();
        for url in [
            "http://agent.example.com",
            "https://operator:password@agent.example.com",
            "https://agent.example.com?token=value",
            "https://agent.example.com/api",
        ] {
            config.control_origin = url.into();
            assert!(config.validate().is_err());
        }
        config.control_origin = "https://agent.example.com".into();
        config.environment = "production".into();
        assert!(config.validate().is_err());
        config.environment = "development".into();
        config.heartbeat_seconds = 1;
        assert!(config.validate().is_err());
    }
    #[cfg(unix)]
    #[test]
    fn native_identity_rejects_symlinks_public_permissions_and_overwrites() {
        use std::{
            io::Write,
            os::unix::fs::{symlink, OpenOptionsExt, PermissionsExt},
        };
        let directory = std::env::temp_dir().join(format!("velora-synthetic-key-{}", new_nonce().unwrap()));
        fs::create_dir(&directory).unwrap();
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
        let path = directory.join("identity.key");
        fs::OpenOptions::new().create_new(true).write(true).mode(0o600).open(&path).unwrap().write_all(&[7u8; 32]).unwrap();
        assert!(Identity::load(&path).is_ok());
        assert!(Identity::create(&path).is_err());
        let link = directory.join("identity.link");
        symlink(&path, &link).unwrap();
        assert!(Identity::load(&link).is_err());
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(Identity::load(&path).is_err());
        fs::remove_file(link).unwrap();
        fs::remove_file(path).unwrap();
        fs::remove_dir(directory).unwrap();
    }
}
