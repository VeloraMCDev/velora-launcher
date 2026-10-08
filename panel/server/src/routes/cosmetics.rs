use crate::state::RequestState as State;
use crate::auth::AdminUser;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use axum::extract::{Path};
use axum::Json;
use serde::{Deserialize, Serialize};
use sqlx::Row;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CosmeticTemplate {
    pub id: i64,
    pub key: String,
    #[serde(rename = "type")]
    pub cosmetic_type: String,
    pub label: String,
    pub description: String,
    pub metadata: serde_json::Value,
    pub auto_grant: bool,
    pub grant_on_level: Option<i64>,
    pub grant_on_achievement: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateTemplate {
    pub key: String,
    #[serde(rename = "type")]
    pub cosmetic_type: String,
    pub label: String,
    pub description: String,
    pub metadata: serde_json::Value,
    pub auto_grant: Option<bool>,
    pub grant_on_level: Option<i64>,
    pub grant_on_achievement: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTemplate {
    pub label: String,
    pub description: String,
    pub metadata: serde_json::Value,
    pub auto_grant: Option<bool>,
    pub grant_on_level: Option<i64>,
    pub grant_on_achievement: Option<String>,
}

const TYPES: [&str; 7] = ["cosmetic", "particle", "pet", "join_message", "leave_message", "title", "badge"];

fn check_key(key: &str) -> AppResult<()> {
    if key.is_empty() || key.len() > 80 || !key.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_:-.".contains(&b)) {
        return Err(AppError::bad_request("Key must use lowercase letters, numbers, _, :, - or ."));
    }
    Ok(())
}

fn check_text(label: &str, description: &str) -> AppResult<()> {
    if label.trim().is_empty() || label.chars().count() > 80 || description.chars().count() > 500 {
        return Err(AppError::bad_request("Label is required (max 80 characters) and the description at most 500"));
    }
    Ok(())
}

/// The base item every modelled cosmetic is drawn on: plain, so only its custom model shows.
const LOOK_ITEM: &str = "minecraft:stick";

/// A cosmetic with a texture or model gets a `look`: the base item and a model-data number that is unique for it, so the resource
/// pack can swap in its model and the game server can show it. Keeps its number across edits; the number is `look.custom_model_data`.
async fn shape_look(state: &AppState, metadata: &mut serde_json::Value, id: Option<i64>) -> AppResult<()> {
    let pick = |m: &serde_json::Value, k: &str| m[k].as_str().map(str::trim).filter(|v| !v.is_empty()).map(String::from);
    let (texture, model) = (pick(metadata, "texture"), pick(metadata, "model"));
    let Some(obj) = metadata.as_object_mut() else { return Ok(()) };
    if texture.is_none() && model.is_none() {
        obj.remove("look");
        return Ok(());
    }
    let mut taken = super::content::taken_model_data(state).await?;
    let mut previous = None;
    if let Some(id) = id {
        // Its own number is not "taken" against itself.
        let old: Option<String> = sqlx::query_scalar("SELECT metadata FROM cosmetic_templates WHERE id = ?").bind(id).fetch_optional(&state.db).await?;
        if let Some(old) = old.and_then(|o| serde_json::from_str::<serde_json::Value>(&o).ok()) {
            if let Some(n) = old["look"]["custom_model_data"].as_i64() {
                taken.remove(&(LOOK_ITEM.to_string(), n));
                previous = Some(n);
            }
        }
    }
    let wanted = obj.get("look").and_then(|l| l["custom_model_data"].as_i64()).or(previous);
    let number = super::content::claim(&mut taken, LOOK_ITEM, wanted);
    obj.insert(
        "look".into(),
        serde_json::json!({ "item": LOOK_ITEM, "custom_model_data": number, "texture": texture, "model": model }),
    );
    Ok(())
}

/// Give a template with auto-grant on to players who already qualify (the triggers only cover future level-ups and achievements).
async fn backfill(state: &AppState, id: i64) -> AppResult<()> {
    let now = crate::db::now();
    sqlx::query(
        "INSERT OR IGNORE INTO player_unlocks(uuid, unlock_key, unlock_type, source_type, source_id, unlocked_at, equipped, metadata)
         SELECT l.uuid, t.key, t.type, 'level', CAST(l.global_level AS TEXT), ?, 0, t.metadata FROM cosmetic_templates t
         JOIN user_levels l ON l.global_level >= t.grant_on_level
         WHERE t.id = ? AND t.auto_grant = 1 AND t.grant_on_level IS NOT NULL",
    )
    .bind(&now)
    .bind(id)
    .execute(&state.db)
    .await?;
    sqlx::query(
        "INSERT OR IGNORE INTO player_unlocks(uuid, unlock_key, unlock_type, source_type, source_id, unlocked_at, equipped, metadata)
         SELECT a.user_uuid, t.key, t.type, 'achievement', a.achievement_id, ?, 0, t.metadata FROM cosmetic_templates t
         JOIN user_achievements a ON a.achievement_id = t.grant_on_achievement
         WHERE t.id = ? AND t.auto_grant = 1",
    )
    .bind(&now)
    .bind(id)
    .execute(&state.db)
    .await?;
    Ok(())
}

pub async fn list(
    _admin: AdminUser,
    State(state): State<AppState>,
) -> AppResult<Json<Vec<CosmeticTemplate>>> {
    let rows = sqlx::query(
        "SELECT id, key, type, label, description, metadata, auto_grant, grant_on_level, grant_on_achievement, created_at
         FROM cosmetic_templates
         ORDER BY created_at DESC"
    )
    .fetch_all(&state.db)
    .await?;

    let templates = rows
        .into_iter()
        .map(|row| {
            Ok(CosmeticTemplate {
                id: row.try_get("id")?,
                key: row.try_get("key")?,
                cosmetic_type: row.try_get("type")?,
                label: row.try_get("label")?,
                description: row.try_get("description")?,
                metadata: row.try_get::<String, _>("metadata")?.parse().unwrap_or(serde_json::json!({})),
                auto_grant: row.try_get::<i32, _>("auto_grant")? != 0,
                grant_on_level: row.try_get("grant_on_level")?,
                grant_on_achievement: row.try_get("grant_on_achievement")?,
                created_at: row.try_get("created_at")?,
            })
        })
        .collect::<AppResult<Vec<_>>>()?;

    Ok(Json(templates))
}

pub async fn get(
    _admin: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<CosmeticTemplate>> {
    let row = sqlx::query(
        "SELECT id, key, type, label, description, metadata, auto_grant, grant_on_level, grant_on_achievement, created_at
         FROM cosmetic_templates WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::not_found("Template not found"))?;

    let template = CosmeticTemplate {
        id: row.try_get("id")?,
        key: row.try_get("key")?,
        cosmetic_type: row.try_get("type")?,
        label: row.try_get("label")?,
        description: row.try_get("description")?,
        metadata: row.try_get::<String, _>("metadata")?.parse().unwrap_or(serde_json::json!({})),
        auto_grant: row.try_get::<i32, _>("auto_grant")? != 0,
        grant_on_level: row.try_get("grant_on_level")?,
        grant_on_achievement: row.try_get("grant_on_achievement")?,
        created_at: row.try_get("created_at")?,
    };

    Ok(Json(template))
}

pub async fn create(
    _admin: AdminUser,
    State(state): State<AppState>,
    Json(body): Json<CreateTemplate>,
) -> AppResult<Json<CosmeticTemplate>> {
    let now = crate::db::now();
    let mut body = body;
    let auto_grant = body.auto_grant.unwrap_or(false);
    check_key(&body.key)?;
    check_text(&body.label, &body.description)?;
    if !TYPES.contains(&body.cosmetic_type.as_str()) {
        return Err(AppError::bad_request("Unsupported cosmetic type"));
    }
    let taken: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM cosmetic_templates WHERE key = ?)").bind(&body.key).fetch_one(&state.db).await?;
    if taken {
        return Err(AppError::conflict("A template with that key already exists"));
    }
    if body.cosmetic_type == "cosmetic" {
        shape_look(&state, &mut body.metadata, None).await?;
    }
    let metadata = serde_json::to_string(&body.metadata)?;

    let result = sqlx::query(
        "INSERT INTO cosmetic_templates (key, type, label, description, metadata, auto_grant, grant_on_level, grant_on_achievement, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&body.key)
    .bind(&body.cosmetic_type)
    .bind(&body.label)
    .bind(&body.description)
    .bind(&metadata)
    .bind(auto_grant as i32)
    .bind(body.grant_on_level)
    .bind(&body.grant_on_achievement)
    .bind(&now)
    .execute(&state.db)
    .await?;

    let id = result.last_insert_rowid();
    backfill(&state, id).await?;

    Ok(Json(CosmeticTemplate {
        id,
        key: body.key,
        cosmetic_type: body.cosmetic_type,
        label: body.label,
        description: body.description,
        metadata: body.metadata,
        auto_grant,
        grant_on_level: body.grant_on_level,
        grant_on_achievement: body.grant_on_achievement,
        created_at: now,
    }))
}

pub async fn update(
    _admin: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<UpdateTemplate>,
) -> AppResult<Json<serde_json::Value>> {
    let mut body = body;
    let auto_grant = body.auto_grant.unwrap_or(false);
    check_text(&body.label, &body.description)?;
    let kind: Option<String> = sqlx::query_scalar("SELECT type FROM cosmetic_templates WHERE id = ?").bind(id).fetch_optional(&state.db).await?;
    if kind.as_deref() == Some("cosmetic") {
        shape_look(&state, &mut body.metadata, Some(id)).await?;
    }
    let metadata = serde_json::to_string(&body.metadata)?;

    let changed = sqlx::query(
        "UPDATE cosmetic_templates
         SET label = ?, description = ?, metadata = ?, auto_grant = ?, grant_on_level = ?, grant_on_achievement = ?
         WHERE id = ?"
    )
    .bind(&body.label)
    .bind(&body.description)
    .bind(&metadata)
    .bind(auto_grant as i32)
    .bind(body.grant_on_level)
    .bind(&body.grant_on_achievement)
    .bind(id)
    .execute(&state.db)
    .await?
    .rows_affected();
    if changed == 0 {
        return Err(AppError::not_found("Template not found"));
    }
    backfill(&state, id).await?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn delete(
    _admin: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<serde_json::Value>> {
    sqlx::query("DELETE FROM cosmetic_templates WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({ "ok": true })))
}
