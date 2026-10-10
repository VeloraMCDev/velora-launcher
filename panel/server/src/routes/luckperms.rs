//! LuckPerms manager: create and edit groups (display name, weight, prefix, suffix, parents, permissions), link them to
//! level milestones and panel groups, and move players between groups.
//!
//! The game server owns LuckPerms, so every change is an *instruction* queued in `luckperms_commands`. The Paper plugin
//! collects them every few seconds, runs them through the LuckPerms API and reports back, so an admin sees whether each
//! one worked. What the page shows is the last snapshot the plugin reported.

use crate::state::RequestState as State;
use crate::auth::AdminUser;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use axum::extract::{Path, Query};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::SqliteConnection;

const MAX_PENDING_PER_SERVER: i64 = 500;
/// A queued instruction nobody collected within this long is given up on, so a dead server can't hoard a backlog.
const EXPIRE_SECS: i64 = 3600;
/// Collected but never acknowledged: offered again after this long.
const RETAKE_SECS: i64 = 120;

fn ago(secs: i64) -> String {
    (chrono::Utc::now() - chrono::Duration::seconds(secs)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn group_name(raw: &str) -> AppResult<String> {
    let name = raw.trim().to_lowercase();
    if name.is_empty() || name.len() > 64 || !name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.')) {
        return Err(AppError::bad_request("Group names are 1-64 letters, digits, _ - and ."));
    }
    Ok(name)
}

fn node_key(raw: &str) -> AppResult<String> {
    let key = raw.trim();
    if key.is_empty() || key.len() > 200 || !key.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '*' | ':' | '/')) {
        return Err(AppError::bad_request("A permission is letters, digits and . _ - * : / (for example scopenet.casino.use)"));
    }
    Ok(key.to_string())
}

/// Free text for display names, prefixes and suffixes: `&` and `§` codes are allowed, control characters are not.
fn text(raw: &str, max: usize, what: &str) -> AppResult<String> {
    let t: String = raw.chars().filter(|c| !c.is_control() || *c == '\u{a7}').collect();
    if t.chars().count() > max {
        return Err(AppError::bad_request(format!("{what} can be at most {max} characters")));
    }
    Ok(t)
}

fn context_value(raw: &Option<String>, what: &str) -> AppResult<String> {
    let v = raw.as_deref().unwrap_or("").trim().to_lowercase();
    if v.len() > 64 || !v.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.')) {
        return Err(AppError::bad_request(format!("{what} can only use letters, digits, _ - and .")));
    }
    Ok(v)
}

// ---------------------------------------------------------------------------
// The queue
// ---------------------------------------------------------------------------

/// The servers an instruction goes to: the one asked for, or every server that has reported LuckPerms.
async fn targets(state: &AppState, requested: Option<i64>) -> AppResult<Vec<i64>> {
    if let Some(id) = requested {
        crate::routes::servers::get_server(state, id).await?;
        return Ok(vec![id]);
    }
    let ids: Vec<i64> = sqlx::query_scalar("SELECT server_id FROM server_integrations WHERE name = 'luckperms' ORDER BY server_id")
        .fetch_all(&state.db)
        .await?;
    if ids.is_empty() {
        return Err(AppError::bad_request("No game server has reported LuckPerms yet. Install the Velora plugin next to LuckPerms and wait a minute."));
    }
    Ok(ids)
}

async fn enqueue(state: &AppState, admin: &AdminUser, server: Option<i64>, summary: &str, op: Value) -> AppResult<Value> {
    let servers = targets(state, server).await?;
    let now = crate::db::now();
    let mut ids = Vec::new();
    for sid in &servers {
        let pending: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM luckperms_commands WHERE server_id = ? AND done_at IS NULL")
            .bind(sid)
            .fetch_one(&state.db)
            .await?;
        if pending >= MAX_PENDING_PER_SERVER {
            return Err(AppError::bad_request("Too many instructions are waiting for that server. Is it online?"));
        }
        let id = sqlx::query("INSERT INTO luckperms_commands (server_id, op, summary, actor, created_at) VALUES (?, ?, ?, ?, ?)")
            .bind(sid)
            .bind(op.to_string())
            .bind(summary)
            .bind(&admin.0.username)
            .bind(&now)
            .execute(&state.db)
            .await?
            .last_insert_rowid();
        ids.push(id);
    }
    Ok(json!({ "ok": true, "queued": ids, "servers": servers }))
}

