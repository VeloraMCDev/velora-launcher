//! Minecraft-style Achievements system.

use crate::state::RequestState as State;
use crate::auth::{AdminUser, AuthUser};
use crate::error::AppResult;
use crate::state::AppState;
use axum::extract::{Path};
use axum::Json;
use velora_shared::Achievement;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Get all achievements for authenticated user with their unlock status.
pub async fn get_my_achievements(auth: AuthUser, State(state): State<AppState>) -> AppResult<Json<Vec<Achievement>>> {
    get_achievements_for_user(&state, &auth.uuid, false).await.map(Json)
}

/// Get public unlocked achievements for a specific player profile.
pub async fn get_player_achievements(Path(uuid): Path<String>, State(state): State<AppState>) -> AppResult<Json<Vec<Achievement>>> {
    get_achievements_for_user(&state, &uuid, true).await.map(Json)
}

pub async fn get_achievements_for_user(state: &AppState, uuid: &str, unlocked_only: bool) -> AppResult<Vec<Achievement>> {
    let rows: Vec<(String, String, String, String, String, String, String, String, i64, bool, Option<String>)> = sqlx::query_as(
        "SELECT a.id, a.title, a.description, a.category, a.icon_frame, a.icon_item,
                a.icon_bg, a.icon_border, a.xp_reward, a.secret, ua.unlocked_at
         FROM achievements a
         LEFT JOIN user_achievements ua ON ua.achievement_id = a.id AND ua.user_uuid = ?
         ORDER BY a.category ASC, a.xp_reward ASC",
    )
    .bind(uuid)
    .fetch_all(&state.db)
    .await?;

    let list = rows
        .into_iter()
        .filter_map(|(id, title, desc, cat, frame, item, bg, border, xp, secret, unlocked_at)| {
            let unlocked = unlocked_at.is_some();
            if unlocked_only && !unlocked {
                return None;
            }
            Some(Achievement {
                id,
                title,
                description: desc,
                category: cat,
                icon_frame: frame.clone(),
                frame_type: Some(frame),
                icon_item: item,
                icon_bg: bg,
                icon_border: border,
                xp_reward: xp,
                secret,
                stat_type: None,
                target_count: None,
                unlocked,
                unlocked_at,
            })
        })
        .collect();

    Ok(list)
}

// ---------------------------------------------------------------------------
// Admin Achievements API
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct AdminAchievementRow {
    #[serde(flatten)]
    pub achievement: Achievement,
    pub requirement_type: String,
    pub requirement_key: String,
    pub requirement_value: i64,
    pub stat_type: String,
    pub target_count: i64,
    pub total_unlocked: i64,
}

/// Admin list all achievements.
pub async fn admin_list_achievements(_admin: AdminUser, State(state): State<AppState>) -> AppResult<Json<Vec<AdminAchievementRow>>> {
    let rows: Vec<(String, String, String, String, String, String, String, String, String, String, i64, i64, bool, i64)> = sqlx::query_as(
        "SELECT a.id, a.title, a.description, a.category, a.icon_frame, a.icon_item,
                a.icon_bg, a.icon_border, a.requirement_type, a.requirement_key,
                a.requirement_value, a.xp_reward, a.secret,
                COUNT(ua.user_uuid) as total_unlocked
         FROM achievements a
         LEFT JOIN user_achievements ua ON ua.achievement_id = a.id
         GROUP BY a.id
         ORDER BY a.category ASC, a.created_at DESC",
    )
    .fetch_all(&state.db)
    .await?;

    let list = rows
        .into_iter()
        .map(|(id, title, desc, cat, frame, item, bg, border, req_type, req_key, req_val, xp, secret, total_unlocked)| {
            AdminAchievementRow {
                achievement: Achievement {
                    id,
                    title,
                    description: desc,
                    category: cat,
                    icon_frame: frame.clone(),
                    frame_type: Some(frame),
                    icon_item: item,
                    icon_bg: bg,
                    icon_border: border,
                    xp_reward: xp,
                    secret,
                    stat_type: Some(req_key.clone()),
                    target_count: Some(req_val),
                    unlocked: false,
                    unlocked_at: None,
                },
                requirement_type: req_type,
                requirement_key: req_key.clone(),
                requirement_value: req_val,
                stat_type: req_key,
                target_count: req_val,
                total_unlocked,
            }
        })
        .collect();

    Ok(Json(list))
}

