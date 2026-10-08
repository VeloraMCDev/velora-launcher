//! Administrator-managed Discord OAuth and Resend SMTP.
use crate::state::RequestState as State;
use crate::auth::{self, AdminUser, AuthUser, MaybeUser, UserRow};
use crate::error::{AppError, AppResult};
use crate::routes::public;
use crate::state::AppState;
use crate::store;
use axum::extract::{Query};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse};
use axum::Json;
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ConnectionsSettings {
    pub discord_client_id: String,
    pub discord_client_secret: String,
    pub discord_bot_token: String,
    pub discord_guild_id: String,
    pub resend_api_key: String,
    pub sender_email: String,
    pub sender_name: String,
}

pub async fn settings(state: &AppState) -> AppResult<ConnectionsSettings> {
    store::kv_get(state, "connections_settings").await
}

async fn configured_base(state: &AppState) -> AppResult<String> {
    let configured = store::settings(state)
        .await?
        .public_url
        .or_else(|| state.cfg.public_url.clone())
        .ok_or_else(|| AppError::bad_request("set the panel Public address in Settings before using Discord or password reset"))?;
    if !configured.starts_with("https://") && !configured.starts_with("http://localhost") {
        return Err(AppError::bad_request("the panel Public address must use HTTPS"));
    }
    Ok(configured.trim_end_matches('/').to_string())
}

fn masked(s: &ConnectionsSettings) -> Value {
    json!({
        "discord_client_id": s.discord_client_id,
        "discord_client_secret_set": !s.discord_client_secret.is_empty(),
        "discord_bot_token_set": !s.discord_bot_token.is_empty(),
        "discord_guild_id": s.discord_guild_id,
        "resend_api_key_set": !s.resend_api_key.is_empty(),
        "sender_email": s.sender_email,
        "sender_name": s.sender_name,
        "discord_enabled": !s.discord_client_id.is_empty() && !s.discord_client_secret.is_empty(),
        "email_enabled": !s.resend_api_key.is_empty() && !s.sender_email.is_empty()
    })
}

pub async fn admin_get(_: AdminUser, State(state): State<AppState>) -> AppResult<Json<Value>> {
    Ok(Json(masked(&settings(&state).await?)))
}

#[derive(Deserialize)]
pub struct SettingsInput {
    discord_client_id: String,
    discord_client_secret: String,
    discord_bot_token: String,
    discord_guild_id: String,
    resend_api_key: String,
    sender_email: String,
    sender_name: String,
}

fn secret(input: &str, previous: &str) -> String {
    match input.trim() {
        "" => previous.to_string(),
        "-" => String::new(),
        value => value.to_string(),
    }
}

pub async fn admin_put(_: AdminUser, State(state): State<AppState>, Json(input): Json<SettingsInput>) -> AppResult<Json<Value>> {
    let old = settings(&state).await?;
    let next = ConnectionsSettings {
        discord_client_id: input.discord_client_id.trim().to_string(),
        discord_client_secret: secret(&input.discord_client_secret, &old.discord_client_secret),
        discord_bot_token: secret(&input.discord_bot_token, &old.discord_bot_token),
        discord_guild_id: input.discord_guild_id.trim().to_string(),
        resend_api_key: secret(&input.resend_api_key, &old.resend_api_key),
        sender_email: input.sender_email.trim().to_string(),
        sender_name: input.sender_name.trim().to_string(),
    };
    if !next.sender_email.is_empty() && next.sender_email.parse::<Mailbox>().is_err() {
        return Err(AppError::bad_request("enter a valid sender email address"));
    }
    if !next.discord_client_id.is_empty() && !next.discord_client_id.bytes().all(|b| b.is_ascii_digit()) {
        return Err(AppError::bad_request("Discord client ID must contain digits only"));
    }
    if !next.discord_guild_id.is_empty() && !next.discord_guild_id.bytes().all(|b| b.is_ascii_digit()) {
        return Err(AppError::bad_request("Discord server ID must contain digits only"));
    }
    store::kv_set(&state, "connections_settings", &next).await?;
    Ok(Json(masked(&next)))
}

