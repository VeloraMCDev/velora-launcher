//! Updates from admin-hosted installers or published repository releases.
use crate::state::build;
use anyhow::{anyhow, bail, Context, Result};
use futures::StreamExt;
pub use scopenet_shared::LauncherUpdate as UpdateInfo;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{path::Path, time::Duration};
use tokio::io::AsyncWriteExt;

const MAX_INSTALLER: u64 = 256 * 1024 * 1024;
#[derive(Deserialize)]
struct Release {
    tag_name: String,
    #[serde(default)]
    body: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    assets: Vec<Asset>,
}
#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
    size: u64,
    #[serde(default)]
    digest: Option<String>,
}
pub fn is_newer(candidate: &str, current: &str) -> bool {
    scopenet_shared::newer_release(candidate, current)
}
fn latest_release_api(repo: &reqwest::Url) -> String {
    let path = repo.path().trim_end_matches('/').trim_end_matches(".git");
    if repo.host_str() == Some("github.com") {
        format!("https://api.github.com/repos{path}/releases/latest")
    } else {
        format!("{}/api/v1/repos{path}/releases/latest", repo.origin().ascii_serialization())
    }
}
fn valid_digest(digest: &str) -> bool {
    digest.len() == 64 && digest.bytes().all(|b| b.is_ascii_hexdigit())
}
fn validate_size(size: u64) -> Result<()> {
    if size == 0 || size > MAX_INSTALLER {
        bail!("Invalid installer size");
    }
    Ok(())
}

