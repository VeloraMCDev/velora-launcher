//! API v1 account HTTP composition. Hosts are mandatory; no policy/audit/presence/email defaults.
use crate::{
    account::{self, MutationHost},
    error::{AppError, AppResult},
    reset::{self, ResetHost},
    web::{self, AccountHost, AccountState},
};
use axum::{
    body::Body,
    extract::{DefaultBodyLimit, FromRequestParts, Multipart, Path, Query, State},
    http::{header, request::Parts, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post, put},
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;
use velora_auth_core::{
    admission::{session_admission, AccountStatus, SessionDenied},
    identity::IdentityRecord,
    identity_store,
};
use velora_platform_contracts::{AuthResponse, LoginRequest, PlayerProfile, PublicUser, RegisterRequest};

#[derive(Clone)]
pub struct AccountPorts {
    pub policy: Arc<dyn AccountHost>,
    pub mutations: Arc<dyn MutationHost>,
    pub reset: Arc<dyn ResetHost>,
}
#[derive(Clone)]
pub struct AccountHttpState {
    pub accounts: AccountState,
    pub mutations: Arc<dyn MutationHost>,
    pub reset: Arc<dyn ResetHost>,
}
fn denied(status: StatusCode, message: impl Into<String>) -> AppError {
    AppError { status, message: message.into() }
}

/// Original header casing, JWT expiry/signature and live status/version precedence.
pub async fn authenticate(state: &AccountState, token: &str) -> AppResult<IdentityRecord> {
    let claims =
        state.tokens.verify(token).ok_or_else(|| denied(StatusCode::UNAUTHORIZED, "your session expired, please sign in again"))?;
    let user = identity_store::find_by_id(&state.authority.identity_db, claims.sub).await?;
    session_admission(claims.version, user.as_ref().map(|user| AccountStatus { status: &user.status, auth_version: user.auth_version }))
        .map_err(|reason| match reason {
            SessionDenied::Pending => denied(StatusCode::FORBIDDEN, reason.message()),
            SessionDenied::DisabledOrRemoved => denied(StatusCode::UNAUTHORIZED, reason.message()),
        })?;
    user.ok_or_else(|| denied(StatusCode::UNAUTHORIZED, "account disabled or removed"))
}
struct AuthUser(IdentityRecord);
impl FromRequestParts<AccountHttpState> for AuthUser {
    type Rejection = AppError;
    async fn from_request_parts(parts: &mut Parts, state: &AccountHttpState) -> AppResult<Self> {
        let token = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer ").or_else(|| value.strip_prefix("bearer ")))
            .ok_or_else(|| denied(StatusCode::UNAUTHORIZED, "sign in required"))?;
        authenticate(&state.accounts, token).await.map(Self)
    }
}

pub fn routes(state: AccountHttpState) -> Router {
    Router::new()
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/register", post(register))
        .route("/api/v1/auth/me", get(me))
        .route("/api/v1/auth/forgot-password", post(forgot))
        .route("/api/v1/auth/reset-password", post(reset_password))
        .route("/api/v1/account/profile", get(profile))
        .route("/api/v1/account/username", put(set_username))
        .route("/api/v1/account/skin", post(upload_skin).delete(delete_skin))
        .route("/api/v1/account/skin/model", put(set_model))
        .route("/api/v1/account/cape", put(set_cape))
        .route("/api/v1/avatar/{id}", get(avatar))
        .layer(DefaultBodyLimit::max(4 * 1024 * 1024))
        .with_state(state)
}
async fn login(State(state): State<AccountHttpState>, Json(request): Json<LoginRequest>) -> AppResult<Json<AuthResponse>> {
    Ok(Json(web::login(&state.accounts, request).await?))
}
async fn register(State(state): State<AccountHttpState>, Json(request): Json<RegisterRequest>) -> AppResult<Json<AuthResponse>> {
    Ok(Json(web::register(&state.accounts, request).await?))
}
async fn me(State(state): State<AccountHttpState>, AuthUser(user): AuthUser) -> AppResult<Json<PublicUser>> {
    Ok(Json(web::public_user(&state.accounts, &user).await?))
}
#[derive(Deserialize)]
struct Forgot {
    email: String,
}
async fn forgot(State(state): State<AccountHttpState>, Json(input): Json<Forgot>) -> AppResult<Json<serde_json::Value>> {
    Ok(Json(reset::forgot_password(&state.accounts.authority.db, &*state.reset, &input.email).await?))
}
#[derive(Deserialize)]
struct Reset {
    token: String,
    password: String,
}
async fn reset_password(State(state): State<AccountHttpState>, Json(input): Json<Reset>) -> AppResult<Json<serde_json::Value>> {
    Ok(Json(reset::reset_password(&state.accounts.authority.db, &input.token, &input.password).await?))
}
async fn profile(State(state): State<AccountHttpState>, headers: HeaderMap, AuthUser(user): AuthUser) -> AppResult<Json<PlayerProfile>> {
    Ok(Json(account::profile(&state.accounts.authority, &headers, &user).await?))
}
#[derive(Deserialize)]
struct Username {
    username: String,
    password: String,
}
async fn set_username(
    State(state): State<AccountHttpState>,
    AuthUser(user): AuthUser,
    Json(input): Json<Username>,
) -> AppResult<Json<AuthResponse>> {
    Ok(Json(account::set_username(&state.accounts, &*state.mutations, &user, &input.username, &input.password).await?))
}
fn upload_error(error: axum::extract::multipart::MultipartError) -> AppError {
    AppError::bad_request(format!("upload failed: {error}"))
}
async fn upload_skin(
    State(state): State<AccountHttpState>,
    headers: HeaderMap,
    AuthUser(user): AuthUser,
    mut form: Multipart,
) -> AppResult<Json<PlayerProfile>> {
    let mut model = String::from("classic");
    let mut file = None;
    while let Some(field) = form.next_field().await.map_err(upload_error)? {
        match field.name().unwrap_or_default() {
            "model" => model = field.text().await.map_err(upload_error)?,
            "file" => file = Some(field.bytes().await.map_err(upload_error)?.to_vec()),
            _ => {}
        }
    }
    let bytes = file.ok_or_else(|| AppError::bad_request("no file uploaded"))?;
    Ok(Json(account::upload_skin(&state.accounts.authority, &headers, &user, &bytes, &model).await?))
}
async fn delete_skin(
    State(state): State<AccountHttpState>,
    headers: HeaderMap,
    AuthUser(user): AuthUser,
) -> AppResult<Json<PlayerProfile>> {
    Ok(Json(account::delete_skin(&state.accounts.authority, &headers, &user).await?))
}
#[derive(Deserialize)]
struct Model {
    model: String,
}
async fn set_model(
    State(state): State<AccountHttpState>,
    headers: HeaderMap,
    AuthUser(user): AuthUser,
    Json(input): Json<Model>,
) -> AppResult<Json<PlayerProfile>> {
    Ok(Json(account::set_model(&state.accounts.authority, &headers, &user, &input.model).await?))
}
#[derive(Deserialize)]
struct Cape {
    cape_id: Option<i64>,
}
async fn set_cape(
    State(state): State<AccountHttpState>,
    headers: HeaderMap,
    AuthUser(user): AuthUser,
    Json(input): Json<Cape>,
) -> AppResult<Json<PlayerProfile>> {
    Ok(Json(account::set_cape(&state.accounts.authority, &headers, &user, input.cape_id).await?))
}
#[derive(Deserialize)]
struct AvatarQuery {
    #[serde(default)]
    size: Option<u32>,
}
async fn avatar(State(state): State<AccountHttpState>, Path(id): Path<String>, Query(query): Query<AvatarQuery>) -> AppResult<Response> {
    let png = account::avatar(&state.accounts.authority, &id, query.size).await?;
    Ok(([(header::CONTENT_TYPE, "image/png"), (header::CACHE_CONTROL, "public, max-age=300")], Body::from(png)).into_response())
}