#[derive(Deserialize)]
pub struct AchievementPayload {
    pub id: Option<String>,
    pub title: String,
    pub description: String,
    pub category: String,
    #[serde(alias = "frame_type")]
    pub icon_frame: String,
    pub icon_item: String,
    pub icon_bg: String,
    pub icon_border: String,
    #[serde(default)]
    pub requirement_type: Option<String>,
    #[serde(default, alias = "stat_type")]
    pub requirement_key: Option<String>,
    #[serde(default, alias = "target_count")]
    pub requirement_value: Option<i64>,
    pub xp_reward: i64,
    pub secret: Option<bool>,
}

/// Admin create achievement.
pub async fn admin_create_achievement(
    _admin: AdminUser,
    State(state): State<AppState>,
    Json(payload): Json<AchievementPayload>,
) -> AppResult<Json<Value>> {
    let id = payload.id.filter(|s| !s.trim().is_empty()).unwrap_or_else(|| format!("ach_{}", uuid::Uuid::new_v4().simple()));
    let now = chrono::Utc::now().to_rfc3339();
    let req_type = payload.requirement_type.unwrap_or_else(|| "stat".into());
    let req_key = payload.requirement_key.unwrap_or_else(|| "blocks_broken".into());
    let req_val = payload.requirement_value.unwrap_or(50);

    sqlx::query(
        "INSERT INTO achievements (id, title, description, category, icon_frame, icon_item, icon_bg, icon_border, requirement_type, requirement_key, requirement_value, xp_reward, secret, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(payload.title)
    .bind(payload.description)
    .bind(payload.category)
    .bind(payload.icon_frame)
    .bind(payload.icon_item)
    .bind(payload.icon_bg)
    .bind(payload.icon_border)
    .bind(req_type)
    .bind(req_key)
    .bind(req_val)
    .bind(payload.xp_reward)
    .bind(payload.secret.unwrap_or(false))
    .bind(&now)
    .execute(&state.db)
    .await?;

    crate::rewards::apply_defaults(&state).await?;
    Ok(Json(serde_json::json!({ "id": id, "ok": true })))
}

/// Admin update achievement.
pub async fn admin_update_achievement(
    _admin: AdminUser,
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(payload): Json<AchievementPayload>,
) -> AppResult<Json<Value>> {
    // FIX #19: Only overwrite requirement_type / key / value when the caller
    // explicitly supplies them. Use COALESCE so NULL leaves the existing value.
    let req_type = payload.requirement_type; // None if not in the request
    let req_key = payload.requirement_key; // None if not in the request
    let req_val = payload.requirement_value; // None if not in the request

    sqlx::query(
        "UPDATE achievements SET
            title = ?,
            description = ?,
            category = ?,
            icon_frame = ?,
            icon_item = ?,
            icon_bg = ?,
            icon_border = ?,
            requirement_type = COALESCE(?, requirement_type),
            requirement_key = COALESCE(?, requirement_key),
            requirement_value = COALESCE(?, requirement_value),
            xp_reward = ?,
            secret = COALESCE(?, secret)
         WHERE id = ?",
    )
    .bind(payload.title)
    .bind(payload.description)
    .bind(payload.category)
    .bind(payload.icon_frame)
    .bind(payload.icon_item)
    .bind(payload.icon_bg)
    .bind(payload.icon_border)
    .bind(req_type)
    .bind(req_key)
    .bind(req_val)
    .bind(payload.xp_reward)
    .bind(payload.secret)
    .bind(&id)
    .execute(&state.db)
    .await?;

    crate::rewards::apply_defaults(&state).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// Admin delete achievement.
pub async fn admin_delete_achievement(_admin: AdminUser, Path(id): Path<String>, State(state): State<AppState>) -> AppResult<Json<Value>> {
    sqlx::query("DELETE FROM reward_bundles WHERE source_type = 'achievement' AND source_id = ?").bind(&id).execute(&state.db).await?;
    sqlx::query("DELETE FROM achievements WHERE id = ?").bind(&id).execute(&state.db).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}
