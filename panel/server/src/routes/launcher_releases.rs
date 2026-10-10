//! Launcher releases built by GitHub Actions, published to players only after an admin approves them.
//!
//! The release workflow attaches `release-manifest.json`, listing every installer with its size, SHA-256
//! and an Ed25519 signature by the Velora release key. Approving downloads each installer, checks its size,
//! checksum and signature, then publishes it to the launcher update feeds and the website downloads.
use super::launcher_updates::{PublishedInstaller, KEY as WINDOWS_KEY, MAX_INSTALLER};
use crate::state::RequestState as State;
use crate::{
    auth::AdminUser,
    error::{AppError, AppResult},
    state::AppState,
    store,
};
use axum::{extract::Path, Json};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, time::Duration};
use tokio::io::AsyncWriteExt;
use velora_shared::{release_version, verify_release_signature, LauncherUpdate};

/// Desktop installers other than Windows, keyed by platform ("mac", "linux").
pub const PLATFORM_KEY: &str = "launcher_platform_releases";
const APPROVAL_KEY: &str = "launcher_release_approval";
const TAG_PREFIX: &str = "launcher-v";
const MANIFEST: &str = "release-manifest.json";

/// `owner/repo` whose releases are offered for approval.
pub fn release_repo() -> String {
    std::env::var("VELORA_LAUNCHER_RELEASES_REPO")
        .ok()
        .map(|r| r.trim().to_owned())
        .filter(|r| valid_repo(r))
        .unwrap_or_else(|| "VeloraMCDev/velora-launcher".into())
}
pub(super) fn valid_repo(repo: &str) -> bool {
    let mut parts = repo.split('/');
    let ok = |p: Option<&str>| {
        p.is_some_and(|p| !p.is_empty() && p.len() <= 100 && p.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b)))
    };
    ok(parts.next()) && ok(parts.next()) && parts.next().is_none()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ManifestAsset {
    /// "windows" | "mac" | "linux"
    pub platform: String,
    pub name: String,
    pub size: u64,
    pub sha256: String,
    pub signature: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseManifest {
    pub schema: u32,
    pub version: String,
    pub commit: String,
    pub assets: Vec<ManifestAsset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Approval {
    pub tag: String,
    pub version: String,
    pub commit: String,
    pub approved_at: String,
    pub approved_by: String,
    pub assets: Vec<ManifestAsset>,
}

#[derive(Serialize)]
pub struct ReleaseSummary {
    pub tag: String,
    pub version: String,
    pub name: String,
    pub notes: String,
    pub published_at: String,
    pub prerelease: bool,
    pub html_url: String,
    pub has_manifest: bool,
    pub approved: bool,
}
#[derive(Serialize)]
pub struct ReleasesView {
    pub repository: String,
    pub current: Option<Approval>,
    pub releases: Vec<ReleaseSummary>,
}

#[derive(Deserialize)]
pub(super) struct GithubRelease {
    pub(super) tag_name: String,
    #[serde(default)]
    pub(super) name: Option<String>,
    #[serde(default)]
    pub(super) body: Option<String>,
    #[serde(default)]
    pub(super) draft: bool,
    #[serde(default)]
    pub(super) prerelease: bool,
    #[serde(default)]
    pub(super) published_at: Option<String>,
    #[serde(default)]
    pub(super) html_url: String,
    #[serde(default)]
    pub(super) assets: Vec<GithubAsset>,
}
#[derive(Deserialize)]
pub(super) struct GithubAsset {
    pub(super) name: String,
    pub(super) browser_download_url: String,
    pub(super) size: u64,
    #[serde(default)]
    pub(super) digest: Option<String>,
}

pub(super) fn github(state: &AppState, url: &str) -> reqwest::RequestBuilder {
    state
        .http
        .get(url)
        .header("User-Agent", "velora-panel")
        .header("Accept", "application/vnd.github+json")
        .timeout(Duration::from_secs(20))
}

async fn github_releases(state: &AppState, repo: &str) -> AppResult<Vec<GithubRelease>> {
    github_releases_prefixed(state, repo, TAG_PREFIX).await
}

/// Published (non-draft) releases of `repo` whose tag starts with `prefix`.
pub(super) async fn github_releases_prefixed(state: &AppState, repo: &str, prefix: &str) -> AppResult<Vec<GithubRelease>> {
    let response = github(state, &format!("https://api.github.com/repos/{repo}/releases?per_page=30"))
        .send()
        .await
        .map_err(|e| AppError::bad_request(format!("GitHub is unreachable: {e}")))?;
    if !response.status().is_success() {
        return Err(AppError::bad_request(format!("GitHub returned {} for {repo}", response.status())));
    }
    let releases: Vec<GithubRelease> =
        response.json().await.map_err(|_| AppError::bad_request("GitHub sent an unexpected release list"))?;
    Ok(releases.into_iter().filter(|r| !r.draft && r.tag_name.starts_with(prefix)).collect())
}

/// `GET /api/admin/launcher/releases`: GitHub releases waiting for (or past) approval.
pub async fn list(_: AdminUser, State(state): State<AppState>) -> AppResult<Json<ReleasesView>> {
    let repository = release_repo();
    let current: Option<Approval> = store::kv_get(&state, APPROVAL_KEY).await?;
    let releases = github_releases(&state, &repository)
        .await?
        .into_iter()
        .map(|r| ReleaseSummary {
            version: r.tag_name.trim_start_matches(TAG_PREFIX).to_owned(),
            approved: current.as_ref().is_some_and(|c| c.tag == r.tag_name),
            has_manifest: r.assets.iter().any(|a| a.name == MANIFEST),
            name: r.name.unwrap_or_default(),
            notes: r.body.unwrap_or_default(),
            published_at: r.published_at.unwrap_or_default(),
            prerelease: r.prerelease,
            html_url: r.html_url,
            tag: r.tag_name,
        })
        .collect();
    Ok(Json(ReleasesView { repository, current, releases }))
}

/// Checks the manifest is well formed, matches its release and is signed by the Velora key throughout.
pub fn validate_manifest(manifest: &ReleaseManifest, tag: &str) -> Result<(), String> {
    let version = release_version(&manifest.version).ok_or("manifest version is not a release version")?.to_string();
    if manifest.schema != 1 || format!("{TAG_PREFIX}{version}") != tag {
        return Err(format!("manifest describes {} but the release is {tag}", manifest.version));
    }
    if manifest.assets.is_empty() || manifest.assets.len() > 12 {
        return Err("manifest must list between one and twelve installers".into());
    }
    let mut seen = std::collections::HashSet::new();
    for asset in &manifest.assets {
        if !["windows", "mac", "linux", "android", "ios"].contains(&asset.platform.as_str()) {
            return Err(format!("unknown platform {}", asset.platform));
        }
        if !installer_name_ok(&asset.platform, &asset.name) || !seen.insert(asset.name.clone()) {
            return Err(format!("unexpected installer name {}", asset.name));
        }
        if asset.size == 0 || asset.size > MAX_INSTALLER as u64 {
            return Err(format!("{} has an invalid size", asset.name));
        }
        if !verify_release_signature(&version, &asset.sha256, &asset.signature) {
            return Err(format!("{} is not signed by the Velora release key", asset.name));
        }
    }
    if !manifest.assets.iter().any(|a| a.platform == "windows") {
        return Err("the release has no Windows installer".into());
    }
    Ok(())
}
fn installer_name_ok(platform: &str, name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    let suffix_ok = match platform {
        "windows" => lower.ends_with("-setup.exe"),
        "mac" => lower.ends_with(".dmg"),
        "linux" => lower.ends_with(".appimage") || lower.ends_with(".deb"),
        "android" => lower.ends_with(".apk"),
        "ios" => lower.ends_with(".ipa"),
        _ => false,
    };
    suffix_ok && name.len() <= 160 && name.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_.+".contains(&b)) && !name.starts_with('.')
}

/// Streams one installer into the downloads directory, enforcing its size and checksum.
async fn fetch_installer(state: &AppState, asset: &GithubAsset, expected: &ManifestAsset, dest: &std::path::Path) -> Result<(), String> {
    fetch_verified(state, asset, expected.size, &expected.sha256, dest).await
}

/// Streams a release asset to `dest`, refusing anything whose size or SHA-256 differs from what was signed.
pub(super) async fn fetch_verified(
    state: &AppState,
    asset: &GithubAsset,
    size: u64,
    sha256: &str,
    dest: &std::path::Path,
) -> Result<(), String> {
    if asset.size != size {
        return Err(format!("{} on GitHub differs in size from the manifest", asset.name));
    }
    if let Some(digest) = asset.digest.as_deref().and_then(|d| d.strip_prefix("sha256:")) {
        if !digest.eq_ignore_ascii_case(sha256) {
            return Err(format!("GitHub's checksum for {} differs from the manifest", asset.name));
        }
    }
    let part = dest.with_extension("part");
    let result = async {
        let response = state
            .http
            .get(&asset.browser_download_url)
            .header("User-Agent", "velora-panel")
            .timeout(Duration::from_secs(600))
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|e| format!("downloading {}: {e}", asset.name))?;
        let mut file = tokio::fs::File::create(&part).await.map_err(|e| e.to_string())?;
        let mut hasher = Sha256::new();
        let mut written = 0u64;
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("downloading {}: {e}", asset.name))?;
            written += chunk.len() as u64;
            if written > size {
                return Err(format!("{} is larger than its manifest entry", asset.name));
            }
            hasher.update(&chunk);
            file.write_all(&chunk).await.map_err(|e| e.to_string())?;
        }
        file.flush().await.map_err(|e| e.to_string())?;
        if written != size || !hex::encode(hasher.finalize()).eq_ignore_ascii_case(sha256) {
            return Err(format!("{} failed checksum verification", asset.name));
        }
        tokio::fs::rename(&part, dest).await.map_err(|e| e.to_string())
    }
    .await;
    if result.is_err() {
        tokio::fs::remove_file(&part).await.ok();
    }
    result
}

