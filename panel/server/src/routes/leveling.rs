//! Leveling system (Global Level & Server-Specific Levels), rewards, and progression.

use crate::auth::{AdminUser, AuthUser};
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::state::RequestState as State;
use axum::extract::Path;
use axum::Json;
use velora_shared::{ServerLevelInfo, UserLevelInfo};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use crate::progression::{level_from_xp, xp_for_level};

/// Award each configured entitlement once in the transaction which earned it.
/// Achievement XP is credited automatically via the `achievement_xp` DB trigger;
/// this function only processes level-up rewards (titles, badges, item entitlements).
pub async fn grant_rewards(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, uuid: &str, now: &str) -> AppResult<()> {
    let curve = crate::progression::load(&mut **tx).await?;
    let xp: i64 =
        sqlx::query_scalar("SELECT global_xp FROM user_levels WHERE uuid = ?").bind(uuid).fetch_optional(&mut **tx).await?.unwrap_or(0);
    let level = curve.level_from_xp(xp).0;
    let before: i64 = sqlx::query_scalar("SELECT global_level FROM user_levels WHERE uuid = ?")
        .bind(uuid)
        .fetch_optional(&mut **tx)
        .await?
        .unwrap_or(1)
        .max(1);
    sqlx::query("UPDATE user_levels SET global_level = ? WHERE uuid = ?").bind(level).bind(uuid).execute(&mut **tx).await?;
    // Every level reached pays default money (a long jump pays the levels it passed, up to 50 at a time).
    for reached in (before + 1)..=level.min(before + 50) {
        crate::rewards::enqueue_level(&mut **tx, uuid, reached).await?;
    }
    let rewards: Vec<(i64, String, Option<i64>, String, String, String)> = sqlx::query_as(
        "SELECT r.id, r.level_type, r.server_id, r.reward_type, r.reward_name, r.reward_data FROM level_rewards r
         WHERE (r.level_type = 'global' AND r.level_req <= ?)
         OR (r.level_type = 'server' AND EXISTS (SELECT 1 FROM server_levels sl WHERE sl.uuid = ? AND sl.server_id = r.server_id AND sl.server_level >= r.level_req))
         ORDER BY r.level_req, r.id")
        .bind(level).bind(uuid).fetch_all(&mut **tx).await?;
    for (id, scope, server_id, kind, name, data) in rewards {
        let inserted = sqlx::query("INSERT OR IGNORE INTO granted_rewards(uuid, reward_id, granted_at) VALUES (?, ?, ?)")
            .bind(uuid)
            .bind(id)
            .bind(now)
            .execute(&mut **tx)
            .await?
            .rows_affected();
        if inserted == 0 {
            continue;
        }
        crate::rewards::enqueue(&mut **tx, uuid, "level_reward", &id.to_string()).await?;
        match kind.as_str() {
            "title" if scope == "global" => {
                sqlx::query("UPDATE user_levels SET title = ? WHERE uuid = ?").bind(&name).bind(uuid).execute(&mut **tx).await?;
            }
            "title" => {
                sqlx::query("UPDATE server_levels SET rank_name = ? WHERE server_id = ? AND uuid = ?")
                    .bind(&name)
                    .bind(server_id)
                    .bind(uuid)
                    .execute(&mut **tx)
                    .await?;
            }
            "badge" | "profile_badge" => {
                let raw: String = sqlx::query_scalar("SELECT badges FROM user_levels WHERE uuid = ?")
                    .bind(uuid)
                    .fetch_optional(&mut **tx)
                    .await?
                    .unwrap_or_else(|| "[]".into());
                let mut badges: Vec<String> = serde_json::from_str(&raw).unwrap_or_default();
                let data: Value = serde_json::from_str(&data).unwrap_or_default();
                let badge = data["badge"].as_str().unwrap_or(&name).to_string();
                if !badges.contains(&badge) {
                    badges.push(badge);
                }
                sqlx::query("INSERT INTO user_levels(uuid, badges, updated_at) VALUES (?, ?, ?) ON CONFLICT(uuid) DO UPDATE SET badges = excluded.badges")
                    .bind(uuid).bind(serde_json::to_string(&badges)?).bind(now).execute(&mut **tx).await?;
            }
            _ => {} // Item/cosmetic entitlements are recorded in granted_rewards.
        }
    }
    let active_title: Option<String> = sqlx::query_scalar("SELECT reward_name FROM level_rewards WHERE level_type='global' AND reward_type='title' AND level_req<=? ORDER BY level_req DESC,id DESC LIMIT 1")
        .bind(level).fetch_optional(&mut **tx).await?;
    sqlx::query("UPDATE user_levels SET title=? WHERE uuid=?").bind(active_title).bind(uuid).execute(&mut **tx).await?;
    Ok(())
}

#[derive(Serialize, Deserialize)]
pub struct RewardRow {
    pub id: i64,
    pub level_type: String,
    pub server_id: Option<i64>,
    pub level_req: i64,
    pub reward_type: String,
    pub reward_name: String,
    pub reward_data: Value,
    pub created_at: String,
}

/// Get current user's global and server levels.
pub async fn get_my_levels(auth: AuthUser, State(state): State<AppState>) -> AppResult<Json<UserLevelInfo>> {
    get_user_levels_by_uuid(&state, &auth.uuid).await.map(Json)
}

/// Public / Profile lookup for player levels.
pub async fn get_player_levels(Path(uuid): Path<String>, State(state): State<AppState>) -> AppResult<Json<UserLevelInfo>> {
    get_user_levels_by_uuid(&state, &uuid).await.map(Json)
}

pub async fn get_user_levels_by_uuid(state: &AppState, uuid: &str) -> AppResult<UserLevelInfo> {
    let curve = crate::progression::load_pool(&state.db).await?;
    let row: Option<(i64, i64, Option<String>, String)> =
        sqlx::query_as("SELECT global_xp, global_level, title, badges FROM user_levels WHERE uuid = ?")
            .bind(uuid)
            .fetch_optional(&state.db)
            .await?;

    let (global_xp, _db_lvl, title, badges_json) = row.unwrap_or((0, 1, None, "[]".into()));
    let (global_level, current_level_xp, next_level_xp, progress_pct) = curve.level_from_xp(global_xp);
    let badges: Vec<String> = serde_json::from_str(&badges_json).unwrap_or_default();
    let title_image: Option<String> = sqlx::query_scalar("SELECT json_extract(reward_data, '$.title_image') FROM level_rewards WHERE level_type='global' AND reward_type='title' AND level_req<=? ORDER BY level_req DESC,id DESC LIMIT 1")
        .bind(global_level).fetch_optional(&state.db).await?.flatten();
    let rank_images: Vec<(i64, i64, Option<String>)> = sqlx::query_as("SELECT server_id, level_req, json_extract(reward_data, '$.title_image') FROM level_rewards WHERE server_id IS NOT NULL AND level_type='server' AND reward_type='title' ORDER BY level_req DESC,id DESC")
        .fetch_all(&state.db).await?;

    // Query server levels for this user
    let server_rows: Vec<(i64, String, i64, i64, Option<String>)> = sqlx::query_as(
        "SELECT sl.server_id, gs.name, sl.server_xp, sl.server_level, sl.rank_name
         FROM server_levels sl
         JOIN game_servers gs ON gs.id = sl.server_id
         WHERE sl.uuid = ?
         ORDER BY sl.server_xp DESC",
    )
    .bind(uuid)
    .fetch_all(&state.db)
    .await?;

    let server_levels = server_rows
        .into_iter()
        .map(|(s_id, s_name, s_xp, _s_lvl, r_name)| {
            let (sl, cur, next, pct) = curve.level_from_xp(s_xp);
            ServerLevelInfo {
                server_id: s_id,
                server_name: s_name,
                server_xp: s_xp,
                server_level: sl,
                current_level_xp: cur,
                next_level_xp: next,
                progress_pct: pct,
                rank_name: r_name,
                title_image: rank_images.iter().find(|(id, req, _)| *id == s_id && *req <= sl).and_then(|(_, _, image)| image.clone()),
            }
        })
        .collect();

    // Query rewards
    let rewards_rows: Vec<(i64, i64, String, String, String)> = sqlx::query_as(
        "SELECT id, level_req, reward_type, reward_name, reward_data
         FROM level_rewards
         ORDER BY level_req ASC",
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let rewards = rewards_rows
        .into_iter()
        .map(|(id, req, rtype, rname, rdata)| {
            let data: Value = serde_json::from_str(&rdata).unwrap_or_default();
            let desc = data.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let icon = data.get("icon").and_then(|v| v.as_str()).map(|s| s.to_string());
            velora_shared::LevelReward {
                id,
                level: req,
                reward_type: rtype,
                reward_value: rname,
                description: desc,
                icon,
                server_id: None,
            }
        })
        .collect();

    let rank_opt: Option<i64> = sqlx::query_scalar("SELECT COUNT(*) + 1 FROM user_levels WHERE global_xp > ?")
        .bind(global_xp)
        .fetch_optional(&state.db)
        .await
        .unwrap_or(Some(1));

    Ok(UserLevelInfo {
        uuid: uuid.to_string(),
        global_xp,
        current_xp: global_xp,
        global_level,
        level: global_level,
        current_level_xp,
        next_level_xp,
        progress_pct,
        title,
        title_image,
        badges,
        server_levels,
        rank: rank_opt.map(|r| r as usize),
        rewards,
    })
}

/// Global leveling leaderboard.
pub async fn get_level_leaderboard(State(state): State<AppState>) -> AppResult<Json<Vec<Value>>> {
    let curve = crate::progression::load_pool(&state.db).await?;
    let rows: Vec<(String, String, i64, i64, Option<String>)> = sqlx::query_as(
        "SELECT u.uuid, u.username, ul.global_xp, ul.global_level, ul.title
         FROM user_levels ul
         JOIN users u ON u.uuid = ul.uuid
         ORDER BY ul.global_xp DESC
         LIMIT 50",
    )
    .fetch_all(&state.db)
    .await?;

    let list = rows
        .into_iter()
        .enumerate()
        .map(|(rank, (uuid, username, xp, _, title))| {
            let (level, cur, next, pct) = curve.level_from_xp(xp);
            serde_json::json!({
                "rank": rank + 1,
                "uuid": uuid,
                "username": username,
                "global_level": level,
                "global_xp": xp,
                "current_level_xp": cur,
                "next_level_xp": next,
                "progress_pct": pct,
                "title": title
            })
        })
        .collect();

    Ok(Json(list))
}

/// Get available rewards.
pub async fn list_rewards(State(state): State<AppState>) -> AppResult<Json<Vec<Value>>> {
    let rows: Vec<(i64, String, Option<i64>, i64, String, String, String, String)> = sqlx::query_as(
        "SELECT id, level_type, server_id, level_req, reward_type, reward_name, reward_data, created_at
         FROM level_rewards
         ORDER BY level_req ASC",
    )
    .fetch_all(&state.db)
    .await?;

    let list = rows
        .into_iter()
        .map(|(id, ltype, sid, req, rtype, name, data, created)| {
            let parsed_data: Value = serde_json::from_str(&data).unwrap_or_default();
            let desc = parsed_data.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let icon = parsed_data
                .get("icon")
                .and_then(|v| v.as_str())
                .unwrap_or(match rtype.as_str() {
                    "title" => "👑",
                    "profile_badge" | "badge" => "⭐",
                    "cosmetic" => "✨",
                    "item" => "🗡️",
                    _ => "🎁",
                })
                .to_string();

            serde_json::json!({
                "id": id,
                "level": req,
                "level_req": req,
                "level_type": ltype,
                "server_id": sid,
                "reward_type": rtype,
                "reward_value": name,
                "reward_name": name,
                "description": if desc.is_empty() { format!("Granted at Level {req}") } else { desc },
                "icon": icon,
                "reward_data": parsed_data,
                "created_at": created
            })
        })
        .collect();

    Ok(Json(list))
}

#[derive(Deserialize)]
pub struct RewardPayload {
    pub id: Option<i64>,
    pub level_type: Option<String>,
    pub server_id: Option<i64>,
    #[serde(alias = "level")]
    pub level_req: Option<i64>,
    pub reward_type: String,
    #[serde(alias = "reward_value")]
    pub reward_name: Option<String>,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub reward_data: Option<Value>,
}

fn validate_rank_mapping(data: &serde_json::Map<String, Value>) -> AppResult<()> {
    if let Some(image) = data.get("title_image").filter(|v| !v.is_null()) {
        let image = image.as_str().ok_or_else(|| AppError::bad_request("Rank image must be a PNG URL"))?;
        if !image.is_empty()
            && (image.len() > 2048
                || !(image.starts_with("/uploads/") || image.starts_with("https://"))
                || !image.split(['?', '#']).next().unwrap_or("").to_ascii_lowercase().ends_with(".png"))
        {
            return Err(AppError::bad_request("Rank image must be an uploaded PNG or HTTPS PNG URL"));
        }
    }
    if data.get("discord_role_id").is_some_and(|v| !v.is_string() && !v.is_null())
        || data.get("luckperms_group").is_some_and(|v| !v.is_string() && !v.is_null())
    {
        return Err(AppError::bad_request("Rank role mappings must be text"));
    }
    let discord = data.get("discord_role_id").and_then(Value::as_str).unwrap_or("");
    if !discord.is_empty() && (discord.len() > 24 || !discord.bytes().all(|b| b.is_ascii_digit())) {
        return Err(AppError::bad_request("Discord role ID must contain digits only"));
    }
    let group = data.get("luckperms_group").and_then(Value::as_str).unwrap_or("");
    if !group.is_empty() && (group.len() > 64 || !group.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b == b'.')) {
        return Err(AppError::bad_request("LuckPerms group must contain letters, digits, _, - or ."));
    }
    Ok(())
}

/// Titles and rank names are shown from a copy kept on each player, so they must be recomputed whenever a reward is
/// created, renamed, moved or deleted. Otherwise a rename on the admin page never reaches the launcher.
pub async fn refresh_global_titles(state: &AppState) -> AppResult<(usize, usize)> {
    let curve = crate::progression::load_pool(&state.db).await?;
    let mut tx = state.db.begin().await?;
    let (mut checked, mut changed) = (0usize, 0usize);
    let players: Vec<(String, i64, Option<String>)> =
        sqlx::query_as("SELECT uuid, global_xp, title FROM user_levels").fetch_all(&mut *tx).await?;
    for (uuid, xp, current) in players {
        let level = curve.level_from_xp(xp).0;
        let title: Option<String> = sqlx::query_scalar("SELECT reward_name FROM level_rewards WHERE level_type='global' AND reward_type='title' AND level_req<=? ORDER BY level_req DESC,id DESC LIMIT 1")
            .bind(level).fetch_optional(&mut *tx).await?;
        checked += 1;
        if title != current {
            changed += 1;
            sqlx::query("UPDATE user_levels SET title=? WHERE uuid=?").bind(title).bind(uuid).execute(&mut *tx).await?;
        }
    }
    let members: Vec<(i64, String, i64, Option<String>)> =
        sqlx::query_as("SELECT server_id, uuid, server_xp, rank_name FROM server_levels").fetch_all(&mut *tx).await?;
    for (server_id, uuid, xp, current) in members {
        let level = curve.level_from_xp(xp).0;
        let rank: Option<String> = sqlx::query_scalar("SELECT reward_name FROM level_rewards WHERE level_type='server' AND server_id=? AND reward_type='title' AND level_req<=? ORDER BY level_req DESC,id DESC LIMIT 1")
            .bind(server_id).bind(level).fetch_optional(&mut *tx).await?;
        checked += 1;
        if rank != current {
            changed += 1;
            sqlx::query("UPDATE server_levels SET rank_name=? WHERE server_id=? AND uuid=?")
                .bind(rank)
                .bind(server_id)
                .bind(uuid)
                .execute(&mut *tx)
                .await?;
        }
    }
    tx.commit().await?;
    Ok((checked, changed))
}

/// Admin: recompute every player's title and server rank from the current rewards. Needed after renames made before titles
/// followed rewards automatically, and handy any time the two have drifted apart. Reports how many players actually changed.
pub async fn admin_sync_titles(_admin: AdminUser, State(state): State<AppState>) -> AppResult<Json<Value>> {
    let (checked, changed) = refresh_global_titles(&state).await?;
    Ok(Json(serde_json::json!({ "ok": true, "checked": checked, "changed": changed })))
}

/// Admin create reward.
pub async fn admin_create_reward(
    _admin: AdminUser,
    State(state): State<AppState>,
    Json(payload): Json<RewardPayload>,
) -> AppResult<Json<Value>> {
    let now = chrono::Utc::now().to_rfc3339();
    let level_req = payload.level_req.unwrap_or(2);
    let reward_name = payload.reward_name.unwrap_or_else(|| "Reward".into());
    let level_type = payload.level_type.unwrap_or_else(|| "global".into());

    let mut data_obj = match payload.reward_data {
        Some(Value::Object(map)) => map,
        _ => serde_json::Map::new(),
    };
    if let Some(desc) = payload.description {
        data_obj.insert("description".into(), Value::String(desc));
    }
    if let Some(ico) = payload.icon {
        data_obj.insert("icon".into(), Value::String(ico));
    }
    data_obj.insert("title".into(), Value::String(reward_name.clone()));
    validate_rank_mapping(&data_obj)?;
    let data_str = serde_json::to_string(&data_obj).unwrap_or_default();

    let id: i64 = sqlx::query_scalar(
        "INSERT INTO level_rewards (level_type, server_id, level_req, reward_type, reward_name, reward_data, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)
         RETURNING id",
    )
    .bind(level_type)
    .bind(payload.server_id)
    .bind(level_req)
    .bind(payload.reward_type)
    .bind(reward_name)
    .bind(data_str)
    .bind(&now)
    .fetch_one(&state.db)
    .await?;

    refresh_global_titles(&state).await?;

    crate::rewards::apply_defaults(&state).await?;
    Ok(Json(serde_json::json!({ "id": id, "ok": true })))
}

/// Admin update reward.
pub async fn admin_update_reward(
    _admin: AdminUser,
    Path(id): Path<i64>,
    State(state): State<AppState>,
    Json(payload): Json<RewardPayload>,
) -> AppResult<Json<Value>> {
    let level_req = payload.level_req.unwrap_or(2);
    let reward_name = payload.reward_name.unwrap_or_else(|| "Reward".into());

    let mut data_obj = match payload.reward_data {
        Some(Value::Object(map)) => map,
        _ => serde_json::Map::new(),
    };
    if let Some(desc) = payload.description {
        data_obj.insert("description".into(), Value::String(desc));
    }
    if let Some(ico) = payload.icon {
        data_obj.insert("icon".into(), Value::String(ico));
    }
    data_obj.insert("title".into(), Value::String(reward_name.clone()));
    validate_rank_mapping(&data_obj)?;
    let data_str = serde_json::to_string(&data_obj).unwrap_or_default();

    sqlx::query(
        "UPDATE level_rewards SET
            level_type = COALESCE(?, level_type),
            server_id = CASE WHEN ? = 1 THEN ? ELSE server_id END,
            level_req = ?,
            reward_type = ?,
            reward_name = ?,
            reward_data = ?
         WHERE id = ?",
    )
    .bind(payload.level_type.clone())
    .bind(payload.level_type.is_some() as i64)
    .bind(payload.server_id)
    .bind(level_req)
    .bind(payload.reward_type)
    .bind(reward_name)
    .bind(data_str)
    .bind(id)
    .execute(&state.db)
    .await?;

    refresh_global_titles(&state).await?;

    crate::rewards::apply_defaults(&state).await?;
    Ok(Json(serde_json::json!({ "id": id, "ok": true })))
}

/// Admin delete reward.
pub async fn admin_delete_reward(_admin: AdminUser, Path(id): Path<i64>, State(state): State<AppState>) -> AppResult<Json<Value>> {
    sqlx::query("DELETE FROM reward_bundles WHERE source_type = 'level_reward' AND source_id = ?")
        .bind(id.to_string())
        .execute(&state.db)
        .await?;
    sqlx::query("DELETE FROM level_rewards WHERE id = ?").bind(id).execute(&state.db).await?;
    refresh_global_titles(&state).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}
