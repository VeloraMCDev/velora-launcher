//! Data and actions for in-game plugins: what the SCOPENET Developer API, the
//! PlaceholderAPI expansion and other plugins read and change for a player.

use crate::state::RequestState as State;
use crate::error::{AppError, AppResult};
use crate::routes::economy::{begin_operation, finish_operation};
use crate::routes::servers::GameServer;
use crate::state::AppState;

use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
pub struct Who {
    pub uuid: String,
}

fn dashed_or_err(uuid: &str) -> AppResult<String> {
    crate::yggdrasil::dashed(uuid).ok_or_else(|| AppError::bad_request("invalid player UUID"))
}

/// Everything a placeholder or `getPlayer()` needs in one round trip.
pub async fn player_info(GameServer(server): GameServer, State(state): State<AppState>, Json(p): Json<Who>) -> AppResult<Json<Value>> {
    let uuid = dashed_or_err(&p.uuid)?;
    let user: Option<(String, String, String)> =
        sqlx::query_as("SELECT username, role, created_at FROM users WHERE uuid = ?").bind(&uuid).fetch_optional(&state.db).await?;
    let Some((name, role, joined)) = user else {
        return Ok(Json(json!({ "exists": false, "uuid": uuid })));
    };
    let levels = crate::routes::leveling::get_user_levels_by_uuid(&state, &uuid).await?;
    let here = levels.server_levels.iter().find(|s| s.server_id == server.id);
    let guild: Option<(String, String, String, String, i64)> = sqlx::query_as(
        "SELECT g.id, g.name, g.tag, gm.role, (SELECT COUNT(*) FROM guild_claims gc WHERE gc.guild_id = g.id)
         FROM guild_members gm JOIN guilds g ON g.id = gm.guild_id WHERE gm.uuid = ? AND g.instance_id = ?",
    )
    .bind(&uuid)
    .bind(&server.instance_id)
    .fetch_optional(&state.db)
    .await?;
    let balance = crate::routes::economy::balance_for(&state, server.id, &uuid).await?;
    let (playtime, kills, deaths): (Option<i64>, Option<i64>, Option<i64>) =
        sqlx::query_as("SELECT SUM(playtime_secs), SUM(player_kills + mob_kills), SUM(deaths) FROM player_stats WHERE uuid = ?")
            .bind(&uuid)
            .fetch_one(&state.db)
            .await?;
    let (server_playtime,): (Option<i64>,) = sqlx::query_as("SELECT SUM(playtime_secs) FROM player_stats WHERE uuid = ? AND server_id = ?")
        .bind(&uuid)
        .bind(server.id)
        .fetch_one(&state.db)
        .await?;
    let friends: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM friendships WHERE status = 'accepted' AND (user_uuid = ?1 OR friend_uuid = ?1)")
            .bind(&uuid)
            .fetch_one(&state.db)
            .await?;
    let achievements: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM user_achievements WHERE user_uuid = ?").bind(&uuid).fetch_one(&state.db).await?;
    let unlocks: Vec<(String, String, String)> = sqlx::query_as(
        // A modelled cosmetic follows its template, so edits (a new model, a different size) reach players without a re-grant.
        "SELECT u.unlock_key, u.unlock_type, CASE WHEN u.unlock_type = 'cosmetic' AND t.metadata IS NOT NULL THEN t.metadata ELSE u.metadata END
         FROM player_unlocks u LEFT JOIN cosmetic_templates t ON t.key = u.unlock_key
         WHERE u.uuid=? AND u.equipped=1 ORDER BY u.unlock_type, u.unlocked_at DESC",
    ).bind(&uuid).fetch_all(&state.db).await?;
    let mut identity = serde_json::Map::new();
    let mut cosmetics = serde_json::Map::new();
    let mut showcase = Vec::new();
    for (key, kind, metadata) in unlocks {
        let value = serde_json::from_str::<Value>(&metadata).unwrap_or_else(|_| json!({}));
        match kind.as_str() {
            "title" | "badge" => { identity.insert(kind, json!({"key": key, "metadata": value})); }
            _ => { showcase.push(json!({"key": key, "type": kind, "metadata": value})); }
        }
    }
    let profile: Option<(String, String)> = sqlx::query_as("SELECT profile_theme, showcase FROM user_profiles WHERE uuid=?")
        .bind(&uuid).fetch_optional(&state.db).await?;
    if let Some((theme, configured)) = profile {
        identity.insert("theme".into(), json!(theme));
        let configured = serde_json::from_str::<Value>(&configured).unwrap_or_else(|_| json!({}));
        identity.insert("showcase".into(), configured);
    }
    cosmetics.insert("equipped".into(), Value::Array(showcase.clone()));

    // Quest progress for this period: how many of the player's quests are done.
    let cfg = crate::progression::load_pool(&state.db).await?;
    let mut conn = state.db.acquire().await?;
    let mut quests = serde_json::Map::new();
    for (period, key) in [("daily", crate::routes::quests::current_daily_key()), ("weekly", crate::routes::quests::current_weekly_key())] {
        let ids = crate::progression::assigned_quest_ids(&mut conn, &cfg, &uuid, period, &key).await?;
        let (done, claimed): (i64, i64) = if ids.is_empty() {
            (0, 0)
        } else {
            sqlx::query_as(
                "SELECT COALESCE(SUM(completed), 0), COALESCE(SUM(claimed), 0) FROM user_quests
                 WHERE user_uuid = ? AND period_key = ? AND quest_id IN (SELECT value FROM json_each(?))",
            )
            .bind(&uuid)
            .bind(&key)
            .bind(serde_json::to_string(&ids)?)
            .fetch_one(&mut *conn)
            .await?
        };
        quests.insert(period.into(), json!({ "total": ids.len(), "completed": done, "claimed": claimed }));
    }
    drop(conn); // give the connection back before the pool is used again
    let rank: Option<(String, String, String, String)> =
        sqlx::query_as("SELECT primary_group, display, prefix, suffix FROM player_ranks WHERE server_id = ? AND uuid = ?")
            .bind(server.id)
            .bind(&uuid)
            .fetch_optional(&state.db)
            .await?;
    let online: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM server_online WHERE uuid = ?)").bind(&uuid).fetch_one(&state.db).await?;

    let title_glyph = crate::rank_glyphs::active_global_glyph(&state, levels.global_level).await?;
    Ok(Json(json!({
        "exists": true,
        "uuid": uuid,
        "name": name,
        "role": role,
        "joined": joined,
        "online": online,
        "global": { "level": levels.global_level, "xp": levels.global_xp, "next_level_xp": levels.next_level_xp, "current_level_xp": levels.current_level_xp, "progress_pct": levels.progress_pct, "title": levels.title, "title_glyph": title_glyph, "rank": levels.rank, "badges": levels.badges },
        "server": here.map(|s| json!({ "level": s.server_level, "xp": s.server_xp, "next_level_xp": s.next_level_xp, "current_level_xp": s.current_level_xp, "progress_pct": s.progress_pct, "rank_name": s.rank_name }))
            .unwrap_or(json!({ "level": 1, "xp": 0, "next_level_xp": 0, "current_level_xp": 0, "progress_pct": 0.0, "rank_name": null })),
        "guild": guild.map(|(id, name, tag, role, claims)| json!({ "id": id, "name": name, "tag": tag, "role": role, "claims": claims })),
        "balance": balance,
        "playtime_secs": playtime.unwrap_or(0),
        "server_playtime_secs": server_playtime.unwrap_or(0),
        "kills": kills.unwrap_or(0),
        "deaths": deaths.unwrap_or(0),
        "friends": friends,
        "achievements": achievements,
        "quests": quests,
        "rank": rank.map(|(primary, display, prefix, suffix)| json!({ "group": primary, "display": display, "prefix": prefix, "suffix": suffix })),
        "identity": identity,
        "cosmetics": cosmetics,
    })))
}