pub async fn public_config(State(state): State<AppState>) -> AppResult<Json<Value>> {
    let s = settings(&state).await?;
    Ok(Json(json!({"discord_enabled": !s.discord_client_id.is_empty() && !s.discord_client_secret.is_empty(),
        "email_enabled": !s.resend_api_key.is_empty() && !s.sender_email.is_empty()})))
}

async fn send_email(state: &AppState, to: &str, subject: &str, body: &str) -> AppResult<()> {
    send_mail(state, to, subject, body, None).await
}

/// Send one message, as plain text and, when given, an HTML version of the same content.
pub async fn send_mail(state: &AppState, to: &str, subject: &str, body: &str, html: Option<&str>) -> AppResult<()> {
    let s = settings(state).await?;
    if s.resend_api_key.is_empty() || s.sender_email.is_empty() {
        return Err(AppError::bad_request("configure Resend SMTP and a sender email in Settings first"));
    }
    let from: Mailbox =
        if s.sender_name.is_empty() { s.sender_email.parse() } else { format!("{} <{}>", s.sender_name, s.sender_email).parse() }
            .map_err(|_| AppError::bad_request("invalid sender email"))?;
    let to: Mailbox = to.parse().map_err(|_| AppError::bad_request("invalid recipient email"))?;
    let builder = Message::builder().from(from).to(to).subject(subject);
    let message = match html {
        Some(h) => builder.multipart(lettre::message::MultiPart::alternative_plain_html(body.to_string(), h.to_string())),
        None => builder.body(body.to_string()),
    }
    .map_err(|_| AppError::bad_request("invalid email message"))?;
    let smtp = AsyncSmtpTransport::<Tokio1Executor>::relay("smtp.resend.com")
        .map_err(|e| AppError::bad_request(format!("SMTP configuration failed: {e}")))?
        .credentials(Credentials::new("resend".into(), s.resend_api_key))
        .build();
    smtp.send(message).await.map_err(|e| AppError::new(StatusCode::BAD_GATEWAY, format!("Resend SMTP rejected the email: {e}")))?;
    Ok(())
}

#[derive(Deserialize)]
pub struct TestEmail {
    email: String,
}

pub async fn admin_test_email(_: AdminUser, State(state): State<AppState>, Json(input): Json<TestEmail>) -> AppResult<Json<Value>> {
    send_email(&state, &input.email, "SCOPENET email test", "Your SCOPENET email settings are working.\n\nThis confirms SMTP accepted the message. Check your inbox and spam folder to confirm delivery.").await?;
    Ok(Json(json!({"ok": true, "message": "Resend accepted the test email; check the destination inbox for final delivery."})))
}

#[derive(Deserialize)]
pub struct ForgotInput {
    email: String,
}