/// Public URL of an approved installer. Windows keeps the path launchers already validate.
pub fn installer_url(platform: &str, digest: &str, name: &str) -> String {
    if platform == "windows" {
        format!("/api/v1/launcher/updates/{digest}/setup.exe")
    } else {
        format!("/api/v1/launcher/updates/{digest}/{name}")
    }
}
/// On-disk name of an approved installer: content-addressed, so a release never overwrites another.
pub fn stored_name(platform: &str, digest: &str, name: &str) -> String {
    if platform == "windows" {
        format!("{digest}-setup.exe")
    } else {
        format!("{digest}-{name}")
    }
}

/// `POST /api/admin/launcher/releases/{tag}/approve`: verify a GitHub release and publish it to players.
pub async fn approve(admin: AdminUser, State(state): State<AppState>, Path(tag): Path<String>) -> AppResult<Json<Approval>> {
    let repository = release_repo();
    let release = github_releases(&state, &repository)
        .await?
        .into_iter()
        .find(|r| r.tag_name == tag)
        .ok_or_else(|| AppError::not_found("That release is not on GitHub"))?;
    if release.prerelease {
        return Err(AppError::bad_request("Pre-releases can't be published to players"));
    }
    let manifest_asset = release
        .assets
        .iter()
        .find(|a| a.name == MANIFEST)
        .ok_or_else(|| AppError::bad_request("The release has no release-manifest.json"))?;
    if manifest_asset.size > 64 * 1024 {
        return Err(AppError::bad_request("release-manifest.json is too large"));
    }
    let manifest: ReleaseManifest = github(&state, &manifest_asset.browser_download_url)
        .header("Accept", "application/octet-stream")
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| AppError::bad_request(format!("Downloading the manifest failed: {e}")))?
        .json()
        .await
        .map_err(|_| AppError::bad_request("release-manifest.json is not valid"))?;
    validate_manifest(&manifest, &tag).map_err(AppError::bad_request)?;
    let version = release_version(&manifest.version).map(|v| v.to_string()).unwrap_or_default();

    let dir = state.cfg.downloads_dir();
    tokio::fs::create_dir_all(&dir).await?;
    for expected in &manifest.assets {
        let asset =
            release.assets.iter().find(|a| a.name == expected.name).ok_or_else(|| {
                AppError::bad_request(format!("{} is listed in the manifest but missing from the release", expected.name))
            })?;
        let prefix = format!("https://github.com/{repository}/releases/download/");
        if !asset.browser_download_url.starts_with(&prefix) {
            return Err(AppError::bad_request(format!("{} is not hosted by the release repository", asset.name)));
        }
        let dest = dir.join(stored_name(&expected.platform, &expected.sha256.to_ascii_lowercase(), &expected.name));
        if tokio::fs::metadata(&dest).await.is_ok_and(|m| m.len() == expected.size) {
            continue; // Already downloaded and verified when approved before.
        }
        fetch_installer(&state, asset, expected, &dest).await.map_err(AppError::bad_request)?;
    }
    // Check mobile archive structure before publishing any part of the release.
    for a in manifest.assets.iter().filter(|a| matches!(a.platform.as_str(), "android" | "ios")) {
        let file = dir.join(stored_name(&a.platform, &a.sha256.to_ascii_lowercase(), &a.name));
        let bytes = tokio::fs::read(file).await?;
        if !super::mobile_apps::looks_right(&a.platform, &bytes) {
            return Err(AppError::bad_request(format!("{} is not a valid mobile app archive", a.name)));
        }
    }

    let now = crate::db::now();
    let notes = release.body.clone().unwrap_or_default();
    for a in manifest.assets.iter().filter(|a| matches!(a.platform.as_str(), "android" | "ios")) {
        super::mobile_apps::publish_release_app(
            &state,
            &a.platform,
            super::mobile_apps::PublishedApp {
                version: version.clone(),
                notes: notes.chars().take(8192).collect(),
                filename: a.name.clone(),
                stored: stored_name(&a.platform, &a.sha256.to_ascii_lowercase(), &a.name),
                size: a.size,
                sha256: a.sha256.to_ascii_lowercase(),
                published_at: now.clone(),
                bundle_id: "net.scopenet.player".into(),
            },
        )
        .await?;
    }
    let update = |a: &ManifestAsset| LauncherUpdate {
        version: version.clone(),
        notes: notes.chars().take(8192).collect(),
        url: installer_url(&a.platform, &a.sha256.to_ascii_lowercase(), &a.name),
        size: a.size,
        sha256: Some(a.sha256.to_ascii_lowercase()),
        signature: Some(a.signature.clone()),
    };
    // Launchers update from the Windows setup, the macOS disk image and the Linux AppImage.
    let windows = manifest.assets.iter().find(|a| a.platform == "windows").expect("validated");
    store::kv_set(
        &state,
        WINDOWS_KEY,
        &Some(PublishedInstaller { update: update(windows), filename: windows.name.clone(), published_at: now.clone() }),
    )
    .await?;
    let mut platforms = BTreeMap::new();
    for (platform, suffix) in [("mac", ".dmg"), ("linux", ".appimage")] {
        if let Some(a) = manifest.assets.iter().find(|a| a.platform == platform && a.name.to_ascii_lowercase().ends_with(suffix)) {
            platforms
                .insert(platform.to_owned(), PublishedInstaller { update: update(a), filename: a.name.clone(), published_at: now.clone() });
        }
    }
    store::kv_set(&state, PLATFORM_KEY, &platforms).await?;

    // The website offers every approved installer, replacing earlier desktop downloads.
    let mut landing = super::landing::get_config(&state).await?;
    landing.hosted_downloads.retain(|d| !["windows", "mac", "linux"].contains(&d.platform.as_str()));
    for a in manifest.assets.iter().filter(|a| matches!(a.platform.as_str(), "windows" | "mac" | "linux")) {
        let label = match a.platform.as_str() {
            "windows" => "Windows",
            "mac" => "macOS",
            _ if a.name.to_ascii_lowercase().ends_with(".deb") => "Linux (.deb)",
            _ => "Linux (AppImage)",
        };
        landing.hosted_downloads.push(super::landing::HostedDownload {
            platform: a.platform.clone(),
            label: format!("{label} · {version}"),
            filename: a.name.clone(),
            file_url: installer_url(&a.platform, &a.sha256.to_ascii_lowercase(), &a.name),
            size: a.size as i64,
            uploaded_at: now.clone(),
            version: version.clone(),
            display_name: String::new(),
        });
    }
    // GitHub's latest release must not bypass approval on the website either.
    landing.external_download_url = None;
    store::kv_set(&state, "landing_page", &landing).await?;

    let approval =
        Approval { tag, version, commit: manifest.commit, approved_at: now, approved_by: admin.username.clone(), assets: manifest.assets };
    store::kv_set(&state, APPROVAL_KEY, &Some(approval.clone())).await?;
    tracing::info!("launcher release {} approved by {}", approval.tag, approval.approved_by);
    Ok(Json(approval))
}

