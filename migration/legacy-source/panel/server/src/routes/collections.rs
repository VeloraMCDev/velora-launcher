//! Player identity unlocks and equipped showcase selections.

use crate::state::RequestState as State;
use crate::auth::{AdminUser, AuthUser};
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use axum::extract::{Path};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

pub async fn mine(auth: AuthUser, State(state): State<AppState>) -> AppResult<Json<Value>> {
    let rows: Vec<(String,String,String,String,bool,String)> = sqlx::query_as("SELECT unlock_key,unlock_type,source_type,source_id,equipped,metadata FROM player_unlocks WHERE uuid=? ORDER BY unlocked_at DESC").bind(&auth.uuid).fetch_all(&state.db).await?;
    Ok(Json(json!(rows.into_iter().map(|(key,kind,source_type,source_id,equipped,metadata)| json!({"key":key,"type":kind,"source_type":source_type,"source_id":source_id,"equipped":equipped,"metadata":serde_json::from_str::<Value>(&metadata).unwrap_or_else(|_| json!({}))})).collect::<Vec<_>>())))
}

pub async fn admin_player(_: AdminUser, Path(uuid): Path<String>, State(state): State<AppState>) -> AppResult<Json<Value>> {
    let uuid = crate::yggdrasil::dashed(&uuid).ok_or_else(|| AppError::bad_request("invalid player UUID"))?;
    let rows: Vec<(String,String,String,String,bool,String)> = sqlx::query_as(
        "SELECT unlock_key,unlock_type,source_type,source_id,equipped,metadata FROM player_unlocks WHERE uuid=? ORDER BY unlocked_at DESC",
    ).bind(&uuid).fetch_all(&state.db).await?;
    Ok(Json(json!({"uuid":uuid,"unlocks":rows.into_iter().map(|(key,kind,source_type,source_id,equipped,metadata)| json!({"key":key,"type":kind,"source_type":source_type,"source_id":source_id,"equipped":equipped,"metadata":serde_json::from_str::<Value>(&metadata).unwrap_or_else(|_| json!({}))})).collect::<Vec<_>>()})))
}

#[derive(Deserialize)]
pub struct EquipPayload { pub unlock_key: String, pub equipped: bool }

pub async fn equip(auth: AuthUser, State(state): State<AppState>, Json(p): Json<EquipPayload>) -> AppResult<Json<Value>> {
    set_equipped(&state, &auth.uuid, &p.unlock_key, p.equipped).await?;
    Ok(Json(json!({"ok":true})))
}

/// Equip or unequip one unlock. Equipping puts away whatever else of that type was worn.
pub async fn set_equipped(state: &AppState, uuid: &str, key: &str, equipped: bool) -> AppResult<()> {
    let mut tx = state.db.begin().await?;
    let unlock_type: Option<String> = sqlx::query_scalar("SELECT unlock_type FROM player_unlocks WHERE uuid=? AND unlock_key=?")
        .bind(uuid).bind(key).fetch_optional(&mut *tx).await?;
    let unlock_type = unlock_type.ok_or_else(|| AppError::not_found("unlock not found"))?;
    if equipped && matches!(unlock_type.as_str(), "title" | "badge" | "cosmetic" | "particle" | "pet" | "join_message" | "leave_message") {
        sqlx::query("UPDATE player_unlocks SET equipped=0 WHERE uuid=? AND unlock_type=?")
            .bind(uuid).bind(&unlock_type).execute(&mut *tx).await?;
    }
    let changed = sqlx::query("UPDATE player_unlocks SET equipped=? WHERE uuid=? AND unlock_key=?")
        .bind(equipped).bind(uuid).bind(key).execute(&mut *tx).await?.rows_affected();
    if changed == 0 { return Err(AppError::not_found("unlock not found")); }
    tx.commit().await?;
    Ok(())
}

