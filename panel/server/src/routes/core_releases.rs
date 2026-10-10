//! Velora Core (the Minecraft server and client mod) releases, built by GitHub Actions and published to game
//! servers only after an admin approves them.
//!
//! The release workflow attaches `core-manifest.json`, listing the server and client jar with their size, SHA-256
//! and an Ed25519 signature by the Velora release key. Approving downloads both jars, checks size, checksum and
//! signature, and serves them as Admin Panel downloads for manual installation.
//! Approval does not deploy or replace mod jars.
use super::launcher_releases::{fetch_verified, github, github_releases_prefixed, release_repo, GithubAsset};
use crate::state::RequestState as State;
use crate::{
    auth::AdminUser,
    error::{AppError, AppResult},
    state::AppState,
    store,
};
use axum::{
    extract::Path,
    http::header,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use velora_shared::{release_version, verify_core_signature};

const APPROVAL_KEY: &str = "core_release_approval";
const HISTORY_KEY: &str = "core_release_history";
const TAG_PREFIX: &str = "velora-core-v";
const MANIFEST: &str = "core-manifest.json";
/// A jar larger than this is not a Velora Core build.
const MAX_JAR: u64 = 64 * 1024 * 1024;
/// Older approved releases kept downloadable so a server mid-update is never left with a dead link.
const HISTORY: usize = 3;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CoreAsset {
    /// "server" | "client"
    pub role: String,
    pub name: String,
    pub size: u64,
    pub sha256: String,
    pub signature: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreManifest {
    pub schema: u32,
    pub version: String,
    pub minecraft: String,
    pub commit: String,
    pub assets: Vec<CoreAsset>,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CoreApproval {
    pub tag: String,
    pub version: String,
    pub minecraft: String,
    pub commit: String,
    #[serde(default)]
    pub notes: String,
    pub approved_at: String,
    pub approved_by: String,
    pub assets: Vec<CoreAsset>,
}

#[derive(Serialize)]
pub struct CoreReleaseSummary {
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
pub struct CoreReleasesView {
    pub repository: String,
    pub current: Option<CoreApproval>,
    pub releases: Vec<CoreReleaseSummary>,
}

/// One downloadable jar. `path` is relative to the Panel's public address.
#[derive(Serialize)]
pub struct CoreFile {
    pub name: String,
    pub size: u64,
    pub sha256: String,
    pub signature: String,
    pub path: String,
}
#[derive(Serialize)]
pub struct CoreLatest {
    pub version: String,
    pub minecraft: String,
    pub notes: String,
    pub approved_at: String,
    pub server: CoreFile,
    pub client: CoreFile,
}

fn file_path(asset: &CoreAsset) -> String {
    format!("/api/v1/core/files/{}/{}", asset.sha256.to_ascii_lowercase(), asset.name)
}
fn core_file(asset: &CoreAsset) -> CoreFile {
    CoreFile {
        name: asset.name.clone(),
        size: asset.size,
        sha256: asset.sha256.to_ascii_lowercase(),
        signature: asset.signature.clone(),
        path: file_path(asset),
    }
}
/// On-disk name of an approved jar: content-addressed, so one release never overwrites another.
fn stored_name(sha256: &str, name: &str) -> String {
    format!("core-{}-{name}", sha256.to_ascii_lowercase())
}

/// The name a release's jar must have, so a jar can't be advertised under another role, version or Minecraft version.
fn expected_name(role: &str, minecraft: &str, version: &str) -> String {
    format!("velora-core-{role}-{minecraft}-{version}.jar")
}
fn minecraft_ok(value: &str) -> bool {
    (3..=12).contains(&value.len())
        && value.split('.').count() >= 2
        && value.split('.').all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
}

/// Checks the manifest is well formed, matches its release tag and is signed by the Velora key throughout.
pub fn validate_manifest(manifest: &CoreManifest, tag: &str) -> Result<(), String> {
    let version = release_version(&manifest.version).ok_or("manifest version is not a release version")?.to_string();
    if manifest.schema != 1 || format!("{TAG_PREFIX}{version}") != tag {
        return Err(format!("manifest describes {} but the release is {tag}", manifest.version));
    }
    if !minecraft_ok(&manifest.minecraft) {
        return Err("manifest has an invalid Minecraft version".into());
    }
    if manifest.assets.len() != 2 {
        return Err("manifest must list exactly a server and a client jar".into());
    }
    for role in ["server", "client"] {
        let asset = manifest.assets.iter().find(|a| a.role == role).ok_or_else(|| format!("manifest has no {role} jar"))?;
        if asset.name != expected_name(role, &manifest.minecraft, &version) {
            return Err(format!("unexpected {role} jar name {}", asset.name));
        }
        if asset.size == 0 || asset.size > MAX_JAR {
            return Err(format!("{} has an invalid size", asset.name));
        }
    }
    // Structure first, so a malformed manifest is reported as such rather than as a bad signature.
    for asset in &manifest.assets {
        if !verify_core_signature(&version, &asset.sha256, &asset.signature) {
            return Err(format!("{} is not signed by the Velora release key", asset.name));
        }
    }
    Ok(())
}

/// `GET /api/admin/core/releases`: GitHub releases waiting for (or past) approval.
pub async fn list(_: AdminUser, State(state): State<AppState>) -> AppResult<Json<CoreReleasesView>> {
    let repository = release_repo();
    let current: Option<CoreApproval> = store::kv_get(&state, APPROVAL_KEY).await?;
    let releases = github_releases_prefixed(&state, &repository, TAG_PREFIX)
        .await?
        .into_iter()
        .map(|r| CoreReleaseSummary {
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
    Ok(Json(CoreReleasesView { repository, current, releases }))
}

fn is_jar(bytes: &[u8]) -> bool {
    bytes.starts_with(b"PK\x03\x04")
}

/// `POST /api/admin/core/releases/{tag}/approve`: verify a GitHub release and serve it to game servers.
pub async fn approve(admin: AdminUser, State(state): State<AppState>, Path(tag): Path<String>) -> AppResult<Json<CoreApproval>> {
    let repository = release_repo();
    let release = github_releases_prefixed(&state, &repository, TAG_PREFIX)
        .await?
        .into_iter()
        .find(|r| r.tag_name == tag)
        .ok_or_else(|| AppError::not_found("That release is not on GitHub"))?;
    if release.prerelease {
        return Err(AppError::bad_request("Pre-releases can't be published to servers"));
    }
    let manifest_asset =
        release.assets.iter().find(|a| a.name == MANIFEST).ok_or_else(|| AppError::bad_request("The release has no core-manifest.json"))?;
    if manifest_asset.size > 16 * 1024 {
        return Err(AppError::bad_request("core-manifest.json is too large"));
    }
    let manifest: CoreManifest = github(&state, &manifest_asset.browser_download_url)
        .header("Accept", "application/octet-stream")
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| AppError::bad_request(format!("Downloading the manifest failed: {e}")))?
        .json()
        .await
        .map_err(|_| AppError::bad_request("core-manifest.json is not valid"))?;
    validate_manifest(&manifest, &tag).map_err(AppError::bad_request)?;
    let version = release_version(&manifest.version).map(|v| v.to_string()).unwrap_or_default();

    let dir = state.cfg.downloads_dir();
    tokio::fs::create_dir_all(&dir).await?;
    let prefix = format!("https://github.com/{repository}/releases/download/");
    for expected in &manifest.assets {
        let asset: &GithubAsset =
            release.assets.iter().find(|a| a.name == expected.name).ok_or_else(|| {
                AppError::bad_request(format!("{} is listed in the manifest but missing from the release", expected.name))
            })?;
        if !asset.browser_download_url.starts_with(&prefix) {
            return Err(AppError::bad_request(format!("{} is not hosted by the release repository", asset.name)));
        }
        let dest = dir.join(stored_name(&expected.sha256, &expected.name));
        if !tokio::fs::metadata(&dest).await.is_ok_and(|m| m.len() == expected.size) {
            fetch_verified(&state, asset, expected.size, &expected.sha256, &dest).await.map_err(AppError::bad_request)?;
        }
        let mut head = [0u8; 4];
        let mut file = tokio::fs::File::open(&dest).await?;
        tokio::io::AsyncReadExt::read_exact(&mut file, &mut head)
            .await
            .map_err(|_| AppError::bad_request(format!("{} is not a jar", expected.name)))?;
        if !is_jar(&head) {
            tokio::fs::remove_file(&dest).await.ok();
            return Err(AppError::bad_request(format!("{} is not a jar", expected.name)));
        }
    }

    let approval = CoreApproval {
        tag,
        version,
        minecraft: manifest.minecraft,
        commit: manifest.commit,
        notes: release.body.unwrap_or_default().chars().take(8192).collect(),
        approved_at: crate::db::now(),
        approved_by: admin.username.clone(),
        assets: manifest.assets,
    };
    let previous: Option<CoreApproval> = store::kv_get(&state, APPROVAL_KEY).await?;
    let mut history: Vec<CoreApproval> = store::kv_get(&state, HISTORY_KEY).await?;
    if let Some(previous) = previous.filter(|p| p.tag != approval.tag) {
        history.retain(|h| h.tag != previous.tag && h.tag != approval.tag);
        history.insert(0, previous);
    }
    history.truncate(HISTORY);
    store::kv_set(&state, HISTORY_KEY, &history).await?;
    store::kv_set(&state, APPROVAL_KEY, &Some(approval.clone())).await?;
    tracing::info!("velora core release {} approved by {}", approval.tag, approval.approved_by);
    Ok(Json(approval))
}

/// `GET /api/v1/core/latest`: the approved Velora Core release, or `null` before the first approval.
/// Public on purpose: the jars are public GitHub release assets, and servers authenticate to the Panel separately.
pub async fn latest(State(state): State<AppState>) -> AppResult<Response> {
    let current: Option<CoreApproval> = store::kv_get(&state, APPROVAL_KEY).await?;
    let body = current.and_then(|c| {
        let server = c.assets.iter().find(|a| a.role == "server")?;
        let client = c.assets.iter().find(|a| a.role == "client")?;
        Some(CoreLatest {
            version: c.version.clone(),
            minecraft: c.minecraft.clone(),
            notes: c.notes.clone(),
            approved_at: c.approved_at.clone(),
            server: core_file(server),
            client: core_file(client),
        })
    });
    Ok(([(header::CACHE_CONTROL, "no-store")], Json(body)).into_response())
}

/// `GET /api/v1/core/files/{digest}/{name}`: a jar belonging to an approved (current or recent) release.
pub async fn download(State(state): State<AppState>, Path((digest, name)): Path<(String, String)>) -> AppResult<Response> {
    if digest.len() != 64 || !digest.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(AppError::not_found("Jar not found"));
    }
    let digest = digest.to_ascii_lowercase();
    let current: Option<CoreApproval> = store::kv_get(&state, APPROVAL_KEY).await?;
    let history: Vec<CoreApproval> = store::kv_get(&state, HISTORY_KEY).await?;
    let known = current
        .iter()
        .chain(history.iter())
        .flat_map(|a| a.assets.iter())
        .any(|a| a.sha256.eq_ignore_ascii_case(&digest) && a.name == name);
    if !known {
        return Err(AppError::not_found("Jar not found"));
    }
    let file = tokio::fs::File::open(state.cfg.downloads_dir().join(stored_name(&digest, &name)))
        .await
        .map_err(|_| AppError::not_found("Jar not found"))?;
    super::launcher_updates::stream_file(file, &name).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asset(role: &str, name: &str) -> CoreAsset {
        CoreAsset { role: role.into(), name: name.into(), size: 10, sha256: "ab".repeat(32), signature: "AA==".into() }
    }
    fn manifest(assets: Vec<CoreAsset>) -> CoreManifest {
        CoreManifest { schema: 1, version: "0.6.0".into(), minecraft: "1.20.1".into(), commit: "c".repeat(40), assets }
    }
    fn good() -> Vec<CoreAsset> {
        vec![asset("server", "velora-core-server-1.20.1-0.6.0.jar"), asset("client", "velora-core-client-1.20.1-0.6.0.jar")]
    }

    #[test]
    fn manifests_must_match_their_tag_and_name_both_jars() {
        assert!(validate_manifest(&manifest(good()), "velora-core-v0.6.1").unwrap_err().contains("release is"));
        assert!(validate_manifest(&manifest(good()), "launcher-v0.6.0").is_err());
        assert!(validate_manifest(&manifest(vec![good().remove(0)]), "velora-core-v0.6.0").unwrap_err().contains("exactly"));
        let mut wrong_role = good();
        wrong_role[1].name = "velora-core-server-1.20.1-0.6.0.jar".into();
        assert!(validate_manifest(&manifest(wrong_role), "velora-core-v0.6.0").unwrap_err().contains("unexpected client jar name"));
        let mut evil = good();
        evil[0].name = "../velora-core-server-1.20.1-0.6.0.jar".into();
        assert!(validate_manifest(&manifest(evil), "velora-core-v0.6.0").is_err());
        let mut huge = good();
        huge[0].size = MAX_JAR + 1;
        assert!(validate_manifest(&manifest(huge), "velora-core-v0.6.0").is_err());
        let mut mc = manifest(good());
        mc.minecraft = "1.20.1/../x".into();
        assert!(validate_manifest(&mc, "velora-core-v0.6.0").unwrap_err().contains("Minecraft"));
    }

    #[test]
    fn unsigned_or_foreign_signatures_are_rejected() {
        // Structure is fine; only the signature is wrong, so this must fail on the signature check.
        assert!(validate_manifest(&manifest(good()), "velora-core-v0.6.0").unwrap_err().contains("not signed"));
    }

    #[test]
    fn names_paths_and_jar_checks() {
        assert_eq!(expected_name("server", "1.20.1", "0.6.0"), "velora-core-server-1.20.1-0.6.0.jar");
        assert!(minecraft_ok("1.20.1") && minecraft_ok("1.21"));
        assert!(!minecraft_ok("1") && !minecraft_ok("1..2") && !minecraft_ok("abc"));
        let a = asset("server", "velora-core-server-1.20.1-0.6.0.jar");
        assert_eq!(file_path(&a), format!("/api/v1/core/files/{}/velora-core-server-1.20.1-0.6.0.jar", "ab".repeat(32)));
        assert_eq!(stored_name(&"AB".repeat(32), "x.jar"), format!("core-{}-x.jar", "ab".repeat(32)));
        assert!(is_jar(b"PK\x03\x04rest") && !is_jar(b"MZ\x90\x00"));
    }
}
