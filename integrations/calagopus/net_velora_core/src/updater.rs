//! Read-only inspection of the manually installed Velora Core mod.
use crate::{velora, wings};
use shared::{State, models::server::Server};

pub const MODS: &str = "/mods";
pub const CONFIG_DIR: &str = "/config";
pub const CONFIG_FILE: &str = "/config/velora-core.properties";

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