#[derive(Deserialize)]
pub struct XpPayload {
    pub operation_id: String,
    pub uuid: String,
    /// `global` or `server` (the calling server).
    #[serde(default = "global")]
    pub scope: String,
    /// Positive grants XP, negative takes it away.
    pub amount: i64,
    #[serde(default)]
    pub reason: String,
}
fn global() -> String {
    "global".into()
}

/// `addXP()`: grant (or remove) XP on behalf of a plugin. Idempotent on `operation_id`.
pub async fn add_xp(GameServer(server): GameServer, State(state): State<AppState>, Json(p): Json<XpPayload>) -> AppResult<Json<Value>> {
    let uuid = dashed_or_err(&p.uuid)?;
    if p.amount == 0 || p.amount.abs() > 10_000_000 {
        return Err(AppError::bad_request("amount must be between -10,000,000 and 10,000,000 and not zero"));
    }
    let (mut tx, previous) = begin_operation(&state, server.id, &p.operation_id).await?;
    if let Some(previous) = previous {
        return Ok(Json(previous));
    }
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE uuid = ?)").bind(&uuid).fetch_one(&mut *tx).await?;
    if !exists {
        return Err(AppError::not_found("player not found"));
    }
    let now = chrono::Utc::now().to_rfc3339();
    let (mode, amount) =
        if p.amount > 0 { (crate::progression::Mode::Add, p.amount) } else { (crate::progression::Mode::Remove, -p.amount) };
    let change = match p.scope.as_str() {
        "global" => crate::progression::adjust_global(&mut tx, &uuid, mode, amount, &now).await?,
        "server" => crate::progression::adjust_server(&mut tx, server.id, &uuid, mode, amount, &now).await?,
        _ => return Err(AppError::bad_request("scope must be global or server")),
    };
    if change.new_xp > change.old_xp {
        crate::routes::leveling::grant_rewards(&mut tx, &uuid, &now).await?;
    }
    let detail = format!(
        "{} XP {} → {} (level {} → {}) via {}: {}",
        p.scope,
        change.old_xp,
        change.new_xp,
        change.old_level,
        change.new_level,
        server.name,
        p.reason.chars().filter(|c| !c.is_control()).take(100).collect::<String>()
    );
    sqlx::query("INSERT INTO server_events (server_id, uuid, name, kind, detail, created_at) VALUES (?, ?, NULL, 'xp', ?, ?)")
        .bind(server.id)
        .bind(&uuid)
        .bind(detail)
        .bind(crate::db::now())
        .execute(&mut *tx)
        .await?;
    finish_operation(
        tx,
        server.id,
        &p.operation_id,
        json!({ "ok": true, "scope": p.scope, "xp": change.new_xp, "level": change.new_level, "previous_level": change.old_level }),
    )
    .await
}

