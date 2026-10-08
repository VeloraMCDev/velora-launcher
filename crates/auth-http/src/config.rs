//! Standalone authority configuration. The legacy host retains its existing configuration adapter.
use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};
use velora_platform_contracts::{valid_username, RegistrationMode};

#[derive(Debug, Clone)]
pub struct BootstrapConfig {
    pub username: String,
    pub password_file: PathBuf,
}
// Deliberately no Debug/Serialize implementation for loaded credentials.
pub struct BootstrapCredentials {
    pub username: String,
    pub password: String,
}
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub bind: SocketAddr,
    pub data_dir: PathBuf,
    pub database: PathBuf,
    pub rsa_key: PathBuf,
    pub textures: PathBuf,
    pub persisted_jwt: PathBuf,
    pub configured_jwt_file: Option<PathBuf>,
    pub public_origin: String,
    pub trusted_proxies: Vec<IpAddr>,
    pub panel_accounts: bool,
    pub registration: RegistrationMode,
    pub bootstrap: Option<BootstrapConfig>,
}

fn read(lookup: &mut impl FnMut(&str) -> Option<String>, name: &str) -> Option<String> {
    lookup(name).map(|value| value.trim().to_owned()).filter(|value| !value.is_empty())
}
fn boolean(value: Option<String>, name: &str, default: bool) -> anyhow::Result<bool> {
    match value.as_deref() {
        None => Ok(default),
        Some("true") => Ok(true),
        Some("false") => Ok(false),
        _ => anyhow::bail!("{name} must be true or false"),
    }
}
fn storage_identity(path: &Path) -> anyhow::Result<PathBuf> {
    let absolute = if path.is_absolute() { path.to_path_buf() } else { std::env::current_dir()?.join(path) };
    if let Ok(existing) = absolute.canonicalize() {
        return Ok(existing);
    }
    if let (Some(parent), Some(name)) = (absolute.parent(), absolute.file_name()) {
        if let Ok(parent) = parent.canonicalize() {
            return Ok(parent.join(name));
        }
    }
    Ok(absolute)
}
impl RuntimeConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        Self::from_lookup(|name| std::env::var(name).ok())
    }
    /// Lookup injection keeps tests/config composition independent of process-global environment.
    pub fn from_lookup(mut lookup: impl FnMut(&str) -> Option<String>) -> anyhow::Result<Self> {
        use anyhow::Context;
        let origin = read(&mut lookup, "VELORA_AUTH_PUBLIC_ORIGIN")
            .context("VELORA_AUTH_PUBLIC_ORIGIN is required for signed textures and callbacks")?;
        let origin = url::Url::parse(&origin).context("VELORA_AUTH_PUBLIC_ORIGIN must be an absolute HTTP(S) URL")?;
        if !matches!(origin.scheme(), "http" | "https")
            || origin.host_str().is_none()
            || !origin.username().is_empty()
            || origin.password().is_some()
            || origin.query().is_some()
            || origin.fragment().is_some()
        {
            anyhow::bail!("VELORA_AUTH_PUBLIC_ORIGIN must be HTTP(S), with a host and no credentials, query or fragment");
        }
        let bind = read(&mut lookup, "VELORA_AUTH_BIND")
            .unwrap_or_else(|| "127.0.0.1:8081".into())
            .parse()
            .context("VELORA_AUTH_BIND must be an IP address and port")?;
        let data_dir = PathBuf::from(read(&mut lookup, "VELORA_AUTH_DATA_DIR").unwrap_or_else(|| "./data".into()));
        let database = read(&mut lookup, "VELORA_AUTH_DATABASE_PATH").map(PathBuf::from).unwrap_or_else(|| data_dir.join("auth.sqlite"));
        let rsa_key =
            read(&mut lookup, "VELORA_AUTH_RSA_KEY_PATH").map(PathBuf::from).unwrap_or_else(|| data_dir.join("yggdrasil-signing.pem"));
        let textures = read(&mut lookup, "VELORA_AUTH_TEXTURES_PATH").map(PathBuf::from).unwrap_or_else(|| data_dir.join("textures"));
        let persisted_jwt = read(&mut lookup, "VELORA_AUTH_JWT_PATH").map(PathBuf::from).unwrap_or_else(|| data_dir.join("jwt.secret"));
        let configured_jwt_file = read(&mut lookup, "VELORA_AUTH_JWT_SECRET_FILE").map(PathBuf::from);
        let database_identity = storage_identity(&database)?;
        let rsa_identity = storage_identity(&rsa_key)?;
        let jwt_identity = storage_identity(&persisted_jwt)?;
        if database_identity == rsa_identity || database_identity == jwt_identity || rsa_identity == jwt_identity {
            anyhow::bail!("Authentication database, RSA key and persisted JWT paths must be distinct");
        }
        if let Some(path) = &configured_jwt_file {
            let identity = storage_identity(path)?;
            if identity == database_identity || identity == rsa_identity {
                anyhow::bail!("configured JWT file must be distinct from the database and RSA key");
            }
        }
        let trusted_proxies = read(&mut lookup, "VELORA_AUTH_TRUSTED_PROXIES")
            .map(|value| value.split(',').map(|address| address.trim().parse()).collect::<Result<Vec<IpAddr>, _>>())
            .transpose()
            .context("VELORA_AUTH_TRUSTED_PROXIES must contain comma-separated IP addresses")?
            .unwrap_or_default();
        let panel_accounts = boolean(read(&mut lookup, "VELORA_AUTH_PANEL_ACCOUNTS"), "VELORA_AUTH_PANEL_ACCOUNTS", true)?;
        let registration = match read(&mut lookup, "VELORA_AUTH_REGISTRATION").as_deref() {
            None | Some("closed") => RegistrationMode::Closed,
            Some("open") => RegistrationMode::Open,
            Some("approval") => RegistrationMode::Approval,
            _ => anyhow::bail!("VELORA_AUTH_REGISTRATION must be closed, open or approval"),
        };
        let username = read(&mut lookup, "VELORA_AUTH_BOOTSTRAP_USERNAME");
        let password_file = read(&mut lookup, "VELORA_AUTH_BOOTSTRAP_PASSWORD_FILE");
        let bootstrap = match (username, password_file) {
            (None, None) => None,
            (Some(username), Some(path)) => {
                if !valid_username(&username) {
                    anyhow::bail!("VELORA_AUTH_BOOTSTRAP_USERNAME must contain 3-16 letters, numbers or underscores");
                }
                Some(BootstrapConfig { username, password_file: path.into() })
            }
            _ => anyhow::bail!("configure both VELORA_AUTH_BOOTSTRAP_USERNAME and VELORA_AUTH_BOOTSTRAP_PASSWORD_FILE, or neither"),
        };
        Ok(Self {
            bind,
            data_dir,
            database,
            rsa_key,
            textures,
            persisted_jwt,
            configured_jwt_file,
            public_origin: origin.to_string().trim_end_matches('/').to_owned(),
            trusted_proxies,
            panel_accounts,
            registration,
            bootstrap,
        })
    }
    /// Configured legacy bytes are exact; otherwise preserve/load the persisted secret.
    pub fn jwt_secret(&self) -> anyhow::Result<Vec<u8>> {
        use anyhow::Context;
        let configured = self
            .configured_jwt_file
            .as_ref()
            .map(|path| std::fs::read(path).context("reading VELORA_AUTH_JWT_SECRET_FILE; no persisted-secret fallback is attempted"))
            .transpose()?;
        if configured.as_ref().is_some_and(Vec::is_empty) {
            anyhow::bail!("VELORA_AUTH_JWT_SECRET_FILE must not be empty");
        }
        velora_auth_core::secrets::jwt_secret(configured.as_deref(), &self.persisted_jwt)
    }
    /// Restart composition skips credential-file access if any administrator already exists.
    pub fn bootstrap_credentials(&self, has_admin: bool) -> anyhow::Result<Option<BootstrapCredentials>> {
        use anyhow::Context;
        if has_admin {
            return Ok(None);
        }
        let Some(configuration) = &self.bootstrap else {
            return Ok(None);
        };
        let password =
            std::fs::read_to_string(&configuration.password_file).context("reading VELORA_AUTH_BOOTSTRAP_PASSWORD_FILE as UTF-8")?;
        velora_auth_core::password::validate_password(&password).map_err(anyhow::Error::msg)?;
        Ok(Some(BootstrapCredentials { username: configuration.username.clone(), password }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn config(pairs: &[(&str, String)]) -> anyhow::Result<RuntimeConfig> {
        RuntimeConfig::from_lookup(|name| {
            if name == "VELORA_AUTH_PUBLIC_ORIGIN" {
                Some("https://example.invalid/platform/".into())
            } else {
                pairs.iter().find(|(key, _)| *key == name).map(|(_, value)| value.clone())
            }
        })
    }
    #[test]
    fn defaults_and_explicit_storage_proxy_and_registration_configuration_are_deterministic() {
        let default = config(&[]).unwrap();
        assert_eq!(default.public_origin, "https://example.invalid/platform");
        assert_eq!(default.bind, "127.0.0.1:8081".parse().unwrap());
        assert_eq!(default.registration, RegistrationMode::Closed);
        assert!(default.trusted_proxies.is_empty());
        let directory = tempfile::tempdir().unwrap();
        let value = config(&[
            ("VELORA_AUTH_DATA_DIR", directory.path().display().to_string()),
            ("VELORA_AUTH_DATABASE_PATH", directory.path().join("imported.sqlite").display().to_string()),
            ("VELORA_AUTH_TRUSTED_PROXIES", "127.0.0.1, ::1".into()),
            ("VELORA_AUTH_REGISTRATION", "approval".into()),
        ])
        .unwrap();
        assert_eq!(value.database, directory.path().join("imported.sqlite"));
        assert_eq!(value.rsa_key, directory.path().join("yggdrasil-signing.pem"));
        assert_eq!(value.registration, RegistrationMode::Approval);
        assert_eq!(value.trusted_proxies.len(), 2);
        assert!(!value.database.exists());
    }
    #[test]
    fn missing_or_invalid_public_origins_and_runtime_values_fail_without_defaults_or_secret_echoes() {
        assert!(RuntimeConfig::from_lookup(|_| None).is_err());
        for origin in [
            "https://user:synthetic-password@example.invalid",
            "file:///synthetic",
            "https://example.invalid/?secret=synthetic",
            "https://example.invalid/#fragment",
        ] {
            let error =
                RuntimeConfig::from_lookup(|key| (key == "VELORA_AUTH_PUBLIC_ORIGIN").then(|| origin.into())).unwrap_err().to_string();
            assert!(!error.contains("synthetic-password"));
        }
        for (key, value) in [
            ("VELORA_AUTH_BIND", "localhost:8081"),
            ("VELORA_AUTH_TRUSTED_PROXIES", "not-an-ip"),
            ("VELORA_AUTH_PANEL_ACCOUNTS", "yes"),
            ("VELORA_AUTH_REGISTRATION", "unknown"),
            ("VELORA_AUTH_BOOTSTRAP_USERNAME", "OnlyName"),
        ] {
            assert!(config(&[(key, value.into())]).is_err());
        }
    }
    #[test]
    fn configured_jwt_bytes_are_exact_and_missing_configuration_files_never_rotate_persisted_material() {
        let directory = tempfile::tempdir().unwrap();
        let configured = directory.path().join("configured.secret");
        let bytes = b"synthetic-legacy\n";
        std::fs::write(&configured, bytes).unwrap();
        let value = config(&[
            ("VELORA_AUTH_DATA_DIR", directory.path().display().to_string()),
            ("VELORA_AUTH_JWT_SECRET_FILE", configured.display().to_string()),
        ])
        .unwrap();
        assert_eq!(value.jwt_secret().unwrap(), bytes);
        assert!(!value.persisted_jwt.exists());
        assert!(!format!("{value:?}").contains("synthetic-legacy"));
        std::fs::remove_file(configured).unwrap();
        assert!(value.jwt_secret().is_err());
        assert!(!value.persisted_jwt.exists());
    }
    #[test]
    fn bootstrap_files_are_exact_and_skipped_on_restart_without_secret_debug_output() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("bootstrap.password");
        let value = config(&[
            ("VELORA_AUTH_BOOTSTRAP_USERNAME", "SyntheticAdmin".into()),
            ("VELORA_AUTH_BOOTSTRAP_PASSWORD_FILE", path.display().to_string()),
        ])
        .unwrap();
        assert!(value.bootstrap_credentials(true).unwrap().is_none());
        assert!(value.bootstrap_credentials(false).is_err());
        std::fs::write(&path, "synthetic-password\n").unwrap();
        assert_eq!(value.bootstrap_credentials(false).unwrap().unwrap().password, "synthetic-password\n");
        assert!(!format!("{value:?}").contains("synthetic-password"));
    }
    #[test]
    fn colliding_database_and_signing_paths_fail_before_reading_or_creating_material() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("shared-file");
        let value = path.display().to_string();
        assert!(config(&[("VELORA_AUTH_DATABASE_PATH", value.clone()), ("VELORA_AUTH_JWT_PATH", value.clone())]).is_err());
        assert!(config(&[("VELORA_AUTH_DATABASE_PATH", value.clone()), ("VELORA_AUTH_JWT_SECRET_FILE", value)]).is_err());
        assert!(!path.exists());
    }
}
