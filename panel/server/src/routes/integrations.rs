//! Plugin integrations: what game servers report about the plugins they run
//! (LuckPerms ranks, WorldGuard regions, Spark performance, CoreProtect
//! activity…), the LuckPerms ↔ panel group mapping, and the queue of events
//! (level-ups, guild joins, achievements) handed back to game servers.

use crate::auth::AdminUser;
use crate::error::{AppError, AppResult};
use crate::routes::servers::{get_server, GameServer};
use crate::state::AppState;
use crate::state::RequestState as State;
use axum::extract::Path;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::SqliteConnection;
use std::collections::HashSet;

const MAX_REPORT_BYTES: usize = 1024 * 1024;
const KNOWN: &[&str] = &["luckperms", "placeholderapi", "vault", "coreprotect", "worldguard", "spark", "compat"];

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

/// How LuckPerms groups and panel groups follow each other. LuckPerms stays
/// authoritative for Minecraft permissions; this only moves *membership* of
/// groups that an admin has explicitly mapped.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct IntegrationSettings {
    /// `off` (show ranks only), `game_to_panel`, `panel_to_game` or `both` (adds in both directions, never removes).
    pub luckperms_sync: String,
}

impl Default for IntegrationSettings {
    fn default() -> Self {
        Self { luckperms_sync: "off".into() }
    }
}

pub async fn load_settings(conn: &mut SqliteConnection) -> AppResult<IntegrationSettings> {
    let raw: Option<String> = sqlx::query_scalar("SELECT value FROM kv WHERE key = 'integrations'").fetch_optional(&mut *conn).await?;
    Ok(raw.and_then(|r| serde_json::from_str(&r).ok()).unwrap_or_default())
}

pub async fn get_settings(_: AdminUser, State(state): State<AppState>) -> AppResult<Json<Value>> {
    let mut conn = state.db.acquire().await?;
    Ok(Json(json!({ "settings": load_settings(&mut conn).await? })))
}

pub async fn put_settings(_: AdminUser, State(state): State<AppState>, Json(s): Json<IntegrationSettings>) -> AppResult<Json<Value>> {
    if !matches!(s.luckperms_sync.as_str(), "off" | "game_to_panel" | "panel_to_game" | "both") {
        return Err(AppError::bad_request("luckperms_sync must be off, game_to_panel, panel_to_game or both"));
    }
    crate::store::kv_set(&state, "integrations", &s).await?;
    Ok(Json(json!({ "settings": s })))
}

#[derive(Deserialize)]
pub struct GroupMapping {
    /// The LuckPerms group this panel group follows. Empty clears the mapping.
    pub luckperms_group: String,
}

pub async fn set_group_mapping(
    _: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(m): Json<GroupMapping>,
) -> AppResult<Json<Value>> {
    let lp = m.luckperms_group.trim();
    if lp.len() > 64 || !lp.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.')) {
        return Err(AppError::bad_request("LuckPerms group names are letters, digits, _ - and ."));
    }
    let done = if state.instance_id.is_some() {
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM platform.groups WHERE id=?)").bind(id).fetch_one(&state.db).await?;
        if !exists {
            return Err(AppError::not_found("group not found"));
        }
        sqlx::query("INSERT INTO experience_group_links(group_id,luckperms_group) VALUES(?,?) ON CONFLICT(group_id) DO UPDATE SET luckperms_group=excluded.luckperms_group")
                .bind(id).bind(lp.to_lowercase()).execute(&state.db).await?
    } else {
        sqlx::query("UPDATE groups SET luckperms_group=? WHERE id=?").bind(lp.to_lowercase()).bind(id).execute(&state.platform_db).await?
    };
    if done.rows_affected() == 0 {
        return Err(AppError::not_found("group not found"));
    }
    Ok(Json(json!({ "ok": true })))
}

// ---------------------------------------------------------------------------
// LuckPerms planning (pure, so it can be tested without a database)
// ---------------------------------------------------------------------------

#[derive(Debug, Default, PartialEq)]
pub struct Plan {
    pub panel_add: Vec<i64>,
    pub panel_remove: Vec<i64>,
    pub game_add: Vec<String>,
    pub game_remove: Vec<String>,
}

