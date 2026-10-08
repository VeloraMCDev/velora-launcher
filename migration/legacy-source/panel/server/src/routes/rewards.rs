//! Admin editing of reward bundles, what players see of them, and the game server's delivery endpoints.

use crate::state::RequestState as State;
use crate::auth::{AdminUser, AuthUser};
use crate::error::{AppError, AppResult};
use crate::rewards;
use crate::routes::servers::GameServer;
use crate::state::AppState;
use axum::extract::{Path, Query};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub async fn admin_get(_: AdminUser, State(state): State<AppState>, Path((kind, id)): Path<(String, String)>) -> AppResult<Json<Value>> {
    rewards::check_source(&kind)?;
    let mut conn = state.db.acquire().await?;
    Ok(Json(json!({ "actions": rewards::bundle(&mut conn, &kind, &id).await? })))
}

#[derive(Deserialize)]
pub struct BundleBody {
    actions: Value,
}

pub async fn admin_put(
    _: AdminUser,
    State(state): State<AppState>,
    Path((kind, id)): Path<(String, String)>,
    Json(body): Json<BundleBody>,
) -> AppResult<Json<Value>> {
    rewards::check_source(&kind)?;
    let actions = rewards::validate(&body.actions)?;
    for action in &actions {
        if action["type"] == "custom_item" {
            let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM custom_items WHERE id = ?)")
                .bind(action["custom"].as_str().unwrap_or(""))
                .fetch_one(&state.db)
                .await?;
            if !exists {
                return Err(AppError::bad_request("Custom item does not exist"));
            }
        }
    }
    if actions.is_empty() {
        sqlx::query("DELETE FROM reward_bundles WHERE source_type = ? AND source_id = ?").bind(&kind).bind(&id).execute(&state.db).await?;
    } else {
        sqlx::query("INSERT INTO reward_bundles (source_type, source_id, actions) VALUES (?, ?, ?) ON CONFLICT(source_type, source_id) DO UPDATE SET actions = excluded.actions")
            .bind(&kind).bind(&id).bind(serde_json::to_string(&actions)?).execute(&state.db).await?;
    }
    Ok(Json(json!({ "ok": true, "actions": actions })))
}

#[derive(Deserialize)]
pub struct LogQuery {
    #[serde(default)]
    limit: Option<i64>,
}

/// What was handed out (or is waiting, or failed), newest first.
pub async fn admin_deliveries(_: AdminUser, State(state): State<AppState>, Query(q): Query<LogQuery>) -> AppResult<Json<Value>> {
    let rows: Vec<(i64, String, String, Option<i64>, String, String, String, Option<String>, i64, Option<String>)> = sqlx::query_as(
        "SELECT d.id, COALESCE(u.username, d.uuid), d.kind, d.server_id, d.payload, d.source, d.created_at, d.delivered_at, d.attempts, d.error
         FROM reward_deliveries d LEFT JOIN users u ON u.uuid = d.uuid ORDER BY d.id DESC LIMIT ?",
    )
    .bind(q.limit.unwrap_or(100).clamp(1, 500))
    .fetch_all(&state.db)
    .await?;
    Ok(Json(json!(rows.into_iter().map(|(id, player, kind, server, payload, source, at, delivered, attempts, error)| {
        let p: Value = serde_json::from_str(&payload).unwrap_or(json!({}));
        json!({ "id": id, "player": player, "kind": kind, "server_id": server, "summary": rewards::describe(&p), "source": source, "created_at": at,
                "delivered_at": delivered, "attempts": attempts, "error": error })
    }).collect::<Vec<_>>())))
}

/// `{"quest:daily_miner": ["$200", "Diamond x3"], ...}` for the launcher to show next to each reward.
pub async fn public_summaries(_: AuthUser, State(state): State<AppState>) -> AppResult<Json<Value>> {
    let rows: Vec<(String, String, String)> =
        sqlx::query_as("SELECT source_type, source_id, actions FROM reward_bundles").fetch_all(&state.db).await?;
    let mut out = BTreeMap::new();
    for (kind, id, actions) in rows {
        let list: Vec<Value> = serde_json::from_str(&actions).unwrap_or_default();
        if !list.is_empty() {
            out.insert(format!("{kind}:{id}"), list.iter().map(rewards::describe).collect::<Vec<_>>());
        }
    }
    Ok(Json(json!(out)))
}

pub async fn game_poll(GameServer(server): GameServer, State(state): State<AppState>) -> AppResult<Json<Value>> {
    Ok(Json(json!({ "deliveries": rewards::poll(&state, server.id).await? })))
}

#[derive(Deserialize)]
pub struct AckBody {
    #[serde(default)]
    done: Vec<i64>,
    #[serde(default)]
    failed: Vec<Failure>,
}

#[derive(Deserialize)]
pub struct Failure {
    id: i64,
    #[serde(default)]
    error: String,
}

pub async fn game_ack(GameServer(_): GameServer, State(state): State<AppState>, Json(body): Json<AckBody>) -> AppResult<Json<Value>> {
    if body.done.len() + body.failed.len() > 200 {
        return Err(AppError::bad_request("too many ids"));
    }
    let failed: Vec<(i64, String)> = body.failed.into_iter().map(|f| (f.id, f.error)).collect();
    rewards::ack(&state, &body.done, &failed).await?;
    Ok(Json(json!({ "ok": true })))
}
pub async fn admin_retry_delivery(_: AdminUser, State(state): State<AppState>, Path(id): Path<i64>) -> AppResult<Json<Value>> {
    sqlx::query("UPDATE reward_deliveries SET attempts = 0, sent_at = NULL, error = NULL WHERE id = ? AND delivered_at IS NULL")
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(Json(json!({ "ok": true })))
}

pub async fn admin_delete_delivery(_: AdminUser, State(state): State<AppState>, Path(id): Path<i64>) -> AppResult<Json<Value>> {
    sqlx::query("DELETE FROM reward_deliveries WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(Json(json!({ "ok": true })))
}

pub async fn admin_clear_delivered(_: AdminUser, State(state): State<AppState>) -> AppResult<Json<Value>> {
    let result = sqlx::query("DELETE FROM reward_deliveries WHERE delivered_at IS NOT NULL")
        .execute(&state.db)
        .await?;
    Ok(Json(json!({ "ok": true, "deleted": result.rows_affected() })))
}
