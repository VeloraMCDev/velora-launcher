//! HTTP helpers: a shared client, JSON fetches and verified downloads.

use anyhow::{bail, Context, Result};
use futures::StreamExt;
use serde::de::DeserializeOwned;
use sha1::{Digest, Sha1};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

pub const USER_AGENT: &str = concat!("Velora/", env!("CARGO_PKG_VERSION"), " (+https://github.com/VeloraMCDev/sdk)");

pub fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(Duration::from_secs(15))
        .read_timeout(Duration::from_secs(60))
        .redirect(download_redirect_policy())
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
    get_json_cached_with(client, url, cache, write_cached_json).await
}

fn write_cached_json(cache: &Path, bytes: &[u8]) {
    if let Some(dir) = cache.parent() {
        std::fs::create_dir_all(dir).ok();
    }
    std::fs::write(cache, bytes).ok();
}

async fn get_json_cached_with<T: DeserializeOwned>(
    client: &reqwest::Client,
    url: &str,
    cache: &Path,
    write_cache: impl FnOnce(&Path, &[u8]),
) -> Result<T> {
    match client.get(url).send().await.and_then(|r| r.error_for_status()) {
        Ok(resp) => {
            let bytes = resp.bytes().await?;
            let parsed = serde_json::from_slice(&bytes).with_context(|| format!("parsing JSON from {url}"))?;
            write_cache(cache, &bytes);
            Ok(parsed)
        }
        Err(err) => {
            let bytes = std::fs::read(cache).map_err(|_| anyhow::anyhow!("{err} (and no cached copy)"))?;
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
pub const MAX_UNSPECIFIED_DOWNLOAD_BYTES: u64 = 8 * 1024 * 1024 * 1024;

pub fn download_redirect_policy() -> reqwest::redirect::Policy {
    reqwest::redirect::Policy::custom(|attempt| {
        if !allowed_redirect(attempt.url(), attempt.previous()) {
            attempt.error("unsafe download redirect")
        } else {
            reqwest::redirect::Policy::limited(10).redirect(attempt)
        }
    })
}

fn allowed_redirect(next: &reqwest::Url, previous: &[reqwest::Url]) -> bool {
    let secure_origin = previous.iter().any(|url| url.scheme() == "https");
    valid_download_url(next) && (!secure_origin || next.scheme() == "https")
}

fn valid_download_url(url: &reqwest::Url) -> bool {
    matches!(url.scheme(), "http" | "https") && url.host_str().is_some() && url.username().is_empty() && url.password().is_none()
}

pub async fn download_one(client: &reqwest::Client, dl: &Download, on_bytes: &(dyn Fn(u64) + Sync)) -> Result<()> {
    download_one_with_limit(client, dl, if dl.size > 0 { dl.size } else { MAX_UNSPECIFIED_DOWNLOAD_BYTES }, on_bytes).await
}

pub async fn download_one_with_limit(client: &reqwest::Client, dl: &Download, limit: u64, on_bytes: &(dyn Fn(u64) + Sync)) -> Result<()> {
    let url = reqwest::Url::parse(&dl.url).context("invalid download URL")?;
    if !valid_download_url(&url) {
        bail!("download URL must be HTTP(S) without embedded credentials");
    }
    if dl.size > limit {
        bail!("declared download size exceeds the byte limit");
    }
    let mut last_err = None;
    for attempt in 0..4u32 {
        if attempt > 0 {
            tokio::time::sleep(Duration::from_millis(400 * 2u64.pow(attempt))).await;
        }
        match try_download(client, dl, limit, on_bytes).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                tracing::warn!("download {} failed (attempt {}): {e:#}", dl.url, attempt + 1);
                last_err = Some(e);
            }
        }
    }
    Err(last_err.unwrap()).with_context(|| format!("downloading {}", dl.url))
}

async fn try_download(client: &reqwest::Client, dl: &Download, limit: u64, on_bytes: &(dyn Fn(u64) + Sync)) -> Result<()> {
    if let Some(dir) = dl.dest.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let resp = client.get(&dl.url).send().await?;
    if resp.status() != reqwest::StatusCode::OK {
        bail!("full download {} returned {} (expected 200 OK)", dl.url, resp.status());
    }
    let original = reqwest::Url::parse(&dl.url)?;
    if !valid_download_url(resp.url()) || (original.scheme() == "https" && resp.url().scheme() != "https") {
        bail!("unsafe final download URL");
    }
    if dl.size == 0 && resp.content_length().is_some_and(|size| size > limit) {
        bail!("download exceeds the byte limit");
    }
    let mut partial = PartialDownload::create(&dl.dest)?;
    let result: Result<()> = async {
        let mut hasher = Sha1::new();
        let mut stream = resp.bytes_stream();
        let mut written = 0u64;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("reading download body")?;
            let next_written = written.checked_add(chunk.len() as u64).context("download byte count overflow")?;
            if dl.size > 0 && next_written > dl.size {
                bail!("size mismatch (expected {} bytes, received at least {next_written} bytes)", dl.size);
            }
            if next_written > limit {
                bail!("download exceeds the byte limit");
            }
            partial.file.as_mut().unwrap().write_all(&chunk).context("writing partial download")?;
            hasher.update(&chunk);
            written = next_written;
            on_bytes(chunk.len() as u64);
        }
        partial.file.as_mut().unwrap().flush().context("flushing partial download")?;
        if dl.size > 0 && written != dl.size {
            bail!("size mismatch (expected {} bytes, got {written} bytes)", dl.size);
        }
        if let Some(expected) = &dl.sha1 {
            let got = hex::encode(hasher.finalize());
            if !got.eq_ignore_ascii_case(expected) {
                bail!("checksum mismatch for {} (expected {expected}, got {got}, {written} bytes)", dl.url);
            }
        }
        Ok(())
    }
    .await;
    partial.file.take();
    let result = match result {
        Ok(()) => std::fs::rename(&partial.path, &dl.dest).context("publishing verified download"),
        Err(error) => Err(error),
    };
    if let Err(error) = result {
        return match partial.cleanup() {
            Ok(()) => Err(error),
            Err(cleanup) => Err(error.context(format!("removing owned partial download {} failed: {cleanup}", partial.path.display()))),
        };
    }
    partial.owned = false;
    Ok(())
}