pub async fn forgot_password(State(state): State<AppState>, Json(input): Json<ForgotInput>) -> AppResult<Json<Value>> {
    let email = input.email.trim();
    let generic = json!({"ok": true, "message": "If this address has an account, a reset link is on its way."});
    if email.is_empty() || !email.contains('@') {
        return Ok(Json(generic));
    }
    let s = settings(&state).await?;
    if s.resend_api_key.is_empty() || s.sender_email.is_empty() {
        return Err(AppError::bad_request("password reset email is not configured"));
    }
    let base = configured_base(&state).await?;
    let user: Option<(i64,)> = sqlx::query_as("SELECT id FROM users WHERE lower(email)=lower(?) AND status='active' LIMIT 1")
        .bind(email)
        .fetch_optional(&state.db)
        .await?;
    if let Some((id,)) = user {
        let throttle = (chrono::Utc::now() + chrono::Duration::minutes(25)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        let recently_sent: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM password_resets WHERE user_id=? AND used_at IS NULL AND expires_at>?)")
                .bind(id)
                .bind(throttle)
                .fetch_one(&state.db)
                .await?;
        if recently_sent {
            return Ok(Json(generic));
        }
        let token = uuid::Uuid::new_v4().to_string() + &uuid::Uuid::new_v4().to_string();
        let hash = hex::encode(Sha256::digest(token.as_bytes()));
        let expires = (chrono::Utc::now() + chrono::Duration::minutes(30)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        sqlx::query("DELETE FROM password_resets WHERE user_id=?").bind(id).execute(&state.db).await?;
        sqlx::query("INSERT INTO password_resets(token_hash,user_id,expires_at) VALUES(?,?,?)")
            .bind(&hash)
            .bind(id)
            .bind(expires)
            .execute(&state.db)
            .await?;
        let url = format!("{base}/#/reset-password?token={token}");
        if let Err(e) = send_email(&state, email, "Reset your SCOPENET password", &format!("Use this link to reset your password. It expires in 30 minutes.\n\n{url}\n\nIf you did not request this, ignore this email.")).await {
            tracing::warn!("password reset email failed: {}", e.message);
            sqlx::query("DELETE FROM password_resets WHERE token_hash=?").bind(&hash).execute(&state.db).await?;
        }
    }
    Ok(Json(generic))
}

#[derive(Deserialize)]
pub struct ResetInput {
    token: String,
    password: String,
}

pub async fn reset_password(State(state): State<AppState>, Json(input): Json<ResetInput>) -> AppResult<Json<Value>> {
    auth::validate_password(&input.password)?;
    let hash = hex::encode(Sha256::digest(input.token.as_bytes()));
    let now = crate::db::now();
    let password_hash = auth::hash_password(&input.password)?;
    let mut tx = state.db.begin().await?;
    let changed = sqlx::query("UPDATE password_resets SET used_at=? WHERE token_hash=? AND used_at IS NULL AND expires_at>?")
        .bind(&now)
        .bind(&hash)
        .bind(&now)
        .execute(&mut *tx)
        .await?
        .rows_affected();
    if changed == 0 {
        return Err(AppError::bad_request("this reset link is invalid or expired"));
    }
    sqlx::query(
        "UPDATE users SET password_hash=?, auth_version=auth_version+1 WHERE id=(SELECT user_id FROM password_resets WHERE token_hash=?)",
    )
    .bind(password_hash)
    .bind(&hash)
    .execute(&mut *tx)
    .await?;
    sqlx::query("DELETE FROM ygg_tokens WHERE user_id=(SELECT user_id FROM password_resets WHERE token_hash=?)")
        .bind(&hash)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM ygg_sessions WHERE user_id=(SELECT user_id FROM password_resets WHERE token_hash=?)")
        .bind(&hash)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Json(json!({"ok":true})))
}

#[derive(Deserialize)]
pub struct OAuthStart {
    kind: Option<String>,
}

