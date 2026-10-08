use std::path::PathBuf;

/// Runtime configuration, read from environment variables (see
/// `docker-compose.yml` for the documented list).
#[derive(Debug, Clone)]
pub struct Config {
    pub bind: String,
    pub data_dir: PathBuf,
    pub web_dir: PathBuf,
    /// Packed Iconify icon sets (see panel/icons/build.mjs).
    pub icons_dir: PathBuf,
    pub admin_username: String,
    pub admin_password: Option<String>,
    pub jwt_secret: Option<String>,
    pub curseforge_api_key: Option<String>,
    pub max_upload_mb: usize,
    pub public_url: Option<String>,
    pub trusted_proxies: Vec<std::net::IpAddr>,
}

fn var(name: &str) -> Option<String> {
    std::env::var(name).ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            bind: var("SCOPENET_BIND").unwrap_or_else(|| "0.0.0.0:8080".into()),
            data_dir: var("SCOPENET_DATA_DIR").unwrap_or_else(|| "./data".into()).into(),
            web_dir: var("SCOPENET_WEB_DIR").unwrap_or_else(|| "./panel/web/dist".into()).into(),
            icons_dir: var("SCOPENET_ICONS_DIR").unwrap_or_else(|| "./panel/icons/dist".into()).into(),
            admin_username: var("ADMIN_USERNAME").unwrap_or_else(|| "admin".into()),
            admin_password: var("ADMIN_PASSWORD"),
            jwt_secret: var("JWT_SECRET"),
            curseforge_api_key: var("CURSEFORGE_API_KEY"),
            max_upload_mb: var("MAX_UPLOAD_MB").and_then(|v| v.parse().ok()).unwrap_or(2048),
            public_url: var("PUBLIC_URL"),
            trusted_proxies: var("SCOPENET_TRUSTED_PROXIES")
                .map(|s| s.split(',').map(|v| v.trim().parse().expect("SCOPENET_TRUSTED_PROXIES must contain IP addresses")).collect())
                .unwrap_or_default(),
        }
    }

    pub fn files_dir(&self) -> PathBuf {
        self.data_dir.join("files")
    }
    pub fn uploads_dir(&self) -> PathBuf {
        self.data_dir.join("uploads")
    }
    pub fn textures_dir(&self) -> PathBuf {
        self.data_dir.join("textures")
    }
    pub fn downloads_dir(&self) -> PathBuf {
        self.data_dir.join("downloads")
    }
    pub fn signing_key_path(&self) -> PathBuf {
        self.data_dir.join("yggdrasil-signing.pem")
    }
}
