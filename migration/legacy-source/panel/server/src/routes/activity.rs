//! Metadata-only audit trail. Never persist passwords, tokens, chat or command arguments.
use crate::state::RequestState as State;
use crate::{
    auth::{AdminUser, UserRow},
    error::AppResult,
    state::AppState,
};
use axum::{
    extract::{Query, Request},
    middleware::Next,
    response::Response,
    Json,
};
use serde::{Deserialize, Serialize};

pub async fn record(state: &AppState, user: &UserRow, source: &str, kind: &str, detail: Option<&str>) -> AppResult<()> {
    sqlx::query("INSERT INTO events (username, uuid, source, kind, detail, created_at) VALUES (?, ?, ?, ?, ?, ?)")
        .bind(&user.username)
        .bind(&user.uuid)
        .bind(source)
        .bind(kind)
        .bind(detail.map(|d| d.chars().filter(|c| !c.is_control()).take(256).collect::<String>()))
        .bind(crate::db::now())
        .execute(&state.platform_db)
        .await?;
    Ok(())
}

pub async fn audit(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let path = request.uri().path().to_owned();
    let mutation = matches!(request.method().as_str(), "POST" | "PUT" | "PATCH" | "DELETE")
        && (path.starts_with("/api/admin/") || path.starts_with("/api/v1/account/"));
    let user = if mutation {
        let claims = request
            .headers()
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .and_then(|t| state.keys.verify(t));
        if let Some(claims) = claims {
            sqlx::query_as::<_, UserRow>("SELECT * FROM users WHERE id=? AND auth_version=? AND status='active'")
                .bind(claims.sub)
                .bind(claims.version)
                .fetch_optional(&state.platform_db)
                .await
                .ok()
                .flatten()
        } else {
            None
        }
    } else {
        None
    };
    let detail = format!("{} {path}", request.method());
    let response = next.run(request).await;
    if response.status().is_success() {
        if let Some(user) = user {
            if let Err(e) = record(&state, &user, "panel", "account_or_admin_change", Some(&detail)).await {
                tracing::error!("audit write failed: {}", e.message);
            }
        }
    }
    response
}

#[derive(Deserialize, Default)]
pub struct Filter {
    #[serde(default)]
    source: String,
    #[serde(default)]
    player: String,
    #[serde(default)]
    offset: u32,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct Entry {
    id: i64,
    source: String,
    server: Option<String>,
    uuid: Option<String>,
    name: Option<String>,
    kind: String,
    detail: Option<String>,
    created_at: String,
}

pub async fn list(_: AdminUser, State(state): State<AppState>, Query(filter): Query<Filter>) -> AppResult<Json<Vec<Entry>>> {
    let rows = sqlx::query_as(
        "SELECT * FROM (
            SELECT id, source, NULL AS server, uuid, username AS name, kind, detail, created_at FROM events
            UNION ALL
            SELECT e.id, 'server' AS source, s.name AS server, e.uuid, e.name, e.kind, e.detail, e.created_at
            FROM server_events e JOIN game_servers s ON s.id=e.server_id
         ) WHERE (?='' OR source=?) AND (?='' OR name=? COLLATE NOCASE OR uuid=?)
         ORDER BY created_at DESC, source, id DESC LIMIT 100 OFFSET ?",
    )
    .bind(&filter.source)
    .bind(&filter.source)
    .bind(&filter.player)
    .bind(&filter.player)
    .bind(&filter.player)
    .bind(filter.offset.min(100_000))
    .fetch_all(&state.db)
    .await?;
    Ok(Json(rows))
}
