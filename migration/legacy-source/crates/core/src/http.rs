//! HTTP helpers: a shared client, JSON fetches and verified, resumable-safe
//! parallel downloads.

use crate::progress::{Event, Reporter, Stage};
use anyhow::{bail, Context, Result};
use futures::StreamExt;
use serde::de::DeserializeOwned;
use sha1::{Digest, Sha1};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::AsyncWriteExt;

pub const USER_AGENT: &str = concat!("SCOPENET-Launcher/", env!("CARGO_PKG_VERSION"), " (+https://github.com/scopeddlol/SCOPENET-MC)");

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

/// Download many files in parallel, reporting aggregate byte progress.
pub async fn download_all(
    client: &reqwest::Client,
    downloads: Vec<Download>,
    concurrency: usize,
    stage: Stage,
    reporter: &Reporter,
) -> Result<()> {
    if downloads.is_empty() {
        return Ok(());
    }
    let total: u64 = downloads.iter().map(|d| d.size).sum();
    let files_total = downloads.len() as u32;
    let done = Arc::new(AtomicU64::new(0));
    let files_done = Arc::new(AtomicU32::new(0));
    let last_emit = Arc::new(std::sync::Mutex::new(Instant::now() - Duration::from_secs(1)));

    let emit = {
        let done = done.clone();
        let files_done = files_done.clone();
        let reporter = reporter.clone();
        move |force: bool| {
            let mut last = last_emit.lock().unwrap();
            if force || last.elapsed() >= Duration::from_millis(100) {
                *last = Instant::now();
                let d = done.load(Ordering::Relaxed);
                reporter(Event::Progress {
                    stage,
                    done: d,
                    total: total.max(d),
                    files_done: files_done.load(Ordering::Relaxed),
                    files_total,
                });
            }
        }
    };
    emit(true);

    let results: Vec<Result<()>> = futures::stream::iter(downloads.into_iter().map(|dl| {
        let done = done.clone();
        let files_done = files_done.clone();
        let emit = &emit;
        async move {
            let on_bytes = |n: u64| {
                done.fetch_add(n, Ordering::Relaxed);
                emit(false);
            };
            let r = download_one(client, &dl, &on_bytes).await;
            files_done.fetch_add(1, Ordering::Relaxed);
            emit(false);
            r
        }
    }))
    .buffer_unordered(concurrency.clamp(1, 64))
    .collect()
    .await;
    emit(true);

    let errors: Vec<_> = results.into_iter().filter_map(|r| r.err()).collect();
    if let Some(first) = errors.into_iter().next() {
        return Err(first);
    }
    Ok(())
}
