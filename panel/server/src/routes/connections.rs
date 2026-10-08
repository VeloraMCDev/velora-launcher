//! Administrator-managed Discord OAuth and Resend SMTP.
use crate::auth::{AdminUser, AuthUser, MaybeUser};
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::state::RequestState as State;
use crate::store;
use axum::extract::Query;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

pub use velora_panel_communications::settings::{ConnectionsSettings, SettingsInput};
use velora_panel_communications::{settings as communication_settings, ErrorKind};

fn communication_error(error: velora_panel_communications::Error) -> AppError {
    let status = match error.kind {
        ErrorKind::BadRequest => StatusCode::BAD_REQUEST,
        ErrorKind::BadGateway => StatusCode::BAD_GATEWAY,
    };
    AppError::new(status, error.message)
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

pub async fn admin_get(_: AdminUser, State(state): State<AppState>) -> AppResult<Json<Value>> {
    Ok(Json(communication_settings::masked(&settings(&state).await?)))
}

pub async fn admin_put(_: AdminUser, State(state): State<AppState>, Json(input): Json<SettingsInput>) -> AppResult<Json<Value>> {
    let old = settings(&state).await?;
    let next = communication_settings::updated(&old, input).map_err(communication_error)?;
    store::kv_set(&state, "connections_settings", &next).await?;
    Ok(Json(communication_settings::masked(&next)))
}

pub async fn public_config(State(state): State<AppState>) -> AppResult<Json<Value>> {
    let s = settings(&state).await?;
    Ok(Json(communication_settings::public_config(&s)))
}

async fn send_email(state: &AppState, to: &str, subject: &str, body: &str) -> AppResult<()> {
    send_mail(state, to, subject, body, None).await
}

/// Send one message, as plain text and, when given, an HTML version of the same content.
pub async fn send_mail(state: &AppState, to: &str, subject: &str, body: &str, html: Option<&str>) -> AppResult<()> {
    let s = settings(state).await?;
    velora_panel_communications::mail::send(&s, to, subject, body, html).await.map_err(communication_error)
}

#[derive(Deserialize)]
pub struct TestEmail {
    email: String,
}

pub async fn admin_test_email(_: AdminUser, State(state): State<AppState>, Json(input): Json<TestEmail>) -> AppResult<Json<Value>> {
    send_email(&state, &input.email, "Velora email test", "Your Velora email settings are working.\n\nThis confirms SMTP accepted the message. Check your inbox and spam folder to confirm delivery.").await?;
    Ok(Json(json!({"ok": true, "message": "Resend accepted the test email; check the destination inbox for final delivery."})))
}

#[derive(Deserialize)]
pub struct ForgotInput {
    email: String,
}

struct PasswordResetHost(AppState);
fn reset_owner_error(error: AppError) -> velora_auth_http::error::AppError {
    velora_auth_http::error::AppError { status: error.status, message: error.message }
}
fn reset_host_error(error: velora_auth_http::error::AppError) -> AppError {
    AppError::new(error.status, error.message)
}
impl velora_auth_http::reset::ResetHost for PasswordResetHost {
    fn email_base(&self) -> velora_auth_http::state::HostFuture<'_, Result<String, velora_auth_http::error::AppError>> {
        Box::pin(async move {
            let configured = settings(&self.0).await.map_err(reset_owner_error)?;
            if configured.resend_api_key.is_empty() || configured.sender_email.is_empty() {
                return Err(reset_owner_error(AppError::bad_request("password reset email is not configured")));
            }
            configured_base(&self.0).await.map_err(reset_owner_error)
        })
    }
    fn send_email<'a>(
        &'a self,
        to: &'a str,
        subject: &'a str,
        body: &'a str,
    ) -> velora_auth_http::state::HostFuture<'a, Result<(), velora_auth_http::error::AppError>> {
        Box::pin(async move { send_email(&self.0, to, subject, body).await.map_err(reset_owner_error) })
    }
}
pub async fn forgot_password(State(state): State<AppState>, Json(input): Json<ForgotInput>) -> AppResult<Json<Value>> {
    Ok(Json(
        velora_auth_http::reset::forgot_password(&state.db, &PasswordResetHost(state.clone()), &input.email)
            .await
            .map_err(reset_host_error)?,
    ))
}
#[derive(Deserialize)]
pub struct ResetInput {
    token: String,
    password: String,
}
pub async fn reset_password(State(state): State<AppState>, Json(input): Json<ResetInput>) -> AppResult<Json<Value>> {
    Ok(Json(velora_auth_http::reset::reset_password(&state.db, &input.token, &input.password).await.map_err(reset_host_error)?))
}

