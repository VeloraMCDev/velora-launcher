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
pub use velora_panel_activity::{Entry, Filter};

pub async fn record(state: &AppState, user: &UserRow, source: &str, kind: &str, detail: Option<&str>) -> AppResult<()> {
    velora_panel_activity::record(
        &state.platform_db,
        velora_panel_activity::Record { username: &user.username, uuid: &user.uuid, source, kind, detail, created_at: &crate::db::now() },
    )
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

pub async fn list(_: AdminUser, State(state): State<AppState>, Query(filter): Query<Filter>) -> AppResult<Json<Vec<Entry>>> {
    let rows = velora_panel_activity::list(&state.db, &filter).await?;
    Ok(Json(rows))
}
