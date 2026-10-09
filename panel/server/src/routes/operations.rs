//! Aggregates for the operations dashboard: account, sign-in and audit totals without personal data
//! beyond the usernames admins already see in the activity log.
use crate::state::RequestState as State;
use crate::{auth::AdminUser, error::AppResult, state::AppState};
use axum::Json;
use serde::Serialize;

#[derive(Serialize, sqlx::FromRow)]
pub struct RecentEvent {
    pub name: Option<String>,
    pub kind: String,
    pub detail: Option<String>,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct Summary {
    pub version: &'static str,
    pub users: i64,
    pub admins: i64,
    pub logins_24h: i64,
    pub logins_7d: i64,
    pub unique_logins_7d: i64,
    pub launcher_sign_ins_24h: i64,
    pub launches_24h: i64,
    pub admin_changes_7d: i64,
    pub recent_logins: Vec<RecentEvent>,
    pub recent_admin_changes: Vec<RecentEvent>,
}

fn since(hours: i64) -> String {
    (chrono::Utc::now() - chrono::Duration::hours(hours)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

/// `GET /api/admin/operations/summary`
pub async fn summary(_: AdminUser, State(state): State<AppState>) -> AppResult<Json<Summary>> {
    let db = &state.platform_db;
    let (day, week) = (since(24), since(24 * 7));
    let count = |sql: &'static str, bind: String| async move { sqlx::query_scalar::<_, i64>(sql).bind(bind).fetch_one(db).await };
    let users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users").fetch_one(db).await?;
    let admins: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE role = 'admin' AND status = 'active'").fetch_one(db).await?;
    let logins_24h = count("SELECT COUNT(*) FROM events WHERE source = 'auth' AND kind = 'login' AND created_at >= ?", day.clone()).await?;
    let logins_7d = count("SELECT COUNT(*) FROM events WHERE source = 'auth' AND kind = 'login' AND created_at >= ?", week.clone()).await?;
    let unique_logins_7d = count(
        "SELECT COUNT(DISTINCT lower(username)) FROM events WHERE source = 'auth' AND kind = 'login' AND created_at >= ?",
        week.clone(),
    )
    .await?;
    let launcher_sign_ins_24h = count("SELECT COUNT(*) FROM launcher_sessions WHERE created_at >= ?", day.clone()).await?;
    let launches_24h = count("SELECT COUNT(*) FROM events WHERE kind = 'launch' AND created_at >= ?", day).await?;
    let admin_changes_7d = count("SELECT COUNT(*) FROM events WHERE kind = 'account_or_admin_change' AND created_at >= ?", week).await?;
    let recent = |kind: &'static str| async move {
        sqlx::query_as::<_, RecentEvent>(
            "SELECT username AS name, kind, detail, created_at FROM events WHERE kind = ? ORDER BY created_at DESC, id DESC LIMIT 15",
        )
        .bind(kind)
        .fetch_all(db)
        .await
    };
    Ok(Json(Summary {
        version: env!("CARGO_PKG_VERSION"),
        users,
        admins,
        logins_24h,
        logins_7d,
        unique_logins_7d,
        launcher_sign_ins_24h,
        launches_24h,
        admin_changes_7d,
        recent_logins: recent("login").await?,
        recent_admin_changes: recent("account_or_admin_change").await?,
    }))
}
