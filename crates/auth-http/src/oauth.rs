//! Discord identity workflows. Provider credentials and HTTP exchange use typed host ports.
use crate::{
    error::{AppError, AppResult},
    state::HostFuture,
    web::{self, AccountState},
};
use axum::http::StatusCode;
use serde_json::{json, Value};
use velora_auth_core::{identity::IdentityRecord, identity_store};
use velora_platform_contracts::RegistrationMode;
pub struct DiscordConfiguration {
    pub client_id: String,
    pub configured: bool,
}
pub struct AllocationPolicy {
    pub panel_accounts: bool,
    pub registration: RegistrationMode,
    pub username_blocklist: Vec<String>,
}
pub trait DiscordHost: Send + Sync {
    fn configuration(&self) -> HostFuture<'_, AppResult<DiscordConfiguration>>;
    fn public_base(&self) -> HostFuture<'_, AppResult<String>>;
    fn exchange_profile<'a>(&'a self, code: &'a str) -> HostFuture<'a, AppResult<Value>>;
    fn allocation_policy(&self) -> HostFuture<'_, AppResult<AllocationPolicy>>;
}
fn error(status: StatusCode, message: impl Into<String>) -> AppError {
    AppError { status, message: message.into() }
}
fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}
pub async fn start(state: &AccountState, host: &dyn DiscordHost, user: Option<&IdentityRecord>, kind: Option<&str>) -> AppResult<Value> {
    let config = host.configuration().await?;
    if !config.configured {
        return Err(AppError::bad_request("Discord sign-in is not configured"));
    }
    let kind = kind.unwrap_or("login");
    if !matches!(kind, "login" | "link") {
        return Err(AppError::bad_request("invalid Discord flow"));
    }
    if kind == "link" && user.is_none() {
        return Err(error(StatusCode::UNAUTHORIZED, "sign in before linking Discord"));
    }
    let state_token = uuid::Uuid::new_v4().to_string() + &uuid::Uuid::new_v4().to_string();
    sqlx::query("DELETE FROM oauth_attempts WHERE expires_at<?").bind(now()).execute(&state.authority.db).await?;
    let expires = (chrono::Utc::now() + chrono::Duration::minutes(10)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    sqlx::query("INSERT INTO oauth_attempts(state,kind,user_id,created_at,expires_at) VALUES(?,?,?,?,?)")
        .bind(&state_token)
        .bind(kind)
        .bind(user.map(|u| u.id))
        .bind(now())
        .bind(expires)
        .execute(&state.authority.db)
        .await?;
    let callback = format!("{}/api/v1/auth/discord/callback", host.public_base().await?);
    let url = format!(
        "https://discord.com/oauth2/authorize?response_type=code&client_id={}&scope=identify%20email&state={}&redirect_uri={}",
        config.client_id,
        state_token,
        percent_encoding::utf8_percent_encode(&callback, percent_encoding::NON_ALPHANUMERIC)
    );
    Ok(json!({"url":url,"state":state_token}))
}
pub async fn callback(
    state: &AccountState,
    host: &dyn DiscordHost,
    state_token: &str,
    code: Option<&str>,
    cancelled: bool,
) -> AppResult<&'static str> {
    let now = now();
    let attempt: Option<(String, Option<i64>)> =
        sqlx::query_as("SELECT kind,user_id FROM oauth_attempts WHERE state=? AND expires_at>? AND consumed_at IS NULL")
            .bind(state_token)
            .bind(&now)
            .fetch_optional(&state.authority.db)
            .await?;
    let Some((kind, linked_user)) = attempt else {
        return Err(AppError::bad_request("Discord sign-in expired; try again"));
    };
    let updated = sqlx::query("UPDATE oauth_attempts SET consumed_at=? WHERE state=? AND consumed_at IS NULL")
        .bind(&now)
        .bind(state_token)
        .execute(&state.authority.db)
        .await?
        .rows_affected();
    if updated == 0 {
        return Err(AppError::bad_request("Discord sign-in was already used"));
    }
    if cancelled || code.is_none() {
        sqlx::query("UPDATE oauth_attempts SET result=? WHERE state=?")
            .bind("error:Discord authorization was cancelled")
            .bind(state_token)
            .execute(&state.authority.db)
            .await?;
        return Ok("Discord authorization was cancelled. You may close this window.");
    }
    let profile = host.exchange_profile(code.unwrap_or("")).await?;
    let discord_id = profile["id"].as_str().ok_or_else(|| AppError::bad_request("Discord ID missing"))?;
    let display = profile["global_name"].as_str().or_else(|| profile["username"].as_str()).unwrap_or("Discord");
    let existing: Option<(i64,)> = sqlx::query_as("SELECT user_id FROM account_connections WHERE provider='discord' AND provider_id=?")
        .bind(discord_id)
        .fetch_optional(&state.authority.db)
        .await?;
    let user_id = if kind == "link" {
        let id = linked_user.ok_or_else(|| AppError::bad_request("link target missing"))?;
        if existing.is_some_and(|(owner,)| owner != id) {
            return Err(error(StatusCode::CONFLICT, "Discord account is linked to another player"));
        }
        sqlx::query("DELETE FROM account_connections WHERE user_id=? AND provider='discord'").bind(id).execute(&state.authority.db).await?;
        sqlx::query("INSERT INTO account_connections(user_id,provider,provider_id,display_name,created_at) VALUES(?,'discord',?,?,?)")
            .bind(id)
            .bind(discord_id)
            .bind(display)
            .bind(&now)
            .execute(&state.authority.db)
            .await?;
        id
    } else if let Some((id,)) = existing {
        id
    } else {
        let policy = host.allocation_policy().await?;
        if !policy.panel_accounts || policy.registration == RegistrationMode::Closed {
            return Err(error(StatusCode::FORBIDDEN, "Discord account is not linked; create an account first"));
        }
        let raw_name = profile["username"].as_str().unwrap_or("player");
        let mut base: String = raw_name.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '_').take(12).collect();
        if base.len() < 3 {
            base = "Discord".into();
        }
        if velora_auth_core::identity::username_blocked(&base, &policy.username_blocklist) {
            base = "Player".into();
        }
        let mut name = base.clone();
        for i in 0..1000 {
            if identity_store::find_by_name(&state.authority.identity_db, &name).await?.is_none() {
                break;
            }
            name = format!("{}{}", base, i);
        }
        if identity_store::find_by_name(&state.authority.identity_db, &name).await?.is_some() {
            return Err(error(StatusCode::CONFLICT, "could not allocate a username"));
        }
        let status = if policy.registration == RegistrationMode::Approval { "pending" } else { "active" };
        let random_password = uuid::Uuid::new_v4().to_string() + &uuid::Uuid::new_v4().to_string();
        let email = profile["email"].as_str().filter(|_| profile["verified"].as_bool() == Some(true));
        let id = web::create_account(state, &name, &random_password, email, "player", status).await?;
        sqlx::query("INSERT INTO account_connections(user_id,provider,provider_id,display_name,created_at) VALUES(?,'discord',?,?,?)")
            .bind(id)
            .bind(discord_id)
            .bind(display)
            .bind(&now)
            .execute(&state.authority.db)
            .await?;
        id
    };
    let user = identity_store::find_by_id(&state.authority.db, user_id).await?.ok_or(sqlx::Error::RowNotFound)?;
    let result = if kind == "link" {
        json!({"linked":true})
    } else if user.status == "disabled" {
        return Err(error(StatusCode::FORBIDDEN, "this account is disabled"));
    } else if user.status != "active" {
        json!({"pending":true})
    } else if !host.allocation_policy().await?.panel_accounts && !user.is_admin() {
        return Err(error(StatusCode::FORBIDDEN, "account sign-in is disabled"));
    } else {
        serde_json::to_value(web::signed_in(state, &user).await?).map_err(|e| AppError::bad_request(format!("invalid JSON: {e}")))?
    };
    sqlx::query("UPDATE oauth_attempts SET result=? WHERE state=?")
        .bind(result.to_string())
        .bind(state_token)
        .execute(&state.authority.db)
        .await?;
    Ok("Discord authorization complete. Return to Velora and close this window.")
}
pub async fn poll(state: &AccountState, state_token: &str) -> AppResult<Value> {
    let row: Option<(Option<String>,)> = sqlx::query_as("SELECT result FROM oauth_attempts WHERE state=? AND expires_at>?")
        .bind(state_token)
        .bind(now())
        .fetch_optional(&state.authority.db)
        .await?;
    let Some((result,)) = row else {
        return Err(AppError::bad_request("Discord sign-in expired"));
    };
    if let Some(result) = result {
        let changed =
            sqlx::query("DELETE FROM oauth_attempts WHERE state=?").bind(state_token).execute(&state.authority.db).await?.rows_affected();
        if changed == 0 {
            return Err(AppError::bad_request("Discord result was already consumed"));
        }
        if result.starts_with("error:") {
            return Err(AppError::bad_request(result));
        }
        return serde_json::from_str(&result).map_err(|e| AppError::bad_request(format!("invalid JSON: {e}")));
    }
    Ok(json!({"pending":true}))
}
pub async fn linked(state: &AccountState, user_id: i64) -> AppResult<Value> {
    let row: Option<(String, String)> =
        sqlx::query_as("SELECT provider_id,display_name FROM account_connections WHERE user_id=? AND provider='discord'")
            .bind(user_id)
            .fetch_optional(&state.authority.db)
            .await?;
    Ok(match row {
        Some((id, name)) => json!({"id":id,"name":name}),
        None => Value::Null,
    })
}
pub async fn unlink(state: &AccountState, user_id: i64) -> AppResult<Value> {
    sqlx::query("DELETE FROM account_connections WHERE user_id=? AND provider='discord'")
        .bind(user_id)
        .execute(&state.authority.db)
        .await?;
    Ok(json!({"ok":true}))
}
