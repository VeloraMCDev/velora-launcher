//! Daily and Weekly Quests system.

use crate::auth::{AdminUser, AuthUser};
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::state::RequestState as State;
use axum::extract::Path;
use axum::Json;
use scopenet_shared::{Quest, UserQuest};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub fn current_daily_key() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

pub fn current_weekly_key() -> String {
    chrono::Utc::now().format("%G-W%V").to_string()
}

/// When the current daily and weekly periods end (UTC midnight / next Monday).
fn period_ends() -> (String, String) {
    use chrono::{Datelike, Duration, Utc};
    let today = Utc::now().date_naive();
    let midnight = |d: chrono::NaiveDate| d.and_hms_opt(0, 0, 0).unwrap().and_utc().to_rfc3339();
    let days_to_monday = 7 - today.weekday().num_days_from_monday() as i64;
    (midnight(today + Duration::days(1)), midnight(today + Duration::days(days_to_monday)))
}

fn action_matches(target: &str, action: &str) -> bool {
    let Some(target) = target.strip_prefix("action:") else { return false };
    let (target_action, target_dimension) = target.split_once('@').map_or((target, None), |(a, d)| (a, Some(d)));
    let (actual_action, actual_dimension) = action.split_once('@').map_or((action, None), |(a, d)| (a, Some(d)));
    if target_dimension.is_some_and(|d| Some(d) != actual_dimension) {
        return false;
    }
    let Some((target_kind, materials)) = target_action.split_once(':') else { return false };
    let Some((actual_kind, material)) = actual_action.split_once(':') else { return false };
    if target_kind != actual_kind {
        return false;
    }
    materials.split('|').any(|pattern| {
        if pattern == "*" {
            true
        } else if let Some(suffix) = pattern.strip_prefix('*') {
            material.ends_with(suffix)
        } else if let Some(prefix) = pattern.strip_suffix('*') {
            material.starts_with(prefix)
        } else {
            material == pattern
        }
    })
}

#[cfg(test)]
mod action_tests {
    use super::action_matches;

    #[test]
    fn only_matching_materials_and_dimensions_advance_quests() {
        assert!(action_matches("action:block_broken:DIAMOND_ORE|DEEPSLATE_DIAMOND_ORE", "block_broken:DIAMOND_ORE@minecraft:overworld"));
        assert!(!action_matches("action:block_broken:DIAMOND_ORE", "block_broken:STONE@minecraft:overworld"));
        assert!(action_matches("action:block_placed:*_PLANKS", "block_placed:OAK_PLANKS@minecraft:overworld"));
        assert!(action_matches("action:block_broken:*@minecraft:the_nether", "block_broken:NETHERRACK@minecraft:the_nether"));
        assert!(!action_matches("action:block_broken:*@minecraft:the_nether", "block_broken:STONE@minecraft:overworld"));
    }
}

