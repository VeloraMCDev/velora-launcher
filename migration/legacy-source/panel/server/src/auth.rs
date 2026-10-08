//! Passwords (Argon2), tokens (JWT) and the request extractors that guard
//! routes.

use crate::error::{AppError, AppResult};
use crate::state::AppState;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use scopenet_shared::PublicUser;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const TOKEN_DAYS: i64 = 30;

pub fn hash_password(password: &str) -> AppResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| AppError::new(axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("hashing failed: {e}")))
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    PasswordHash::new(hash).map(|h| Argon2::default().verify_password(password.as_bytes(), &h).is_ok()).unwrap_or(false)
}

pub fn validate_password(password: &str) -> AppResult<()> {
    if password.chars().count() < 8 {
        return Err(AppError::bad_request("passwords need at least 8 characters"));
    }
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    #[serde(default)]
    pub version: i64,
    pub sub: i64,
    pub name: String,
    pub role: String,
    pub exp: i64,
}

pub struct Keys {
    enc: EncodingKey,
    dec: DecodingKey,
}

impl Keys {
    pub fn new(secret: &[u8]) -> Self {
        Self { enc: EncodingKey::from_secret(secret), dec: DecodingKey::from_secret(secret) }
    }

    pub fn issue(&self, user: &UserRow) -> AppResult<String> {
        let claims = Claims {
            version: user.auth_version,
            sub: user.id,
            name: user.username.clone(),
            role: user.role.clone(),
            exp: (chrono::Utc::now() + chrono::Duration::days(TOKEN_DAYS)).timestamp(),
        };
        encode(&Header::default(), &claims, &self.enc)
            .map_err(|e| AppError::new(axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
    }

    pub fn verify(&self, token: &str) -> Option<Claims> {
        decode::<Claims>(token, &self.dec, &Validation::default()).ok().map(|d| d.claims)
    }
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct UserRow {
    #[serde(skip)]
    pub auth_version: i64,
    pub id: i64,
    pub username: String,
    #[serde(skip)]
    pub password_hash: String,
    pub email: Option<String>,
    pub role: String,
    pub status: String,
    pub created_at: String,
    pub last_login: Option<String>,
    /// Dashed player UUID.
    pub uuid: String,
    pub skin_hash: Option<String>,
    pub skin_model: String,
    pub cape_id: Option<i64>,
    /// Why the account is disabled (shown to the player when they're refused).
    pub status_reason: Option<String>,
}

impl UserRow {
    pub fn is_admin(&self) -> bool {
        self.role == "admin"
    }
}

pub async fn user_groups(state: &AppState, user_id: i64) -> AppResult<Vec<String>> {
    Ok(sqlx::query_scalar("SELECT g.name FROM groups g JOIN user_groups ug ON ug.group_id = g.id WHERE ug.user_id = ? ORDER BY g.name")
        .bind(user_id)
        .fetch_all(&state.platform_db)
        .await?)
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
    crate::store::check_username(state, username).await?;
    let hash = hash_password(password)?;
    sqlx::query_scalar(
        "INSERT INTO users (username, password_hash, email, role, status, created_at, uuid) VALUES (?, ?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(username)
    .bind(hash)
    .bind(email.map(str::trim).filter(|e| !e.is_empty()))
    .bind(role)
    .bind(status)
    .bind(crate::db::now())
    .bind(uuid::Uuid::new_v4().to_string())
    .fetch_one(&state.platform_db)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(d) if d.message().contains("UNIQUE") || d.message().contains("username reserved") => {
            AppError::conflict("that username is taken or reserved")
        }
        e => e.into(),
    })
}

pub async fn find_user_by_name(state: &AppState, name: &str) -> AppResult<Option<UserRow>> {
    Ok(sqlx::query_as("SELECT * FROM users WHERE username = ?").bind(name.trim()).fetch_optional(&state.platform_db).await?)
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
    let user: Option<UserRow> =
        sqlx::query_as("SELECT * FROM users WHERE id = ?").bind(claims.sub).fetch_optional(&state.platform_db).await?;
    match user {
        Some(u) if u.status == "active" && u.auth_version == claims.version => Ok(u),
        Some(u) if u.status == "pending" => Err(AppError::forbidden("your account is waiting for approval")),
        _ => Err(AppError::unauthorized("account disabled or removed")),
    }
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
    failures: Mutex<HashMap<String, (u32, Instant)>>,
}

impl LoginGuard {
    const MAX: u32 = 10;
    const WINDOW: Duration = Duration::from_secs(300);

    pub fn check(&self, username: &str) -> AppResult<()> {
        let map = self.failures.lock().unwrap();
        if let Some((count, since)) = map.get(&username.to_lowercase()) {
            if *count >= Self::MAX && since.elapsed() < Self::WINDOW {
                return Err(AppError::new(
                    axum::http::StatusCode::TOO_MANY_REQUESTS,
                    "too many failed attempts, try again in a few minutes",
                ));
            }
        }
        Ok(())
    }

    pub fn fail(&self, username: &str) {
        let mut map = self.failures.lock().unwrap();
        let entry = map.entry(username.to_lowercase()).or_insert((0, Instant::now()));
        if entry.1.elapsed() >= Self::WINDOW {
            *entry = (0, Instant::now());
        }
        entry.0 += 1;
    }

    pub fn succeed(&self, username: &str) {
        self.failures.lock().unwrap().remove(&username.to_lowercase());
    }
}
