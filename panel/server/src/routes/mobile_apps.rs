//! Phone apps published by the admin: an Android `.apk` and an iOS `.ipa`, offered to players on the website, plus an
//! AltStore-compatible source so the (unsigned or self-signed) `.ipa` can be added to AltStore / SideStore.
use crate::state::RequestState as State;
use crate::{
    auth::AdminUser,
    error::{AppError, AppResult},
    net,
    state::AppState,
    store,
};
use axum::{
    body::Body,
    extract::{Multipart, Path},
    http::{header, HeaderMap},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::Cursor;
use tokio::io::AsyncReadExt;
use velora_shared::release_version;

const KEY: &str = "mobile_apps";
const MAX_APP: usize = 512 * 1024 * 1024;
const DEFAULT_BUNDLE_ID: &str = "net.scopenet.player";

#[derive(Clone, Serialize, Deserialize)]
pub struct PublishedApp {
    pub version: String,
    pub notes: String,
    /// The name the admin uploaded, kept for display.
    pub filename: String,
    /// The name on disk in the downloads folder.
    pub stored: String,
    pub size: u64,
    pub sha256: String,
    pub published_at: String,
    pub bundle_id: String,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Apps {
    pub android: Option<PublishedApp>,
    pub ios: Option<PublishedApp>,
}

impl Apps {
    fn get(&self, platform: &str) -> Option<&PublishedApp> {
        match platform {
            "android" => self.android.as_ref(),
            "ios" => self.ios.as_ref(),
            _ => None,
        }
    }
}

fn ext(platform: &str) -> &'static str {
    if platform == "android" {
        "apk"
    } else {
        "ipa"
    }
}

fn view(app: &Option<PublishedApp>, platform: &str, base: &str) -> Value {
    match app {
        None => Value::Null,
        Some(a) => {
            let url = format!("/api/v1/mobile-apps/{platform}/download");
            let mut v = json!({
                "version": a.version, "notes": a.notes, "filename": a.filename, "size": a.size, "sha256": a.sha256,
                "published_at": a.published_at, "bundle_id": a.bundle_id, "url": url, "absolute_url": format!("{base}{url}"),
            });
            if platform == "ios" {
                let source = format!("{base}/api/v1/mobile-apps/altstore.json");
                v["altstore_url"] = json!(source);
                v["altstore_link"] = json!(format!("altstore://source?url={source}"));
            }
            v
        }
    }
}

async fn load(state: &AppState) -> AppResult<Apps> {
    store::kv_get(state, KEY).await
}

fn describe(apps: &Apps, base: &str) -> Value {
    json!({ "android": view(&apps.android, "android", base), "ios": view(&apps.ios, "ios", base) })
}

pub async fn admin_get(_: AdminUser, State(state): State<AppState>, headers: HeaderMap) -> AppResult<Json<Value>> {
    let base = net::public_base(&state, &headers).await;
    Ok(Json(describe(&load(&state).await?, &base)))
}

pub async fn public_get(State(state): State<AppState>, headers: HeaderMap) -> AppResult<Response> {
    let base = net::public_base(&state, &headers).await;
    Ok(([(header::CACHE_CONTROL, "no-store")], Json(describe(&load(&state).await?, &base))).into_response())
}

async fn text_field(mut field: axum::extract::multipart::Field<'_>, limit: usize) -> AppResult<String> {
    let mut bytes = Vec::new();
    while let Some(chunk) = field.chunk().await? {
        if bytes.len().saturating_add(chunk.len()) > limit {
            return Err(AppError::bad_request("A field is too long"));
        }
        bytes.extend_from_slice(&chunk);
    }
    String::from_utf8(bytes).map_err(|_| AppError::bad_request("Fields must be UTF-8 text"))
}

fn valid_bundle_id(id: &str) -> bool {
    id.len() <= 155 && id.contains('.') && id.split('.').all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'))
}

/// An `.apk` has `AndroidManifest.xml` at its root; an `.ipa` has `Payload/<name>.app/Info.plist`.
pub(crate) fn looks_right(platform: &str, bytes: &[u8]) -> bool {
    let Ok(zip) = zip::ZipArchive::new(Cursor::new(bytes)) else { return false };
    let names: Vec<&str> = zip.file_names().collect();
    if platform == "android" {
        names.contains(&"AndroidManifest.xml")
    } else {
        names.iter().any(|n| n.starts_with("Payload/") && n.ends_with(".app/Info.plist"))
    }
}

pub async fn upload(_: AdminUser, State(state): State<AppState>, headers: HeaderMap, mut form: Multipart) -> AppResult<Json<Value>> {
    let (mut platform, mut version, mut notes, mut bundle_id) = (String::new(), String::new(), String::new(), String::new());
    let (mut name, mut bytes, mut seen) = (String::new(), Vec::new(), false);
    let limit = MAX_APP.min(state.cfg.max_upload_mb.saturating_mul(1024 * 1024));
    while let Some(mut field) = form.next_field().await? {
        match field.name().unwrap_or("") {
            "platform" => platform = text_field(field, 16).await?,
            "version" => version = text_field(field, 128).await?,
            "notes" => notes = text_field(field, 8192).await?,
            "bundle_id" => bundle_id = text_field(field, 200).await?,
            "file" => {
                if seen {
                    return Err(AppError::bad_request("Upload one app at a time"));
                }
                seen = true;
                name = field.file_name().unwrap_or("").to_owned();
                while let Some(chunk) = field.chunk().await? {
                    if bytes.len().saturating_add(chunk.len()) > limit {
                        return Err(AppError::bad_request("The app exceeds the upload limit"));
                    }
                    bytes.extend_from_slice(&chunk);
                }
            }
            _ => return Err(AppError::bad_request("Unsupported field")),
        }
    }
    if platform != "android" && platform != "ios" {
        return Err(AppError::bad_request("platform must be android or ios"));
    }
    let version = release_version(&version).ok_or_else(|| AppError::bad_request("Enter the app version, for example 0.9.4"))?.to_string();
    if name.len() > 180 || name.contains(['/', '\\', '\r', '\n']) || !name.to_lowercase().ends_with(&format!(".{}", ext(&platform))) {
        return Err(AppError::bad_request(if platform == "android" { "Upload an Android .apk file" } else { "Upload an iOS .ipa file" }));
    }
    if !looks_right(&platform, &bytes) {
        return Err(AppError::bad_request(if platform == "android" {
            "That file is not a valid Android app (.apk)"
        } else {
            "That file is not a valid iOS app (.ipa)"
        }));
    }
    let bundle_id = if bundle_id.trim().is_empty() { DEFAULT_BUNDLE_ID.to_string() } else { bundle_id.trim().to_string() };
    if !valid_bundle_id(&bundle_id) {
        return Err(AppError::bad_request("The bundle identifier looks like net.example.app"));
    }
    let digest = hex::encode(Sha256::digest(&bytes));
    let dir = state.cfg.downloads_dir();
    tokio::fs::create_dir_all(&dir).await?;
    let stored = format!("app-{platform}-{digest}.{}", ext(&platform));
    let temp = dir.join(format!("{}.part", uuid::Uuid::new_v4()));
    tokio::fs::write(&temp, &bytes).await?;
    if let Err(error) = tokio::fs::rename(&temp, dir.join(&stored)).await {
        tokio::fs::remove_file(&temp).await.ok();
        return Err(error.into());
    }
    let published_at = crate::db::now();
    let app = PublishedApp {
        version,
        notes,
        filename: name,
        stored: stored.clone(),
        size: bytes.len() as u64,
        sha256: digest,
        published_at: published_at.clone(),
        bundle_id,
    };
    let mut apps = load(&state).await?;
    let previous = if platform == "android" { apps.android.replace(app.clone()) } else { apps.ios.replace(app.clone()) };
    store::kv_set(&state, KEY, &apps).await?;
    if let Some(old) = previous.filter(|o| o.stored != stored) {
        tokio::fs::remove_file(dir.join(old.stored)).await.ok();
    }
    set_landing(&state, &platform, Some((&app, published_at))).await?;
    let base = net::public_base(&state, &headers).await;
    Ok(Json(describe(&apps, &base)))
}

/// Publishes a release-key-verified archive already downloaded by the approval route.
pub(crate) async fn publish_release_app(state: &AppState, platform: &str, app: PublishedApp) -> AppResult<()> {
    let mut apps = load(state).await?;
    match platform {
        "android" => apps.android = Some(app.clone()),
        "ios" => apps.ios = Some(app.clone()),
        _ => return Err(AppError::bad_request("Unknown mobile platform")),
    }
    store::kv_set(state, KEY, &apps).await?;
    set_landing(state, platform, Some((&app, app.published_at.clone()))).await
}

pub async fn remove(
    _: AdminUser,
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(platform): Path<String>,
) -> AppResult<Json<Value>> {
    let mut apps = load(&state).await?;
    let old = match platform.as_str() {
        "android" => apps.android.take(),
        "ios" => apps.ios.take(),
        _ => return Err(AppError::not_found("Unknown platform")),
    };
    store::kv_set(&state, KEY, &apps).await?;
    if let Some(old) = old {
        tokio::fs::remove_file(state.cfg.downloads_dir().join(old.stored)).await.ok();
    }
    set_landing(&state, &platform, None).await?;
    let base = net::public_base(&state, &headers).await;
    Ok(Json(describe(&apps, &base)))
}

/// Keeps the public landing page's download list in step.
async fn set_landing(state: &AppState, platform: &str, app: Option<(&PublishedApp, String)>) -> AppResult<()> {
    let mut landing = super::landing::get_config(state).await?;
    landing.hosted_downloads.retain(|d| d.platform != platform);
    if let Some((app, at)) = app {
        landing.hosted_downloads.push(super::landing::HostedDownload {
            platform: platform.into(),
            label: format!("{} · {}", if platform == "android" { "Android" } else { "iOS" }, app.version),
            filename: app.filename.clone(),
            file_url: format!("/api/v1/mobile-apps/{platform}/download"),
            size: app.size as i64,
            uploaded_at: at,
            version: app.version.clone(),
            display_name: String::new(),
        });
    }
    store::kv_set(state, "landing_page", &landing).await
}

pub async fn download(State(state): State<AppState>, Path(platform): Path<String>) -> AppResult<Response> {
    let apps = load(&state).await?;
    let app = apps.get(&platform).ok_or_else(|| AppError::not_found("That app has not been published"))?;
    let file = tokio::fs::File::open(state.cfg.downloads_dir().join(&app.stored))
        .await
        .map_err(|_| AppError::not_found("That app has not been published"))?;
    let size = file.metadata().await?.len();
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
    let brand = store::branding(&state).await.map(|b| b.name).unwrap_or_default();
    let mime = if platform == "android" { "application/vnd.android.package-archive" } else { "application/octet-stream" };
    let filename = super::landing::short_name(&brand, &platform, &format!("app.{}", ext(&platform)), "", &app.version);
    Ok((
        [
            (header::CONTENT_TYPE, mime.to_string()),
            (header::CONTENT_DISPOSITION, format!("attachment; filename=\"{filename}\"")),
            (header::CACHE_CONTROL, "no-cache".to_string()),
            (header::CONTENT_LENGTH, size.to_string()),
        ],
        Body::from_stream(stream),
    )
        .into_response())
}

/// The AltStore "source" for the iOS app: add `<panel>/api/v1/mobile-apps/altstore.json` in AltStore or SideStore.
pub async fn altstore(State(state): State<AppState>, headers: HeaderMap) -> AppResult<Response> {
    let base = net::public_base(&state, &headers).await;
    let branding = store::branding(&state).await?;
    let apps = load(&state).await?;
    let source = format!("{base}/api/v1/mobile-apps/altstore.json");
    let name = branding.name.clone();
    let icon = branding.icon_url.clone().or(branding.logo_url.clone()).map(|u| if u.starts_with('/') { format!("{base}{u}") } else { u });
    let list: Vec<Value> = apps
        .ios
        .iter()
        .map(|a| {
            let url = format!("{base}/api/v1/mobile-apps/ios/download");
            let date = a.published_at.clone();
            json!({
                "name": format!("{name} Player"),
                "bundleIdentifier": a.bundle_id,
                "developerName": name,
                "subtitle": "Market, casino, friends and more",
                "localizedDescription": format!("The {name} player panel as an app: market, casino, friends, factions, quests and leaderboards."),
                "iconURL": icon,
                "tintColor": branding.colors.accent.trim_start_matches('#'),
                "screenshotURLs": [],
                "version": a.version, "versionDate": date, "versionDescription": a.notes,
                "downloadURL": url, "size": a.size,
                "versions": [{ "version": a.version, "date": a.published_at, "localizedDescription": a.notes, "downloadURL": url, "size": a.size }],
            })
        })
        .collect();
    Ok((
        [(header::CONTENT_TYPE, "application/json"), (header::CACHE_CONTROL, "no-cache"), (header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")],
        Json(json!({ "name": format!("{name} Apps"), "identifier": "net.scopenet.source", "sourceURL": source, "apps": list, "news": [] })),
    )
        .into_response())
}
