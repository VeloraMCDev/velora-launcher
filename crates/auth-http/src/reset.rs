//! Password recovery workflows. The host supplies configured public origins and email delivery.
use crate::{
    error::{AppError, AppResult},
    state::HostFuture,
};
use axum::http::StatusCode;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
pub trait ResetHost: Send + Sync {
    /// Preserve email readiness checks before looking up an account, then validate the configured origin.
    fn email_base(&self) -> HostFuture<'_, AppResult<String>>;
    fn send_email<'a>(&'a self, to: &'a str, subject: &'a str, body: &'a str) -> HostFuture<'a, AppResult<()>>;
}
pub async fn forgot_password(pool: &SqlitePool, host: &dyn ResetHost, input_email: &str) -> AppResult<Value> {
    let email = input_email.trim();
    let generic = json!({"ok": true, "message": "If this address has an account, a reset link is on its way."});
    if email.is_empty() || !email.contains('@') {
        return Ok(generic);
    }
    let base = host.email_base().await?;
    let user: Option<(i64,)> = sqlx::query_as("SELECT id FROM users WHERE lower(email)=lower(?) AND status='active' LIMIT 1")
        .bind(email)
        .fetch_optional(pool)
        .await?;
    if let Some((id,)) = user {
        let throttle = (chrono::Utc::now() + chrono::Duration::minutes(25)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        let recently_sent: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM password_resets WHERE user_id=? AND used_at IS NULL AND expires_at>?)")
                .bind(id)
                .bind(throttle)
                .fetch_one(pool)
                .await?;
        if recently_sent {
            return Ok(generic);
        }
        let token = uuid::Uuid::new_v4().to_string() + &uuid::Uuid::new_v4().to_string();
        let hash = hex::encode(Sha256::digest(token.as_bytes()));
        let expires = (chrono::Utc::now() + chrono::Duration::minutes(30)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        sqlx::query("DELETE FROM password_resets WHERE user_id=?").bind(id).execute(pool).await?;
        sqlx::query("INSERT INTO password_resets(token_hash,user_id,expires_at) VALUES(?,?,?)")
            .bind(&hash)
            .bind(id)
            .bind(expires)
            .execute(pool)
            .await?;
        let url = format!("{base}/#/reset-password?token={token}");
        if let Err(error) = host.send_email(email, "Reset your Velora password", &format!("Use this link to reset your password. It expires in 30 minutes.\n\n{url}\n\nIf you did not request this, ignore this email.")).await {
            tracing::warn!("password reset email failed: {}", error.message);
            sqlx::query("DELETE FROM password_resets WHERE token_hash=?").bind(&hash).execute(pool).await?;
        }
    }
    Ok(generic)
}
pub async fn reset_password(pool: &SqlitePool, token: &str, password: &str) -> AppResult<Value> {
    velora_auth_core::password::validate_password(password).map_err(AppError::bad_request)?;
    let hash = hex::encode(Sha256::digest(token.as_bytes()));
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let password_hash = velora_auth_core::password::hash_password(password)
        .map_err(|message| AppError { status: StatusCode::INTERNAL_SERVER_ERROR, message })?;
    let mut tx = pool.begin().await?;
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
    Ok(json!({"ok":true}))
}
