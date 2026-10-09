//! Admin-published installers, advertised to launchers without account credentials.
use crate::state::RequestState as State;
use crate::{
    auth::AdminUser,
    error::{AppError, AppResult},
    state::AppState,
    store,
};
use axum::{
    body::Body,
    extract::{Multipart, Path, Query},
    http::header,
    response::{IntoResponse, Response},
    Json,
};
use scopenet_shared::{release_version, LauncherUpdate};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::io::AsyncReadExt;

pub const MAX_INSTALLER: usize = 256 * 1024 * 1024;
pub const KEY: &str = "launcher_installer_release";

#[derive(Clone, Serialize, Deserialize)]
pub struct PublishedInstaller {
    #[serde(flatten)]
    pub update: LauncherUpdate,
    pub filename: String,
    pub published_at: String,
}

pub async fn admin_get(_: AdminUser, State(state): State<AppState>) -> AppResult<Json<Option<PublishedInstaller>>> {
    Ok(Json(store::kv_get(&state, KEY).await?))
}
/// Take the hosted Windows installer off the website and out of the launcher's update feed.
pub async fn admin_remove(_: AdminUser, State(state): State<AppState>) -> AppResult<Json<Option<PublishedInstaller>>> {
    let old: Option<PublishedInstaller> = store::kv_get(&state, KEY).await?;
    store::kv_set(&state, KEY, &None::<PublishedInstaller>).await?;
    let mut landing = super::landing::get_config(&state).await?;
    landing.hosted_downloads.retain(|d| d.platform != "windows");
    store::kv_set(&state, "landing_page", &landing).await?;
    if let Some(url) = old.and_then(|o| o.update.sha256) {
        tokio::fs::remove_file(state.cfg.downloads_dir().join(format!("{url}-setup.exe"))).await.ok();
    }
    Ok(Json(None))
}
#[derive(Deserialize)]
pub struct FeedQuery {
    platform: Option<String>,
}
/// `GET /api/v1/launcher/update[?platform=windows|macos|linux]`. Launchers that predate the
/// parameter are Windows builds, so its absence means Windows.
pub async fn latest(State(state): State<AppState>, Query(q): Query<FeedQuery>) -> AppResult<Response> {
    let published: Option<PublishedInstaller> = match q.platform.as_deref().unwrap_or("windows") {
        "windows" => store::kv_get(&state, KEY).await?,
        platform @ ("mac" | "macos" | "linux") => {
            let platforms: std::collections::BTreeMap<String, PublishedInstaller> =
                store::kv_get(&state, super::launcher_releases::PLATFORM_KEY).await?;
            platforms.get(if platform == "linux" { "linux" } else { "mac" }).cloned()
        }
        _ => return Err(AppError::bad_request("Unknown platform")),
    };
    Ok(([(header::CACHE_CONTROL, "no-store")], Json(published.map(|p| p.update))).into_response())
}

async fn text_field(mut field: axum::extract::multipart::Field<'_>, limit: usize) -> AppResult<String> {
    let mut bytes = Vec::new();
    while let Some(chunk) = field.chunk().await? {
        if bytes.len().saturating_add(chunk.len()) > limit {
            return Err(AppError::bad_request("Installer metadata is too long"));
        }
        bytes.extend_from_slice(&chunk);
    }
    String::from_utf8(bytes).map_err(|_| AppError::bad_request("Installer metadata must be UTF-8 text"))
}

