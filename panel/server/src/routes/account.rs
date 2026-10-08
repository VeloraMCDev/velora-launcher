//! Player-facing account API used by the launcher: profile, skin, cape,
//! avatars and the authlib-injector mirror.

use crate::auth::account::{context, host_error, owner_error, response};
use crate::auth::{AuthUser, UserRow};
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::state::RequestState as State;
use crate::yggdrasil;
use axum::body::Body;
use axum::extract::{Multipart, Path, Query};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use scopenet_shared::PlayerProfile;
use serde::Deserialize;
use std::time::Duration;
use velora_auth_http::account as authority;

struct MutationHost(AppState);
impl authority::MutationHost for MutationHost {
    fn online<'a>(&'a self, uuid: &'a str) -> velora_auth_http::state::HostFuture<'a, Result<bool, velora_auth_http::error::AppError>> {
        Box::pin(async move {
            let cutoff = (chrono::Utc::now() - chrono::Duration::seconds(90)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
            sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM server_online o JOIN game_servers s ON s.id=o.server_id WHERE o.uuid=? AND s.last_seen>=?)",
            )
            .bind(uuid)
            .bind(cutoff)
            .fetch_one(&self.0.db)
            .await
            .map_err(|error| owner_error(error.into()))
        })
    }
    fn renamed<'a, 'c>(
        &'a self,
        tx: &'a mut sqlx::Transaction<'c, sqlx::Sqlite>,
        user: &'a UserRow,
        name: &'a str,
    ) -> velora_auth_http::state::HostFuture<'a, Result<(), velora_auth_http::error::AppError>> {
        Box::pin(async move {
            velora_auth_core::launcher_sessions::revoke(&mut **tx, user.id).await?;
            sqlx::query("INSERT INTO events (username, uuid, kind, detail, created_at) VALUES (?, ?, 'username_change', ?, ?)")
                .bind(name)
                .bind(&user.uuid)
                .bind(format!("{} → {name}", user.username))
                .bind(crate::db::now())
                .execute(&mut **tx)
                .await?;
            Ok(())
        })
    }
}

pub async fn profile(State(state): State<AppState>, headers: HeaderMap, AuthUser(user): AuthUser) -> AppResult<Json<PlayerProfile>> {
    Ok(Json(yggdrasil::profile_response(
        authority::profile(&crate::yggdrasil::context(&state), &headers, &user).await.map_err(host_error)?,
    )))
}

#[derive(Deserialize)]
pub struct UsernameInput {
    username: String,
    password: String,
}

pub async fn set_username(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(input): Json<UsernameInput>,
) -> AppResult<Json<scopenet_shared::AuthResponse>> {
    Ok(Json(response(
        authority::set_username(&context(&state), &MutationHost(state.clone()), &user, &input.username, &input.password)
            .await
            .map_err(host_error)?,
    )))
}

/// Read a `file` (+ optional `model`) multipart upload.
pub async fn read_texture_form(form: &mut Multipart) -> AppResult<(Vec<u8>, String)> {
    let mut model = String::from("classic");
    let mut file = None;
    while let Some(field) = form.next_field().await? {
        match field.name().unwrap_or_default() {
            "model" => model = field.text().await?,
            "file" => file = Some(field.bytes().await?.to_vec()),
            _ => {}
        }
    }
    Ok((file.ok_or_else(|| AppError::bad_request("no file uploaded"))?, model))
}

pub async fn upload_skin(
    State(state): State<AppState>,
    headers: HeaderMap,
    AuthUser(user): AuthUser,
    mut form: Multipart,
) -> AppResult<Json<PlayerProfile>> {
    let (bytes, model) = read_texture_form(&mut form).await?;
    Ok(Json(yggdrasil::profile_response(
        authority::upload_skin(&crate::yggdrasil::context(&state), &headers, &user, &bytes, &model).await.map_err(host_error)?,
    )))
}

#[derive(Deserialize)]
pub struct ModelInput {
    model: String,
}

pub async fn set_model(
    State(state): State<AppState>,
    headers: HeaderMap,
    AuthUser(user): AuthUser,
    Json(input): Json<ModelInput>,
) -> AppResult<Json<PlayerProfile>> {
    Ok(Json(yggdrasil::profile_response(
        authority::set_model(&crate::yggdrasil::context(&state), &headers, &user, &input.model).await.map_err(host_error)?,
    )))
}

pub async fn delete_skin(State(state): State<AppState>, headers: HeaderMap, AuthUser(user): AuthUser) -> AppResult<Json<PlayerProfile>> {
    Ok(Json(yggdrasil::profile_response(
        authority::delete_skin(&crate::yggdrasil::context(&state), &headers, &user).await.map_err(host_error)?,
    )))
}

