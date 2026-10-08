//! Community event definitions, participation and contribution snapshots.

use crate::state::RequestState as State;
use crate::auth::{AdminUser, AuthUser};
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use axum::extract::{Path};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
pub struct EventPayload {
    pub id: Option<String>,
    pub title: String,
    #[serde(default)] pub description: String,
    pub instance_id: Option<String>,
    pub server_id: Option<i64>,
    pub starts_at: String,
    pub ends_at: String,
    #[serde(default)] pub status: String,
    #[serde(default)] pub objectives: Value,
    #[serde(default)] pub rewards: Value,
}

pub async fn list_public(_: AuthUser, State(state): State<AppState>) -> AppResult<Json<Value>> {
    refresh_statuses(&state.db).await?;
    let rows: Vec<(String,String,String,Option<String>,Option<i64>,String,String,String,String)> = sqlx::query_as(
        "SELECT id,title,description,instance_id,server_id,status,starts_at,ends_at,objective_data
         FROM community_events WHERE status IN ('published','active') ORDER BY starts_at ASC LIMIT 50")
        .fetch_all(&state.db).await?;
    let mut list = Vec::with_capacity(rows.len());
    for (id,title,description,instance_id,server_id,status,starts_at,ends_at,objectives) in rows {
        let participants: Vec<(String,i64,Option<i64>)> = sqlx::query_as(
            "SELECT uuid,contribution,placement FROM community_event_participants WHERE event_id=? ORDER BY contribution DESC,joined_at ASC LIMIT 10",
        ).bind(&id).fetch_all(&state.db).await?;
        list.push(json!({"id":id,"title":title,"description":description,"instance_id":instance_id,"server_id":server_id,"status":status,"starts_at":starts_at,"ends_at":ends_at,
            "objectives":parse_json(&objectives,json!([])),"leaderboard":participants.into_iter().map(|(uuid,contribution,placement)| json!({"uuid":uuid,"contribution":contribution,"placement":placement})).collect::<Vec<_>>() }));
    }
    Ok(Json(Value::Array(list)))
}

pub async fn join(auth: AuthUser, Path(id): Path<String>, State(state): State<AppState>) -> AppResult<Json<Value>> {
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM community_events WHERE id=? AND status IN ('published','active'))").bind(&id).fetch_one(&state.db).await?;
    if !exists { return Err(AppError::not_found("event not found or not open")); }
    let now = crate::db::now();
    sqlx::query("INSERT INTO community_event_participants(event_id,uuid,joined_at) VALUES(?,?,?) ON CONFLICT(event_id,uuid) DO NOTHING").bind(&id).bind(&auth.uuid).bind(now).execute(&state.db).await?;
    Ok(Json(json!({"ok":true})))
}

pub async fn detail(auth: AuthUser, Path(id): Path<String>, State(state): State<AppState>) -> AppResult<Json<Value>> {
    refresh_statuses(&state.db).await?;
    let event: Option<(String,String,String,Option<String>,Option<i64>,String,String,String,String,String)> = sqlx::query_as(
        "SELECT id,title,description,instance_id,server_id,status,starts_at,ends_at,objective_data,reward_data
         FROM community_events WHERE id = ?",
    ).bind(&id).fetch_optional(&state.db).await?;
    let (id,title,description,instance_id,server_id,status,starts_at,ends_at,objectives,rewards) = event.ok_or_else(|| AppError::not_found("event not found"))?;
    let participant: Option<(i64,i64,Option<i64>)> = sqlx::query_as(
        "SELECT contribution,participation_rewarded,placement FROM community_event_participants WHERE event_id=? AND uuid=?",
    ).bind(&id).bind(&auth.uuid).fetch_optional(&state.db).await?;
    let participants: Vec<(String,i64,Option<i64>)> = sqlx::query_as(
        "SELECT uuid,contribution,placement FROM community_event_participants WHERE event_id=? ORDER BY contribution DESC, joined_at ASC LIMIT 100",
    ).bind(&id).fetch_all(&state.db).await?;
    Ok(Json(json!({"id":id,"title":title,"description":description,"instance_id":instance_id,"server_id":server_id,"status":status,"starts_at":starts_at,"ends_at":ends_at,
        "objectives": parse_json(&objectives, json!([])), "rewards": parse_json(&rewards, json!({})),
        "joined": participant.is_some(), "contribution": participant.as_ref().map(|p| p.0).unwrap_or(0),
        "placement": participant.and_then(|p| p.2), "leaderboard": participants.into_iter().map(|(uuid,contribution,placement)| json!({"uuid":uuid,"contribution":contribution,"placement":placement})).collect::<Vec<_>>()
    })))
}

