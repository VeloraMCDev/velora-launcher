//! Version metadata from Mojang and the mod-loader projects. Used by the
//! launcher (to install) and by the panel (to populate version pickers).

use crate::http::get_json;
use anyhow::{bail, Context, Result};
use velora_shared::Loader;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const MOJANG_MANIFEST: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
pub const FABRIC_META: &str = "https://meta.fabricmc.net/v2";
pub const QUILT_META: &str = "https://meta.quiltmc.org/v3";
pub const FORGE_MAVEN: &str = "https://maven.minecraftforge.net";
pub const FORGE_FILES: &str = "https://files.minecraftforge.net/net/minecraftforge/forge";
pub const NEOFORGE_MAVEN: &str = "https://maven.neoforged.net/releases";
pub const NEOFORGE_VERSIONS: &str = "https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionManifest {
    pub latest: Latest,
    pub versions: Vec<ManifestVersion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Latest {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestVersion {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub url: String,
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(default)]
    pub release_time: Option<String>,
}

pub async fn mojang_manifest(client: &reqwest::Client) -> Result<VersionManifest> {
    get_json(client, MOJANG_MANIFEST).await
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LoaderVersion {
    pub version: String,
    pub stable: bool,
}

#[derive(Deserialize)]
struct FabricLoaderEntry {
    loader: FabricLoaderInfo,
}
#[derive(Deserialize)]
struct FabricLoaderInfo {
    version: String,
    #[serde(default)]
    stable: Option<bool>,
}

#[derive(Deserialize)]
struct NeoVersions {
    versions: Vec<String>,
}

#[derive(Deserialize)]
struct ForgePromos {
    promos: HashMap<String, String>,
}

/// Map a NeoForge version to the Minecraft version it targets.
/// `21.1.77` → `1.21.1`, `21.0.5` → `1.21`, `26.1.0.3` → `26.1` (new
/// year-based Minecraft numbering).
pub fn neoforge_mc_version(neo: &str) -> Option<String> {
    let core = neo.split('-').next()?;
    let parts: Vec<&str> = core.split('.').collect();
    let major: u32 = parts.first()?.parse().ok()?;
    let minor: u32 = parts.get(1)?.parse().ok()?;
    if major >= 26 {
        let patch: u32 = parts.get(2).and_then(|p| p.parse().ok()).unwrap_or(0);
        return Some(if patch == 0 { format!("{major}.{minor}") } else { format!("{major}.{minor}.{patch}") });
    }
    Some(if minor == 0 { format!("1.{major}") } else { format!("1.{major}.{minor}") })
}

/// All loader versions for a Minecraft version, newest first.
pub async fn loader_versions(client: &reqwest::Client, loader: Loader, mc: &str) -> Result<Vec<LoaderVersion>> {
    Ok(match loader {
        Loader::Vanilla => vec![],
        Loader::Fabric | Loader::Quilt => {
            let url = if loader == Loader::Fabric {
                format!("{FABRIC_META}/versions/loader/{mc}")
            } else {
                format!("{QUILT_META}/versions/loader/{mc}")
            };
            let entries: Vec<FabricLoaderEntry> = get_json(client, &url).await?;
            entries
                .into_iter()
                .map(|e| {
                    let stable = e.loader.stable.unwrap_or_else(|| !e.loader.version.contains("beta"));
                    LoaderVersion { version: e.loader.version, stable }
                })
                .collect()
        }
        Loader::Forge => {
            let all: HashMap<String, Vec<String>> = get_json(client, &format!("{FORGE_FILES}/maven-metadata.json")).await?;
            let promos: ForgePromos =
                get_json(client, &format!("{FORGE_FILES}/promotions_slim.json")).await.unwrap_or(ForgePromos { promos: HashMap::new() });
            let recommended = promos.promos.get(&format!("{mc}-recommended")).map(|v| format!("{mc}-{v}"));
            let mut list: Vec<LoaderVersion> = all
                .get(mc)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .rev()
                .map(|v| LoaderVersion { stable: Some(&v) == recommended.as_ref(), version: v })
                .collect();
            // Put the recommended build first.
            list.sort_by_key(|v| !v.stable);
            list
        }
        Loader::NeoForge => {
            let all: NeoVersions = get_json(client, NEOFORGE_VERSIONS).await?;
            all.versions
                .into_iter()
                .rev()
                .filter(|v| neoforge_mc_version(v).as_deref() == Some(mc))
                .map(|v| LoaderVersion { stable: !v.contains("beta") && !v.contains("alpha"), version: v })
                .collect()
        }
    })
}

/// Turn "latest"/"recommended"/empty into a concrete loader version.
pub async fn resolve_loader_version(client: &reqwest::Client, loader: Loader, mc: &str, requested: Option<&str>) -> Result<Option<String>> {
    if loader == Loader::Vanilla {
        return Ok(None);
    }
    if let Some(v) = requested.map(str::trim).filter(|v| !v.is_empty() && *v != "latest" && *v != "recommended") {
        return Ok(Some(normalize_forge_version(loader, mc, v)));
    }
    let versions = loader_versions(client, loader, mc).await.context("fetching loader versions")?;
    let pick = versions.iter().find(|v| v.stable).or_else(|| versions.first());
    match pick {
        Some(v) => Ok(Some(v.version.clone())),
        None => bail!("no {} versions available for Minecraft {mc}", loader.as_str()),
    }
}

/// Forge versions are stored as `<mc>-<forge>` (the Maven version);
/// accept a bare `47.2.0` too.
pub fn normalize_forge_version(loader: Loader, mc: &str, v: &str) -> String {
    if loader == Loader::Forge && !v.contains('-') {
        format!("{mc}-{v}")
    } else {
        v.to_string()
    }
}

pub fn forge_installer_url(loader: Loader, version: &str) -> String {
    match loader {
        Loader::NeoForge => format!("{NEOFORGE_MAVEN}/net/neoforged/neoforge/{version}/neoforge-{version}-installer.jar"),
        _ => format!("{FORGE_MAVEN}/net/minecraftforge/forge/{version}/forge-{version}-installer.jar"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neoforge_mapping() {
        assert_eq!(neoforge_mc_version("21.1.77").as_deref(), Some("1.21.1"));
        assert_eq!(neoforge_mc_version("20.4.237").as_deref(), Some("1.20.4"));
        assert_eq!(neoforge_mc_version("21.0.5-beta").as_deref(), Some("1.21"));
        assert_eq!(neoforge_mc_version("26.1.0.3").as_deref(), Some("26.1"));
        assert_eq!(neoforge_mc_version("26.1.2.1").as_deref(), Some("26.1.2"));
    }

    #[test]
    fn forge_urls() {
        assert_eq!(normalize_forge_version(Loader::Forge, "1.20.1", "47.2.0"), "1.20.1-47.2.0");
        assert_eq!(
            forge_installer_url(Loader::Forge, "1.20.1-47.2.0"),
            "https://maven.minecraftforge.net/net/minecraftforge/forge/1.20.1-47.2.0/forge-1.20.1-47.2.0-installer.jar"
        );
    }
}
