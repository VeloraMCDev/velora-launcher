//! Player-facing account API used by the launcher: profile, skin, cape,
//! avatars and the authlib-injector mirror.

use crate::state::RequestState as State;
use crate::auth::{AuthUser, UserRow};
use crate::error::{AppError, AppResult};
use crate::net;
use crate::state::AppState;
use crate::textures;
use crate::yggdrasil;
use axum::body::Body;
use axum::extract::{Multipart, Path, Query};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use scopenet_shared::PlayerProfile;
use serde::Deserialize;
use std::time::Duration;

pub async fn profile(State(state): State<AppState>, headers: HeaderMap, AuthUser(user): AuthUser) -> AppResult<Json<PlayerProfile>> {
    let base = net::public_base(&state, &headers).await;
    Ok(Json(yggdrasil::player_profile(&state, &base, &user).await?))
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
    let name = input.username.trim();
    if !scopenet_shared::valid_username(name) {
        return Err(AppError::bad_request("usernames are 3–16 letters, numbers or underscores"));
    }
    crate::store::check_username(&state, name).await?;
    state.login_guard.check(&user.username)?;
    if !crate::auth::verify_password(&input.password, &user.password_hash) {
        state.login_guard.fail(&user.username);
        return Err(AppError::unauthorized("incorrect password"));
    }
    state.login_guard.succeed(&user.username);
    let cutoff = (chrono::Utc::now() - chrono::Duration::seconds(90)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let online: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM server_online o JOIN game_servers s ON s.id=o.server_id WHERE o.uuid=? AND s.last_seen>=?)",
    )
    .bind(&user.uuid)
    .bind(cutoff)
    .fetch_one(&state.db)
    .await?;
    if online {
        return Err(AppError::conflict("disconnect from your servers before changing your username"));
    }
    let mut tx = state.db.begin().await?;
    sqlx::query("UPDATE users SET username=?, auth_version=auth_version+1 WHERE id=?")
        .bind(name)
        .bind(user.id)
        .execute(&mut *tx)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(d) if d.message().contains("UNIQUE") || d.message().contains("username reserved") => {
                AppError::conflict("that username is taken or reserved")
            }
            e => e.into(),
        })?;
    sqlx::query("DELETE FROM ygg_tokens WHERE user_id=?").bind(user.id).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM ygg_sessions WHERE user_id=?").bind(user.id).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM launcher_sessions WHERE user_id=?").bind(user.id).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO events (username, uuid, kind, detail, created_at) VALUES (?, ?, 'username_change', ?, ?)")
        .bind(name)
        .bind(&user.uuid)
        .bind(format!("{} → {name}", user.username))
        .bind(crate::db::now())
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Json(super::public::signed_in(&state, &reload(&state, user.id).await?).await?))
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

async fn reload(state: &AppState, id: i64) -> AppResult<UserRow> {
    Ok(sqlx::query_as("SELECT * FROM users WHERE id = ?").bind(id).fetch_one(&state.db).await?)
}

pub async fn upload_skin(
    State(state): State<AppState>,
    headers: HeaderMap,
    AuthUser(user): AuthUser,
    mut form: Multipart,
) -> AppResult<Json<PlayerProfile>> {
    let (bytes, model) = read_texture_form(&mut form).await?;
    yggdrasil::set_skin(&state, user.id, &bytes, &model).await?;
    let base = net::public_base(&state, &headers).await;
    Ok(Json(yggdrasil::player_profile(&state, &base, &reload(&state, user.id).await?).await?))
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
    let model = if input.model == "slim" { "slim" } else { "classic" };
    sqlx::query("UPDATE users SET skin_model = ? WHERE id = ?").bind(model).bind(user.id).execute(&state.db).await?;
    let base = net::public_base(&state, &headers).await;
    Ok(Json(yggdrasil::player_profile(&state, &base, &reload(&state, user.id).await?).await?))
}

pub async fn delete_skin(State(state): State<AppState>, headers: HeaderMap, AuthUser(user): AuthUser) -> AppResult<Json<PlayerProfile>> {
    sqlx::query("UPDATE users SET skin_hash = NULL WHERE id = ?").bind(user.id).execute(&state.db).await?;
    let base = net::public_base(&state, &headers).await;
    Ok(Json(yggdrasil::player_profile(&state, &base, &reload(&state, user.id).await?).await?))
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
    if let Some(id) = input.cape_id {
        let allowed = yggdrasil::available_capes(&state, &user).await?;
        if !allowed.iter().any(|c| c.id == id) {
            return Err(AppError::forbidden("that cape isn't available to you"));
        }
    }
    sqlx::query("UPDATE users SET cape_id = ? WHERE id = ?").bind(input.cape_id).bind(user.id).execute(&state.db).await?;
    let base = net::public_base(&state, &headers).await;
    Ok(Json(yggdrasil::player_profile(&state, &base, &reload(&state, user.id).await?).await?))
}

#[derive(Deserialize)]
pub struct AvatarQuery {
    #[serde(default)]
    size: Option<u32>,
}

/// Player head render by UUID or name. 404 when the player has no skin
/// (callers show their own placeholder).
pub async fn avatar(State(state): State<AppState>, Path(id): Path<String>, Query(q): Query<AvatarQuery>) -> AppResult<Response> {
    let user = match yggdrasil::user_by_uuid(&state, &id).await? {
        Some(u) => Some(u),
        None => crate::auth::find_user_by_name(&state, &id).await?,
    };
    let hash = user.and_then(|u| u.skin_hash).ok_or_else(|| AppError::not_found("no skin"))?;
    let path = textures::path(&state.cfg.textures_dir(), &hash).ok_or_else(|| AppError::not_found("no skin"))?;
    let bytes = tokio::fs::read(path).await.map_err(|_| AppError::not_found("no skin"))?;
    let size = q.size.unwrap_or(64);
    let png = tokio::task::spawn_blocking(move || textures::render_head(&bytes, size))
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))??;
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
async fn mirror_artifact(state: &AppState) -> AppResult<scopenet_core::authlib::Artifact> {
    use scopenet_core::authlib::{sha256_file, Artifact, OFFICIAL_LATEST};
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
        let artifact: Artifact = scopenet_core::http::get_json(&state.http, OFFICIAL_LATEST).await?;
        if !jar_ok(&artifact) {
            let tmp = dir.join("authlib-injector.jar.download");
            scopenet_core::http::download_one(
                &state.http,
                &scopenet_core::http::Download::new(artifact.download_url.clone(), tmp.clone(), None, None),
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

pub async fn authlib_index(State(state): State<AppState>) -> AppResult<Json<scopenet_core::authlib::Artifact>> {
    let mut a = mirror_artifact(&state).await?;
    a.download_url = "/api/v1/launcher/authlib-injector.jar".into();
    Ok(Json(a))
}

pub async fn authlib_jar(State(state): State<AppState>) -> AppResult<Response> {
    mirror_artifact(&state).await?;
    let bytes = tokio::fs::read(mirror_dir(&state).join("authlib-injector.jar")).await?;
    Ok(([(header::CONTENT_TYPE, "application/java-archive")], Body::from(bytes)).into_response())
}