/// `mapped`: (panel group id, LuckPerms group). `member_of`: the panel groups the player is in.
pub fn plan(mode: &str, lp_groups: &HashSet<String>, mapped: &[(i64, String)], member_of: &HashSet<i64>) -> Plan {
    let mut p = Plan::default();
    let (to_panel, to_game, remove) = match mode {
        "game_to_panel" => (true, false, true),
        "panel_to_game" => (false, true, true),
        "both" => (true, true, false),
        _ => return p,
    };
    for (gid, lp) in mapped {
        let in_game = lp_groups.contains(lp);
        let in_panel = member_of.contains(gid);
        match (in_game, in_panel) {
            (true, false) if to_panel => p.panel_add.push(*gid),
            (false, true) if to_panel && remove && mode == "game_to_panel" => p.panel_remove.push(*gid),
            (false, true) if to_game => p.game_add.push(lp.clone()),
            (true, false) if to_game && remove && mode == "panel_to_game" => p.game_remove.push(lp.clone()),
            _ => {}
        }
    }
    p
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct LpPlayer {
    uuid: String,
    primary: String,
    display: String,
    prefix: String,
    suffix: String,
    weight: i64,
    groups: Vec<String>,
    permissions: Vec<String>,
}

fn clean(s: &str, max: usize) -> String {
    s.chars().filter(|c| !c.is_control() || *c == '\u{a7}').take(max).collect()
}

/// Store reported ranks and work out group changes for the players in the report.
async fn apply_luckperms(
    conn: &mut SqliteConnection,
    platform_db: Option<&sqlx::SqlitePool>,
    server_id: i64,
    data: &Value,
) -> AppResult<Value> {
    let settings = load_settings(conn).await?;
    let players: Vec<LpPlayer> = serde_json::from_value(data.get("players").cloned().unwrap_or(json!([]))).unwrap_or_default();
    let mapped: Vec<(i64, String)> =
        sqlx::query_as("SELECT id, luckperms_group FROM groups WHERE luckperms_group <> ''").fetch_all(&mut *conn).await?;
    // Milestones: this page's links plus those set on level rewards, highest level first.
    let level_groups: Vec<(i64, String)> = {
        let mut v: Vec<(i64, String)> = crate::routes::luckperms::level_links_conn(&mut *conn).await?;
        v.sort_by(|a, b| b.0.cmp(&a.0));
        v
    };
    let now = crate::db::now();
    let mut assign = Vec::new();
    let mut level_assign = Vec::new();
    for p in players.into_iter().take(2000) {
        let Some(uuid) = crate::yggdrasil::dashed(&p.uuid) else { continue };
        let groups: Vec<String> = p.groups.iter().map(|g| clean(g, 64).to_lowercase()).filter(|g| !g.is_empty()).take(64).collect();
        let perms: Vec<String> = p.permissions.iter().map(|x| clean(x, 100)).take(40).collect();
        sqlx::query(
            "INSERT INTO player_ranks (server_id, uuid, primary_group, display, prefix, suffix, groups, permissions, weight, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT (server_id, uuid) DO UPDATE SET primary_group = excluded.primary_group, display = excluded.display,
                prefix = excluded.prefix, suffix = excluded.suffix, groups = excluded.groups, permissions = excluded.permissions,
                weight = excluded.weight, updated_at = excluded.updated_at",
        )
        .bind(server_id)
        .bind(&uuid)
        .bind(clean(&p.primary, 64).to_lowercase())
        .bind(clean(&p.display, 64))
        .bind(clean(&p.prefix, 64))
        .bind(clean(&p.suffix, 64))
        .bind(serde_json::to_string(&groups)?)
        .bind(serde_json::to_string(&perms)?)
        .bind(p.weight.clamp(-1_000_000, 1_000_000))
        .bind(&now)
        .execute(&mut *conn)
        .await?;
        if !level_groups.is_empty() {
            let xp: i64 = sqlx::query_scalar("SELECT global_xp FROM user_levels WHERE uuid=?")
                .bind(&uuid)
                .fetch_optional(&mut *conn)
                .await?
                .unwrap_or(0);
            let level = crate::progression::load(conn).await?.level_from_xp(xp).0;
            let target = level_groups.iter().find(|(required, _)| *required <= level).map(|(_, group)| group.to_lowercase());
            let mut add = Vec::new();
            let mut remove = Vec::new();
            for (_, group) in &level_groups {
                let key = group.to_lowercase();
                if target.as_deref() == Some(key.as_str()) {
                    if !groups.contains(&key) {
                        add.push(key);
                    }
                } else if groups.contains(&key) {
                    remove.push(key);
                }
            }
            if !add.is_empty() || !remove.is_empty() {
                level_assign.push(json!({"uuid":uuid,"add":add,"remove":remove}));
            }
        }
        if settings.luckperms_sync == "off" || mapped.is_empty() {
            continue;
        }
        let user_id: Option<i64> =
            sqlx::query_scalar("SELECT id FROM users WHERE uuid = ? AND status = 'active'").bind(&uuid).fetch_optional(&mut *conn).await?;
        let Some(user_id) = user_id else { continue };
        let member_of: HashSet<i64> = sqlx::query_scalar("SELECT group_id FROM user_groups WHERE user_id = ?")
            .bind(user_id)
            .fetch_all(&mut *conn)
            .await?
            .into_iter()
            .collect();
        let lp_set: HashSet<String> = groups.iter().cloned().collect();
        let plan = plan(&settings.luckperms_sync, &lp_set, &mapped, &member_of);
        for gid in &plan.panel_add {
            let query = sqlx::query("INSERT OR IGNORE INTO user_groups(user_id,group_id) VALUES(?,?)").bind(user_id).bind(gid);
            if let Some(pool) = platform_db {
                query.execute(pool).await?;
            } else {
                query.execute(&mut *conn).await?;
            }
        }
        for gid in &plan.panel_remove {
            let query = sqlx::query("DELETE FROM user_groups WHERE user_id=? AND group_id=?").bind(user_id).bind(gid);
            if let Some(pool) = platform_db {
                query.execute(pool).await?;
            } else {
                query.execute(&mut *conn).await?;
            }
        }
        if !plan.game_add.is_empty() || !plan.game_remove.is_empty() {
            assign.push(json!({ "uuid": uuid, "add": plan.game_add, "remove": plan.game_remove }));
        }
    }
    Ok(json!({ "mode": settings.luckperms_sync, "assign": assign, "level_assign": level_assign }))
}

// ---------------------------------------------------------------------------
// Game server reports
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct Report {
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub data: Value,
}

pub async fn server_report(GameServer(server): GameServer, State(state): State<AppState>, Json(r): Json<Report>) -> AppResult<Json<Value>> {
    let name = r.name.trim().to_lowercase();
    if !KNOWN.contains(&name.as_str()) {
        return Err(AppError::bad_request("unknown integration"));
    }
    let raw = serde_json::to_string(&r.data)?;
    if raw.len() > MAX_REPORT_BYTES {
        return Err(AppError::bad_request("report is too large"));
    }
    let mut tx = state.db.begin().await?;
    let mut config = json!({});
    if name == "luckperms" {
        // A light report is the plugin's quick poll for instructions: no snapshot, nothing to store.
        let light = r.data.get("light").and_then(Value::as_bool).unwrap_or(false);
        let commands = crate::routes::luckperms::sync_commands(&mut tx, server.id, &r.data).await?;
        if light {
            tx.commit().await?;
            return Ok(Json(json!({ "ok": true, "config": { "commands": commands } })));
        }
        config = apply_luckperms(&mut tx, state.instance_id.as_ref().map(|_| &state.platform_db), server.id, &r.data).await?;
        config["commands"] = json!(commands);
    }
    // Player-specific rank rows are the stored form of the LuckPerms report; keep the group snapshot.
    let stored = if name == "luckperms" {
        json!({
            "groups": r.data.get("groups").cloned().unwrap_or(json!([])),
            "can_manage": r.data.get("can_manage").and_then(Value::as_bool).unwrap_or(false),
            "players_reported": r.data.get("players").and_then(|p| p.as_array()).map_or(0, |a| a.len()),
        })
        .to_string()
    } else {
        raw
    };
    sqlx::query(
        "INSERT INTO server_integrations (server_id, name, version, data, updated_at) VALUES (?, ?, ?, ?, ?)
         ON CONFLICT (server_id, name) DO UPDATE SET version = excluded.version, data = excluded.data, updated_at = excluded.updated_at",
    )
    .bind(server.id)
    .bind(&name)
    .bind(clean(&r.version, 40))
    .bind(stored)
    .bind(crate::db::now())
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(json!({ "ok": true, "config": config })))
}