pub async fn discord_start(
    State(state): State<AppState>,
    MaybeUser(user): MaybeUser,
    Query(input): Query<OAuthStart>,
) -> AppResult<Json<Value>> {
    let s = settings(&state).await?;
    if s.discord_client_id.is_empty() || s.discord_client_secret.is_empty() {
        return Err(AppError::bad_request("Discord sign-in is not configured"));
    }
    let kind = input.kind.as_deref().unwrap_or("login");
    if !matches!(kind, "login" | "link") {
        return Err(AppError::bad_request("invalid Discord flow"));
    }
    if kind == "link" && user.is_none() {
        return Err(AppError::unauthorized("sign in before linking Discord"));
    }
    let state_token = uuid::Uuid::new_v4().to_string() + &uuid::Uuid::new_v4().to_string();
    sqlx::query("DELETE FROM oauth_attempts WHERE expires_at<?").bind(crate::db::now()).execute(&state.db).await?;
    let expires = (chrono::Utc::now() + chrono::Duration::minutes(10)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    sqlx::query("INSERT INTO oauth_attempts(state,kind,user_id,created_at,expires_at) VALUES(?,?,?,?,?)")
        .bind(&state_token)
        .bind(kind)
        .bind(user.map(|u| u.id))
        .bind(crate::db::now())
        .bind(expires)
        .execute(&state.db)
        .await?;
    let callback = format!("{}/api/v1/auth/discord/callback", configured_base(&state).await?);
    let url = format!(
        "https://discord.com/oauth2/authorize?response_type=code&client_id={}&scope=identify%20email&state={}&redirect_uri={}",
        s.discord_client_id,
        state_token,
        percent_encoding::utf8_percent_encode(&callback, percent_encoding::NON_ALPHANUMERIC)
    );
    Ok(Json(json!({"url":url,"state":state_token})))
}

#[derive(Deserialize)]
pub struct OAuthCallback {
    code: Option<String>,
    state: String,
    error: Option<String>,
}

pub async fn discord_callback(State(state): State<AppState>, Query(input): Query<OAuthCallback>) -> AppResult<impl IntoResponse> {
    let now = crate::db::now();
    let attempt: Option<(String, Option<i64>)> =
        sqlx::query_as("SELECT kind,user_id FROM oauth_attempts WHERE state=? AND expires_at>? AND consumed_at IS NULL")
            .bind(&input.state)
            .bind(&now)
            .fetch_optional(&state.db)
            .await?;
    let Some((kind, linked_user)) = attempt else {
        return Err(AppError::bad_request("Discord sign-in expired; try again"));
    };
    let updated = sqlx::query("UPDATE oauth_attempts SET consumed_at=? WHERE state=? AND consumed_at IS NULL")
        .bind(&now)
        .bind(&input.state)
        .execute(&state.db)
        .await?
        .rows_affected();
    if updated == 0 {
        return Err(AppError::bad_request("Discord sign-in was already used"));
    }
    if input.error.is_some() || input.code.is_none() {
        sqlx::query("UPDATE oauth_attempts SET result=? WHERE state=?")
            .bind("error:Discord authorization was cancelled")
            .bind(&input.state)
            .execute(&state.db)
            .await?;
        return Ok(Html("Discord authorization was cancelled. You may close this window."));
    }
    let s = settings(&state).await?;
    let callback = format!("{}/api/v1/auth/discord/callback", configured_base(&state).await?);
    let response = state
        .http
        .post("https://discord.com/api/v10/oauth2/token")
        .form(&[
            ("client_id", s.discord_client_id.as_str()),
            ("client_secret", s.discord_client_secret.as_str()),
            ("grant_type", "authorization_code"),
            ("code", input.code.as_deref().unwrap_or("")),
            ("redirect_uri", callback.as_str()),
        ])
        .send()
        .await
        .map_err(|e| AppError::new(StatusCode::BAD_GATEWAY, format!("Discord token exchange failed: {e}")))?;
    if !response.status().is_success() {
        return Err(AppError::bad_request("Discord rejected authorization"));
    }
    let token: Value = response.json().await.map_err(|_| AppError::bad_request("invalid Discord response"))?;
    let bearer = token["access_token"].as_str().ok_or_else(|| AppError::bad_request("Discord token missing"))?;
    let response = state
        .http
        .get("https://discord.com/api/v10/users/@me")
        .bearer_auth(bearer)
        .send()
        .await
        .map_err(|e| AppError::new(StatusCode::BAD_GATEWAY, format!("Discord profile failed: {e}")))?;
    if !response.status().is_success() {
        return Err(AppError::bad_request("Discord profile unavailable"));
    }
    let profile: Value = response.json().await.map_err(|_| AppError::bad_request("invalid Discord profile"))?;
    let discord_id = profile["id"].as_str().ok_or_else(|| AppError::bad_request("Discord ID missing"))?;
    let display = profile["global_name"].as_str().or_else(|| profile["username"].as_str()).unwrap_or("Discord");
    let existing: Option<(i64,)> = sqlx::query_as("SELECT user_id FROM account_connections WHERE provider='discord' AND provider_id=?")
        .bind(discord_id)
        .fetch_optional(&state.db)
        .await?;
    let user_id = if kind == "link" {
        let id = linked_user.ok_or_else(|| AppError::bad_request("link target missing"))?;
        if existing.is_some_and(|(owner,)| owner != id) {
            return Err(AppError::conflict("Discord account is linked to another player"));
        }
        sqlx::query("DELETE FROM account_connections WHERE user_id=? AND provider='discord'").bind(id).execute(&state.db).await?;
        sqlx::query("INSERT INTO account_connections(user_id,provider,provider_id,display_name,created_at) VALUES(?,'discord',?,?,?)")
            .bind(id)
            .bind(discord_id)
            .bind(display)
            .bind(&now)
            .execute(&state.db)
            .await?;
        id
    } else if let Some((id,)) = existing {
        id
    } else {
        let app_settings = store::settings(&state).await?;
        if !app_settings.auth.panel_accounts || app_settings.auth.registration == scopenet_shared::RegistrationMode::Closed {
            return Err(AppError::forbidden("Discord account is not linked; create an account first"));
        }
        let raw_name = profile["username"].as_str().unwrap_or("player");
        let mut base: String = raw_name.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '_').take(12).collect();
        if base.len() < 3 {
            base = "Discord".into();
        }
        if store::username_blocked(&base, &app_settings.username_blocklist) {
            base = "Player".into();
        }
        let mut name = base.clone();
        for i in 0..1000 {
            if auth::find_user_by_name(&state, &name).await?.is_none() {
                break;
            }
            name = format!("{}{}", base, i);
        }
        if auth::find_user_by_name(&state, &name).await?.is_some() {
            return Err(AppError::conflict("could not allocate a username"));
        }
        let status = if app_settings.auth.registration == scopenet_shared::RegistrationMode::Approval { "pending" } else { "active" };
        let random_password = uuid::Uuid::new_v4().to_string() + &uuid::Uuid::new_v4().to_string();
        let email = profile["email"].as_str().filter(|_| profile["verified"].as_bool() == Some(true));
        let id = auth::create_user(&state, &name, &random_password, email, "player", status).await?;
        sqlx::query("INSERT INTO account_connections(user_id,provider,provider_id,display_name,created_at) VALUES(?,'discord',?,?,?)")
            .bind(id)
            .bind(discord_id)
            .bind(display)
            .bind(&now)
            .execute(&state.db)
            .await?;
        id
    };
    let user: UserRow = sqlx::query_as("SELECT * FROM users WHERE id=?").bind(user_id).fetch_one(&state.db).await?;
    let result = if kind == "link" {
        json!({"linked":true})
    } else if user.status == "disabled" {
        return Err(AppError::forbidden("this account is disabled"));
    } else if user.status != "active" {
        json!({"pending":true})
    } else if !store::settings(&state).await?.auth.panel_accounts && !user.is_admin() {
        return Err(AppError::forbidden("account sign-in is disabled"));
    } else {
        serde_json::to_value(public::signed_in(&state, &user).await?).map_err(AppError::from)?
    };
    sqlx::query("UPDATE oauth_attempts SET result=? WHERE state=?").bind(result.to_string()).bind(&input.state).execute(&state.db).await?;
    Ok(Html("Discord authorization complete. Return to SCOPENET and close this window."))
}