pub async fn upload(_: AdminUser, State(state): State<AppState>, mut form: Multipart) -> AppResult<Json<PublishedInstaller>> {
    let mut version = String::new();
    let mut notes = String::new();
    let mut name = String::new();
    let mut bytes = Vec::new();
    let mut seen_file = false;
    let limit = MAX_INSTALLER.min(state.cfg.max_upload_mb.saturating_mul(1024 * 1024));
    while let Some(mut field) = form.next_field().await? {
        match field.name().unwrap_or("") {
            "version" => {
                version = text_field(field, 128).await?;
            }
            "notes" => {
                notes = text_field(field, 8192).await?;
            }
            "file" => {
                if seen_file {
                    return Err(AppError::bad_request("Upload one installer at a time"));
                }
                seen_file = true;
                name = field.file_name().unwrap_or("").to_owned();
                while let Some(chunk) = field.chunk().await? {
                    if bytes.len().saturating_add(chunk.len()) > limit {
                        return Err(AppError::bad_request("Installer exceeds the upload limit (at most 256 MB)"));
                    }
                    bytes.extend_from_slice(&chunk);
                }
            }
            _ => return Err(AppError::bad_request("Unsupported installer field")),
        }
    }
    let version =
        release_version(&version).ok_or_else(|| AppError::bad_request("Enter the release version, for example 0.10.0"))?.to_string();
    if name.len() > 180 || name.contains(['/', '\\', '\r', '\n']) || !name.to_lowercase().ends_with(".exe") || !is_pe(&bytes) {
        return Err(AppError::bad_request("Upload a Windows Setup.exe installer"));
    }
    let digest = hex::encode(Sha256::digest(&bytes));
    let dir = state.cfg.downloads_dir();
    tokio::fs::create_dir_all(&dir).await?;
    let filename = format!("{digest}-setup.exe");
    let temp = dir.join(format!("{}.part", uuid::Uuid::new_v4()));
    tokio::fs::write(&temp, &bytes).await?;
    if let Err(error) = tokio::fs::rename(&temp, dir.join(&filename)).await {
        tokio::fs::remove_file(&temp).await.ok();
        return Err(error.into());
    }
    let published_at = crate::db::now();
    let published = PublishedInstaller {
        update: LauncherUpdate {
            version,
            notes,
            url: format!("/api/v1/launcher/updates/{digest}/setup.exe"),
            size: bytes.len() as u64,
            sha256: Some(digest),
            // Manual uploads are unsigned: launchers that verify release signatures ignore them.
            signature: None,
        },
        filename: name,
        published_at: published_at.clone(),
    };
    store::kv_set(&state, KEY, &Some(published.clone())).await?;
    // Keep the existing public Windows download button useful as well.
    let mut landing = super::landing::get_config(&state).await?;
    landing.hosted_downloads.retain(|d| d.platform != "windows");
    landing.hosted_downloads.push(super::landing::HostedDownload {
        platform: "windows".into(),
        label: format!("Windows · {}", published.update.version),
        filename,
        file_url: published.update.url.clone(),
        size: bytes.len() as i64,
        uploaded_at: published_at,
        version: published.update.version.clone(),
        display_name: String::new(),
    });
    store::kv_set(&state, "landing_page", &landing).await?;
    Ok(Json(published))
}

fn is_pe(bytes: &[u8]) -> bool {
    if bytes.len() < 68 || !bytes.starts_with(b"MZ") {
        return false;
    }
    let offset = u32::from_le_bytes(bytes[60..64].try_into().unwrap()) as usize;
    offset >= 64 && offset.checked_add(4).is_some_and(|end| bytes.get(offset..end) == Some(b"PE\0\0"))
}

/// `GET /api/v1/launcher/updates/{digest}/{name}`: an approved macOS or Linux installer.
pub async fn download_file(State(state): State<AppState>, Path((digest, name)): Path<(String, String)>) -> AppResult<Response> {
    if digest.len() != 64 || !digest.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(AppError::not_found("Installer not found"));
    }
    let (name, path) =
        super::launcher_releases::published_file(&state, &digest, &name).await?.ok_or_else(|| AppError::not_found("Installer not found"))?;
    let file = tokio::fs::File::open(path).await.map_err(|_| AppError::not_found("Installer not found"))?;
    stream_file(file, &name).await
}

pub async fn download(State(state): State<AppState>, Path(digest): Path<String>) -> AppResult<Response> {
    if digest.len() != 64 || !digest.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(AppError::not_found("Installer not found"));
    }
    let file = tokio::fs::File::open(state.cfg.downloads_dir().join(format!("{digest}-setup.exe")))
        .await
        .map_err(|_| AppError::not_found("Installer not found"))?;
    // The file on disk is named after its checksum; players get a tidy name.
    let published: Option<PublishedInstaller> = store::kv_get(&state, KEY).await?;
    let brand = store::branding(&state).await.map(|b| b.name).unwrap_or_default();
    let (version, original) = published.map(|p| (p.update.version, p.filename)).unwrap_or_default();
    let name = super::landing::short_name(&brand, "windows", &original, "", &version);
    let name = if name.to_ascii_lowercase().ends_with(".exe") { name } else { format!("{name}.exe") };
    stream_file(file, &name).await
}

async fn stream_file(file: tokio::fs::File, name: &str) -> AppResult<Response> {
    let size = file.metadata().await?.len();
    let disposition = format!("attachment; filename=\"{name}\"");
    let stream = futures::stream::try_unfold(file, |mut file| async {
        let mut bytes = vec![0; 64 * 1024];
        let n = file.read(&mut bytes).await?;
        if n == 0 {
            Ok::<_, std::io::Error>(None)
        } else {
            bytes.truncate(n);
            Ok(Some((bytes, file)))
        }
    });
    Ok((
        [
            (header::CONTENT_TYPE, "application/octet-stream"),
            (header::CONTENT_DISPOSITION, disposition.as_str()),
            (header::CACHE_CONTROL, "public, max-age=31536000, immutable"),
            (header::CONTENT_LENGTH, &size.to_string()),
        ],
        Body::from_stream(stream),
    )
        .into_response())
}