// ---------------------------------------------------------------------------
// Admin view
// ---------------------------------------------------------------------------

pub async fn admin_server(_: AdminUser, State(state): State<AppState>, Path(id): Path<i64>) -> AppResult<Json<Value>> {
    get_server(&state, id).await?;
    let rows: Vec<(String, String, String, String)> =
        sqlx::query_as("SELECT name, version, data, updated_at FROM server_integrations WHERE server_id = ? ORDER BY name")
            .bind(id)
            .fetch_all(&state.db)
            .await?;
    let list: Vec<Value> = rows
        .into_iter()
        .map(|(name, version, data, updated_at)| json!({ "name": name, "version": version, "data": serde_json::from_str::<Value>(&data).unwrap_or(Value::Null), "updated_at": updated_at }))
        .collect();
    let mut conn = state.db.acquire().await?;
    let settings = load_settings(&mut conn).await?;
    let ranks: Vec<(String, String, String, String, String, String, String, i64)> = sqlx::query_as(
        "SELECT r.uuid, COALESCE(u.username, r.uuid), r.primary_group, r.display, r.prefix, r.groups, r.permissions, r.weight
         FROM player_ranks r LEFT JOIN users u ON u.uuid = r.uuid WHERE r.server_id = ? ORDER BY r.weight DESC, 2 LIMIT 200",
    )
    .bind(id)
    .fetch_all(&mut *conn)
    .await?;
    Ok(Json(json!({
        "integrations": list,
        "settings": settings,
        "ranks": ranks.into_iter().map(|(uuid, name, primary, display, prefix, groups, perms, weight)| json!({
            "uuid": uuid, "name": name, "primary": primary, "display": display, "prefix": prefix, "weight": weight,
            "groups": serde_json::from_str::<Value>(&groups).unwrap_or(json!([])),
            "permissions": serde_json::from_str::<Value>(&perms).unwrap_or(json!([])),
        })).collect::<Vec<_>>(),
    })))
}