#[derive(Deserialize)]
pub struct ObjectivePayload {
    pub operation_id: String,
    pub uuid: String,
    /// A quest id (`q_d_mine_coal`), or an action such as `block_broken:DIAMOND_ORE`.
    pub objective: String,
    /// Progress to add. `0` completes the quest outright.
    #[serde(default)]
    pub amount: i64,
}

/// `completeQuestObjective()`: move a player's quest forward from another plugin.
pub async fn quest_objective(
    GameServer(server): GameServer,
    State(state): State<AppState>,
    Json(p): Json<ObjectivePayload>,
) -> AppResult<Json<Value>> {
    let uuid = dashed_or_err(&p.uuid)?;
    if !(0..=1_000_000).contains(&p.amount) || p.objective.is_empty() || p.objective.len() > 120 {
        return Err(AppError::bad_request("invalid objective"));
    }
    let (mut tx, previous) = begin_operation(&state, server.id, &p.operation_id).await?;
    if let Some(previous) = previous {
        return Ok(Json(previous));
    }
    let now = crate::db::now();
    let quest: Option<(String, i64)> = sqlx::query_as("SELECT period, target_count FROM quests WHERE id = ? AND enabled = 1")
        .bind(&p.objective)
        .fetch_optional(&mut *tx)
        .await?;
    let advanced = if let Some((period, target)) = quest {
        let cfg = crate::progression::load(&mut tx).await?;
        let key = if period == "weekly" { crate::routes::quests::current_weekly_key() } else { crate::routes::quests::current_daily_key() };
        if !crate::progression::is_assigned(&mut tx, &cfg, &uuid, &period, &key, &p.objective).await? {
            return Err(AppError::bad_request("that quest isn't one of the player's quests this period"));
        }
        let add = if p.amount == 0 { target } else { p.amount };
        sqlx::query(
            "INSERT INTO user_quests (user_uuid, quest_id, period_key, progress, completed, claimed, updated_at) VALUES (?, ?, ?, ?, ?, 0, ?)
             ON CONFLICT (user_uuid, quest_id, period_key) DO UPDATE SET progress = progress + excluded.progress,
                completed = CASE WHEN progress + excluded.progress >= ? THEN 1 ELSE completed END, updated_at = excluded.updated_at",
        )
        .bind(&uuid).bind(&p.objective).bind(&key).bind(add).bind(add >= target).bind(&now).bind(target)
        .execute(&mut *tx)
        .await?;
        true
    } else {
        // Not a quest id: treat it as an action ("block_broken:STONE") like the plugin's own events.
        let before: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(progress), 0) FROM user_quests WHERE user_uuid = ?")
            .bind(&uuid)
            .fetch_one(&mut *tx)
            .await?;
        crate::routes::quests::advance_action_quests(&mut tx, &uuid, &p.objective, p.amount.max(1), &now).await?;
        let after: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(progress), 0) FROM user_quests WHERE user_uuid = ?")
            .bind(&uuid)
            .fetch_one(&mut *tx)
            .await?;
        after > before
    };
    finish_operation(tx, server.id, &p.operation_id, json!({ "ok": true, "advanced": advanced })).await
}

