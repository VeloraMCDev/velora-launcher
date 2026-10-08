//! A Yggdrasil-compatible authentication server, following the
//! authlib-injector specification:
//! <https://github.com/yushijinhun/authlib-injector/wiki/Yggdrasil-%E6%9C%8D%E5%8A%A1%E7%AB%AF%E6%8A%80%E6%9C%AF%E8%A7%84%E8%8C%83>
//!
//! Game clients and servers run authlib-injector pointed at
//! `{panel}/api/yggdrasil`. Sign-in, server joins, skins and capes then all
//! come from the panel — no Mojang or Microsoft account needed.

pub mod account;
pub mod admin;
pub mod auth;
pub mod cosmetics;
pub mod error;
pub mod net;
pub mod state;
mod textures;

use crate::auth::UserRow;
use crate::error::AppResult;
use crate::net::ClientIp;
use crate::state::AuthorityState as AppState;
use axum::body::Body;
use axum::extract::{Multipart, Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use base64::Engine;
use serde::Deserialize;
use serde_json::{json, Value};
use velora_platform_contracts::{CapeInfo, PlayerProfile};

pub const ROOT: &str = "/api/yggdrasil";
const SESSION_SECS: i64 = 60;

// ---------------------------------------------------------------------------
// Errors (Yggdrasil has its own error shape)
// ---------------------------------------------------------------------------

pub struct YggError {
    status: StatusCode,
    error: &'static str,
    message: String,
}

impl YggError {
    fn forbidden(message: impl Into<String>) -> Self {
        Self { status: StatusCode::FORBIDDEN, error: "ForbiddenOperationException", message: message.into() }
    }
    fn bad_request(message: impl Into<String>) -> Self {
        Self { status: StatusCode::BAD_REQUEST, error: "IllegalArgumentException", message: message.into() }
    }
    fn invalid_token() -> Self {
        Self::forbidden("Invalid token.")
    }
}

impl IntoResponse for YggError {
    fn into_response(self) -> Response {
        (self.status, Json(json!({ "error": self.error, "errorMessage": self.message }))).into_response()
    }
}

impl From<crate::error::AppError> for YggError {
    fn from(e: crate::error::AppError) -> Self {
        Self { status: e.status, error: "InternalServerError", message: e.message }
    }
}

impl From<sqlx::Error> for YggError {
    fn from(e: sqlx::Error) -> Self {
        crate::error::AppError::from(e).into()
    }
}

type YggResult<T> = Result<T, YggError>;

fn no_content() -> Response {
    StatusCode::NO_CONTENT.into_response()
}

// ---------------------------------------------------------------------------
// Helpers shared with the launcher/admin APIs
// ---------------------------------------------------------------------------

pub fn undashed(uuid: &str) -> String {
    uuid.replace('-', "").to_ascii_lowercase()
}

pub fn dashed(uuid: &str) -> Option<String> {
    let u = undashed(uuid);
    (u.len() == 32 && u.chars().all(|c| c.is_ascii_hexdigit()))
        .then(|| format!("{}-{}-{}-{}-{}", &u[0..8], &u[8..12], &u[12..16], &u[16..20], &u[20..32]))
}

pub use velora_auth_core::capes::CapeRow;
trait CapeProjection {
    fn info(&self, base: &str) -> CapeInfo;
}
impl CapeProjection for CapeRow {
    fn info(&self, base: &str) -> CapeInfo {
        CapeInfo { id: self.id, name: self.name.clone(), url: texture_url(base, &self.hash) }
    }
}
pub fn texture_url(base: &str, hash: &str) -> String {
    format!("{base}/textures/{hash}")
}
pub async fn user_by_uuid(state: &AppState, uuid: &str) -> AppResult<Option<UserRow>> {
    let Some(d) = dashed(uuid) else { return Ok(None) };
    Ok(velora_auth_core::identity_store::find_by_uuid(&state.db, &d).await?)
}
pub async fn cape_of(state: &AppState, user: &UserRow) -> AppResult<Option<CapeRow>> {
    let Some(id) = user.cape_id else { return Ok(None) };
    Ok(velora_auth_core::capes::find(&state.db, id).await?)
}
pub async fn available_capes(state: &AppState, user: &UserRow) -> AppResult<Vec<CapeRow>> {
    let groups = auth::user_groups(state, user.id).await?;
    Ok(velora_auth_core::capes::available(&state.db, user, &groups).await?)
}

pub async fn player_profile(state: &AppState, base: &str, user: &UserRow) -> AppResult<PlayerProfile> {
    Ok(PlayerProfile {
        uuid: user.uuid.clone(),
        name: user.username.clone(),
        skin_url: user.skin_hash.as_deref().map(|h| texture_url(base, h)),
        skin_model: user.skin_model.clone(),
        cape: cape_of(state, user).await?.map(|c| c.info(base)),
        available_capes: available_capes(state, user).await?.iter().map(|c| c.info(base)).collect(),
    })
}

/// The base64 `textures` property value.
async fn textures_value(state: &AppState, base: &str, user: &UserRow) -> AppResult<String> {
    let mut textures = serde_json::Map::new();
    if let Some(hash) = &user.skin_hash {
        let mut skin = json!({ "url": texture_url(base, hash) });
        if user.skin_model == "slim" {
            skin["metadata"] = json!({ "model": "slim" });
        }
        textures.insert("SKIN".into(), skin);
    }
    if let Some(cape) = cape_of(state, user).await? {
        textures.insert("CAPE".into(), json!({ "url": texture_url(base, &cape.hash) }));
    }
    let value = json!({
        "timestamp": chrono::Utc::now().timestamp_millis(),
        "profileId": undashed(&user.uuid),
        "profileName": user.username,
        "textures": textures,
    });
    Ok(base64::engine::general_purpose::STANDARD.encode(value.to_string()))
}

/// A full game profile, optionally with signed properties.
pub async fn profile_json(state: &AppState, base: &str, user: &UserRow, signed: bool) -> AppResult<Value> {
    let value = textures_value(state, base, user).await?;
    let mut textures = json!({ "name": "textures", "value": value });
    let mut uploadable = json!({ "name": "uploadableTextures", "value": "skin" });
    if signed {
        textures["signature"] = json!(state.ygg.sign_b64(value.as_bytes()));
        uploadable["signature"] = json!(state.ygg.sign_b64(b"skin"));
    }
    Ok(json!({ "id": undashed(&user.uuid), "name": user.username, "properties": [textures, uploadable] }))
}

fn short_profile(user: &UserRow) -> Value {
    json!({ "id": undashed(&user.uuid), "name": user.username })
}

/// Issue a game access token through authority-owned storage.
pub async fn issue_token(state: &AppState, user_id: i64, client_token: Option<String>) -> AppResult<(String, String)> {
    Ok(velora_auth_core::ygg_store::issue_token(&state.db, user_id, client_token).await?)
}
pub async fn token_user(state: &AppState, access: &str, client: Option<&str>) -> AppResult<Option<UserRow>> {
    Ok(velora_auth_core::ygg_store::token_user(&state.db, access, client).await?)
}

async fn check_password(state: &AppState, username: &str, password: &str) -> YggResult<UserRow> {
    state.login_guard.check(username).map_err(YggError::forbidden)?;
    // Email or username (`feature.non_email_login`).
    let user = velora_auth_core::identity_store::find_by_login(&state.db, username).await?;
    let Some(user) = user.filter(|u| auth::verify_password(password, &u.password_hash)) else {
        state.login_guard.fail(username);
        return Err(YggError::forbidden("Invalid credentials. Invalid username or password."));
    };
    state.login_guard.succeed(username);
    match user.status.as_str() {
        "active" => Ok(user),
        "pending" => Err(YggError::forbidden("Your account is waiting for an admin to approve it.")),
        _ => Err(YggError::forbidden(user.status_reason.clone().unwrap_or_else(|| "This account has been disabled.".into()))),
    }
}

// ---------------------------------------------------------------------------
// Metadata
// ---------------------------------------------------------------------------

async fn metadata(State(state): State<AppState>, headers: HeaderMap) -> AppResult<Json<Value>> {
    let base = net::public_base(&state, &headers).await;
    let server_name = state.host.server_name().await?;
    Ok(Json(json!({
        "meta": {
            "serverName": server_name,
            "implementationName": "Velora",
            "implementationVersion": state.implementation_version,
            "links": { "homepage": base, "register": base },
            "feature.non_email_login": true,
            "feature.enable_profile_key": true,
            "feature.no_mojang_namespace": true,
        },
        "skinDomains": [net::host_of(&base)],
        "signaturePublickey": state.ygg.public_pem,
    })))
}

// ---------------------------------------------------------------------------
// authserver
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuthenticateReq {
    username: String,
    password: String,
    #[serde(default)]
    client_token: Option<String>,
    #[serde(default)]
    request_user: bool,
}

fn user_json(user: &UserRow) -> Value {
    json!({ "id": undashed(&user.uuid), "properties": [{ "name": "preferredLanguage", "value": "en" }] })
}

async fn authenticate(State(state): State<AppState>, Json(req): Json<AuthenticateReq>) -> YggResult<Json<Value>> {
    let user = check_password(&state, &req.username, &req.password).await?;
    let (access, client) = issue_token(&state, user.id, req.client_token).await?;
    let mut body = json!({
        "accessToken": access,
        "clientToken": client,
        "availableProfiles": [short_profile(&user)],
        "selectedProfile": short_profile(&user),
    });
    if req.request_user {
        body["user"] = user_json(&user);
    }
    Ok(Json(body))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RefreshReq {
    access_token: String,
    #[serde(default)]
    client_token: Option<String>,
    #[serde(default)]
    request_user: bool,
}

async fn refresh(State(state): State<AppState>, Json(req): Json<RefreshReq>) -> YggResult<Json<Value>> {
    let client_token = velora_auth_core::ygg_store::client_token(&state.db, &req.access_token).await?;
    let user = token_user(&state, &req.access_token, req.client_token.as_deref()).await?.ok_or_else(YggError::invalid_token)?;
    velora_auth_core::ygg_store::invalidate(&state.db, &req.access_token).await?;
    let (access, client) = issue_token(&state, user.id, client_token).await?;
    let mut body = json!({ "accessToken": access, "clientToken": client, "selectedProfile": short_profile(&user) });
    if req.request_user {
        body["user"] = user_json(&user);
    }
    Ok(Json(body))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenReq {
    access_token: String,
    #[serde(default)]
    client_token: Option<String>,
}

async fn validate(State(state): State<AppState>, Json(req): Json<TokenReq>) -> YggResult<Response> {
    token_user(&state, &req.access_token, req.client_token.as_deref()).await?.ok_or_else(YggError::invalid_token)?;
    Ok(no_content())
}

async fn invalidate(State(state): State<AppState>, Json(req): Json<TokenReq>) -> YggResult<Response> {
    velora_auth_core::ygg_store::invalidate(&state.db, &req.access_token).await?;
    Ok(no_content())
}

#[derive(Deserialize)]
struct SignoutReq {
    username: String,
    password: String,
}

async fn signout(State(state): State<AppState>, Json(req): Json<SignoutReq>) -> YggResult<Response> {
    let user = check_password(&state, &req.username, &req.password).await?;
    velora_auth_core::ygg_store::signout(&state.db, user.id).await?;
    Ok(no_content())
}

// ---------------------------------------------------------------------------
// sessionserver
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct JoinReq {
    access_token: String,
    selected_profile: String,
    server_id: String,
}

async fn join(State(state): State<AppState>, ClientIp(ip): ClientIp, Json(req): Json<JoinReq>) -> YggResult<Response> {
    let user = token_user(&state, &req.access_token, None).await?.ok_or_else(YggError::invalid_token)?;
    if undashed(&req.selected_profile) != undashed(&user.uuid) {
        return Err(YggError::forbidden("Invalid token."));
    }
    if req.server_id.is_empty() || req.server_id.len() > 64 {
        return Err(YggError::bad_request("invalid serverId"));
    }
    let cutoff = (chrono::Utc::now() - chrono::Duration::seconds(SESSION_SECS)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    velora_auth_core::ygg_store::join(&state.db, &req.server_id, user.id, ip, &cutoff).await?;
    Ok(no_content())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct HasJoinedQuery {
    username: String,
    server_id: String,
    #[serde(default)]
    ip: Option<String>,
}

async fn has_joined(State(state): State<AppState>, headers: HeaderMap, Query(q): Query<HasJoinedQuery>) -> YggResult<Response> {
    let cutoff = (chrono::Utc::now() - chrono::Duration::seconds(SESSION_SECS)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let row = velora_auth_core::ygg_store::joined(&state.db, &q.server_id, &cutoff).await?;
    let Some((user_id, joined_ip)) = row else { return Ok(no_content()) };
    let Some(user) = velora_auth_core::ygg_store::active_user(&state.db, user_id).await? else {
        return Ok(no_content());
    };
    if !user.username.eq_ignore_ascii_case(&q.username) {
        return Ok(no_content());
    }
    if let Some(expected) = q.ip.as_deref().filter(|i| !i.is_empty()) {
        if joined_ip.as_deref() != Some(expected) {
            return Ok(no_content());
        }
    }
    let base = net::public_base(&state, &headers).await;
    Ok(Json(profile_json(&state, &base, &user, true).await?).into_response())
}

#[derive(Deserialize)]
struct ProfileQuery {
    #[serde(default)]
    unsigned: Option<String>,
}

async fn session_profile(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(uuid): Path<String>,
    Query(q): Query<ProfileQuery>,
) -> YggResult<Response> {
    let Some(user) = user_by_uuid(&state, &uuid).await? else { return Ok(no_content()) };
    let signed = q.unsigned.as_deref() == Some("false");
    let base = net::public_base(&state, &headers).await;
    Ok(Json(profile_json(&state, &base, &user, signed).await?).into_response())
}

// ---------------------------------------------------------------------------
// Mojang-style profile API
// ---------------------------------------------------------------------------

async fn profiles_by_names(State(state): State<AppState>, Json(names): Json<Vec<String>>) -> YggResult<Json<Vec<Value>>> {
    let mut out = Vec::new();
    for name in names.iter().take(100) {
        if let Some(user) = auth::find_user_by_name(&state, name).await? {
            if !out.iter().any(|v: &Value| v["id"] == undashed(&user.uuid)) {
                out.push(short_profile(&user));
            }
        }
    }
    Ok(Json(out))
}

async fn profile_by_name(State(state): State<AppState>, Path(name): Path<String>) -> YggResult<Response> {
    match auth::find_user_by_name(&state, &name).await? {
        Some(user) => Ok(Json(short_profile(&user)).into_response()),
        None => Ok(no_content()),
    }
}

fn bearer(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer ").or_else(|| v.strip_prefix("bearer ")))
}

async fn bearer_user(state: &AppState, headers: &HeaderMap) -> YggResult<UserRow> {
    let token = bearer(headers).ok_or_else(|| YggError {
        status: StatusCode::UNAUTHORIZED,
        error: "Unauthorized",
        message: "Missing token.".into(),
    })?;
    token_user(state, token, None).await?.ok_or_else(|| YggError {
        status: StatusCode::UNAUTHORIZED,
        error: "Unauthorized",
        message: "Invalid token.".into(),
    })
}

/// `PUT /api/user/profile/{uuid}/skin` (multipart: `model`, `file`).
async fn upload_texture(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((uuid, kind)): Path<(String, String)>,
    mut form: Multipart,
) -> YggResult<Response> {
    let user = bearer_user(&state, &headers).await?;
    if undashed(&uuid) != undashed(&user.uuid) {
        return Err(YggError::forbidden("You can only change your own skin."));
    }
    if kind != "skin" {
        return Err(YggError::forbidden("Capes are assigned by the server admins."));
    }
    let mut model = String::from("classic");
    let mut file = None;
    while let Some(field) = form.next_field().await.map_err(|e| YggError::bad_request(e.to_string()))? {
        match field.name().unwrap_or_default() {
            "model" => model = field.text().await.map_err(|e| YggError::bad_request(e.to_string()))?,
            "file" => file = Some(field.bytes().await.map_err(|e| YggError::bad_request(e.to_string()))?),
            _ => {}
        }
    }
    let bytes = file.ok_or_else(|| YggError::bad_request("no file"))?;
    set_skin(&state, user.id, &bytes, &model).await?;
    Ok(no_content())
}

async fn delete_texture(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((uuid, kind)): Path<(String, String)>,
) -> YggResult<Response> {
    let user = bearer_user(&state, &headers).await?;
    if undashed(&uuid) != undashed(&user.uuid) || kind != "skin" {
        return Err(YggError::forbidden("Not allowed."));
    }
    velora_auth_core::identity_store::clear_skin(&state.db, user.id).await?;
    Ok(no_content())
}

/// Store a skin for a user (validated PNG, "classic" or "slim").
pub async fn set_skin(state: &AppState, user_id: i64, bytes: &[u8], model: &str) -> AppResult<()> {
    let model = if model == "slim" { "slim" } else { "classic" };
    let dir = state.textures_dir.clone();
    let data = bytes.to_vec();
    let hash = tokio::task::spawn_blocking(move || textures::store(&dir, textures::Kind::Skin, &data))
        .await
        .map_err(|e| crate::error::AppError::bad_request(e.to_string()))??;
    velora_auth_core::identity_store::set_skin(&state.db, user_id, &hash, model).await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// minecraftservices (1.19+ chat signing, social features)
// ---------------------------------------------------------------------------

use velora_auth_core::certificates::PlayerKeyRow;

async fn player_certificates(State(state): State<AppState>, headers: HeaderMap) -> YggResult<Json<Value>> {
    let user = bearer_user(&state, &headers).await?;
    let existing = velora_auth_core::certificates::load(&state.db, user.id).await?;
    let row = match existing.filter(|k| velora_auth_core::certificates::fresh(k, chrono::Utc::now())) {
        Some(k) => k,
        None => {
            let row = generate_player_key(&state, &user).await?;
            velora_auth_core::certificates::store(&state.db, user.id, &row).await?;
            row
        }
    };
    Ok(Json(json!({
        "keyPair": { "privateKey": row.private_pem, "publicKey": row.public_pem },
        "publicKeySignature": row.signature_v1,
        "publicKeySignatureV2": row.signature_v2,
        "expiresAt": row.expires_at,
        "refreshedAfter": row.refreshed_after,
    })))
}

async fn generate_player_key(state: &AppState, user: &UserRow) -> YggResult<PlayerKeyRow> {
    let key = tokio::task::spawn_blocking(velora_auth_core::certificates::new_player_key)
        .await
        .map_err(|e| YggError::bad_request(e.to_string()))?
        .map_err(|e| YggError::bad_request(e.to_string()))?;
    velora_auth_core::certificates::certify(&state.ygg, &key, &user.uuid).map_err(|e| YggError::bad_request(e.to_string()))
}

async fn player_attributes(State(state): State<AppState>, headers: HeaderMap) -> YggResult<Json<Value>> {
    bearer_user(&state, &headers).await?;
    Ok(Json(json!({
        "privileges": {
            "onlineChat": { "enabled": true },
            "multiplayerServer": { "enabled": true },
            "multiplayerRealms": { "enabled": false },
            "telemetry": { "enabled": false },
        },
        "profanityFilterPreferences": { "profanityFilterOn": false },
    })))
}

async fn blocklist(State(state): State<AppState>, headers: HeaderMap) -> YggResult<Json<Value>> {
    bearer_user(&state, &headers).await?;
    Ok(Json(json!({ "blockedProfiles": [] })))
}

async fn public_keys(State(state): State<AppState>) -> Json<Value> {
    let key = json!([{ "publicKey": state.ygg.public_der_b64 }]);
    Json(json!({ "profilePropertyKeys": key, "playerCertificateKeys": key }))
}

async fn services_profile(State(state): State<AppState>, headers: HeaderMap) -> YggResult<Json<Value>> {
    let user = bearer_user(&state, &headers).await?;
    let base = net::public_base(&state, &headers).await;
    let skins: Vec<Value> = user
        .skin_hash
        .iter()
        .map(|h| json!({ "id": h, "state": "ACTIVE", "url": texture_url(&base, h), "variant": if user.skin_model == "slim" { "SLIM" } else { "CLASSIC" } }))
        .collect();
    let capes: Vec<Value> = cape_of(&state, &user)
        .await?
        .iter()
        .map(|c| json!({ "id": c.id.to_string(), "state": "ACTIVE", "url": texture_url(&base, &c.hash), "alias": c.name }))
        .collect();
    Ok(Json(json!({ "id": undashed(&user.uuid), "name": user.username, "skins": skins, "capes": capes })))
}

// ---------------------------------------------------------------------------
// Texture files
// ---------------------------------------------------------------------------

pub async fn texture_file(State(state): State<AppState>, Path(hash): Path<String>) -> Response {
    let Some(path) = textures::path(&state.textures_dir.clone(), &hash) else { return StatusCode::NOT_FOUND.into_response() };
    match tokio::fs::read(path).await {
        Ok(bytes) => {
            ([(header::CONTENT_TYPE, "image/png"), (header::CACHE_CONTROL, "public, max-age=31536000, immutable")], Body::from(bytes))
                .into_response()
        }
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

pub fn routes() -> Router<AppState> {
    let p = |path: &str| format!("{ROOT}{path}");
    Router::new()
        .route(ROOT, get(metadata))
        .route(&p("/"), get(metadata))
        .route(&p("/authserver/authenticate"), post(authenticate))
        .route(&p("/authserver/refresh"), post(refresh))
        .route(&p("/authserver/validate"), post(validate))
        .route(&p("/authserver/invalidate"), post(invalidate))
        .route(&p("/authserver/signout"), post(signout))
        .route(&p("/sessionserver/session/minecraft/join"), post(join))
        .route(&p("/sessionserver/session/minecraft/hasJoined"), get(has_joined))
        .route(&p("/sessionserver/session/minecraft/profile/{uuid}"), get(session_profile))
        .route(&p("/api/profiles/minecraft"), post(profiles_by_names))
        .route(&p("/api/users/profiles/minecraft/{name}"), get(profile_by_name))
        .route(&p("/api/user/profile/{uuid}/{kind}"), put(upload_texture).delete(delete_texture))
        .route(&p("/minecraftservices/player/certificates"), post(player_certificates))
        .route(&p("/minecraftservices/player/attributes"), get(player_attributes))
        .route(&p("/minecraftservices/privacy/blocklist"), get(blocklist))
        .route(&p("/minecraftservices/publickeys"), get(public_keys))
        .route(&p("/minecraftservices/minecraft/profile"), get(services_profile))
        .route("/textures/{hash}", get(texture_file))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uuid_forms() {
        assert_eq!(undashed("B50AD385-829D-3141-A216-7E7D7539BA7F"), "b50ad385829d3141a2167e7d7539ba7f");
        assert_eq!(dashed("b50ad385829d3141a2167e7d7539ba7f").as_deref(), Some("b50ad385-829d-3141-a216-7e7d7539ba7f"));
        assert!(dashed("nope").is_none());
    }
}

pub mod web;

pub mod reset;

pub mod config;
pub mod oauth;
pub mod web_routes;