// ---------------------------------------------------------------------------
// Notifications: events the panel hands to game servers with their next sync
// ---------------------------------------------------------------------------

/// Queue an event for every game server the player is online on (or for one
/// specific server). Plugins turn these into Bukkit events.
pub async fn notify(conn: &mut SqliteConnection, server_id: Option<i64>, kind: &str, uuid: &str, payload: Value) -> AppResult<()> {
    let servers: Vec<i64> = match server_id {
        Some(id) => vec![id],
        None => sqlx::query_scalar("SELECT server_id FROM server_online WHERE uuid = ?").bind(uuid).fetch_all(&mut *conn).await?,
    };
    for id in servers {
        sqlx::query("INSERT INTO server_notifications (server_id, kind, uuid, payload, created_at) VALUES (?, ?, ?, ?, ?)")
            .bind(id)
            .bind(kind)
            .bind(uuid)
            .bind(payload.to_string())
            .bind(crate::db::now())
            .execute(&mut *conn)
            .await?;
    }
    Ok(())
}

/// Take (and delete) what is waiting for this server. Oldest first, bounded.
pub async fn take_notifications(conn: &mut SqliteConnection, server_id: i64) -> AppResult<Vec<Value>> {
    let rows: Vec<(i64, String, String, String, String)> =
        sqlx::query_as("SELECT id, kind, uuid, payload, created_at FROM server_notifications WHERE server_id = ? ORDER BY id LIMIT 100")
            .bind(server_id)
            .fetch_all(&mut *conn)
            .await?;
    if let Some(last) = rows.last().map(|r| r.0) {
        sqlx::query("DELETE FROM server_notifications WHERE server_id = ? AND id <= ?")
            .bind(server_id)
            .bind(last)
            .execute(&mut *conn)
            .await?;
    }
    // Stale entries (a server that was offline for days) aren't worth replaying.
    sqlx::query("DELETE FROM server_notifications WHERE created_at < ?")
        .bind((chrono::Utc::now() - chrono::Duration::hours(6)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
        .execute(&mut *conn)
        .await?;
    Ok(rows.into_iter().map(|(_, kind, uuid, payload, at)| json!({ "kind": kind, "uuid": uuid, "data": serde_json::from_str::<Value>(&payload).unwrap_or(Value::Null), "at": at })).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(v: &[&str]) -> HashSet<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn sync_modes_only_move_mapped_groups() {
        let mapped = vec![(1, "vip".to_string()), (2, "mod".to_string())];
        let none: HashSet<i64> = HashSet::new();
        let in_vip: HashSet<i64> = [1].into();

        assert_eq!(plan("off", &set(&["vip"]), &mapped, &none), Plan::default());
        // LuckPerms is followed, in both directions, for mapped groups only.
        let p = plan("game_to_panel", &set(&["vip", "default"]), &mapped, &none);
        assert_eq!((p.panel_add, p.panel_remove), (vec![1], vec![]));
        let p = plan("game_to_panel", &set(&["default"]), &mapped, &in_vip);
        assert_eq!((p.panel_add, p.panel_remove), (vec![], vec![1]));
        // The panel leads the other way round.
        let p = plan("panel_to_game", &set(&[]), &mapped, &in_vip);
        assert_eq!((p.game_add, p.game_remove), (vec!["vip".to_string()], vec![]));
        let p = plan("panel_to_game", &set(&["mod"]), &mapped, &none);
        assert_eq!(p.game_remove, vec!["mod".to_string()]);
        // "both" only ever adds.
        let p = plan("both", &set(&["mod"]), &mapped, &in_vip);
        assert_eq!((p.panel_add, p.panel_remove, p.game_add, p.game_remove), (vec![2], vec![], vec!["vip".to_string()], vec![]));
    }
}
