//! Installing Velora Core on a server and keeping it current.
use crate::{settings, velora, wings};
use shared::{State, models::{ByUuid, server::Server}};
use std::{collections::{HashMap, HashSet}, sync::LazyLock, time::Duration};
use tokio::sync::Mutex;

pub const MODS: &str = "/mods";
pub const CONFIG_DIR: &str = "/config";
pub const CONFIG_FILE: &str = "/config/velora-core.properties";

/// Servers being changed right now, so a manual install and the background task never overlap.
static BUSY: LazyLock<Mutex<HashSet<uuid::Uuid>>> = LazyLock::new(|| Mutex::new(HashSet::new()));
/// The last thing the extension did to each server, shown on its page.
static LAST: LazyLock<Mutex<HashMap<uuid::Uuid, String>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

pub async fn last_result(server: uuid::Uuid) -> Option<String> {
    LAST.lock().await.get(&server).cloned()
}
async fn remember(server: uuid::Uuid, message: impl Into<String>) {
    LAST.lock().await.insert(server, message.into());
}

/// What is installed on a server right now.
pub struct Installed {
    pub jars: Vec<velora::InstalledJar>,
    pub fabric_api: bool,
}
impl Installed {
    /// The newest installed Velora Core server jar, if any.
    pub fn newest(&self) -> Option<&velora::InstalledJar> {
        self.jars.iter().fold(None, |best: Option<&velora::InstalledJar>, jar| match best {
            Some(b) if !velora::newer(&jar.version, &b.version) => Some(b),
            _ => Some(jar),
        })
    }
}

pub async fn installed(state: &State, server: &Server) -> Result<Installed, anyhow::Error> {
    let names = wings::list(state, server, MODS).await?;
    Ok(Installed {
        fabric_api: names.iter().any(|n| n.to_ascii_lowercase().starts_with("fabric-api") && n.ends_with(".jar")),
        jars: names.iter().filter_map(|n| velora::parse_installed(n)).collect(),
    })
}

pub enum Outcome {
    /// The release was already installed.
    Current(String),
    Installed { from: Option<String>, to: String },
}
impl Outcome {
    pub fn message(&self) -> String {
        match self {
            Outcome::Current(v) => format!("Velora Core {v} is already installed."),
            Outcome::Installed { from: Some(f), to } => format!("Updated Velora Core {f} to {to}. Restart the server to use it."),
            Outcome::Installed { from: None, to } => format!("Installed Velora Core {to}. Start or restart the server to load it."),
        }
    }
}

/// Puts the approved release's server jar in `mods`, verifies its checksum, then removes older Velora Core jars.
/// The caller decides whether the server may be changed right now (see [`may_change`]).
pub async fn install(state: &State, server: &Server, panel_url: &str, latest: &velora::Latest) -> Result<Outcome, anyhow::Error> {
    if !BUSY.lock().await.insert(server.uuid) {
        anyhow::bail!("Another Velora Core change is already running on this server");
    }
    let result = install_inner(state, server, panel_url, latest).await;
    BUSY.lock().await.remove(&server.uuid);
    match &result {
        Ok(outcome) => remember(server.uuid, outcome.message()).await,
        Err(err) => remember(server.uuid, format!("Could not install Velora Core {}: {err}", latest.version)).await,
    }
    result
}

async fn install_inner(state: &State, server: &Server, panel_url: &str, latest: &velora::Latest) -> Result<Outcome, anyhow::Error> {
    let before = installed(state, server).await?;
    let previous = before.newest().map(|j| j.version.clone());
    let target = format!("{MODS}/{}", latest.server.name);
    let wanted = latest.server.sha256.to_ascii_lowercase();
    if before.jars.iter().any(|j| j.file == latest.server.name && j.version == latest.version) && wings::sha256(state, server, &target).await?.as_deref() == Some(wanted.as_str()) {
        // Same release already in place; still clear any older jar that would load alongside it.
        let stale: Vec<String> = before.jars.iter().filter(|j| j.file != latest.server.name).map(|j| j.file.clone()).collect();
        wings::delete(state, server, MODS, stale).await?;
        return Ok(Outcome::Current(latest.version.clone()));
    }
    let url = velora::download_url(panel_url, &latest.server).map_err(anyhow::Error::msg)?;
    wings::create_directory(state, server, "/", "mods").await;
    if before.jars.iter().any(|j| j.file == latest.server.name) {
        wings::delete(state, server, MODS, vec![latest.server.name.clone()]).await?;
    }
    wings::pull(state, server, MODS, &url, &latest.server.name).await?;
    let got = wings::sha256(state, server, &target).await?;
    if got.as_deref() != Some(wanted.as_str()) {
        wings::delete(state, server, MODS, vec![latest.server.name.clone()]).await.ok();
        anyhow::bail!("the downloaded jar did not match the checksum the Velora Panel published, so it was removed");
    }
    let stale: Vec<String> = before.jars.iter().filter(|j| j.file != latest.server.name).map(|j| j.file.clone()).collect();
    wings::delete(state, server, MODS, stale).await?;
    tracing::info!(server = %server.uuid, version = %latest.version, "installed Velora Core");
    Ok(Outcome::Installed { from: previous, to: latest.version.clone() })
}

/// A jar may only be swapped while the server is stopped; a first install can happen any time because nothing is loaded yet.
pub async fn may_change(state: &State, server: &Server, has_jar: bool) -> Result<bool, anyhow::Error> {
    if !has_jar {
        return Ok(true);
    }
    Ok(wings::power_state(state, server).await?.as_deref() == Some("offline"))
}

/// One pass of the background task. Returns true while some linked server still waits for a restart, so the caller can poll quickly.
pub async fn tick(state: &State) -> Result<bool, anyhow::Error> {
    let config = settings::load(state).await?;
    if !config.auto_update || config.linked.is_empty() || config.panel_url.is_empty() {
        return Ok(false);
    }
    let Some(latest) = velora::latest(&config.panel_url, Duration::from_secs(60)).await? else { return Ok(false) };
    let mut waiting = false;
    for id in &config.linked {
        let server = match Server::by_uuid(&state.database, *id).await {
            Ok(server) => server,
            Err(_) => continue,
        };
        if server.suspended {
            continue;
        }
        let current = match installed(state, &server).await {
            Ok(current) => current,
            Err(err) => {
                tracing::debug!(%err, server = %id, "could not read mods folder");
                continue;
            }
        };
        let outdated = match current.newest() {
            Some(jar) => velora::newer(&latest.version, &jar.version) || jar.legacy,
            None => false, // Never installed here: the operator installs it deliberately the first time.
        };
        if !outdated {
            continue;
        }
        match may_change(state, &server, true).await {
            Ok(true) => {
                if let Err(err) = install(state, &server, &config.panel_url, &latest).await {
                    tracing::warn!(%err, server = %id, "automatic Velora Core update failed");
                }
            }
            Ok(false) => waiting = true,
            Err(err) => tracing::debug!(%err, server = %id, "could not read power state"),
        }
    }
    Ok(waiting)
}
