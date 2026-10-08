//! Authority-owned launcher account workflows with explicit local policy/audit ports.
use crate::{
    error::{AppError, AppResult},
    state::{AuthorityState, HostFuture},
};
use axum::http::StatusCode;
use std::sync::Arc;
use velora_auth_core::{identity::IdentityRecord, identity_store, password, tokens};
use velora_platform_contracts::{
    valid_username, AuthResponse, LoginRequest, PublicUser, RegisterRequest, RegistrationMode, YggdrasilTokens,
};

pub struct AccountPolicy {
    pub panel_accounts: bool,
    pub registration: RegistrationMode,
}
pub trait AccountHost: Send + Sync {
    fn policy(&self) -> HostFuture<'_, AppResult<AccountPolicy>>;
    fn check_username<'a>(&'a self, name: &'a str) -> HostFuture<'a, AppResult<()>>;
    fn record_login<'a>(&'a self, user: &'a IdentityRecord) -> HostFuture<'a, AppResult<()>>;
}
#[derive(Clone)]
pub struct AccountState {
    pub authority: AuthorityState,
    pub tokens: Arc<tokens::Keys>,
    pub host: Arc<dyn AccountHost>,
}
fn error(status: StatusCode, message: impl Into<String>) -> AppError {
    AppError { status, message: message.into() }
}
fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}
pub async fn public_user(state: &AccountState, user: &IdentityRecord) -> AppResult<PublicUser> {
    Ok(PublicUser {
        id: user.id,
        username: user.username.clone(),
        uuid: user.uuid.clone(),
        role: user.role.clone(),
        groups: identity_store::groups(&state.authority.identity_db, user.id).await?,
    })
}
pub async fn signed_in(state: &AccountState, user: &IdentityRecord) -> AppResult<AuthResponse> {
    let (access_token, client_token) = crate::issue_token(&state.authority, user.id, None).await?;
    let token = state
        .tokens
        .issue(tokens::Subject { id: user.id, name: &user.username, role: &user.role, auth_version: user.auth_version })
        .map_err(|e| error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(AuthResponse {
        token,
        user: public_user(state, user).await?,
        pending: false,
        yggdrasil: Some(YggdrasilTokens { access_token, client_token }),
    })
}
pub async fn login(state: &AccountState, request: LoginRequest) -> AppResult<AuthResponse> {
    let username = request.username.trim();
    state.authority.login_guard.check(username).map_err(|e| error(StatusCode::TOO_MANY_REQUESTS, e))?;
    let policy = state.host.policy().await?;
    let user = identity_store::find_by_name(&state.authority.identity_db, username).await?;
    let Some(user) = user.filter(|u| password::verify_password(&request.password, &u.password_hash)) else {
        state.authority.login_guard.fail(username);
        return Err(error(StatusCode::UNAUTHORIZED, "wrong username or password"));
    };
    state.authority.login_guard.succeed(username);
    match user.status.as_str() {
        "pending" => return Err(error(StatusCode::FORBIDDEN, "your account is waiting for an admin to approve it")),
        "disabled" => {
            return Err(error(StatusCode::FORBIDDEN, user.status_reason.clone().unwrap_or_else(|| "this account has been disabled".into())))
        }
        _ => {}
    }
    if !policy.panel_accounts && !user.is_admin() {
        return Err(error(StatusCode::FORBIDDEN, "account sign-in is currently disabled"));
    }
    sqlx::query("UPDATE users SET last_login = ? WHERE id = ?").bind(now()).bind(user.id).execute(&state.authority.db).await?;
    state.host.record_login(&user).await?;
    signed_in(state, &user).await
}
pub async fn register(state: &AccountState, request: RegisterRequest) -> AppResult<AuthResponse> {
    let policy = state.host.policy().await?;
    if !policy.panel_accounts || policy.registration == RegistrationMode::Closed {
        return Err(error(StatusCode::FORBIDDEN, "sign-ups are closed — ask an admin for an account"));
    }
    let username = request.username.trim();
    if !valid_username(username) {
        return Err(AppError::bad_request("usernames are 3-16 letters, numbers or underscores"));
    }
    password::validate_password(&request.password).map_err(AppError::bad_request)?;
    if identity_store::find_by_name(&state.authority.identity_db, username).await?.is_some() {
        return Err(error(StatusCode::CONFLICT, "that username is taken"));
    }
    let status = if policy.registration == RegistrationMode::Approval { "pending" } else { "active" };
    let id = create_account(state, username, &request.password, request.email.as_deref(), "player", status).await?;
    let user = identity_store::find_by_id(&state.authority.db, id).await?.ok_or(sqlx::Error::RowNotFound)?;
    if status == "pending" {
        return Ok(AuthResponse { token: String::new(), user: public_user(state, &user).await?, pending: true, yggdrasil: None });
    }
    signed_in(state, &user).await
}

/// Supplied caller policy validates roles/status/passwords as required by its flow.
/// The host retains live username policy; identity allocation is authority-owned.
pub async fn create_account(
    state: &AccountState,
    username: &str,
    password: &str,
    email: Option<&str>,
    role: &str,
    status: &str,
) -> AppResult<i64> {
    create_account_inner(state, username, password, AccountKind::Regular { email, role, status })
        .await?
        .ok_or(sqlx::Error::RowNotFound.into())
}

/// Restarts never alter existing administrators, even if bootstrap configuration changed.
pub async fn bootstrap_admin(state: &AccountState, username: &str, password: &str) -> AppResult<Option<i64>> {
    if identity_store::has_admin(&state.authority.identity_db).await? {
        return Ok(None);
    }
    create_account_inner(state, username, password, AccountKind::FirstAdmin).await
}

enum AccountKind<'a> {
    Regular { email: Option<&'a str>, role: &'a str, status: &'a str },
    FirstAdmin,
}

async fn create_account_inner(state: &AccountState, username: &str, password: &str, kind: AccountKind<'_>) -> AppResult<Option<i64>> {
    let (email, role, status, first_admin) = match kind {
        AccountKind::Regular { email, role, status } => (email, role, status, false),
        AccountKind::FirstAdmin => (None, "admin", "active", true),
    };
    state.host.check_username(username).await?;
    let hash = password::hash_password(password).map_err(|e| error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let created_at = now();
    let uuid = uuid::Uuid::new_v4().to_string();
    let account = identity_store::NewAccount { username, password_hash: &hash, email, role, status, created_at: &created_at, uuid: &uuid };
    let inserted = if first_admin {
        identity_store::insert_first_admin(&state.authority.identity_db, account).await
    } else {
        identity_store::insert(&state.authority.identity_db, account).await.map(Some)
    };
    let id = inserted.map_err(|e| match e {
        sqlx::Error::Database(d) if d.message().contains("UNIQUE") || d.message().contains("username reserved") => {
            error(StatusCode::CONFLICT, "that username is taken or reserved")
        }
        e => e.into(),
    })?;

    Ok(id)
}