#[derive(Deserialize)]
pub struct GamePayload { pub uuid: String, #[serde(default)] pub key: String, #[serde(default)] pub equipped: bool }

/// `/cosmetic`: what a player has unlocked, with each one's display name and whether it is worn.
pub async fn server_list(_: crate::routes::servers::GameServer, State(state): State<AppState>, Json(p): Json<GamePayload>) -> AppResult<Json<Value>> {
    let uuid = crate::yggdrasil::dashed(&p.uuid).ok_or_else(|| AppError::bad_request("invalid player UUID"))?;
    let rows: Vec<(String, String, bool, Option<String>, String)> = sqlx::query_as(
        "SELECT u.unlock_key, u.unlock_type, u.equipped, t.label, u.metadata FROM player_unlocks u
         LEFT JOIN cosmetic_templates t ON t.key = u.unlock_key WHERE u.uuid = ? ORDER BY u.unlock_type, u.unlocked_at DESC",
    ).bind(&uuid).fetch_all(&state.db).await?;
    let list = rows.into_iter().filter(|r| r.1 != "showcase").map(|(key, kind, equipped, label, metadata)| {
        let label = label.or_else(|| serde_json::from_str::<Value>(&metadata).ok().and_then(|m| m["label"].as_str().map(String::from))).unwrap_or_else(|| key.clone());
        json!({"key": key, "type": kind, "label": label, "equipped": equipped})
    }).collect::<Vec<_>>();
    Ok(Json(json!({"cosmetics": list})))
}

/// `/cosmetic equip` and `/cosmetic unequip`.
pub async fn server_equip(_: crate::routes::servers::GameServer, State(state): State<AppState>, Json(p): Json<GamePayload>) -> AppResult<Json<Value>> {
    let uuid = crate::yggdrasil::dashed(&p.uuid).ok_or_else(|| AppError::bad_request("invalid player UUID"))?;
    set_equipped(&state, &uuid, &p.key, p.equipped).await?;
    Ok(Json(json!({"ok": true})))
}

#[derive(Deserialize)]
pub struct GrantPayload { pub uuid: String, pub unlock_key: String, pub unlock_type: String, #[serde(default)] pub metadata: Value }

pub async fn grant(_: AdminUser, State(state): State<AppState>, Json(p): Json<GrantPayload>) -> AppResult<Json<Value>> {
    let uuid = crate::yggdrasil::dashed(&p.uuid).ok_or_else(|| AppError::bad_request("invalid player UUID"))?;
    let key = p.unlock_key.trim();
    if key.is_empty() || key.len() > 80 || !key.bytes().all(|b| b.is_ascii_alphanumeric() || b"_:-.".contains(&b)) {
        return Err(AppError::bad_request("unlock key must use letters, numbers, _, :, - or ."));
    }
    if !matches!(p.unlock_type.as_str(), "title" | "badge" | "cosmetic" | "particle" | "pet" | "join_message" | "leave_message" | "showcase") {
        return Err(AppError::bad_request("unsupported unlock type"));
    }
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE uuid=?)").bind(&uuid).fetch_one(&state.db).await?;
    if !exists { return Err(AppError::not_found("player not found")); }
    sqlx::query("INSERT INTO player_unlocks(uuid,unlock_key,unlock_type,source_type,source_id,unlocked_at,equipped,metadata) VALUES(?,?,?,'admin','manual',?,0,?) ON CONFLICT(uuid,unlock_key) DO NOTHING")
        .bind(&uuid).bind(key).bind(&p.unlock_type).bind(crate::db::now()).bind(p.metadata.to_string()).execute(&state.db).await?;
    Ok(Json(json!({"ok":true,"uuid":uuid,"unlock_key":key})))
}

pub async fn revoke(_: AdminUser, Path((uuid, key)): Path<(String,String)>, State(state): State<AppState>) -> AppResult<Json<Value>> {
    let uuid = crate::yggdrasil::dashed(&uuid).ok_or_else(|| AppError::bad_request("invalid player UUID"))?;
    let changed = sqlx::query("DELETE FROM player_unlocks WHERE uuid=? AND unlock_key=?").bind(uuid).bind(key).execute(&state.db).await?.rows_affected();
    if changed == 0 { return Err(AppError::not_found("unlock not found")); }
    Ok(Json(json!({"ok":true})))
}