#[derive(Deserialize)]
pub struct Poll {
    state: String,
}

pub async fn discord_poll(State(state): State<AppState>, Query(input): Query<Poll>) -> AppResult<Json<Value>> {
    let row: Option<(Option<String>,)> = sqlx::query_as("SELECT result FROM oauth_attempts WHERE state=? AND expires_at>?")
        .bind(&input.state)
        .bind(crate::db::now())
        .fetch_optional(&state.db)
        .await?;
    let Some((result,)) = row else {
        return Err(AppError::bad_request("Discord sign-in expired"));
    };
    if let Some(result) = result {
        let changed = sqlx::query("DELETE FROM oauth_attempts WHERE state=?").bind(&input.state).execute(&state.db).await?.rows_affected();
        if changed == 0 {
            return Err(AppError::bad_request("Discord result was already consumed"));
        }
        if result.starts_with("error:") {
            return Err(AppError::bad_request(result));
        }
        return Ok(Json(serde_json::from_str(&result)?));
    }
    Ok(Json(json!({"pending":true})))
}

pub async fn my_discord(State(state): State<AppState>, AuthUser(user): AuthUser) -> AppResult<Json<Value>> {
    let row: Option<(String, String)> =
        sqlx::query_as("SELECT provider_id,display_name FROM account_connections WHERE user_id=? AND provider='discord'")
            .bind(user.id)
            .fetch_optional(&state.db)
            .await?;
    Ok(Json(match row {
        Some((id, name)) => json!({"id":id,"name":name}),
        None => Value::Null,
    }))
}

pub async fn unlink_discord(State(state): State<AppState>, AuthUser(user): AuthUser) -> AppResult<Json<Value>> {
    sqlx::query("DELETE FROM account_connections WHERE user_id=? AND provider='discord'").bind(user.id).execute(&state.db).await?;
    Ok(Json(json!({"ok":true})))
}
