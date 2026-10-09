use crate::accounts::AccountsFile;
use crate::secrets::Secrets;
use crate::settings::Settings;
use scopenet_core::Layout;
use scopenet_shared::LauncherManifest;
use std::path::PathBuf;
use std::sync::{Mutex, RwLock};

/// Values baked in at build time by CI (see `.github/workflows/release.yml`).
pub mod build {
    /// Default panel URL for this build.
    pub const PANEL_URL: Option<&str> = match option_env!("VELORA_PANEL_URL") {
        Some(u) if !u.is_empty() => Some(u),
        _ => match option_env!("SCOPENET_PANEL_URL") {
            Some(u) if !u.is_empty() => Some(u),
            _ => None,
        },
    };
    /// Repository whose latest release is checked for updates directly. Unset by default:
    /// official builds update only from releases an admin approved onto the panel.
    pub const REPO: Option<&str> = match option_env!("VELORA_REPO") {
        Some(r) if !r.is_empty() => Some(r),
        _ => match option_env!("SCOPENET_REPO") {
            Some(r) if !r.is_empty() => Some(r),
            _ => None,
        },
    };
    /// When set to "1", players can't change the panel URL.
    pub const LOCK_PANEL: Option<&str> = match option_env!("VELORA_LOCK_PANEL") {
        Some(value) => Some(value),
        None => option_env!("SCOPENET_LOCK_PANEL"),
    };

    pub fn panel_locked() -> bool {
        LOCK_PANEL == Some("1") && PANEL_URL.is_some_and(|u| !u.is_empty())
    }
}

pub struct RunningGame {
    pub run_id: String,
    pub instance_id: String,
    pub kill: Option<tokio::sync::oneshot::Sender<()>>,
}

pub struct AppState {
    pub data_dir: PathBuf,
    pub layout: Layout,
    pub http: reqwest::Client,
    pub settings: RwLock<Settings>,
    pub accounts: RwLock<AccountsFile>,
    pub secrets: Secrets,
    pub manifest: RwLock<Option<LauncherManifest>>,
    /// Install/launch pipeline in progress (abortable).
    pub task: Mutex<Option<tokio::task::AbortHandle>>,
    pub game: Mutex<Vec<RunningGame>>,
}

impl AppState {
    pub fn new(data_dir: PathBuf) -> Self {
        std::fs::create_dir_all(&data_dir).ok();
        let settings = Settings::load(&data_dir.join("settings.json"));
        let accounts = AccountsFile::load(&data_dir.join("accounts.json"));
        let manifest = std::fs::read(data_dir.join("manifest.json")).ok().and_then(|b| serde_json::from_slice(&b).ok());
        Self {
            layout: Layout::new(data_dir.join("minecraft")),
            http: scopenet_core::http::client(),
            secrets: Secrets::open(&data_dir),
            settings: RwLock::new(settings),
            accounts: RwLock::new(accounts),
            manifest: RwLock::new(manifest),
            task: Mutex::new(None),
            game: Mutex::new(Vec::new()),
            data_dir,
        }
    }

    /// The panel this launcher talks to (baked-in when locked).
    pub fn panel_url(&self) -> Option<String> {
        let chosen = self.settings.read().unwrap().panel_url.clone().filter(|u| !u.is_empty());
        let baked = build::PANEL_URL.filter(|u| !u.is_empty()).map(String::from);
        let url = if build::panel_locked() { baked } else { chosen.or(baked) };
        url.map(|u| u.trim_end_matches('/').to_string())
    }

    pub fn save_settings(&self) -> anyhow::Result<()> {
        self.settings.read().unwrap().save(&self.data_dir.join("settings.json"))
    }

    pub fn save_accounts(&self) -> anyhow::Result<()> {
        self.accounts.read().unwrap().save(&self.data_dir.join("accounts.json"))
    }

    pub fn cache_manifest(&self, m: &LauncherManifest) {
        *self.manifest.write().unwrap() = Some(m.clone());
        if let Ok(bytes) = serde_json::to_vec(m) {
            std::fs::write(self.data_dir.join("manifest.json"), bytes).ok();
        }
    }
}
