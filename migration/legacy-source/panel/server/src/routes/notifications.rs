//! The notification bell: short messages for a player (guild renamed, guild disbanded, auction won, …) shown in the
//! launcher and the admin panel. Rows are keyed by player UUID so they survive a rename and work for in-game events.

use crate::state::RequestState as State;
use crate::auth::AuthUser;
use crate::error::AppResult;
use crate::state::AppState;
use axum::extract::{Path, Query};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::SqlitePool;

const KEEP_PER_PLAYER: i64 = 200;

/// Queue a bell notification. Never fails the caller's real work: a lost notification is logged and dropped.
pub async fn push(db: &SqlitePool, uuid: &str, kind: &str, title: &str, body: &str, link: Option<&str>) {
    let res = sqlx::query("INSERT INTO user_notifications (uuid, kind, title, body, link, created_at) VALUES (?, ?, ?, ?, ?, ?)")
        .bind(uuid)
        .bind(kind)
        .bind(title)
        .bind(body)
        .bind(link)
        .bind(crate::db::now())
        .execute(db)
        .await;
    match res {
        Ok(_) => {
            // A player who is in game right now hears about it in chat too.
            if let Ok(mut conn) = db.acquire().await {
                let _ = super::integrations::notify(&mut conn, None, "chat_message", uuid, json!({ "title": title, "body": body })).await;
            }
            let _ = sqlx::query(
                "DELETE FROM user_notifications WHERE uuid = ? AND id NOT IN (SELECT id FROM user_notifications WHERE uuid = ? ORDER BY id DESC LIMIT ?)",
            )
            .bind(uuid)
            .bind(uuid)
            .bind(KEEP_PER_PLAYER)
            .execute(db)
            .await;
        }
        Err(e) => tracing::warn!("could not store notification for {uuid}: {e}"),
    }
}

pub async fn push_many(db: &SqlitePool, uuids: &[String], kind: &str, title: &str, body: &str, link: Option<&str>) {
    for u in uuids {
        push(db, u, kind, title, body, link).await;
    }
}

#[derive(Deserialize)]
pub struct ListQuery {
    #[serde(default)]
    unread: bool,
    limit: Option<i64>,
}

pub async fn list(auth: AuthUser, State(state): State<AppState>, Query(q): Query<ListQuery>) -> AppResult<Json<Value>> {
    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let rows: Vec<(i64, String, String, String, Option<String>, String, Option<String>)> = sqlx::query_as(
        "SELECT id, kind, title, body, link, created_at, read_at FROM user_notifications
         WHERE uuid = ? AND (? = 0 OR read_at IS NULL) ORDER BY id DESC LIMIT ?",
    )
    .bind(&auth.uuid)
    .bind(q.unread as i64)
    .bind(limit)
    .fetch_all(&state.db)
    .await?;
    let unread: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM user_notifications WHERE uuid = ? AND read_at IS NULL")
        .bind(&auth.uuid)
        .fetch_one(&state.db)
        .await?;
    let items: Vec<Value> = rows
        .into_iter()
        .map(|(id, kind, title, body, link, created_at, read_at)| {
            json!({ "id": id, "kind": kind, "title": title, "body": body, "link": link, "created_at": created_at, "read": read_at.is_some() })
        })
        .collect();
    Ok(Json(json!({ "unread": unread, "items": items })))
}

#[derive(Deserialize, Default)]
pub struct ReadPayload {
    ids: Option<Vec<i64>>,
}

/// Mark the given notifications (or all of them when no ids are sent) as read.
pub async fn mark_read(auth: AuthUser, State(state): State<AppState>, body: Option<Json<ReadPayload>>) -> AppResult<Json<Value>> {
    let ids = body.and_then(|b| b.0.ids);
    let now = crate::db::now();
    match ids {
        Some(ids) => {
            for id in ids {
                sqlx::query("UPDATE user_notifications SET read_at = ? WHERE id = ? AND uuid = ? AND read_at IS NULL")
                    .bind(&now)
                    .bind(id)
                    .bind(&auth.uuid)
                    .execute(&state.db)
                    .await?;
            }
        }
        None => {
            sqlx::query("UPDATE user_notifications SET read_at = ? WHERE uuid = ? AND read_at IS NULL")
                .bind(&now)
                .bind(&auth.uuid)
                .execute(&state.db)
                .await?;
        }
    }
    Ok(Json(json!({ "ok": true })))
}

pub async fn remove(auth: AuthUser, State(state): State<AppState>, Path(id): Path<i64>) -> AppResult<Json<Value>> {
    sqlx::query("DELETE FROM user_notifications WHERE id = ? AND uuid = ?").bind(id).bind(&auth.uuid).execute(&state.db).await?;
    Ok(Json(json!({ "ok": true })))
}

pub async fn clear(auth: AuthUser, State(state): State<AppState>) -> AppResult<Json<Value>> {
    sqlx::query("DELETE FROM user_notifications WHERE uuid = ?").bind(&auth.uuid).execute(&state.db).await?;
    Ok(Json(json!({ "ok": true })))
}
