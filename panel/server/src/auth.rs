//! Passwords (Argon2), tokens (JWT) and the request extractors that guard
//! routes.

use crate::error::{AppError, AppResult};
use crate::state::AppState;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use velora_shared::PublicUser;
pub(crate) mod account;

pub fn hash_password(password: &str) -> AppResult<String> {
    velora_auth_core::password::hash_password(password).map_err(|e| AppError::new(axum::http::StatusCode::INTERNAL_SERVER_ERROR, e))
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    velora_auth_core::password::verify_password(password, hash)
}

pub fn validate_password(password: &str) -> AppResult<()> {
    velora_auth_core::password::validate_password(password).map_err(AppError::bad_request)
}

pub use velora_auth_core::tokens::Claims;

pub struct Keys {
    inner: std::sync::Arc<velora_auth_core::tokens::Keys>,
}

impl Keys {
    pub fn new(secret: &[u8]) -> Self {
        Self { inner: std::sync::Arc::new(velora_auth_core::tokens::Keys::new(secret)) }
    }
    pub(crate) fn authority_keys(&self) -> std::sync::Arc<velora_auth_core::tokens::Keys> {
        self.inner.clone()
    }

    pub fn issue(&self, user: &UserRow) -> AppResult<String> {
        self.inner
            .issue(velora_auth_core::tokens::Subject {
                id: user.id,
                name: &user.username,
                role: &user.role,
                auth_version: user.auth_version,
            })
            .map_err(|e| AppError::new(axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
    }

    pub fn verify(&self, token: &str) -> Option<Claims> {
        self.inner.verify(token)
    }
}

pub use velora_auth_core::identity::IdentityRecord as UserRow;

pub async fn user_groups(state: &AppState, user_id: i64) -> AppResult<Vec<String>> {
    Ok(velora_auth_core::identity_store::groups(&state.platform_db, user_id).await?)
}

pub async fn public_user(state: &AppState, user: &UserRow) -> AppResult<PublicUser> {
    Ok(PublicUser {
        id: user.id,
        username: user.username.clone(),
        uuid: user.uuid.clone(),
        role: user.role.clone(),
        groups: user_groups(state, user.id).await?,
    })
}

/// Allocate identity once. Names can change; the account UUID never does.
pub async fn create_user(
    state: &AppState,
    username: &str,
    password: &str,
    email: Option<&str>,
    role: &str,
    status: &str,
) -> AppResult<i64> {
    account::create_account(state, username, password, email, role, status).await
}

pub async fn find_user_by_name(state: &AppState, name: &str) -> AppResult<Option<UserRow>> {
    Ok(velora_auth_core::identity_store::find_by_name(&state.platform_db, name).await?)
}

pub fn bearer(parts: &Parts) -> Option<&str> {
    parts
        .headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer ").or_else(|| v.strip_prefix("bearer ")))
}

async fn resolve(parts: &Parts, state: &AppState) -> AppResult<Option<UserRow>> {
    let Some(token) = bearer(parts) else { return Ok(None) };
    authenticate(state, token).await.map(Some)
}

pub async fn authenticate(state: &AppState, token: &str) -> AppResult<UserRow> {
    let Some(claims) = state.keys.verify(token) else {
        return Err(AppError::unauthorized("your session expired, please sign in again"));
    };
    let user = velora_auth_core::identity_store::find_by_id(&state.platform_db, claims.sub).await?;
    use velora_auth_core::admission::{session_admission, AccountStatus, SessionDenied};
    session_admission(claims.version, user.as_ref().map(|u| AccountStatus { status: &u.status, auth_version: u.auth_version })).map_err(
        |denied| match denied {
            SessionDenied::Pending => AppError::forbidden(denied.message()),
            SessionDenied::DisabledOrRemoved => AppError::unauthorized(denied.message()),
        },
    )?;
    user.ok_or_else(|| AppError::unauthorized("account disabled or removed"))
}

/// Signed-in user if a valid token was sent, otherwise `None`.
pub struct MaybeUser(pub Option<UserRow>);

impl FromRequestParts<AppState> for MaybeUser {
    type Rejection = AppError;
    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        // A bad token on a public endpoint just means "anonymous".
        Ok(MaybeUser(resolve(parts, state).await.unwrap_or(None)))
    }
}

pub struct AuthUser(pub UserRow);

impl std::ops::Deref for AuthUser {
    type Target = UserRow;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;
    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        resolve(parts, state).await?.map(AuthUser).ok_or_else(|| AppError::unauthorized("sign in required"))
    }
}

pub struct AdminUser(pub UserRow);

impl std::ops::Deref for AdminUser {
    type Target = UserRow;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromRequestParts<AppState> for AdminUser {
    type Rejection = AppError;
    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let user = resolve(parts, state).await?.ok_or_else(|| AppError::unauthorized("sign in required"))?;
        if !user.is_admin() {
            return Err(AppError::forbidden("admins only"));
        }
        Ok(AdminUser(user))
    }
}

/// Very small brute-force guard: 10 failed attempts per username locks it
/// for 5 minutes.
#[derive(Default)]
pub struct LoginGuard {
    inner: std::sync::Arc<velora_auth_core::login_guard::LoginGuard>,
}

impl LoginGuard {
    pub(crate) fn authority_guard(&self) -> std::sync::Arc<velora_auth_core::login_guard::LoginGuard> {
        self.inner.clone()
    }
    pub fn check(&self, username: &str) -> AppResult<()> {
        self.inner.check(username).map_err(|message| AppError::new(axum::http::StatusCode::TOO_MANY_REQUESTS, message))
    }

    pub fn fail(&self, username: &str) {
        self.inner.fail(username);
    }

    pub fn succeed(&self, username: &str) {
        self.inner.succeed(username);
    }
}
