use crate::accounts::AccountsFile;
use crate::preparation::{self, PreparationManager};
use crate::secrets::Secrets;
use crate::settings::Settings;
use crate::shutdown::{Shutdown, WorkPermit};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use velora_launcher_core::Layout;
use velora_shared::LauncherManifest;

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

pub struct MutationLease {
    _resource: tokio::sync::OwnedMutexGuard<()>,
    _work: WorkPermit,
}

pub struct AppState {
    pub data_dir: PathBuf,
    pub layout: Layout,
    pub http: reqwest::Client,
    pub settings: Arc<RwLock<Settings>>,
    pub accounts: RwLock<AccountsFile>,
    pub profiles: Mutex<()>,
    pub secrets: Secrets,
    pub manifest: RwLock<Option<LauncherManifest>>,
    pub(crate) manifest_denials: crate::game::manifest_denials::ManifestDenials,
    pub preparations: Mutex<PreparationManager>,
    pub mutations: Arc<tokio::sync::Mutex<()>>,
    pub game: Mutex<Vec<RunningGame>>,
    pub shutdown: Arc<Shutdown>,
}

impl AppState {
    #[cfg(test)]
    pub fn new(data_dir: PathBuf) -> Self {
        Self::try_new(data_dir).expect("synthetic launcher state must initialize")
    }

    pub fn try_new(data_dir: PathBuf) -> anyhow::Result<Self> {
        std::fs::create_dir_all(&data_dir)?;
        let secrets = Secrets::try_open(&data_dir)?;
        let settings = Settings::load(&data_dir.join("settings.json"));
        let accounts = AccountsFile::load(&data_dir.join("accounts.json"));
        let manifest = std::fs::read(data_dir.join("manifest.json")).ok().and_then(|b| serde_json::from_slice(&b).ok());
        Ok(Self {
            layout: Layout::new(data_dir.join("minecraft")),
            http: velora_launcher_core::http::client(),
            secrets,
            settings: Arc::new(RwLock::new(settings)),
            accounts: RwLock::new(accounts),
            profiles: Mutex::new(()),
            manifest: RwLock::new(manifest),
            manifest_denials: crate::game::manifest_denials::ManifestDenials::default(),
            preparations: Mutex::new(PreparationManager::default()),
            mutations: Arc::new(tokio::sync::Mutex::new(())),
            game: Mutex::new(Vec::new()),
            shutdown: Arc::new(Shutdown::default()),
            data_dir,
        })
    }

    pub fn mutation_lease(&self) -> Result<MutationLease, &'static str> {
        let work = self.shutdown.enter()?;
        let resource = preparation::mutation_lease(&self.mutations, &self.preparations)?;
        Ok(MutationLease { _resource: resource, _work: work })
    }

    /// The panel this launcher talks to (baked-in when locked).
    pub fn panel_url(&self) -> Option<String> {
        Self::panel_url_from_settings(&self.settings.read().unwrap())
    }

    pub(crate) fn panel_url_from_settings(settings: &Settings) -> Option<String> {
        let chosen = settings.panel_url.clone().filter(|u| !u.is_empty());
        let baked = build::PANEL_URL.filter(|u| !u.is_empty()).map(String::from);
        let url = if build::panel_locked() { baked } else { chosen.or(baked) };
        url.map(|u| u.trim_end_matches('/').to_string())
    }

    pub fn update_settings(&self, change: impl FnOnce(&mut Settings)) -> anyhow::Result<()> {
        let _work = self.shutdown.enter().map_err(anyhow::Error::msg)?;
        Settings::update(&mut self.settings.write().unwrap(), &self.data_dir.join("settings.json"), change)
    }

    pub fn update_accounts<Value>(&self, change: impl FnOnce(&mut AccountsFile) -> anyhow::Result<Value>) -> anyhow::Result<Value> {
        let _work = self.shutdown.enter().map_err(anyhow::Error::msg)?;
        let mut accounts = self.accounts.write().unwrap();
        crate::account_transaction::ensure_clean(&self.data_dir)?;
        AccountsFile::update(&mut accounts, &self.data_dir.join("accounts.json"), change)
    }

    pub fn cache_manifest(&self, m: &LauncherManifest) {
        let Ok(_work) = self.shutdown.enter() else { return };
        *self.manifest.write().unwrap() = Some(m.clone());
        if let Ok(bytes) = serde_json::to_vec(m) {
            std::fs::write(self.data_dir.join("manifest.json"), bytes).ok();
        }
    }
}