#[derive(Deserialize)]
pub struct CapeInput {
    cape_id: Option<i64>,
}

pub async fn set_cape(
    State(state): State<AppState>,
    headers: HeaderMap,
    AuthUser(user): AuthUser,
    Json(input): Json<CapeInput>,
) -> AppResult<Json<PlayerProfile>> {
    Ok(Json(yggdrasil::profile_response(
        authority::set_cape(&crate::yggdrasil::context(&state), &headers, &user, input.cape_id).await.map_err(host_error)?,
    )))
}

#[derive(Deserialize)]
pub struct AvatarQuery {
    #[serde(default)]
    size: Option<u32>,
}

/// Player head render by UUID or name. 404 when the player has no skin
/// (callers show their own placeholder).
pub async fn avatar(State(state): State<AppState>, Path(id): Path<String>, Query(q): Query<AvatarQuery>) -> AppResult<Response> {
    let png = authority::avatar(&crate::yggdrasil::context(&state), &id, q.size).await.map_err(host_error)?;
    Ok(([(header::CONTENT_TYPE, "image/png"), (header::CACHE_CONTROL, "public, max-age=300")], Body::from(png)).into_response())
}

// ---------------------------------------------------------------------------
// authlib-injector mirror
// ---------------------------------------------------------------------------

fn mirror_dir(state: &AppState) -> std::path::PathBuf {
    state.cfg.data_dir.join("authlib-injector")
}

/// Refresh the cached authlib-injector from the official source at most
/// every six hours; serve the cache when the official site is unreachable.
async fn mirror_artifact(state: &AppState) -> AppResult<velora_platform_utils::authlib::Artifact> {
    use velora_platform_utils::authlib::{sha256_file, Artifact, OFFICIAL_LATEST};
    let dir = mirror_dir(state);
    let index = dir.join("latest.json");
    let fresh = std::fs::metadata(&index)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.elapsed().ok())
        .is_some_and(|age| age < Duration::from_secs(6 * 3600));
    let cached: Option<Artifact> = std::fs::read(&index).ok().and_then(|b| serde_json::from_slice(&b).ok());
    let jar_ok = |a: &Artifact| {
        sha256_file(&dir.join("authlib-injector.jar")).map(|h| h == a.checksums.sha256.to_ascii_lowercase()).unwrap_or(false)
    };
    if let Some(a) = cached.as_ref().filter(|a| fresh && jar_ok(a)) {
        return Ok(a.clone());
    }
    let fetched: anyhow::Result<Artifact> = async {
        let artifact: Artifact = velora_platform_utils::http::get_json(&state.http, OFFICIAL_LATEST).await?;
        if !jar_ok(&artifact) {
            let tmp = dir.join("authlib-injector.jar.download");
            velora_platform_utils::http::download_one(
                &state.http,
                &velora_platform_utils::http::Download::new(artifact.download_url.clone(), tmp.clone(), None, None),
                &|_| {},
            )
            .await?;
            if sha256_file(&tmp)? != artifact.checksums.sha256.to_ascii_lowercase() {
                std::fs::remove_file(&tmp).ok();
                anyhow::bail!("checksum mismatch");
            }
            std::fs::rename(&tmp, dir.join("authlib-injector.jar"))?;
        }
        std::fs::create_dir_all(&dir)?;
        std::fs::write(&index, serde_json::to_vec(&artifact)?)?;
        Ok(artifact)
    }
    .await;
    match (fetched, cached) {
        (Ok(a), _) => Ok(a),
        (Err(e), Some(a)) if jar_ok(&a) => {
            tracing::warn!("authlib-injector refresh failed, serving cached {}: {e:#}", a.version);
            Ok(a)
        }
        (Err(e), _) => Err(AppError::new(StatusCode::BAD_GATEWAY, format!("authlib-injector unavailable: {e:#}"))),
    }
}

pub async fn authlib_index(State(state): State<AppState>) -> AppResult<Json<velora_platform_utils::authlib::Artifact>> {
    let mut a = mirror_artifact(&state).await?;
    a.download_url = "/api/v1/launcher/authlib-injector.jar".into();
    Ok(Json(a))
}

pub async fn authlib_jar(State(state): State<AppState>) -> AppResult<Response> {
    mirror_artifact(&state).await?;
    let bytes = tokio::fs::read(mirror_dir(&state).join("authlib-injector.jar")).await?;
    Ok(([(header::CONTENT_TYPE, "application/java-archive")], Body::from(bytes)).into_response())
}