/// Called by the game server's report: record what it finished, expire dead instructions, hand out what is waiting.
pub async fn sync_commands(conn: &mut SqliteConnection, server_id: i64, data: &Value) -> AppResult<Vec<Value>> {
    let now = crate::db::now();
    if let Some(results) = data.get("command_results").and_then(Value::as_array) {
        for r in results.iter().take(200) {
            let Some(id) = r.get("id").and_then(Value::as_i64) else { continue };
            let ok = r.get("ok").and_then(Value::as_bool).unwrap_or(false);
            let error: String = r.get("error").and_then(Value::as_str).unwrap_or("").chars().filter(|c| !c.is_control()).take(300).collect();
            sqlx::query("UPDATE luckperms_commands SET done_at = ?, ok = ?, error = ? WHERE id = ? AND server_id = ? AND done_at IS NULL")
                .bind(&now)
                .bind(ok)
                .bind(if ok || error.is_empty() { None } else { Some(error) })
                .bind(id)
                .bind(server_id)
                .execute(&mut *conn)
                .await?;
        }
    }
    sqlx::query("UPDATE luckperms_commands SET done_at = ?, ok = 0, error = 'Expired: the game server did not collect it in time' WHERE done_at IS NULL AND created_at < ?")
        .bind(&now)
        .bind(ago(EXPIRE_SECS))
        .execute(&mut *conn)
        .await?;
    sqlx::query("DELETE FROM luckperms_commands WHERE done_at IS NOT NULL AND done_at < ?").bind(ago(7 * 86400)).execute(&mut *conn).await?;

    let rows: Vec<(i64, String)> = sqlx::query_as(
        "SELECT id, op FROM luckperms_commands WHERE server_id = ? AND done_at IS NULL AND (taken_at IS NULL OR taken_at < ?) ORDER BY id LIMIT 50",
    )
    .bind(server_id)
    .bind(ago(RETAKE_SECS))
    .fetch_all(&mut *conn)
    .await?;
    let mut out = Vec::new();
    for (id, op) in rows {
        sqlx::query("UPDATE luckperms_commands SET taken_at = ? WHERE id = ?").bind(&now).bind(id).execute(&mut *conn).await?;
        out.push(json!({ "id": id, "op": serde_json::from_str::<Value>(&op).unwrap_or(Value::Null) }));
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Overview
// ---------------------------------------------------------------------------

#[derive(Deserialize, Default)]
pub struct ServerQuery {
    server_id: Option<i64>,
}

/// Everything the manager page needs in one go.
pub async fn overview(_: AdminUser, State(state): State<AppState>, Query(q): Query<ServerQuery>) -> AppResult<Json<Value>> {
    let servers: Vec<(i64, String)> = sqlx::query_as("SELECT id, name FROM game_servers ORDER BY name COLLATE NOCASE").fetch_all(&state.db).await?;
    let reports: Vec<(i64, String, String, String)> =
        sqlx::query_as("SELECT server_id, version, data, updated_at FROM server_integrations WHERE name = 'luckperms'").fetch_all(&state.db).await?;
    let server_list: Vec<Value> = servers
        .iter()
        .map(|(id, name)| {
            let r = reports.iter().find(|r| r.0 == *id);
            json!({ "id": id, "name": name, "luckperms": r.is_some(), "version": r.map(|r| r.1.clone()), "reported_at": r.map(|r| r.3.clone()) })
        })
        .collect();
    // Default to the server that reported most recently.
    let chosen = q.server_id.or_else(|| reports.iter().max_by_key(|r| r.3.clone()).map(|r| r.0)).or_else(|| servers.first().map(|s| s.0));
    let snapshot = chosen.and_then(|id| reports.iter().find(|r| r.0 == id));
    let data: Value = snapshot.and_then(|r| serde_json::from_str(&r.2).ok()).unwrap_or(json!({}));

    let panel_groups: Vec<(i64, String, String, String, i64)> = sqlx::query_as(
        "SELECT g.id, g.name, g.color, g.luckperms_group, (SELECT COUNT(*) FROM user_groups ug WHERE ug.group_id = g.id) FROM groups g ORDER BY g.name COLLATE NOCASE",
    )
    .fetch_all(&state.db)
    .await?;
    let links = level_links(&state).await?;
    let commands: Vec<(i64, i64, String, String, String, Option<String>, Option<i64>, Option<String>)> = sqlx::query_as(
        "SELECT id, server_id, summary, actor, created_at, done_at, ok, error FROM luckperms_commands WHERE (? IS NULL OR server_id = ?) ORDER BY id DESC LIMIT 40",
    )
    .bind(chosen)
    .bind(chosen)
    .fetch_all(&state.db)
    .await?;
    let mut conn = state.db.acquire().await?;
    let settings = crate::routes::integrations::load_settings(&mut conn).await?;
    Ok(Json(json!({
        "servers": server_list,
        "server_id": chosen,
        "connected": snapshot.is_some(),
        "reported_at": snapshot.map(|r| r.3.clone()),
        "plugin_version": snapshot.map(|r| r.1.clone()),
        "can_manage": data.get("can_manage").and_then(Value::as_bool).unwrap_or(false),
        // An older plugin build never sends the flag at all: it cannot take instructions, whatever its config says.
        "manager_supported": data.get("can_manage").is_some(),
        "groups": data.get("groups").cloned().unwrap_or(json!([])),
        "panel_groups": panel_groups.into_iter().map(|(id, name, color, lp, members)| json!({ "id": id, "name": name, "color": color, "luckperms_group": lp, "members": members })).collect::<Vec<_>>(),
        "level_links": links,
        "sync_mode": settings.luckperms_sync,
        "commands": commands.into_iter().map(|(id, server_id, summary, actor, created, done, ok, error)| json!({
            "id": id, "server_id": server_id, "summary": summary, "actor": actor, "created_at": created, "done_at": done,
            "status": if done.is_none() { "pending" } else if ok == Some(1) { "done" } else { "failed" }, "error": error })).collect::<Vec<_>>(),
    })))
}

// ---------------------------------------------------------------------------
// Groups
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct PermissionIn {
    permission: String,
    #[serde(default = "yes")]
    value: bool,
    #[serde(default)]
    server: Option<String>,
    #[serde(default)]
    world: Option<String>,
}

fn yes() -> bool {
    true
}

fn permission_op(group: &str, p: &PermissionIn) -> AppResult<Value> {
    Ok(json!({ "kind": "perm_set", "group": group, "permission": node_key(&p.permission)?, "value": p.value,
               "server": context_value(&p.server, "Server context")?, "world": context_value(&p.world, "World context")? }))
}

#[derive(Deserialize)]
pub struct CreateGroup {
    name: String,
    #[serde(default)]
    server_id: Option<i64>,
    #[serde(default)]
    display: String,
    #[serde(default)]
    weight: i64,
    #[serde(default)]
    prefix: String,
    #[serde(default)]
    suffix: String,
    #[serde(default)]
    parents: Vec<String>,
    #[serde(default)]
    permissions: Vec<PermissionIn>,
}

pub async fn create_group(admin: AdminUser, State(state): State<AppState>, Json(b): Json<CreateGroup>) -> AppResult<Json<Value>> {
    let name = group_name(&b.name)?;
    if b.permissions.len() > 300 || b.parents.len() > 30 {
        return Err(AppError::bad_request("That is too many permissions or parents for one request"));
    }
    let parents = b.parents.iter().map(|p| group_name(p)).collect::<AppResult<Vec<_>>>()?;
    let mut ops = vec![json!({ "kind": "group_create", "group": name })];
    ops.push(json!({ "kind": "group_update", "group": name, "display": text(&b.display, 64, "The display name")?,
                     "weight": b.weight.clamp(-100_000, 100_000), "prefix": text(&b.prefix, 64, "The prefix")?, "suffix": text(&b.suffix, 64, "The suffix")? }));
    for parent in parents {
        ops.push(json!({ "kind": "parent_add", "group": name, "parent": parent }));
    }
    for p in &b.permissions {
        ops.push(permission_op(&name, p)?);
    }
    // One instruction so the plugin applies the whole group in order and the log stays readable.
    enqueue(&state, &admin, b.server_id, &format!("Create group {name}"), json!({ "kind": "batch", "ops": ops })).await.map(Json)
}

#[derive(Deserialize)]
pub struct UpdateGroup {
    #[serde(default)]
    server_id: Option<i64>,
    display: Option<String>,
    weight: Option<i64>,
    prefix: Option<String>,
    suffix: Option<String>,
}

pub async fn update_group(admin: AdminUser, State(state): State<AppState>, Path(name): Path<String>, Json(b): Json<UpdateGroup>) -> AppResult<Json<Value>> {
    let name = group_name(&name)?;
    let mut op = json!({ "kind": "group_update", "group": name });
    if let Some(v) = &b.display {
        op["display"] = json!(text(v, 64, "The display name")?);
    }
    if let Some(v) = b.weight {
        op["weight"] = json!(v.clamp(-100_000, 100_000));
    }
    if let Some(v) = &b.prefix {
        op["prefix"] = json!(text(v, 64, "The prefix")?);
    }
    if let Some(v) = &b.suffix {
        op["suffix"] = json!(text(v, 64, "The suffix")?);
    }
    if op.as_object().is_some_and(|o| o.len() == 2) {
        return Err(AppError::bad_request("Nothing to change"));
    }
    enqueue(&state, &admin, b.server_id, &format!("Update group {name}"), op).await.map(Json)
}

pub async fn delete_group(admin: AdminUser, State(state): State<AppState>, Path(name): Path<String>, Query(q): Query<ServerQuery>) -> AppResult<Json<Value>> {
    let name = group_name(&name)?;
    if name == "default" {
        return Err(AppError::bad_request("LuckPerms always needs its default group"));
    }
    // Links to a deleted group would keep trying to hand it out.
    sqlx::query("DELETE FROM luckperms_level_links WHERE group_name = ?").bind(&name).execute(&state.db).await?;
    sqlx::query("UPDATE groups SET luckperms_group = '' WHERE luckperms_group = ?").bind(&name).execute(&state.db).await?;
    enqueue(&state, &admin, q.server_id, &format!("Delete group {name}"), json!({ "kind": "group_delete", "group": name })).await.map(Json)
}

#[derive(Deserialize)]
pub struct PermissionBody {
    #[serde(default)]
    server_id: Option<i64>,
    #[serde(flatten)]
    node: PermissionIn,
}

pub async fn set_permission(admin: AdminUser, State(state): State<AppState>, Path(name): Path<String>, Json(b): Json<PermissionBody>) -> AppResult<Json<Value>> {
    let name = group_name(&name)?;
    let op = permission_op(&name, &b.node)?;
    let summary = format!("{} {} for {name}", if b.node.value { "Allow" } else { "Deny" }, b.node.permission.trim());
    enqueue(&state, &admin, b.server_id, &summary, op).await.map(Json)
}

#[derive(Deserialize)]
pub struct UnsetBody {
    #[serde(default)]
    server_id: Option<i64>,
    permission: String,
    #[serde(default)]
    server: Option<String>,
    #[serde(default)]
    world: Option<String>,
}

pub async fn unset_permission(admin: AdminUser, State(state): State<AppState>, Path(name): Path<String>, Json(b): Json<UnsetBody>) -> AppResult<Json<Value>> {
    let name = group_name(&name)?;
    let key = node_key(&b.permission)?;
    let op = json!({ "kind": "perm_unset", "group": name, "permission": key,
                     "server": context_value(&b.server, "Server context")?, "world": context_value(&b.world, "World context")? });
    enqueue(&state, &admin, b.server_id, &format!("Remove {key} from {name}"), op).await.map(Json)
}

#[derive(Deserialize)]
pub struct ParentBody {
    #[serde(default)]
    server_id: Option<i64>,
    parent: String,
}

pub async fn add_parent(admin: AdminUser, State(state): State<AppState>, Path(name): Path<String>, Json(b): Json<ParentBody>) -> AppResult<Json<Value>> {
    let (name, parent) = (group_name(&name)?, group_name(&b.parent)?);
    if name == parent {
        return Err(AppError::bad_request("A group cannot inherit from itself"));
    }
    enqueue(&state, &admin, b.server_id, &format!("{name} inherits {parent}"), json!({ "kind": "parent_add", "group": name, "parent": parent })).await.map(Json)
}

pub async fn remove_parent(admin: AdminUser, State(state): State<AppState>, Path((name, parent)): Path<(String, String)>, Query(q): Query<ServerQuery>) -> AppResult<Json<Value>> {
    let (name, parent) = (group_name(&name)?, group_name(&parent)?);
    enqueue(&state, &admin, q.server_id, &format!("{name} no longer inherits {parent}"), json!({ "kind": "parent_remove", "group": name, "parent": parent })).await.map(Json)
}

// ---------------------------------------------------------------------------
// Level milestones
// ---------------------------------------------------------------------------

/// Milestones from this page plus the ones set on a level reward (the older place for it). This page wins for the same level.
pub async fn level_links(state: &AppState) -> AppResult<Vec<Value>> {
    let mut conn = state.db.acquire().await?;
    Ok(merged_links(&mut conn).await?.into_iter().map(|(level, group, source)| json!({ "level": level, "group": group, "source": source })).collect())
}

/// The same milestones as `(level, group)`, for the sync that hands groups out.
pub async fn level_links_conn(conn: &mut SqliteConnection) -> AppResult<Vec<(i64, String)>> {
    Ok(merged_links(conn).await?.into_iter().map(|(level, group, _)| (level, group)).collect())
}

async fn merged_links(conn: &mut SqliteConnection) -> AppResult<Vec<(i64, String, &'static str)>> {
    let own: Vec<(i64, String)> = sqlx::query_as("SELECT level, group_name FROM luckperms_level_links ORDER BY level").fetch_all(&mut *conn).await?;
    let legacy: Vec<(i64, String)> = sqlx::query_as(
        "SELECT level_req, lower(json_extract(reward_data, '$.luckperms_group')) FROM level_rewards
         WHERE level_type = 'global' AND reward_type = 'title' AND json_extract(reward_data, '$.luckperms_group') <> '' ORDER BY level_req",
    )
    .fetch_all(&mut *conn)
    .await?;
    let mut all: std::collections::BTreeMap<i64, (String, &'static str)> = legacy.into_iter().map(|(l, g)| (l, (g, "reward"))).collect();
    all.extend(own.into_iter().map(|(l, g)| (l, (g, "manager"))));
    Ok(all.into_iter().map(|(level, (group, source))| (level, group, source)).collect())
}

#[derive(Deserialize)]
pub struct LevelLink {
    level: i64,
    group: String,
}

#[derive(Deserialize)]
pub struct LevelLinks {
    links: Vec<LevelLink>,
}

/// Replace the manager's milestone list. Players are moved at their next report: they get the group of the highest milestone they
/// have reached, and lose the other milestone groups.
pub async fn put_level_links(_: AdminUser, State(state): State<AppState>, Json(b): Json<LevelLinks>) -> AppResult<Json<Value>> {
    if b.links.len() > 100 {
        return Err(AppError::bad_request("At most 100 milestones"));
    }
    let mut seen = std::collections::HashSet::new();
    let mut clean = Vec::new();
    for l in &b.links {
        if !(1..=10_000).contains(&l.level) || !seen.insert(l.level) {
            return Err(AppError::bad_request("Each milestone needs its own level between 1 and 10000"));
        }
        clean.push((l.level, group_name(&l.group)?));
    }
    let mut tx = state.db.begin().await?;
    sqlx::query("DELETE FROM luckperms_level_links").execute(&mut *tx).await?;
    let now = crate::db::now();
    for (level, group) in &clean {
        sqlx::query("INSERT INTO luckperms_level_links (level, group_name, created_at) VALUES (?, ?, ?)").bind(level).bind(group).bind(&now).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(Json(json!({ "ok": true, "level_links": level_links(&state).await? })))
}

// ---------------------------------------------------------------------------
// Players
// ---------------------------------------------------------------------------

#[derive(Deserialize, Default)]
pub struct PlayerQuery {
    #[serde(default)]
    q: String,
}

/// Panel players with the last rank a game server reported for them.
pub async fn players(_: AdminUser, State(state): State<AppState>, Query(q): Query<PlayerQuery>) -> AppResult<Json<Value>> {
    let like = format!("%{}%", q.q.trim().replace('%', "").replace('_', "\\_"));
    let rows: Vec<(String, String, Option<String>, Option<String>, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT u.uuid, u.username,
                (SELECT r.primary_group FROM player_ranks r WHERE r.uuid = u.uuid ORDER BY r.updated_at DESC LIMIT 1),
                (SELECT r.groups FROM player_ranks r WHERE r.uuid = u.uuid ORDER BY r.updated_at DESC LIMIT 1),
                (SELECT r.prefix FROM player_ranks r WHERE r.uuid = u.uuid ORDER BY r.updated_at DESC LIMIT 1),
                (SELECT r.updated_at FROM player_ranks r WHERE r.uuid = u.uuid ORDER BY r.updated_at DESC LIMIT 1)
         FROM users u WHERE u.status = 'active' AND u.username LIKE ? ESCAPE '\\' ORDER BY u.username COLLATE NOCASE LIMIT 60",
    )
    .bind(like)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(json!({ "players": rows.into_iter().map(|(uuid, name, primary, groups, prefix, seen)| json!({
        "uuid": uuid, "name": name, "primary": primary, "prefix": prefix, "last_reported": seen,
        "groups": groups.and_then(|g| serde_json::from_str::<Value>(&g).ok()).unwrap_or(json!([])),
    })).collect::<Vec<_>>() })))
}

#[derive(Deserialize)]
pub struct MovePlayer {
    #[serde(default)]
    server_id: Option<i64>,
    #[serde(default)]
    add: Vec<String>,
    #[serde(default)]
    remove: Vec<String>,
}

/// Add a player to groups and/or take them out of others. A "move" is a remove plus an add in one request.
pub async fn move_player(admin: AdminUser, State(state): State<AppState>, Path(uuid): Path<String>, Json(b): Json<MovePlayer>) -> AppResult<Json<Value>> {
    let uuid = crate::yggdrasil::dashed(&uuid).ok_or_else(|| AppError::bad_request("invalid player UUID"))?;
    let name: Option<String> = sqlx::query_scalar("SELECT username FROM users WHERE uuid = ?").bind(&uuid).fetch_optional(&state.db).await?;
    let name = name.ok_or_else(|| AppError::not_found("Player not found"))?;
    if b.add.is_empty() && b.remove.is_empty() {
        return Err(AppError::bad_request("Pick a group to add or remove"));
    }
    let add = b.add.iter().map(|g| group_name(g)).collect::<AppResult<Vec<_>>>()?;
    let remove = b.remove.iter().map(|g| group_name(g)).collect::<AppResult<Vec<_>>>()?;
    let summary = match (add.is_empty(), remove.is_empty()) {
        (false, false) => format!("Move {name}: {} → {}", remove.join(", "), add.join(", ")),
        (false, true) => format!("Add {name} to {}", add.join(", ")),
        _ => format!("Remove {name} from {}", remove.join(", ")),
    };
    enqueue(&state, &admin, b.server_id, &summary, json!({ "kind": "user_groups", "uuid": uuid, "add": add, "remove": remove })).await.map(Json)
}

#[derive(Deserialize)]
pub struct BulkMove {
    #[serde(default)]
    server_id: Option<i64>,
    from: String,
    to: String,
    uuids: Vec<String>,
}

/// Move several players from one group to another at once.
pub async fn bulk_move(admin: AdminUser, State(state): State<AppState>, Json(b): Json<BulkMove>) -> AppResult<Json<Value>> {
    if b.uuids.is_empty() || b.uuids.len() > 100 {
        return Err(AppError::bad_request("Pick between 1 and 100 players"));
    }
    let (from, to) = (group_name(&b.from)?, group_name(&b.to)?);
    let mut queued = 0;
    for raw in &b.uuids {
        let uuid = crate::yggdrasil::dashed(raw).ok_or_else(|| AppError::bad_request("invalid player UUID"))?;
        enqueue(&state, &admin, b.server_id, &format!("Move {uuid}: {from} → {to}"), json!({ "kind": "user_groups", "uuid": uuid, "add": [to], "remove": [from] })).await?;
        queued += 1;
    }
    Ok(Json(json!({ "ok": true, "players": queued })))
}

// ---------------------------------------------------------------------------
// Log
// ---------------------------------------------------------------------------

pub async fn retry_command(_: AdminUser, State(state): State<AppState>, Path(id): Path<i64>) -> AppResult<Json<Value>> {
    let done = sqlx::query("UPDATE luckperms_commands SET done_at = NULL, ok = NULL, error = NULL, taken_at = NULL, created_at = ? WHERE id = ? AND ok = 0")
        .bind(crate::db::now())
        .bind(id)
        .execute(&state.db)
        .await?;
    if done.rows_affected() == 0 {
        return Err(AppError::not_found("Only failed instructions can be retried"));
    }
    Ok(Json(json!({ "ok": true })))
}

pub async fn clear_commands(_: AdminUser, State(state): State<AppState>) -> AppResult<Json<Value>> {
    let done = sqlx::query("DELETE FROM luckperms_commands WHERE done_at IS NOT NULL").execute(&state.db).await?;
    Ok(Json(json!({ "ok": true, "deleted": done.rows_affected() })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_nodes_are_checked() {
        assert_eq!(group_name(" VIP.Gold ").unwrap(), "vip.gold");
        assert!(group_name("has space").is_err());
        assert!(group_name("").is_err());
        assert!(node_key("velora.command.faction.*").is_ok());
        assert!(node_key("bad node").is_err());
        assert!(node_key("semi;colon").is_err());
        assert!(text("&c[Admin] ", 64, "x").is_ok());
        assert!(text(&"x".repeat(65), 64, "x").is_err());
    }
}