pub async fn admin_list(_: AdminUser, State(state): State<AppState>) -> AppResult<Json<Value>> {
    let rows: Vec<(String,String,String,Option<String>,Option<i64>,String,String,String,String)> = sqlx::query_as("SELECT id,title,description,instance_id,server_id,status,starts_at,ends_at,objective_data FROM community_events ORDER BY starts_at DESC").fetch_all(&state.db).await?;
    Ok(Json(json!(rows.into_iter().map(|(id,title,description,instance_id,server_id,status,starts_at,ends_at,objectives)| json!({"id":id,"title":title,"description":description,"instance_id":instance_id,"server_id":server_id,"status":status,"starts_at":starts_at,"ends_at":ends_at,"objectives":serde_json::from_str::<Value>(&objectives).unwrap_or_else(|_| json!([]))})).collect::<Vec<_>>())))
}

pub async fn admin_create(_: AdminUser, State(state): State<AppState>, Json(p): Json<EventPayload>) -> AppResult<Json<Value>> {
    if p.title.trim().is_empty() || p.starts_at >= p.ends_at { return Err(AppError::bad_request("Event title and a valid time range are required")); }
    let rewards = crate::rewards::validate(&p.rewards)?;
    let id = p.id.filter(|x| !x.trim().is_empty()).unwrap_or_else(|| format!("event_{}", uuid::Uuid::new_v4().simple()));
    let now = crate::db::now();
    let status = if p.status.is_empty() { "draft" } else { &p.status };
    sqlx::query("INSERT INTO community_events(id,title,description,instance_id,server_id,status,starts_at,ends_at,objective_data,reward_data,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?,?,?,?)")
        .bind(&id).bind(p.title.trim()).bind(p.description).bind(p.instance_id).bind(p.server_id).bind(status).bind(p.starts_at).bind(p.ends_at).bind(p.objectives.to_string()).bind(serde_json::to_string(&rewards)?).bind(&now).bind(&now).execute(&state.db).await?;
    Ok(Json(json!({"id":id,"ok":true})))
}

pub async fn admin_update(_: AdminUser, Path(id): Path<String>, State(state): State<AppState>, Json(p): Json<EventPayload>) -> AppResult<Json<Value>> {
    if p.title.trim().is_empty() || p.starts_at >= p.ends_at { return Err(AppError::bad_request("Event title and a valid time range are required")); }
    let rewards = crate::rewards::validate(&p.rewards)?;
    let status = if p.status.is_empty() { "draft" } else { &p.status };
    let changed = sqlx::query("UPDATE community_events SET title=?,description=?,instance_id=?,server_id=?,status=?,starts_at=?,ends_at=?,objective_data=?,reward_data=?,updated_at=? WHERE id=?")
        .bind(p.title.trim()).bind(p.description).bind(p.instance_id).bind(p.server_id).bind(status).bind(p.starts_at).bind(p.ends_at).bind(p.objectives.to_string()).bind(serde_json::to_string(&rewards)?).bind(crate::db::now()).bind(&id).execute(&state.db).await?.rows_affected();
    if changed == 0 { return Err(AppError::not_found("event not found")); }
    Ok(Json(json!({"ok":true,"id":id})))
}

pub async fn admin_delete(_: AdminUser, Path(id): Path<String>, State(state): State<AppState>) -> AppResult<Json<Value>> {
    let changed = sqlx::query("DELETE FROM community_events WHERE id=?").bind(&id).execute(&state.db).await?.rows_affected();
    if changed == 0 { return Err(AppError::not_found("event not found")); }
    Ok(Json(json!({"ok":true})))
}

fn parse_json(text: &str, fallback: Value) -> Value { serde_json::from_str(text).unwrap_or(fallback) }

