//! HTTP helpers: a shared client, JSON fetches and verified, resumable-safe
//! parallel downloads.

use anyhow::{bail, Context, Result};
use futures::StreamExt;
use serde::de::DeserializeOwned;
use sha1::{Digest, Sha1};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::io::AsyncWriteExt;

pub const USER_AGENT: &str = concat!("Velora/", env!("CARGO_PKG_VERSION"), " (+https://github.com/VeloraMCDev/sdk)");

pub fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(Duration::from_secs(15))
        .read_timeout(Duration::from_secs(60))
        .pool_max_idle_per_host(16)
        .build()
        .expect("http client")
}

pub async fn get_json<T: DeserializeOwned>(client: &reqwest::Client, url: &str) -> Result<T> {
    let resp = client.get(url).send().await.with_context(|| format!("GET {url}"))?;
    let status = resp.status();
    if !status.is_success() {
        bail!("GET {url} returned {status}");
    }
    let bytes = resp.bytes().await?;
    serde_json::from_slice(&bytes).with_context(|| format!("parsing JSON from {url}"))
}

/// Fetch JSON, caching it at `cache`. Falls back to the cached copy when
/// offline so the game can still launch without internet.
pub async fn get_json_cached<T: DeserializeOwned>(client: &reqwest::Client, url: &str, cache: &Path) -> Result<T> {
    match client.get(url).send().await.and_then(|r| r.error_for_status()) {
        Ok(resp) => {
            let bytes = resp.bytes().await?;
            let parsed = serde_json::from_slice(&bytes).with_context(|| format!("parsing JSON from {url}"))?;
            if let Some(dir) = cache.parent() {
                tokio::fs::create_dir_all(dir).await.ok();
            }
            tokio::fs::write(cache, &bytes).await.ok();
            Ok(parsed)
        }
        Err(err) => {
            let bytes = tokio::fs::read(cache).await.map_err(|_| anyhow::anyhow!("{err} (and no cached copy)"))?;
            tracing::warn!("{url} unreachable, using cached copy: {err}");
            Ok(serde_json::from_slice(&bytes)?)
        }
    }
}

pub fn sha1_file(path: &Path) -> Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha1::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

pub fn sha1_bytes(bytes: &[u8]) -> String {
    hex::encode(Sha1::digest(bytes))
}

/// Cheap check used before every launch: size match (and hash match when
/// `deep` is set).
pub fn file_ok(path: &Path, sha1: Option<&str>, size: Option<u64>, deep: bool) -> bool {
    let Ok(meta) = std::fs::metadata(path) else { return false };
    if let Some(size) = size {
        if size > 0 && meta.len() != size {
            return false;
        }
    }
    if deep {
        if let Some(expected) = sha1.filter(|s| !s.is_empty()) {
            return sha1_file(path).map(|h| h.eq_ignore_ascii_case(expected)).unwrap_or(false);
        }
    }
    true
}

#[derive(Debug, Clone)]
pub struct Download {
    pub url: String,
    pub dest: PathBuf,
    pub sha1: Option<String>,
    pub size: u64,
}

impl Download {
    pub fn new(url: impl Into<String>, dest: PathBuf, sha1: Option<String>, size: Option<u64>) -> Self {
        Self { url: url.into(), dest, sha1: sha1.filter(|s| !s.is_empty()), size: size.unwrap_or(0) }
    }
}

/// Download a single file to `dest` atomically (via a `.part` file) and
/// verify its SHA-1. Retries transient failures.
pub async fn download_one(client: &reqwest::Client, dl: &Download, on_bytes: &(dyn Fn(u64) + Sync)) -> Result<()> {
    let mut last_err = None;
    for attempt in 0..4u32 {
        if attempt > 0 {
            tokio::time::sleep(Duration::from_millis(400 * 2u64.pow(attempt))).await;
        }
        match try_download(client, dl, on_bytes).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                tracing::warn!("download {} failed (attempt {}): {e:#}", dl.url, attempt + 1);
                last_err = Some(e);
            }
        }
    }
    Err(last_err.unwrap()).with_context(|| format!("downloading {}", dl.url))
}

async fn try_download(client: &reqwest::Client, dl: &Download, on_bytes: &(dyn Fn(u64) + Sync)) -> Result<()> {
    if let Some(dir) = dl.dest.parent() {
        tokio::fs::create_dir_all(dir).await?;
    }
    let resp = client.get(&dl.url).send().await?.error_for_status()?;
    let part =
        dl.dest.with_extension(format!("{}part", dl.dest.extension().map(|e| format!("{}.", e.to_string_lossy())).unwrap_or_default()));
    let mut file = tokio::fs::File::create(&part).await?;
    let mut hasher = Sha1::new();
    let mut stream = resp.bytes_stream();
    let mut written = 0u64;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        hasher.update(&chunk);
        file.write_all(&chunk).await?;
        written += chunk.len() as u64;
        on_bytes(chunk.len() as u64);
    }
    file.flush().await?;
    drop(file);
    if let Some(expected) = &dl.sha1 {
        let got = hex::encode(hasher.finalize());
        if !got.eq_ignore_ascii_case(expected) {
            tokio::fs::remove_file(&part).await.ok();
            bail!("checksum mismatch for {} (expected {expected}, got {got}, {written} bytes)", dl.url);
        }
    }
    tokio::fs::rename(&part, &dl.dest).await?;
    Ok(())
}