async fn panel_update(http: &reqwest::Client, panel: Option<&str>) -> Result<Option<UpdateInfo>> {
    let Some(panel) = panel else { return Ok(None) };
    let base = reqwest::Url::parse(panel).context("invalid panel URL")?;
    if !matches!(base.scheme(), "https" | "http")
        || base.host_str().is_none()
        || !base.username().is_empty()
        || base.password().is_some()
        || base.query().is_some()
        || base.fragment().is_some()
    {
        bail!("Invalid panel update URL");
    }
    let response =
        http.get(format!("{}/api/v1/launcher/update", panel.trim_end_matches('/'))).timeout(Duration::from_secs(10)).send().await?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    let Some(mut info) = response.error_for_status()?.json::<Option<UpdateInfo>>().await? else { return Ok(None) };
    if !is_newer(&info.version, env!("CARGO_PKG_VERSION")) {
        return Ok(None);
    }
    validate_size(info.size)?;
    let digest = info.sha256.as_deref().filter(|s| valid_digest(s)).ok_or_else(|| anyhow!("Panel installer has no valid checksum"))?;
    if info.url != format!("/api/v1/launcher/updates/{digest}/setup.exe") {
        bail!("Unexpected panel installer URL");
    }
    info.url = format!("{}{}", panel.trim_end_matches('/'), info.url);
    Ok(Some(info))
}
/// The installer asset this platform downloads from a release: the NSIS setup on Windows, the disk image on macOS, the AppImage on Linux.
fn installer_suffix() -> &'static str {
    if cfg!(windows) {
        "-setup.exe"
    } else if cfg!(target_os = "macos") {
        ".dmg"
    } else {
        ".appimage"
    }
}
async fn repository_update(http: &reqwest::Client) -> Result<Option<UpdateInfo>> {
    let Some(repo) = build::REPO.filter(|r| !r.is_empty()) else { return Ok(None) };
    let repo = reqwest::Url::parse(repo).context("invalid release repository URL")?;
    if repo.scheme() != "https" {
        bail!("release repository must use HTTPS");
    }
    let response = http.get(latest_release_api(&repo)).header("Accept", "application/json").timeout(Duration::from_secs(10)).send().await?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    let release: Release = response.error_for_status()?.json().await?;
    if release.draft || release.prerelease || !is_newer(&release.tag_name, env!("CARGO_PKG_VERSION")) {
        return Ok(None);
    }
    let asset = release
        .assets
        .iter()
        .find(|a| a.name.to_lowercase().ends_with(installer_suffix()))
        .ok_or_else(|| anyhow!("release {} has no installer", release.tag_name))?;
    let url = reqwest::Url::parse(&asset.browser_download_url)?;
    if url.scheme() != "https" || url.origin() != repo.origin() {
        bail!("Unexpected repository installer URL");
    }
    validate_size(asset.size)?;
    Ok(Some(UpdateInfo {
        version: release.tag_name.trim_start_matches('v').into(),
        notes: release.body,
        url: url.to_string(),
        size: asset.size,
        sha256: asset.digest.as_deref().and_then(|s| s.strip_prefix("sha256:")).filter(|s| valid_digest(s)).map(str::to_owned),
    }))
}
fn select_update(panel: Result<Option<UpdateInfo>>, repository: Result<Option<UpdateInfo>>) -> Result<Option<UpdateInfo>> {
    match (panel, repository) {
        (Ok(Some(p)), Ok(Some(r))) => Ok(Some(if is_newer(&r.version, &p.version) { r } else { p })),
        (Ok(Some(p)), _) => Ok(Some(p)),
        (_, Ok(Some(r))) => Ok(Some(r)),
        (Err(e), _) | (_, Err(e)) => Err(e),
        _ => Ok(None),
    }
}
pub async fn check(http: &reqwest::Client, panel: Option<&str>) -> Result<Option<UpdateInfo>> {
    let (hosted, repository) = tokio::join!(
        async {
            // The panel hosts the Windows installer only; macOS and Linux builds update from the release page.
            if cfg!(windows) {
                panel_update(http, panel).await
            } else {
                Ok(None)
            }
        },
        repository_update(http)
    );
    select_update(hosted, repository)
}
/// Bounded download and verification, separated from execution for testing.
async fn download_installer(http: &reqwest::Client, info: &UpdateInfo, dest: &Path) -> Result<()> {
    validate_size(info.size)?;
    let expected =
        info.sha256.as_deref().filter(|digest| valid_digest(digest)).ok_or_else(|| anyhow!("Installer has no valid checksum"))?;
    let part = dest.with_extension("exe.part");
    let result = async {
        let response = http.get(&info.url).timeout(Duration::from_secs(300)).send().await?.error_for_status()?;
        if response.content_length().is_some_and(|n| n != info.size) {
            bail!("Installer size differs from its release metadata");
        }
        let mut stream = response.bytes_stream();
        let mut file = tokio::fs::File::create(&part).await?;
        let mut hasher = Sha256::new();
        let mut written = 0;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            written += chunk.len() as u64;
            if written > info.size {
                bail!("Installer exceeds its advertised size");
            }
            hasher.update(&chunk);
            file.write_all(&chunk).await?;
        }
        file.flush().await?;
        drop(file);
        if written != info.size {
            bail!("Installer download was incomplete");
        }
        if !format!("{:x}", hasher.finalize()).eq_ignore_ascii_case(expected) {
            bail!("Installer checksum verification failed");
        }
        tokio::fs::rename(&part, dest).await?;
        Ok(())
    }
    .await;
    if result.is_err() {
        tokio::fs::remove_file(&part).await.ok();
    }
    result
}
pub async fn download_and_run(http: &reqwest::Client, info: &UpdateInfo) -> Result<()> {
    let ext = if cfg!(windows) {
        "setup.exe"
    } else if cfg!(target_os = "macos") {
        "dmg"
    } else {
        "AppImage"
    };
    let dest = std::env::temp_dir().join(format!("scopenet-{}-{ext}", uuid::Uuid::new_v4()));
    download_installer(http, info, &dest).await.context("downloading update")?;
    if cfg!(windows) {
        std::process::Command::new(&dest).args(["/P", "/R"]).spawn().context("starting the installer")?;
    } else if cfg!(target_os = "macos") {
        // Mounts the disk image; the player drags the app into Applications.
        std::process::Command::new("open").arg(&dest).spawn().context("opening the update")?;
    } else {
        // An AppImage is the app itself: make it runnable and show it in the file manager to replace the old one.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o755)).ok();
        }
        std::process::Command::new("xdg-open").arg(dest.parent().unwrap_or(&dest)).spawn().context("opening the update folder")?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    async fn serve(body: Vec<u8>, status: &str) -> String {
        use tokio::{io::AsyncReadExt, net::TcpListener};
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let status = status.to_owned();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = vec![0; 4096];
            socket.read(&mut request).await.unwrap();
            socket
                .write_all(
                    format!(
                        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n",
                        body.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
            socket.write_all(&body).await.unwrap();
        });
        format!("http://{address}")
    }
    #[tokio::test]
    async fn panel_feed_accepts_only_its_checksummed_downloads_and_handles_old_panels() {
        let http = reqwest::Client::new();
        let digest = "ab".repeat(32);
        let mut info = update("99.0.0");
        info.sha256 = Some(digest.clone());
        info.url = format!("/api/v1/launcher/updates/{digest}/setup.exe");
        let base = serve(serde_json::to_vec(&info).unwrap(), "200 OK").await;
        let found = panel_update(&http, Some(&base)).await.unwrap().unwrap();
        assert_eq!(found.url, format!("{base}{}", info.url));
        info.url = "https://other.test/setup.exe".into();
        let base = serve(serde_json::to_vec(&info).unwrap(), "200 OK").await;
        assert!(panel_update(&http, Some(&base)).await.is_err());
        let base = serve(Vec::new(), "404 Not Found").await;
        assert!(panel_update(&http, Some(&base)).await.unwrap().is_none());
        info.version = env!("CARGO_PKG_VERSION").into();
        let base = serve(serde_json::to_vec(&info).unwrap(), "200 OK").await;
        assert!(panel_update(&http, Some(&base)).await.unwrap().is_none());
    }
    #[tokio::test]
    async fn download_verifies_checksum_and_size_before_creating_installer() {
        let http = reqwest::Client::new();
        let bytes = b"installer fixture".to_vec();
        let dir = std::env::temp_dir().join(format!("scopenet-updater-test-{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir(&dir).await.unwrap();
        let dest = dir.join("setup.exe");
        let mut info = update("99.0.0");
        info.size = bytes.len() as u64;
        info.sha256 = Some(format!("{:x}", Sha256::digest(&bytes)));
        info.url = serve(bytes.clone(), "200 OK").await;
        download_installer(&http, &info, &dest).await.unwrap();
        assert_eq!(tokio::fs::read(&dest).await.unwrap(), bytes);
        tokio::fs::remove_file(&dest).await.unwrap();
        info.sha256 = Some("00".repeat(32));
        info.url = serve(bytes.clone(), "200 OK").await;
        assert!(download_installer(&http, &info, &dest).await.is_err());
        assert!(!dest.exists());
        assert!(!dest.with_extension("exe.part").exists());
        info.size += 1;
        info.url = serve(bytes, "200 OK").await;
        assert!(download_installer(&http, &info, &dest).await.is_err());
        assert!(!dest.exists());
        assert!(!dest.with_extension("exe.part").exists());
        tokio::fs::remove_dir(&dir).await.unwrap();
    }
    fn update(v: &str) -> UpdateInfo {
        UpdateInfo { version: v.into(), notes: String::new(), url: "https://panel.test/setup.exe".into(), size: 100, sha256: None }
    }
    #[tokio::test]
    async fn installer_without_checksum_cannot_be_downloaded_or_executed() {
        let dir = std::env::temp_dir().join(format!("velora-no-checksum-{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir(&dir).await.unwrap();
        let dest = dir.join("setup.exe");
        let mut info = update("99.0.0");
        for checksum in [None, Some("invalid".into())] {
            info.sha256 = checksum;
            let error = download_installer(&reqwest::Client::new(), &info, &dest).await.unwrap_err();
            assert!(error.to_string().contains("no valid checksum"));
            assert!(!dest.exists());
        }
        tokio::fs::remove_dir(dir).await.unwrap();
    }
    #[test]
    fn chooses_newest_source_and_survives_one_unavailable_source() {
        assert_eq!(select_update(Ok(Some(update("0.9.0"))), Ok(Some(update("0.10.0")))).unwrap().unwrap().version, "0.10.0");
        assert_eq!(select_update(Ok(Some(update("0.10.0"))), Ok(Some(update("0.9.0")))).unwrap().unwrap().version, "0.10.0");
        assert!(select_update(Ok(Some(update("0.10.0"))), Err(anyhow!("offline"))).unwrap().is_some());
        assert!(select_update(Err(anyhow!("offline")), Ok(None)).is_err());
    }
    #[test]
    fn release_api_follows_the_host() {
        let url = |s| reqwest::Url::parse(s).unwrap();
        assert_eq!(
            latest_release_api(&url("https://github.com/scopeddlol/SCOPENET-MC.git/")),
            "https://api.github.com/repos/scopeddlol/SCOPENET-MC/releases/latest"
        );
        assert_eq!(latest_release_api(&url("https://git.example.com/me/mc")), "https://git.example.com/api/v1/repos/me/mc/releases/latest");
    }
}