struct DiscordHost(AppState);
impl velora_auth_http::oauth::DiscordHost for DiscordHost {
    fn configuration(
        &self,
    ) -> velora_auth_http::state::HostFuture<'_, Result<velora_auth_http::oauth::DiscordConfiguration, velora_auth_http::error::AppError>>
    {
        Box::pin(async move {
            let configured = settings(&self.0).await.map_err(reset_owner_error)?;
            let ready = !configured.discord_client_id.is_empty() && !configured.discord_client_secret.is_empty();
            Ok(velora_auth_http::oauth::DiscordConfiguration { client_id: configured.discord_client_id, configured: ready })
        })
    }
    fn public_base(&self) -> velora_auth_http::state::HostFuture<'_, Result<String, velora_auth_http::error::AppError>> {
        Box::pin(async move { configured_base(&self.0).await.map_err(reset_owner_error) })
    }
    fn exchange_profile<'a>(
        &'a self,
        code: &'a str,
    ) -> velora_auth_http::state::HostFuture<'a, Result<Value, velora_auth_http::error::AppError>> {
        Box::pin(async move {
            let result: AppResult<Value> = async move {
                let state = &self.0;
                let s = settings(&state).await?;
                let callback = format!("{}/api/v1/auth/discord/callback", configured_base(&state).await?);
                velora_panel_communications::discord::profile(&state.http, &s, &callback, code).await.map_err(communication_error)
            }
            .await;
            result.map_err(reset_owner_error)
        })
    }
    fn allocation_policy(
        &self,
    ) -> velora_auth_http::state::HostFuture<'_, Result<velora_auth_http::oauth::AllocationPolicy, velora_auth_http::error::AppError>> {
        Box::pin(async move {
            let configured = store::settings(&self.0).await.map_err(reset_owner_error)?;
            Ok(velora_auth_http::oauth::AllocationPolicy {
                panel_accounts: configured.auth.panel_accounts,
                registration: crate::auth::account::registration(configured.auth.registration),
                username_blocklist: configured.username_blocklist,
            })
        })
    }
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
    Ok(Json(
        velora_auth_http::oauth::start(
            &crate::auth::account::context(&state),
            &DiscordHost(state.clone()),
            user.as_ref(),
            input.kind.as_deref(),
        )
        .await
        .map_err(reset_host_error)?,
    ))
}
#[derive(Deserialize)]
pub struct OAuthCallback {
    code: Option<String>,
    state: String,
    error: Option<String>,
}
pub async fn discord_callback(State(state): State<AppState>, Query(input): Query<OAuthCallback>) -> AppResult<impl IntoResponse> {
    Ok(Html(
        velora_auth_http::oauth::callback(
            &crate::auth::account::context(&state),
            &DiscordHost(state.clone()),
            &input.state,
            input.code.as_deref(),
            input.error.is_some(),
        )
        .await
        .map_err(reset_host_error)?,
    ))
}
#[derive(Deserialize)]
pub struct Poll {
    state: String,
}
pub async fn discord_poll(State(state): State<AppState>, Query(input): Query<Poll>) -> AppResult<Json<Value>> {
    Ok(Json(velora_auth_http::oauth::poll(&crate::auth::account::context(&state), &input.state).await.map_err(reset_host_error)?))
}
pub async fn my_discord(State(state): State<AppState>, AuthUser(user): AuthUser) -> AppResult<Json<Value>> {
    Ok(Json(velora_auth_http::oauth::linked(&crate::auth::account::context(&state), user.id).await.map_err(reset_host_error)?))
}
pub async fn unlink_discord(State(state): State<AppState>, AuthUser(user): AuthUser) -> AppResult<Json<Value>> {
    Ok(Json(velora_auth_http::oauth::unlink(&crate::auth::account::context(&state), user.id).await.map_err(reset_host_error)?))
}
