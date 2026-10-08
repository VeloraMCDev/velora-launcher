//! authlib-injector: a small Java agent that points the game's
//! authentication (sessions, skins, profile signatures) at the panel's
//! Yggdrasil server instead of Mojang's.
//!
//! The launcher fetches it from the panel's mirror first and falls back to
//! the official download, verifying the SHA-256 either way.

use crate::http::{self, Download};
use crate::paths::Layout;
use anyhow::{anyhow, bail, Context, Result};
use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub const OFFICIAL_LATEST: &str = "https://authlib-injector.yushi.moe/artifact/latest.json";

/// Shape of `latest.json` (the panel mirror uses the same format).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub build_number: u64,
    pub version: String,
    pub download_url: String,
    pub checksums: Checksums,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checksums {
    pub sha256: String,
}

pub fn sha256_file(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path)?;
    Ok(hex::encode(Sha256::digest(&bytes)))
}

fn cache_dir(layout: &Layout) -> PathBuf {
    layout.cache().join("authlib-injector")
}

/// Newest jar already on disk (used when offline).
fn newest_cached(layout: &Layout) -> Option<PathBuf> {
    std::fs::read_dir(cache_dir(layout))
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "jar"))
        .max_by_key(|p| std::fs::metadata(p).and_then(|m| m.modified()).ok())
}

async fn from_source(client: &reqwest::Client, layout: &Layout, index_url: &str, base: Option<&str>) -> Result<PathBuf> {
    let artifact: Artifact = http::get_json(client, index_url).await?;
    let url = match base {
        Some(b) => crate::sync::resolve_url(b, &artifact.download_url),
        None => artifact.download_url.clone(),
    };
    let dest = cache_dir(layout).join(format!("authlib-injector-{}.jar", artifact.version));
    let expected = artifact.checksums.sha256.to_ascii_lowercase();
    if dest.exists() && sha256_file(&dest)? == expected {
        return Ok(dest);
    }
    http::download_one(client, &Download::new(url, dest.clone(), None, None), &|_| {}).await?;
    let got = sha256_file(&dest)?;
    if got != expected {
        std::fs::remove_file(&dest).ok();
        bail!("authlib-injector checksum mismatch (expected {expected}, got {got})");
    }
    Ok(dest)
}

/// Make sure authlib-injector is available locally and return its path.
pub async fn ensure(client: &reqwest::Client, layout: &Layout, panel_base: Option<&str>) -> Result<PathBuf> {
    let mut errors = Vec::new();
    if let Some(base) = panel_base.filter(|b| !b.is_empty()) {
        let index = format!("{}/api/v1/launcher/authlib-injector.json", base.trim_end_matches('/'));
        match from_source(client, layout, &index, Some(base)).await {
            Ok(p) => return Ok(p),
            Err(e) => errors.push(format!("panel mirror: {e:#}")),
        }
    }
    match from_source(client, layout, OFFICIAL_LATEST, None).await {
        Ok(p) => return Ok(p),
        Err(e) => errors.push(format!("official download: {e:#}")),
    }
    if let Some(cached) = newest_cached(layout) {
        tracing::warn!("using cached authlib-injector ({})", errors.join("; "));
        return Ok(cached);
    }
    Err(anyhow!("couldn't download authlib-injector: {}", errors.join("; ")))
}

/// Fetch the Yggdrasil metadata so it can be handed to the agent up front
/// (saves the game a network round-trip at startup).
pub async fn prefetch_metadata(client: &reqwest::Client, api_url: &str) -> Result<String> {
    let resp = client.get(api_url).send().await?.error_for_status().context("fetching auth server metadata")?;
    let bytes = resp.bytes().await?;
    // Must be valid JSON, or the agent would refuse to start.
    serde_json::from_slice::<serde_json::Value>(&bytes).context("auth server metadata isn't JSON")?;
    Ok(base64::engine::general_purpose::STANDARD.encode(&bytes))
}

/// JVM arguments that load the agent. They must come before the main class.
pub fn jvm_args(jar: &Path, api_url: &str, prefetched: Option<&str>) -> Vec<String> {
    let mut args = vec![format!("-javaagent:{}={}", jar.display(), api_url)];
    if let Some(p) = prefetched {
        args.push(format!("-Dauthlibinjector.yggdrasil.prefetched={p}"));
    }
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_agent_args() {
        let args = jvm_args(Path::new("/c/ai.jar"), "https://panel.example/api/yggdrasil", Some("e30="));
        assert_eq!(args[0], "-javaagent:/c/ai.jar=https://panel.example/api/yggdrasil");
        assert_eq!(args[1], "-Dauthlibinjector.yggdrasil.prefetched=e30=");
    }

    #[test]
    fn parses_latest_json() {
        let a: Artifact = serde_json::from_str(
            r#"{"build_number":53,"version":"1.2.5","release_time":"x","download_url":"https://x/a.jar","checksums":{"sha256":"ab"}}"#,
        )
        .unwrap();
        assert_eq!(a.version, "1.2.5");
    }
}