/// `getFriends()`: accepted friends with where they are playing.
pub async fn friends(GameServer(_): GameServer, State(state): State<AppState>, Json(p): Json<Who>) -> AppResult<Json<Value>> {
    let uuid = dashed_or_err(&p.uuid)?;
    let rows: Vec<(String, String, bool, Option<String>)> = sqlx::query_as(
        "SELECT u.uuid, u.username, EXISTS(SELECT 1 FROM server_online so WHERE so.uuid = u.uuid),
                (SELECT gs.name FROM server_online so JOIN game_servers gs ON gs.id = so.server_id WHERE so.uuid = u.uuid LIMIT 1)
         FROM friendships f JOIN users u ON u.uuid = (CASE WHEN f.user_uuid = ?1 THEN f.friend_uuid ELSE f.user_uuid END)
         WHERE f.status = 'accepted' AND (f.user_uuid = ?1 OR f.friend_uuid = ?1) ORDER BY 3 DESC, u.username COLLATE NOCASE LIMIT 500",
    )
    .bind(&uuid)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(
        json!({ "friends": rows.into_iter().map(|(uuid, name, online, playing_on)| json!({ "uuid": uuid, "name": name, "online": online, "playing_on": playing_on })).collect::<Vec<_>>() }),
    ))
}

#[derive(Deserialize)]
pub struct GuildQuery {
    /// Look a guild up by id, tag or name — or by one of its members.
    #[serde(default)]
    pub guild: String,
    #[serde(default)]
    pub member: String,
}

/// `getGuild()`.
pub async fn guild(GameServer(server): GameServer, State(state): State<AppState>, Json(q): Json<GuildQuery>) -> AppResult<Json<Value>> {
    let id: Option<String> = if !q.member.is_empty() {
        let uuid = dashed_or_err(&q.member)?;
        sqlx::query_scalar("SELECT g.id FROM guild_members gm JOIN guilds g ON g.id = gm.guild_id WHERE gm.uuid = ? AND g.instance_id = ?")
            .bind(uuid)
            .bind(&server.instance_id)
            .fetch_optional(&state.db)
            .await?
    } else {
        sqlx::query_scalar(
            "SELECT id FROM guilds WHERE instance_id = ?2 AND (id = ?1 OR tag = ?1 COLLATE NOCASE OR name = ?1 COLLATE NOCASE)",
        )
        .bind(q.guild.trim())
        .bind(&server.instance_id)
        .fetch_optional(&state.db)
        .await?
    };
    let Some(id) = id else { return Ok(Json(json!({ "exists": false }))) };
    let (name, tag, desc, leader, created): (String, String, String, String, String) =
        sqlx::query_as("SELECT name, tag, description, leader_uuid, created_at FROM guilds WHERE id = ?")
            .bind(&id)
            .fetch_one(&state.db)
            .await?;
    let members: Vec<(String, String, String)> =
        sqlx::query_as("SELECT uuid, name, role FROM guild_members WHERE guild_id = ? ORDER BY joined_at")
            .bind(&id)
            .fetch_all(&state.db)
            .await?;
    let claims: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM guild_claims WHERE guild_id = ? AND server_id = ?")
        .bind(&id)
        .bind(server.id)
        .fetch_one(&state.db)
        .await?;
    let balance: f64 = sqlx::query_scalar("SELECT COALESCE((SELECT balance FROM guild_wallets WHERE guild_id = ? AND server_id = ?), 0.0)")
        .bind(&id)
        .bind(server.id)
        .fetch_one(&state.db)
        .await?;
    Ok(Json(json!({
        "exists": true, "id": id, "name": name, "tag": tag, "description": desc, "leader": leader, "created": created,
        "claims": claims, "balance": balance,
        "members": members.into_iter().map(|(uuid, name, role)| json!({ "uuid": uuid, "name": name, "role": role })).collect::<Vec<_>>(),
    })))
}