/// Advance quests that require a particular block, mob, or dimension rather
/// than a generic stat counter.
pub async fn advance_action_quests(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    uuid: &str,
    action: &str,
    count: i64,
    now: &str,
) -> AppResult<()> {
    if count <= 0 {
        return Ok(());
    }
    let rows: Vec<(String, String, String, i64)> = sqlx::query_as(
        "SELECT q.id, q.period, q.target_stat, q.target_count FROM quests q
             WHERE q.enabled = 1 AND q.target_stat LIKE 'action:%'
               AND (q.instance_id IS NULL OR EXISTS (
                 SELECT 1 FROM server_online so JOIN game_servers gs ON gs.id = so.server_id
                 WHERE so.uuid = ? AND gs.instance_id = q.instance_id
               ))
               AND (q.server_id IS NULL OR EXISTS (
                 SELECT 1 FROM server_online so WHERE so.uuid = ? AND so.server_id = q.server_id
               ))",
    )
    .bind(uuid)
    .bind(uuid)
    .fetch_all(&mut **tx)
    .await?;
    let daily = current_daily_key();
    let weekly = current_weekly_key();
    let cfg = crate::progression::load(&mut **tx).await?;
    let mut assigned: std::collections::HashMap<&str, Vec<String>> = std::collections::HashMap::new();
    for (id, period, target, required) in rows {
        if !action_matches(&target, action) {
            continue;
        }
        let key = if period == "weekly" { &weekly } else { &daily };
        // Only quests this player was given for the period make progress.
        let period_name = if period == "weekly" { "weekly" } else { "daily" };
        if !assigned.contains_key(period_name) {
            let ids = crate::progression::assigned_quest_ids(&mut **tx, &cfg, uuid, period_name, key).await?;
            assigned.insert(period_name, ids);
        }
        if !assigned[period_name].contains(&id) {
            continue;
        }
        sqlx::query(
            "INSERT INTO user_quests (user_uuid, quest_id, period_key, progress, completed, claimed, updated_at)
             VALUES (?, ?, ?, ?, ?, 0, ?)
             ON CONFLICT (user_uuid, quest_id, period_key) DO UPDATE SET
               progress = progress + excluded.progress,
               completed = CASE WHEN progress + excluded.progress >= ? THEN 1 ELSE completed END,
               updated_at = excluded.updated_at",
        )
        .bind(uuid)
        .bind(&id)
        .bind(key)
        .bind(count)
        .bind(count >= required)
        .bind(now)
        .bind(required)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

/// Get all active daily and weekly quests for the authenticated user with live progress.
pub async fn get_my_quests(auth: AuthUser, State(state): State<AppState>) -> AppResult<Json<Vec<UserQuest>>> {
    Ok(Json(quests_for(&state, &auth.uuid).await?))
}

/// A player's current daily and weekly quests with progress, for the launcher, the game and Discord.
pub async fn quests_for(state: &AppState, uuid: &str) -> AppResult<Vec<UserQuest>> {
    let auth_uuid = uuid;
    let daily_key = current_daily_key();
    let weekly_key = current_weekly_key();

    #[derive(sqlx::FromRow)]
    struct QuestRow {
        id: String,
        title: String,
        description: String,
        period: String,
        category: String,
        target_stat: String,
        target_count: i64,
        xp_reward: i64,
        icon: String,
        enabled: bool,
        instance_id: Option<String>,
        server_id: Option<i64>,
        difficulty: String,
        chain_id: Option<String>,
        chain_step: i64,
        progress: Option<i64>,
        completed: Option<bool>,
        claimed: Option<bool>,
        period_key: Option<String>,
    }
    let rows: Vec<QuestRow> = sqlx::query_as(
        "SELECT q.id, q.title, q.description, q.period, q.category, q.target_stat,
                q.target_count, q.xp_reward, q.icon, q.enabled,
                q.instance_id, q.server_id, q.difficulty, q.chain_id, q.chain_step,
                uq.progress, uq.completed, uq.claimed, uq.period_key
         FROM quests q
         LEFT JOIN user_quests uq ON uq.quest_id = q.id 
              AND uq.user_uuid = ? 
              AND uq.period_key = (CASE WHEN q.period = 'weekly' THEN ? ELSE ? END)
          WHERE q.enabled = 1
            AND (q.instance_id IS NULL OR EXISTS (
              SELECT 1 FROM server_online so JOIN game_servers gs ON gs.id = so.server_id
              WHERE so.uuid = ? AND gs.instance_id = q.instance_id
            ))
            AND (q.server_id IS NULL OR EXISTS (
              SELECT 1 FROM server_online so WHERE so.uuid = ? AND so.server_id = q.server_id
            ))
         ORDER BY q.period ASC, q.xp_reward ASC",
    )
    .bind(auth_uuid)
    .bind(&weekly_key)
    .bind(&daily_key)
    .bind(auth_uuid)
    .bind(auth_uuid)
    .fetch_all(&state.db)
    .await?;

    let cfg = crate::progression::load_pool(&state.db).await?;
    let mut conn = state.db.acquire().await?;
    let daily_ids = crate::progression::assigned_quest_ids(&mut conn, &cfg, auth_uuid, "daily", &daily_key).await?;
    let weekly_ids = crate::progression::assigned_quest_ids(&mut conn, &cfg, auth_uuid, "weekly", &weekly_key).await?;
    drop(conn);
    let (daily_ends, weekly_ends) = period_ends();

    let list = rows
        .into_iter()
        .filter(|r| if r.period == "weekly" { weekly_ids.contains(&r.id) } else { daily_ids.contains(&r.id) })
        .map(|row| {
            let quest_period_is_weekly = row.period == "weekly";
            let progress = row.progress.unwrap_or(0);
            let completed = row.completed.unwrap_or(progress >= row.target_count);
            let claimed = row.claimed.unwrap_or(false);
            let pkey = row.period_key.unwrap_or_else(|| if row.period == "weekly" { weekly_key.clone() } else { daily_key.clone() });

            UserQuest {
                quest: Quest {
                    id: row.id,
                    title: row.title,
                    description: row.description,
                    period: row.period.clone(),
                    category: row.category,
                    target_stat: row.target_stat.clone(),
                    target_count: row.target_count,
                    xp_reward: row.xp_reward,
                    icon: row.icon,
                    enabled: row.enabled,
                    quest_type: Some(row.period),
                    stat_type: Some(row.target_stat),
                    active: Some(row.enabled),
                    instance_id: row.instance_id,
                    server_id: row.server_id,
                    difficulty: Some(row.difficulty),
                    chain_id: row.chain_id,
                    chain_step: Some(row.chain_step),
                },
                progress,
                current_count: progress,
                completed,
                claimed,
                period_key: pkey,
                expires_at: Some(if quest_period_is_weekly { weekly_ends.clone() } else { daily_ends.clone() }),
            }
        })
        .collect();

    Ok(list)
}

/// Find a player by name (case-insensitive) or UUID among accounts and everyone the game has seen.
pub async fn find_player(state: &AppState, who: &str) -> AppResult<Option<(String, String)>> {
    let who = who.trim();
    if who.is_empty() {
        return Ok(None);
    }
    let found: Option<(String, String)> =
        sqlx::query_as("SELECT uuid, username FROM users WHERE (username = ?1 COLLATE NOCASE OR uuid = ?1) AND uuid <> '' LIMIT 1")
            .bind(who)
            .fetch_optional(&state.db)
            .await?;
    if found.is_some() {
        return Ok(found);
    }
    Ok(sqlx::query_as("SELECT uuid, name FROM player_stats WHERE name = ?1 COLLATE NOCASE OR uuid = ?1 ORDER BY last_seen DESC LIMIT 1")
        .bind(who)
        .fetch_optional(&state.db)
        .await?)
}

/// What `/quests <player>` and Discord's `/quests` show: both lists, progress and when they reset.
pub async fn player_summary(state: &AppState, uuid: &str, name: &str) -> AppResult<Value> {
    let all = quests_for(state, uuid).await?;
    let (daily_ends, weekly_ends) = period_ends();
    let shape = |q: &UserQuest| {
        serde_json::json!({
            "id": q.quest.id, "title": q.quest.title, "description": q.quest.description, "progress": q.progress.min(q.quest.target_count),
            "target": q.quest.target_count, "xp": q.quest.xp_reward, "completed": q.completed, "claimed": q.claimed,
        })
    };
    let daily: Vec<Value> = all.iter().filter(|q| q.quest.period != "weekly").map(shape).collect();
    let weekly: Vec<Value> = all.iter().filter(|q| q.quest.period == "weekly").map(shape).collect();
    Ok(
        serde_json::json!({ "ok": true, "player": name, "uuid": uuid, "daily": daily, "weekly": weekly, "daily_resets": daily_ends, "weekly_resets": weekly_ends }),
    )
}

#[derive(Deserialize)]
pub struct PlayerQuestsPayload {
    /// A name or UUID; leave out for the asking player.
    #[serde(default)]
    pub player: String,
    #[serde(default)]
    pub uuid: String,
}

/// Anyone can look at anyone's quests from in game: `/quests` and `/quests <player>`.
pub async fn server_player_quests(
    crate::routes::servers::GameServer(_server): crate::routes::servers::GameServer,
    State(state): State<AppState>,
    Json(p): Json<PlayerQuestsPayload>,
) -> AppResult<Json<Value>> {
    let who = if p.player.trim().is_empty() { p.uuid.as_str() } else { p.player.as_str() };
    let (uuid, name) =
        find_player(&state, who).await?.ok_or_else(|| AppError::not_found(format!("No player called \"{}\"", who.trim())))?;
    Ok(Json(player_summary(&state, &uuid, &name).await?))
}

/// Claim a completed quest reward.
pub async fn claim_quest(auth: AuthUser, Path(quest_id): Path<String>, State(state): State<AppState>) -> AppResult<Json<Value>> {
    let mut tx = state.db.begin().await?;

    let quest: Option<(String, i64, i64)> = sqlx::query_as(
        "SELECT q.period, q.target_count, q.xp_reward FROM quests q
             WHERE q.id = ? AND q.enabled = 1
               AND (q.instance_id IS NULL OR EXISTS (
                 SELECT 1 FROM server_online so JOIN game_servers gs ON gs.id = so.server_id
                 WHERE so.uuid = ? AND gs.instance_id = q.instance_id
               ))
               AND (q.server_id IS NULL OR EXISTS (
                 SELECT 1 FROM server_online so WHERE so.uuid = ? AND so.server_id = q.server_id
               ))",
    )
    .bind(&quest_id)
    .bind(&auth.uuid)
    .bind(&auth.uuid)
    .fetch_optional(&mut *tx)
    .await?;

    let (period, target_count, xp_reward) = quest.ok_or_else(|| AppError::bad_request("Quest not found or disabled"))?;

    let period_key = if period == "weekly" { current_weekly_key() } else { current_daily_key() };

    let cfg = crate::progression::load(&mut tx).await?;
    if !crate::progression::is_assigned(&mut tx, &cfg, &auth.uuid, &period, &period_key, &quest_id).await? {
        return Err(AppError::bad_request("That quest isn't one of yours this period"));
    }

    let user_quest: Option<(i64, bool, bool)> =
        sqlx::query_as("SELECT progress, completed, claimed FROM user_quests WHERE user_uuid = ? AND quest_id = ? AND period_key = ?")
            .bind(&auth.uuid)
            .bind(&quest_id)
            .bind(&period_key)
            .fetch_optional(&mut *tx)
            .await?;

    let (progress, completed, claimed) = user_quest.unwrap_or((0, false, false));

    if claimed {
        return Err(AppError::bad_request("Quest reward already claimed for this period"));
    }
    if !completed && progress < target_count {
        return Err(AppError::bad_request("Quest is not yet completed"));
    }

    let now = chrono::Utc::now().to_rfc3339();

    // Mark as completed & claimed
    sqlx::query(
        "INSERT INTO user_quests (user_uuid, quest_id, period_key, progress, completed, claimed, updated_at)
         VALUES (?, ?, ?, ?, 1, 1, ?)
         ON CONFLICT (user_uuid, quest_id, period_key) DO UPDATE SET
            completed = 1,
            claimed = 1,
            updated_at = excluded.updated_at",
    )
    .bind(&auth.uuid)
    .bind(&quest_id)
    .bind(&period_key)
    .bind(progress.max(target_count))
    .bind(&now)
    .execute(&mut *tx)
    .await?;

    // Grant XP to user_levels
    sqlx::query(
        "INSERT INTO user_levels (uuid, global_xp, global_level, updated_at)
         VALUES (?, ?, 1, ?)
         ON CONFLICT (uuid) DO UPDATE SET
            global_xp = global_xp + excluded.global_xp,
            updated_at = excluded.updated_at",
    )
    .bind(&auth.uuid)
    .bind(xp_reward)
    .bind(&now)
    .execute(&mut *tx)
    .await?;

    // Read new total XP & compute new level
    let new_xp: i64 = sqlx::query_scalar("SELECT global_xp FROM user_levels WHERE uuid = ?").bind(&auth.uuid).fetch_one(&mut *tx).await?;

    let (new_lvl, _, _, _) = crate::progression::load(&mut tx).await?.level_from_xp(new_xp);
    sqlx::query("UPDATE user_levels SET global_level = ? WHERE uuid = ?").bind(new_lvl).bind(&auth.uuid).execute(&mut *tx).await?;

    crate::routes::leveling::grant_rewards(&mut tx, &auth.uuid, &now).await?;
    crate::rewards::enqueue(&mut tx, &auth.uuid, "quest", &quest_id).await?;
    tx.commit().await?;
    let _ = crate::rewards::process_queue(&state).await;

    Ok(Json(serde_json::json!({
        "ok": true,
        "xp_granted": xp_reward,
        "new_global_xp": new_xp,
        "new_global_level": new_lvl
    })))
}

// ---------------------------------------------------------------------------
// Admin Quests API
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct AdminQuestRow {
    #[serde(flatten)]
    pub quest: Quest,
    pub total_completions: i64,
    /// Always handed out first when a player's quests are limited.
    pub pinned: bool,
}

#[derive(sqlx::FromRow)]
struct AdminQuestDbRow {
    id: String,
    title: String,
    description: String,
    period: String,
    category: String,
    target_stat: String,
    target_count: i64,
    xp_reward: i64,
    icon: String,
    enabled: bool,
    total_completions: i64,
    pinned: bool,
    instance_id: Option<String>,
    server_id: Option<i64>,
    difficulty: String,
    chain_id: Option<String>,
    chain_step: i64,
}

/// Admin list all quests.
pub async fn admin_list_quests(_admin: AdminUser, State(state): State<AppState>) -> AppResult<Json<Vec<AdminQuestRow>>> {
    let rows: Vec<AdminQuestDbRow> = sqlx::query_as(
        "SELECT q.id, q.title, q.description, q.period, q.category, q.target_stat,
                q.target_count, q.xp_reward, q.icon, q.enabled,
                COUNT(uq.user_uuid) as total_completions, q.pinned,
                q.instance_id, q.server_id, q.difficulty, q.chain_id, q.chain_step
         FROM quests q
         LEFT JOIN user_quests uq ON uq.quest_id = q.id AND uq.completed = 1
         GROUP BY q.id
         ORDER BY q.period ASC, q.created_at DESC",
    )
    .fetch_all(&state.db)
    .await?;

    let list = rows
        .into_iter()
        .map(|row| AdminQuestRow {
            quest: Quest {
                id: row.id,
                title: row.title,
                description: row.description,
                period: row.period.clone(),
                category: row.category,
                target_stat: row.target_stat.clone(),
                target_count: row.target_count,
                xp_reward: row.xp_reward,
                icon: row.icon,
                enabled: row.enabled,
                quest_type: Some(row.period),
                stat_type: Some(row.target_stat),
                active: Some(row.enabled),
                instance_id: row.instance_id,
                server_id: row.server_id,
                difficulty: Some(row.difficulty),
                chain_id: row.chain_id,
                chain_step: Some(row.chain_step),
            },
            total_completions: row.total_completions,
            pinned: row.pinned,
        })
        .collect();

    Ok(Json(list))
}

#[derive(Deserialize)]
pub struct QuestPayload {
    pub id: Option<String>,
    pub title: String,
    pub description: String,
    #[serde(alias = "quest_type")]
    pub period: Option<String>,
    pub category: String,
    #[serde(alias = "stat_type")]
    pub target_stat: Option<String>,
    pub target_count: i64,
    pub xp_reward: i64,
    pub icon: Option<String>,
    #[serde(alias = "active")]
    pub enabled: Option<bool>,
    pub pinned: Option<bool>,
    #[serde(default)]
    pub instance_id: Option<String>,
    #[serde(default)]
    pub server_id: Option<i64>,
    #[serde(default)]
    pub difficulty: Option<String>,
    #[serde(default)]
    pub chain_id: Option<String>,
    #[serde(default)]
    pub chain_step: Option<i64>,
}

/// Admin create quest.
pub async fn admin_create_quest(
    _admin: AdminUser,
    State(state): State<AppState>,
    Json(payload): Json<QuestPayload>,
) -> AppResult<Json<Value>> {
    validate_admin_quest(&payload)?;
    validate_quest_scope(&state, &payload).await?;
    let period = payload.period.unwrap_or_else(|| "daily".into());
    let target_stat = payload.target_stat.unwrap_or_else(|| "blocks_broken".into());
    let id = payload.id.filter(|s| !s.trim().is_empty()).unwrap_or_else(|| format!("q_{}_{}", period, uuid::Uuid::new_v4().simple()));
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO quests (id, title, description, period, category, target_stat, target_count, xp_reward, icon, enabled, created_at, pinned, instance_id, server_id, difficulty, chain_id, chain_step)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(payload.title)
    .bind(payload.description)
    .bind(period)
    .bind(payload.category)
    .bind(target_stat)
    .bind(payload.target_count)
    .bind(payload.xp_reward)
    .bind(payload.icon.unwrap_or_else(|| "⛏️".into()))
    .bind(payload.enabled.unwrap_or(true))
    .bind(&now)
    .bind(payload.pinned.unwrap_or(false))
    .bind(payload.instance_id)
    .bind(payload.server_id)
    .bind(payload.difficulty.unwrap_or_else(|| "standard".into()))
    .bind(payload.chain_id)
    .bind(payload.chain_step.unwrap_or(0))
    .execute(&state.db)
    .await?;

    crate::rewards::apply_defaults(&state).await?;
    Ok(Json(serde_json::json!({ "id": id, "ok": true })))
}

/// Admin update quest.
pub async fn admin_update_quest(
    _admin: AdminUser,
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(payload): Json<QuestPayload>,
) -> AppResult<Json<Value>> {
    validate_admin_quest(&payload)?;
    validate_quest_scope(&state, &payload).await?;
    let period = payload.period.unwrap_or_else(|| "daily".into());
    let target_stat = payload.target_stat.unwrap_or_else(|| "blocks_broken".into());

    sqlx::query(
        "UPDATE quests SET
            title = ?,
            description = ?,
            period = ?,
            category = ?,
            target_stat = ?,
            target_count = ?,
            xp_reward = ?,
            icon = COALESCE(?, icon),
            enabled = COALESCE(?, enabled),
            pinned = COALESCE(?, pinned)
            ,instance_id = ?, server_id = ?, difficulty = COALESCE(?, difficulty), chain_id = ?, chain_step = COALESCE(?, chain_step)
         WHERE id = ?",
    )
    .bind(payload.title)
    .bind(payload.description)
    .bind(period)
    .bind(payload.category)
    .bind(target_stat)
    .bind(payload.target_count)
    .bind(payload.xp_reward)
    .bind(payload.icon)
    .bind(payload.enabled)
    .bind(payload.pinned)
    .bind(payload.instance_id)
    .bind(payload.server_id)
    .bind(payload.difficulty)
    .bind(payload.chain_id)
    .bind(payload.chain_step)
    .bind(&id)
    .execute(&state.db)
    .await?;

    crate::rewards::apply_defaults(&state).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

fn validate_admin_quest(payload: &QuestPayload) -> AppResult<()> {
    if payload.title.trim().is_empty() || payload.title.chars().count() > 100 {
        return Err(AppError::bad_request("quest title must be 1-100 characters"));
    }
    if payload.description.trim().is_empty() || payload.description.chars().count() > 1000 {
        return Err(AppError::bad_request("quest description must be 1-1000 characters"));
    }
    if payload.target_count < 1 || payload.xp_reward < 0 {
        return Err(AppError::bad_request("quest target must be positive and XP reward cannot be negative"));
    }
    if let Some(server_id) = payload.server_id {
        if server_id <= 0 {
            return Err(AppError::bad_request("server ID must be positive"));
        }
    }
    if let Some(instance_id) = &payload.instance_id {
        if instance_id.trim().is_empty() || instance_id.len() > 80 {
            return Err(AppError::bad_request("instance ID must be 1-80 characters"));
        }
    }
    if let Some(difficulty) = &payload.difficulty {
        if !matches!(difficulty.as_str(), "easy" | "standard" | "hard" | "challenge") {
            return Err(AppError::bad_request("difficulty must be easy, standard, hard, or challenge"));
        }
    }
    if payload.chain_step.unwrap_or(0) < 0 {
        return Err(AppError::bad_request("quest chain step cannot be negative"));
    }
    if payload.chain_id.as_ref().is_some_and(|id| id.trim().is_empty() || id.len() > 80) {
        return Err(AppError::bad_request("quest chain ID must be 1-80 characters"));
    }
    Ok(())
}

async fn validate_quest_scope(state: &AppState, payload: &QuestPayload) -> AppResult<()> {
    if let Some(server_id) = payload.server_id {
        let instance_id: Option<String> =
            sqlx::query_scalar("SELECT instance_id FROM game_servers WHERE id=?").bind(server_id).fetch_optional(&state.db).await?;
        let instance_id = instance_id.ok_or_else(|| AppError::bad_request("quest server does not exist"))?;
        if payload.instance_id.as_ref().is_some_and(|scope| scope != &instance_id) {
            return Err(AppError::bad_request("quest server must belong to its selected instance scope"));
        }
    }
    Ok(())
}

/// Admin delete quest.
pub async fn admin_delete_quest(_admin: AdminUser, Path(id): Path<String>, State(state): State<AppState>) -> AppResult<Json<Value>> {
    sqlx::query("DELETE FROM reward_bundles WHERE source_type = 'quest' AND source_id = ?").bind(&id).execute(&state.db).await?;
    sqlx::query("DELETE FROM quests WHERE id = ?").bind(&id).execute(&state.db).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

// ---- quest chains ---------------------------------------------------------------------------------------------------

#[derive(Serialize, sqlx::FromRow)]
pub struct ChainRow {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[derive(Deserialize)]
pub struct ChainPayload {
    pub name: String,
    #[serde(default)]
    pub description: String,
}

fn check_chain(p: &ChainPayload) -> AppResult<()> {
    if p.name.trim().is_empty() || p.name.chars().count() > 60 || p.description.chars().count() > 200 {
        return Err(AppError::bad_request("A chain needs a name (up to 60 characters); the description can be up to 200"));
    }
    Ok(())
}

/// Every chain: the saved ones, plus any chain ID that quests already use without a record (older data, the quest editor's free-text field).
pub async fn list_chains(_admin: AdminUser, State(state): State<AppState>) -> AppResult<Json<Vec<ChainRow>>> {
    let mut chains: Vec<ChainRow> = sqlx::query_as("SELECT id, name, description FROM quest_chains ORDER BY created_at, id").fetch_all(&state.db).await?;
    let used: Vec<String> = sqlx::query_scalar("SELECT DISTINCT chain_id FROM quests WHERE chain_id IS NOT NULL AND chain_id <> '' ORDER BY chain_id")
        .fetch_all(&state.db)
        .await?;
    for id in used {
        if !chains.iter().any(|c| c.id == id) {
            let name = id.replace('_', " ").split_whitespace().map(|w| {
                let mut c = w.chars();
                c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
            }).collect::<Vec<_>>().join(" ");
            chains.push(ChainRow { id, name, description: String::new() });
        }
    }
    Ok(Json(chains))
}

pub async fn create_chain(_admin: AdminUser, State(state): State<AppState>, Json(p): Json<ChainPayload>) -> AppResult<Json<ChainRow>> {
    check_chain(&p)?;
    let name = p.name.trim().to_string();
    let id: String = name.to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect::<String>().split('_').filter(|s| !s.is_empty()).collect::<Vec<_>>().join("_");
    if id.is_empty() {
        return Err(AppError::bad_request("The chain name needs at least one letter or number"));
    }
    let taken: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM quest_chains WHERE id = ?) OR EXISTS(SELECT 1 FROM quests WHERE chain_id = ?)")
        .bind(&id).bind(&id).fetch_one(&state.db).await?;
    if taken {
        return Err(AppError::conflict("A chain with this name already exists"));
    }
    sqlx::query("INSERT INTO quest_chains (id, name, description, created_at) VALUES (?, ?, ?, ?)")
        .bind(&id).bind(&name).bind(p.description.trim()).bind(crate::db::now()).execute(&state.db).await?;
    Ok(Json(ChainRow { id, name, description: p.description.trim().to_string() }))
}

pub async fn update_chain(_admin: AdminUser, State(state): State<AppState>, Path(id): Path<String>, Json(p): Json<ChainPayload>) -> AppResult<Json<Value>> {
    check_chain(&p)?;
    // A chain that only existed through its quests gets its record now.
    sqlx::query("INSERT INTO quest_chains (id, name, description, created_at) VALUES (?, ?, ?, ?) ON CONFLICT(id) DO UPDATE SET name = excluded.name, description = excluded.description")
        .bind(&id).bind(p.name.trim()).bind(p.description.trim()).bind(crate::db::now()).execute(&state.db).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// Deleting a chain keeps its quests; they just stop being part of it.
pub async fn delete_chain(_admin: AdminUser, State(state): State<AppState>, Path(id): Path<String>) -> AppResult<Json<Value>> {
    let mut tx = state.db.begin().await?;
    sqlx::query("UPDATE quests SET chain_id = NULL, chain_step = 0 WHERE chain_id = ?").bind(&id).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM quest_chains WHERE id = ?").bind(&id).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}