/// Recompute a participant's contribution from immutable server telemetry.
/// This makes retries harmless and keeps rankings consistent after reconnects.
pub async fn refresh_participant(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, server_id: i64, uuid: &str) -> AppResult<()> {
    let events: Vec<(String,String,String)> = sqlx::query_as(
        "SELECT e.id,e.objective_data,e.starts_at FROM community_events e
         JOIN game_servers s ON s.id=?
         WHERE e.status IN ('published','active')
           AND (e.server_id IS NULL OR e.server_id=?)
           AND (e.instance_id IS NULL OR e.instance_id=s.instance_id)",
    ).bind(server_id).bind(server_id).fetch_all(&mut **tx).await?;
    for (event_id, objective_data, starts_at) in events {
        let joined: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM community_event_participants WHERE event_id=? AND uuid=? AND joined_at >= ?)")
            .bind(&event_id).bind(uuid).bind(&starts_at).fetch_one(&mut **tx).await?;
        if !joined { continue; }
        let objectives = parse_json(&objective_data, json!([]));
        let mut contribution = 0i64;
        if let Some(items) = objectives.as_array() {
            for objective in items {
                let kind = objective.get("kind").and_then(Value::as_str).unwrap_or("");
                let target = objective.get("target").and_then(Value::as_str).unwrap_or(kind);
                let count: i64 = sqlx::query_scalar(
                    "SELECT COALESCE(SUM(CASE WHEN detail LIKE ? THEN CAST(substr(detail, instr(detail, ' + ')+3) AS INTEGER) ELSE 1 END),0)
                     FROM server_events WHERE server_id=? AND uuid=? AND created_at >= ? AND (kind=? OR detail LIKE ?)",
                ).bind(format!("{}%", target)).bind(server_id).bind(uuid).bind(&starts_at).bind(kind).bind(format!("{}%", target)).fetch_one(&mut **tx).await.unwrap_or(0);
                contribution = contribution.saturating_add(count.min(objective.get("target_count").and_then(Value::as_i64).unwrap_or(i64::MAX)));
            }
        }
        sqlx::query("UPDATE community_event_participants SET contribution=? WHERE event_id=? AND uuid=?")
            .bind(contribution).bind(&event_id).bind(uuid).execute(&mut **tx).await?;
        sqlx::query("UPDATE community_event_participants SET placement=(SELECT COUNT(*)+1 FROM community_event_participants p2 WHERE p2.event_id=? AND p2.contribution > community_event_participants.contribution) WHERE event_id=?")
            .bind(&event_id).bind(&event_id).execute(&mut **tx).await?;
    }
    Ok(())
}

async fn refresh_statuses(db: &sqlx::SqlitePool) -> AppResult<()> {
    let now = crate::db::now();
    let mut tx = db.begin().await?;
    sqlx::query("UPDATE community_events SET status='active',updated_at=? WHERE status='published' AND starts_at <= ? AND ends_at > ?")
        .bind(&now).bind(&now).bind(&now).execute(&mut *tx).await?;
    let ending: Vec<(String, Option<i64>, Option<String>)> = sqlx::query_as(
        "SELECT id,server_id,instance_id FROM community_events WHERE status IN ('published','active') AND ends_at <= ?",
    ).bind(&now).fetch_all(&mut *tx).await?;
    for (event_id, event_server, event_instance) in &ending {
        let players: Vec<(String, Option<i64>)> = sqlx::query_as(
            "SELECT p.uuid,COALESCE(?,(SELECT MIN(s.id) FROM game_servers s WHERE s.instance_id=COALESCE(?,(SELECT instance_id FROM game_servers WHERE id=?))))
             FROM community_event_participants p WHERE p.event_id=? AND p.participation_rewarded=0",
        ).bind(event_server).bind(event_instance).bind(event_server).bind(event_id).fetch_all(&mut *tx).await?;
        for (uuid, server_id) in players {
            if let Some(server_id) = server_id {
                refresh_participant(&mut tx, server_id, &uuid).await?;
            }
        }
    }
    let ended: Vec<(String, Option<i64>, Option<String>)> = sqlx::query_as(
        "UPDATE community_events SET status='ended',updated_at=? WHERE status IN ('published','active') AND ends_at <= ? RETURNING id,server_id,instance_id",
    ).bind(&now).bind(&now).fetch_all(&mut *tx).await?;
    for (event_id, _, _) in ended {
        let players: Vec<String> = sqlx::query_scalar(
            "SELECT uuid FROM community_event_participants WHERE event_id=? AND participation_rewarded=0",
        ).bind(&event_id).fetch_all(&mut *tx).await?;
        for uuid in players {
            crate::rewards::enqueue(&mut tx, &uuid, "event", &event_id).await?;
            sqlx::query("UPDATE community_event_participants SET participation_rewarded=1 WHERE event_id=? AND uuid=? AND participation_rewarded=0")
                .bind(&event_id).bind(uuid).execute(&mut *tx).await?;
        }
    }
    tx.commit().await?;
    Ok(())
}
