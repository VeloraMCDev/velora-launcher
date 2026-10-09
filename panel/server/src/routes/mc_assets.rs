//! Minecraft's own item, block, HUD and effect textures for the panel and the public pages.
//!
//! Nothing from Mojang is committed to Velora or shipped in its images. The panel downloads the official vanilla client from
//! Mojang's servers (checking Mojang's SHA-1), or takes a client jar an admin uploads on a host with no internet, and copies the
//! textures out into `<data>/mc-textures/`. Until that has happened every endpoint answers 404 and the web apps keep their fallback
//! icons. The extraction and lookup rules are shared with the launcher (`velora_platform_utils::mc_textures`).

use crate::state::RequestState as State;
use crate::{
    auth::AdminUser,
    error::{AppError, AppResult},
    state::AppState,
};
use axum::{
    extract::{Multipart, Path},
    http::header,
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;
use velora_platform_utils::mc_textures as tex;

/// A client jar is ~30 MB; leave generous room for snapshots and modified clients.
pub const MAX_JAR: u64 = 256 * 1024 * 1024;

/// Only one download/extract at a time.
static BUSY: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn describe(state: &AppState) -> Value {
    let root = state.cfg.mc_textures_dir();
    let s = tex::status(&root);
    json!({
        "ready": s.ready,
        "count": s.count,
        "version": s.version,
        "source": tex::origin(&root),
        // Changes whenever the files do, so URLs carrying it can be cached forever.
        "v": format!("{}-{}", s.version.as_deref().unwrap_or("0"), s.count),
    })
}

/// `GET /api/v1/mc/status` — public; the web apps ask once and fall back to their own icons while `ready` is false.
pub async fn status(State(state): State<AppState>) -> Json<Value> {
    Json(describe(&state))
}

fn png(bytes: Vec<u8>) -> Response {
    ([(header::CONTENT_TYPE, "image/png"), (header::CACHE_CONTROL, "public, max-age=604800")], bytes).into_response()
}

/// `GET /api/v1/mc/{kind}/{path}.png` — public. `item` also answers for blocks that have no item texture (`stone`, `oak_log`…).
pub async fn texture(State(state): State<AppState>, Path((kind, file)): Path<(String, String)>) -> AppResult<Response> {
    let root = state.cfg.mc_textures_dir();
    let bytes = tokio::task::spawn_blocking(move || {
        if kind == "item" {
            let id = file.strip_suffix(".png")?;
            return tex::find_item(&root, id);
        }
        std::fs::read(tex::file(&root, &kind, &file)?).ok()
    })
    .await
    .map_err(|e| AppError::bad_request(e.to_string()))?;
    bytes.map(png).ok_or_else(|| AppError::not_found("No such texture"))
}

/// `GET /api/admin/mc-textures` — what is installed, plus the item ids the pickers can list.
pub async fn admin_get(_: AdminUser, State(state): State<AppState>) -> AppResult<Json<Value>> {
    let root = state.cfg.mc_textures_dir();
    let items = tokio::task::spawn_blocking(move || tex::item_names(&root)).await.map_err(|e| AppError::bad_request(e.to_string()))?;
    let mut out = describe(&state);
    out["items"] = json!(items);
    out["auto"] = json!(auto_enabled());
    Ok(Json(out))
}

#[derive(Deserialize, Default)]
pub struct FetchBody {
    /// A release id such as `1.21.1`; the newest release when empty.
    version: Option<String>,
}

/// Download the official client and extract it. Shared by the admin button and the startup check.
pub async fn fetch(state: &AppState, version: Option<&str>) -> anyhow::Result<String> {
    let _busy = BUSY.lock().await;
    let work = state.cfg.data_dir.join("mc-textures-work");
    let _ = tokio::fs::remove_dir_all(&work).await;
    tokio::fs::create_dir_all(&work).await?;
    let jar = work.join("client.jar");
    let result = async {
        let (id, size) = tex::fetch_client_jar(&state.http, version, &jar).await?;
        install(state.cfg.mc_textures_dir(), jar.clone(), id.clone(), size, "mojang").await?;
        anyhow::Ok(id)
    }
    .await;
    let _ = tokio::fs::remove_dir_all(&work).await;
    result
}

async fn install(root: PathBuf, jar: PathBuf, id: String, size: u64, origin: &'static str) -> anyhow::Result<()> {
    tokio::task::spawn_blocking(move || tex::ensure(&root, &id, &jar, size, Some(origin)).map(|_| ())).await?
}

/// `POST /api/admin/mc-textures/fetch`
pub async fn admin_fetch(_: AdminUser, State(state): State<AppState>, body: Option<Json<FetchBody>>) -> AppResult<Json<Value>> {
    let version = body.and_then(|Json(b)| b.version).map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
    if version.as_deref().is_some_and(|v| v.len() > 40 || !v.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_' | b' '))) {
        return Err(AppError::bad_request("Enter a Minecraft version such as 1.21.1"));
    }
    fetch(&state, version.as_deref()).await.map_err(|e| AppError::bad_request(format!("Couldn't download the client from Mojang: {e:#}. On a host without internet, upload a client.jar instead.")))?;
    Ok(Json(describe(&state)))
}

/// `POST /api/admin/mc-textures/upload` — multipart `file`: a vanilla `client.jar` (`.minecraft/versions/<v>/<v>.jar`).
pub async fn admin_upload(_: AdminUser, State(state): State<AppState>, mut form: Multipart) -> AppResult<Json<Value>> {
    let _busy = BUSY.lock().await;
    let work = state.cfg.data_dir.join("mc-textures-work");
    let _ = tokio::fs::remove_dir_all(&work).await;
    tokio::fs::create_dir_all(&work).await?;
    let jar = work.join("upload.jar");
    let result = async {
        let mut size = 0u64;
        let mut seen = false;
        while let Some(mut field) = form.next_field().await? {
            if field.name() != Some("file") || seen {
                return Err(AppError::bad_request("Upload one client.jar in the `file` field"));
            }
            seen = true;
            let mut out = tokio::fs::File::create(&jar).await?;
            while let Some(chunk) = field.chunk().await? {
                size += chunk.len() as u64;
                if size > MAX_JAR {
                    return Err(AppError::bad_request("That file is too large to be a Minecraft client"));
                }
                out.write_all(&chunk).await?;
            }
            out.flush().await?;
        }
        if size == 0 {
            return Err(AppError::bad_request("Choose a client.jar to upload"));
        }
        let probe = jar.clone();
        let id = tokio::task::spawn_blocking(move || tex::jar_version(&probe)).await.map_err(|e| AppError::bad_request(e.to_string()))?.unwrap_or_else(|| "unknown".into());
        install(state.cfg.mc_textures_dir(), jar.clone(), id, size, "upload").await.map_err(|e| AppError::bad_request(format!("{e:#}")))?;
        Ok(())
    }
    .await;
    let _ = tokio::fs::remove_dir_all(&work).await;
    result?;
    Ok(Json(describe(&state)))
}

/// `DELETE /api/admin/mc-textures`
pub async fn admin_clear(_: AdminUser, State(state): State<AppState>) -> AppResult<Json<Value>> {
    let _busy = BUSY.lock().await;
    let root = state.cfg.mc_textures_dir();
    tokio::task::spawn_blocking(move || tex::clear(&root)).await.map_err(|e| AppError::bad_request(e.to_string()))??;
    Ok(Json(describe(&state)))
}

fn auto_enabled() -> bool {
    let read = |k: &str| std::env::var(k).ok().map(|v| v.trim().to_ascii_lowercase());
    let v = read("VELORA_MC_TEXTURES").or_else(|| read("SCOPENET_MC_TEXTURES"));
    !matches!(v.as_deref(), Some("off" | "0" | "false" | "no"))
}

/// At startup, fetch the textures once if they are missing. Offline hosts just log and wait for an upload.
/// Set `VELORA_MC_TEXTURES=off` to never contact Mojang on its own.
pub async fn ensure_on_startup(state: AppState) {
    if !auto_enabled() || tex::status(&state.cfg.mc_textures_dir()).ready {
        return;
    }
    match fetch(&state, None).await {
        Ok(id) => tracing::info!("downloaded Minecraft {id} textures"),
        Err(e) => tracing::warn!("couldn't download Minecraft textures ({e:#}); upload a client.jar in the admin panel to enable them"),
    }
}