/// Looks up the published installer a download path refers to.
pub async fn published_file(state: &AppState, digest: &str, name: &str) -> AppResult<Option<(String, std::path::PathBuf)>> {
    let platforms: BTreeMap<String, PublishedInstaller> = store::kv_get(state, PLATFORM_KEY).await?;
    let landing = super::landing::get_config(state).await?;
    let known = platforms.values().any(|p| p.update.sha256.as_deref() == Some(digest) && p.filename == name)
        || landing.hosted_downloads.iter().any(|d| d.file_url == format!("/api/v1/launcher/updates/{digest}/{name}"));
    if !known || !installer_name_ok(if name.to_ascii_lowercase().ends_with(".dmg") { "mac" } else { "linux" }, name) {
        return Ok(None);
    }
    Ok(Some((name.to_owned(), state.cfg.downloads_dir().join(format!("{digest}-{name}")))))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(assets: Vec<ManifestAsset>) -> ReleaseManifest {
        ReleaseManifest { schema: 1, version: "1.3.0".into(), commit: "c".repeat(40), assets }
    }
    fn asset(platform: &str, name: &str) -> ManifestAsset {
        ManifestAsset { platform: platform.into(), name: name.into(), size: 10, sha256: "ab".repeat(32), signature: "AA==".into() }
    }

    #[test]
    fn manifests_must_match_their_tag_and_be_signed() {
        let unsigned = manifest(vec![asset("windows", "Velora_1.3.0_x64-setup.exe")]);
        assert!(validate_manifest(&unsigned, "launcher-v1.3.1").unwrap_err().contains("release is"));
        assert!(validate_manifest(&unsigned, "launcher-v1.3.0").unwrap_err().contains("not signed"));
        assert!(validate_manifest(&manifest(vec![asset("windows", "../evil-setup.exe")]), "launcher-v1.3.0").is_err());
        assert!(validate_manifest(&manifest(vec![asset("mac", "Velora.dmg")]), "launcher-v1.3.0").is_err());
        assert!(validate_manifest(&manifest(vec![asset("bsd", "Velora.pkg")]), "launcher-v1.3.0").is_err());
    }

    #[test]
    fn installer_names_and_urls_keep_existing_launcher_paths() {
        assert!(installer_name_ok("windows", "Velora_1.3.0_x64-setup.exe"));
        assert!(!installer_name_ok("windows", "Velora_1.3.0_x64.msi"));
        assert!(installer_name_ok("linux", "velora_1.3.0_amd64.AppImage"));
        assert!(!installer_name_ok("linux", "a b.deb"));
        assert!(installer_name_ok("android", "Velora-Player_1.3.1_android.apk"));
        assert!(installer_name_ok("ios", "Velora-Player_1.3.1_ios.ipa"));
        assert!(!installer_name_ok("android", "Velora.ipa"));
        assert!(!installer_name_ok("ios", "Velora.apk"));
        let d = "ab".repeat(32);
        assert_eq!(installer_url("windows", &d, "x-setup.exe"), format!("/api/v1/launcher/updates/{d}/setup.exe"));
        assert_eq!(stored_name("mac", &d, "V.dmg"), format!("{d}-V.dmg"));
        assert!(valid_repo("VeloraMCDev/velora-launcher"));
        assert!(!valid_repo("a/b/c"));
        assert!(!valid_repo("owner/re po"));
    }
}