struct PartialDownload {
    path: PathBuf,
    file: Option<std::fs::File>,
    owned: bool,
}

impl PartialDownload {
    fn create(destination: &Path) -> Result<Self> {
        static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);
        for _attempt in 0..128 {
            let sequence = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
            let path = destination.with_file_name(format!(".velora-download-{}-{sequence}.part", std::process::id()));
            if path == destination {
                continue;
            }
            match std::fs::OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(file) => return Ok(Self { path, file: Some(file), owned: true }),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error).context("creating owned partial download"),
            }
        }
        bail!("could not exclusively create a partial download after 128 candidates")
    }

    fn cleanup(&mut self) -> std::io::Result<()> {
        self.file.take();
        self.owned = false;
        match std::fs::remove_file(&self.path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            result => result,
        }
    }
}

impl Drop for PartialDownload {
    fn drop(&mut self) {
        if self.owned {
            if let Err(error) = self.cleanup() {
                tracing::warn!("owned download temporary cleanup failed: {error}");
            }
        }
    }
}

#[cfg(test)]
#[path = "http/cache_cancellation_tests.rs"]
mod cache_cancellation_tests;

#[cfg(test)]
mod download_policy_tests {
    use super::*;

    #[test]
    fn redirects_allow_cdn_hops_but_reject_credentials_non_http_and_downgrades() {
        let secure = reqwest::Url::parse("https://synthetic.invalid/source").unwrap();
        let plain = reqwest::Url::parse("http://synthetic.invalid/source").unwrap();
        let cdn = reqwest::Url::parse("https://cdn.synthetic.invalid/artifact").unwrap();
        assert!(allowed_redirect(&cdn, &[secure.clone()]));
        assert!(allowed_redirect(&cdn, &[plain.clone()]));
        assert!(allowed_redirect(&plain, &[plain.clone()]));
        assert!(!allowed_redirect(&plain, &[secure.clone()]));
        assert!(!allowed_redirect(&plain, &[plain.clone(), secure]));
        for candidate in ["file:///synthetic", "ftp://synthetic.invalid/artifact", "https://user:synthetic@synthetic.invalid/artifact"] {
            assert!(!allowed_redirect(&reqwest::Url::parse(candidate).unwrap(), &[plain.clone()]));
        }
    }

    #[test]
    fn owner_drop_closes_file_and_removes_only_its_exclusive_temporary() {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("managed.jar");
        std::fs::write(&destination, b"synthetic previous").unwrap();
        let other = directory.path().join("unowned.part");
        std::fs::write(&other, b"synthetic unowned").unwrap();
        let path = {
            let mut partial = PartialDownload::create(&destination).unwrap();
            partial.file.as_mut().unwrap().write_all(b"synthetic incomplete").unwrap();
            partial.path.clone()
        };
        assert!(!path.exists());
        assert_eq!(std::fs::read(&destination).unwrap(), b"synthetic previous");
        assert_eq!(std::fs::read(&other).unwrap(), b"synthetic unowned");
    }

    #[tokio::test]
    async fn invalid_scheme_and_credentials_fail_before_a_request_or_temp_creation() {
        let directory = tempfile::tempdir().unwrap();
        for url in ["file:///synthetic", "ftp://synthetic.invalid/artifact", "https://user:synthetic@synthetic.invalid/artifact"] {
            let download = Download::new(url, directory.path().join("managed.jar"), None, None);
            assert!(download_one(&client(), &download, &|_| panic!("must not accept bytes")).await.is_err());
        }
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
    }

    #[tokio::test]
    async fn canonical_redirect_policy_follows_http_hops_and_bounds_redirect_loops() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            loop {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut request = Vec::new();
                let mut byte = [0];
                while !request.ends_with(b"\r\n\r\n") {
                    if stream.read_exact(&mut byte).await.is_err() {
                        break;
                    }
                    request.push(byte[0]);
                }
                let response = if request.starts_with(b"GET /complete ") {
                    "HTTP/1.1 200 OK\r\nContent-Length: 8\r\nConnection: close\r\n\r\ncomplete"
                } else if request.starts_with(b"GET /start ") {
                    "HTTP/1.1 302 Found\r\nLocation: /complete\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                } else {
                    "HTTP/1.1 302 Found\r\nLocation: /loop\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                };
                stream.write_all(response.as_bytes()).await.ok();
            }
        });
        let http =
            reqwest::Client::builder().no_proxy().redirect(download_redirect_policy()).timeout(Duration::from_secs(2)).build().unwrap();
        let result = http.get(format!("{base}/start")).send().await.unwrap();
        assert_eq!(result.status(), reqwest::StatusCode::OK);
        assert_eq!(result.bytes().await.unwrap().as_ref(), b"complete");
        assert!(http.get(format!("{base}/loop")).send().await.unwrap_err().is_redirect());
        server.abort();
        assert!(server.await.unwrap_err().is_cancelled());
    }
}
