//! Admin claims: named, described regions that belong to the server rather than a guild (spawn, shops, arenas…).
//!
//! They live beside guild claims and reach game servers inside the same claim index, so protection, the map and the
//! "you are entering…" banner all treat them the same way. Only players with `scopenet.claims.bypass` may build in them,
//! and guilds can't claim over them.

use crate::state::RequestState as State;
use crate::auth::AdminUser;
use crate::error::{AppError, AppResult};
use crate::routes::servers::GameServer;
use crate::state::AppState;
use axum::extract::{Path};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

const MAX_CHUNKS_PER_CALL: i64 = 4096;

/// What an admin claim can allow or forbid. `true` means *allowed*. The defaults keep a fresh claim behaving the way every admin
/// claim always has: protected from building and griefing, otherwise ordinary. Enforced by the game server plugin.
pub const FLAGS: &[(&str, &str, &str, &str, bool)] = &[
    ("build", "Player break & build", "Place and break blocks, and use buckets. Staff with scopenet.claims.bypass always can.", "Protection", false),
    ("interact", "Player interaction", "Use doors, buttons, levers, pressure plates, beds and other blocks.", "Protection", false),
    ("containers", "Open containers", "Open chests, barrels, furnaces, hoppers, shulker boxes and similar.", "Protection", false),
    ("entry", "Entry", "Players can walk in. Off keeps everyone but staff out.", "Protection", true),
    ("pvp", "Player PVP", "Players can hurt each other.", "Combat", true),
    ("player_damage", "Player damage", "Players take damage. Off makes them invincible.", "Combat", true),
    ("animal_damage", "Hurt animals & villagers", "Players can hurt passive mobs and villagers.", "Combat", true),
    ("mob_spawning", "Mob spawning", "Mobs spawn naturally, from spawners and from eggs.", "Mobs", true),
    ("mob_griefing", "Mob griefing", "Endermen, ravagers, silverfish and others can change blocks.", "Mobs", false),
    ("explosions", "Explosions", "TNT, creepers and beds can destroy blocks.", "World", false),
    ("fire_spread", "Fire spread", "Fire can start, burn blocks and spread.", "World", false),
    ("fluid_flow", "Fluid flow", "Water and lava can flow into the region.", "World", false),
    ("fly", "Flying", "Players can fly (creative flight, /fly and other flight). Off grounds everyone but staff.", "Movement", true),
    ("ender_pearls", "Ender pearls & chorus fruit", "Players can teleport into the region this way.", "Movement", true),
    ("hunger", "Hunger", "Players lose food while inside.", "Players", true),
    ("item_drop", "Dropping items", "Players can drop items on the ground.", "Players", true),
];

/// Every flag with its value: what is stored, or the default for anything not set.
pub fn resolve_flags(stored: &str) -> serde_json::Map<String, Value> {
    let saved: serde_json::Map<String, Value> = serde_json::from_str(stored).unwrap_or_default();
    FLAGS.iter().map(|(id, .., default)| (id.to_string(), json!(saved.get(*id).and_then(Value::as_bool).unwrap_or(*default)))).collect()
}

fn clean_flags(input: &serde_json::Map<String, Value>) -> AppResult<String> {
    let mut out = serde_json::Map::new();
    for (key, value) in input {
        if !FLAGS.iter().any(|f| f.0 == key) {
            return Err(AppError::bad_request(format!("Unknown flag \"{key}\"")));
        }
        out.insert(key.clone(), json!(value.as_bool().ok_or_else(|| AppError::bad_request("Flags are true or false"))?));
    }
    Ok(Value::Object(out).to_string())
}

fn catalog() -> Vec<Value> {
    FLAGS.iter().map(|(id, label, help, group, default)| json!({ "id": id, "label": label, "help": help, "group": group, "default": default })).collect()
}

fn clean_name(raw: &str) -> AppResult<String> {
    let n = raw.trim();
    if n.is_empty() || n.chars().count() > 32 || n.chars().any(|c| c.is_control()) {
        return Err(AppError::bad_request("Name must be 1 to 32 characters"));
    }
    Ok(n.to_string())
}

fn clean_description(raw: &str) -> AppResult<String> {
    let d = raw.trim();
    if d.chars().count() > 300 || d.chars().any(|c| c.is_control() && c != '\n') {
        return Err(AppError::bad_request("Description can be up to 300 characters"));
    }
    Ok(d.to_string())
}

fn clean_color(raw: &str) -> AppResult<String> {
    let c = raw.trim();
    let ok = c.len() == 7 && c.starts_with('#') && c[1..].chars().all(|x| x.is_ascii_hexdigit());
    if !ok {
        return Err(AppError::bad_request("Colour must look like #f59e0b"));
    }
    Ok(c.to_lowercase())
}

async fn list_for(state: &AppState, server_id: i64) -> AppResult<Value> {
    let rows: Vec<(String, String, String, String, String, String)> =
        sqlx::query_as("SELECT id, name, description, color, created_at, flags FROM admin_claims WHERE server_id = ? ORDER BY name")
            .bind(server_id)
            .fetch_all(&state.db)
            .await?;
    let mut out = Vec::new();
    for (id, name, description, color, created_at, flags) in rows {
        let (chunks, min_x, max_x, min_z, max_z): (i64, Option<i64>, Option<i64>, Option<i64>, Option<i64>) = sqlx::query_as(
            "SELECT COUNT(*), MIN(chunk_x), MAX(chunk_x), MIN(chunk_z), MAX(chunk_z) FROM admin_claim_chunks WHERE claim_id = ?",
        )
        .bind(&id)
        .fetch_one(&state.db)
        .await?;
        let dims: Vec<String> = sqlx::query_scalar("SELECT DISTINCT dimension FROM admin_claim_chunks WHERE claim_id = ?")
            .bind(&id)
            .fetch_all(&state.db)
            .await?;
        out.push(json!({
            "id": id, "name": name, "description": description, "color": color, "created_at": created_at,
            "chunks": chunks, "dimensions": dims, "flags": resolve_flags(&flags),
            "bounds": min_x.map(|x| json!({ "min_x": x, "max_x": max_x, "min_z": min_z, "max_z": max_z })),
        }));
    }
    Ok(json!({ "claims": out, "flag_catalog": catalog() }))
}

async fn create_claim(state: &AppState, server_id: i64, name: &str, description: &str, color: &str) -> AppResult<String> {
    let (name, description, color) = (clean_name(name)?, clean_description(description)?, clean_color(color)?);
    let id = format!("ac_{}", uuid::Uuid::new_v4().simple());
    let res = sqlx::query("INSERT INTO admin_claims (id, server_id, name, description, color, created_at) VALUES (?, ?, ?, ?, ?, ?)")
        .bind(&id)
        .bind(server_id)
        .bind(&name)
        .bind(&description)
        .bind(&color)
        .bind(crate::db::now())
        .execute(&state.db)
        .await;
    match res {
        Ok(_) => Ok(id),
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
            Err(AppError::bad_request("An admin claim with that name already exists"))
        }
        Err(e) => Err(e.into()),
    }
}

/// Claim a rectangle of chunks (inclusive corners). Chunks that already belong to a guild or another admin claim are skipped.
async fn add_area(state: &AppState, claim_id: &str, dim: &str, (x1, z1, x2, z2): (i64, i64, i64, i64)) -> AppResult<Value> {
    if [x1, z1, x2, z2].iter().any(|c| !(-1_875_000..=1_875_000).contains(c)) {
        return Err(AppError::bad_request("Claim coordinates are outside the Minecraft world"));
    }
    let (lx, hx, lz, hz) = (x1.min(x2), x1.max(x2), z1.min(z2), z1.max(z2));
    if (hx - lx + 1) * (hz - lz + 1) > MAX_CHUNKS_PER_CALL {
        return Err(AppError::bad_request(format!("That is more than {MAX_CHUNKS_PER_CALL} chunks at once; claim it in pieces")));
    }
    let server_id: i64 = sqlx::query_scalar("SELECT server_id FROM admin_claims WHERE id = ?")
        .bind(claim_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::not_found("Admin claim not found"))?;
    let (mut added, mut skipped) = (0u64, 0u64);
    let mut tx = state.db.begin().await?;
    for x in lx..=hx {
        for z in lz..=hz {
            let taken: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM guild_claims WHERE server_id = ? AND dimension = ? AND chunk_x = ? AND chunk_z = ?)",
            )
            .bind(server_id)
            .bind(dim)
            .bind(x)
            .bind(z)
            .fetch_one(&mut *tx)
            .await?;
            let done = if taken {
                0
            } else {
                sqlx::query(
                    "INSERT OR IGNORE INTO admin_claim_chunks (claim_id, server_id, dimension, chunk_x, chunk_z) VALUES (?, ?, ?, ?, ?)",
                )
                .bind(claim_id)
                .bind(server_id)
                .bind(dim)
                .bind(x)
                .bind(z)
                .execute(&mut *tx)
                .await?
                .rows_affected()
            };
            if done > 0 {
                added += 1
            } else {
                skipped += 1
            }
        }
    }
    tx.commit().await?;
    Ok(json!({ "ok": true, "added": added, "skipped": skipped }))
}

async fn remove_area(state: &AppState, claim_id: &str, dim: &str, (x1, z1, x2, z2): (i64, i64, i64, i64)) -> AppResult<Value> {
    let done = sqlx::query(
        "DELETE FROM admin_claim_chunks WHERE claim_id = ? AND dimension = ? AND chunk_x BETWEEN ? AND ? AND chunk_z BETWEEN ? AND ?",
    )
    .bind(claim_id)
    .bind(dim)
    .bind(x1.min(x2))
    .bind(x1.max(x2))
    .bind(z1.min(z2))
    .bind(z1.max(z2))
    .execute(&state.db)
    .await?;
    Ok(json!({ "ok": true, "removed": done.rows_affected() }))
}

// ---------------------------------------------------------------------------
// Admin panel
// ---------------------------------------------------------------------------

pub async fn list(_: AdminUser, State(state): State<AppState>, Path(server_id): Path<i64>) -> AppResult<Json<Value>> {
    Ok(Json(list_for(&state, server_id).await?))
}

#[derive(Deserialize)]
pub struct CreatePayload {
    name: String,
    #[serde(default)]
    description: String,
    color: Option<String>,
}

pub async fn create(
    _: AdminUser,
    State(state): State<AppState>,
    Path(server_id): Path<i64>,
    Json(p): Json<CreatePayload>,
) -> AppResult<Json<Value>> {
    crate::routes::servers::get_server(&state, server_id).await?;
    let id = create_claim(&state, server_id, &p.name, &p.description, p.color.as_deref().unwrap_or("#f59e0b")).await?;
    Ok(Json(json!({ "ok": true, "id": id })))
}

#[derive(Deserialize)]
pub struct UpdatePayload {
    name: Option<String>,
    description: Option<String>,
    color: Option<String>,
    /// Flag id to allowed. Only the flags sent change; the rest keep their value.
    flags: Option<serde_json::Map<String, Value>>,
}

pub async fn update(
    _: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(p): Json<UpdatePayload>,
) -> AppResult<Json<Value>> {
    if let Some(n) = &p.name {
        let n = clean_name(n)?;
        let res = sqlx::query("UPDATE admin_claims SET name = ? WHERE id = ?").bind(n).bind(&id).execute(&state.db).await;
        match res {
            Ok(_) => {}
            Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
                return Err(AppError::bad_request("An admin claim with that name already exists"))
            }
            Err(e) => return Err(e.into()),
        }
    }
    if let Some(d) = &p.description {
        sqlx::query("UPDATE admin_claims SET description = ? WHERE id = ?")
            .bind(clean_description(d)?)
            .bind(&id)
            .execute(&state.db)
            .await?;
    }
    if let Some(c) = &p.color {
        sqlx::query("UPDATE admin_claims SET color = ? WHERE id = ?").bind(clean_color(c)?).bind(&id).execute(&state.db).await?;
    }
    if let Some(changes) = &p.flags {
        clean_flags(changes)?;
        let stored: Option<String> = sqlx::query_scalar("SELECT flags FROM admin_claims WHERE id = ?").bind(&id).fetch_optional(&state.db).await?;
        let mut merged: serde_json::Map<String, Value> = stored.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        merged.extend(changes.clone());
        sqlx::query("UPDATE admin_claims SET flags = ? WHERE id = ?").bind(clean_flags(&merged)?).bind(&id).execute(&state.db).await?;
    }
    Ok(Json(json!({ "ok": true })))
}

pub async fn delete(_: AdminUser, State(state): State<AppState>, Path(id): Path<String>) -> AppResult<Json<Value>> {
    sqlx::query("DELETE FROM admin_claim_chunks WHERE claim_id = ?").bind(&id).execute(&state.db).await?;
    sqlx::query("DELETE FROM admin_claims WHERE id = ?").bind(&id).execute(&state.db).await?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
pub struct AreaPayload {
    #[serde(default = "overworld")]
    dimension: String,
    x1: i64,
    z1: i64,
    x2: i64,
    z2: i64,
    /// Coordinates are blocks (default) or chunks.
    #[serde(default)]
    chunks: bool,
    #[serde(default)]
    remove: bool,
}

fn overworld() -> String {
    "minecraft:overworld".into()
}

pub async fn area(
    _: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(p): Json<AreaPayload>,
) -> AppResult<Json<Value>> {
    let c = |v: i64| if p.chunks { v } else { v.div_euclid(16) };
    let rect = (c(p.x1), c(p.z1), c(p.x2), c(p.z2));
    Ok(Json(if p.remove { remove_area(&state, &id, &p.dimension, rect).await? } else { add_area(&state, &id, &p.dimension, rect).await? }))
}

// ---------------------------------------------------------------------------
// Game server (in-game `/adminclaim`)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct GamePayload {
    action: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    value: String,
    #[serde(default = "overworld")]
    dimension: String,
    #[serde(default)]
    chunk_x: i64,
    #[serde(default)]
    chunk_z: i64,
    /// A radius in chunks around (chunk_x, chunk_z) for `add`/`remove`.
    #[serde(default)]
    radius: i64,
}

async fn by_name(state: &AppState, server_id: i64, name: &str) -> AppResult<String> {
    sqlx::query_scalar("SELECT id FROM admin_claims WHERE server_id = ? AND name = ?")
        .bind(server_id)
        .bind(name.trim())
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::bad_request(format!("No admin claim called \"{}\"", name.trim())))
}

/// One endpoint for every `/adminclaim` sub-command; the plugin checks the player's permission before calling it.
pub async fn game(GameServer(server): GameServer, State(state): State<AppState>, Json(p): Json<GamePayload>) -> AppResult<Json<Value>> {
    let r = p.radius.clamp(0, 32);
    let rect = (p.chunk_x - r, p.chunk_z - r, p.chunk_x + r, p.chunk_z + r);
    match p.action.as_str() {
        "list" => Ok(Json(list_for(&state, server.id).await?)),
        "create" => {
            let id = create_claim(&state, server.id, &p.name, &p.value, "#f59e0b").await?;
            Ok(Json(add_area(&state, &id, &p.dimension, rect).await.map(|mut v| {
                v["id"] = json!(id);
                v
            })?))
        }
        "add" => {
            let id = by_name(&state, server.id, &p.name).await?;
            Ok(Json(add_area(&state, &id, &p.dimension, rect).await?))
        }
        "remove" => {
            // Without a name: whichever claim holds this chunk.
            let id = if p.name.trim().is_empty() {
                sqlx::query_scalar(
                    "SELECT claim_id FROM admin_claim_chunks WHERE server_id = ? AND dimension = ? AND chunk_x = ? AND chunk_z = ?",
                )
                .bind(server.id)
                .bind(&p.dimension)
                .bind(p.chunk_x)
                .bind(p.chunk_z)
                .fetch_optional(&state.db)
                .await?
                .ok_or_else(|| AppError::bad_request("You are not standing in an admin claim"))?
            } else {
                by_name(&state, server.id, &p.name).await?
            };
            Ok(Json(remove_area(&state, &id, &p.dimension, rect).await?))
        }
        "delete" => {
            let id = by_name(&state, server.id, &p.name).await?;
            sqlx::query("DELETE FROM admin_claim_chunks WHERE claim_id = ?").bind(&id).execute(&state.db).await?;
            sqlx::query("DELETE FROM admin_claims WHERE id = ?").bind(&id).execute(&state.db).await?;
            Ok(Json(json!({ "ok": true })))
        }
        "rename" | "describe" | "color" => {
            let id = by_name(&state, server.id, &p.name).await?;
            let payload = match p.action.as_str() {
                "rename" => UpdatePayload { name: Some(p.value), description: None, color: None, flags: None },
                "describe" => UpdatePayload { name: None, description: Some(p.value), color: None, flags: None },
                _ => UpdatePayload { name: None, description: None, color: Some(p.value), flags: None },
            };
            if let Some(n) = &payload.name {
                let n = clean_name(n)?;
                let res = sqlx::query("UPDATE admin_claims SET name = ? WHERE id = ?").bind(n).bind(&id).execute(&state.db).await;
                if let Err(sqlx::Error::Database(e)) = &res {
                    if e.is_unique_violation() {
                        return Err(AppError::bad_request("An admin claim with that name already exists"));
                    }
                }
                res?;
            }
            if let Some(d) = &payload.description {
                sqlx::query("UPDATE admin_claims SET description = ? WHERE id = ?")
                    .bind(clean_description(d)?)
                    .bind(&id)
                    .execute(&state.db)
                    .await?;
            }
            if let Some(c) = &payload.color {
                sqlx::query("UPDATE admin_claims SET color = ? WHERE id = ?").bind(clean_color(c)?).bind(&id).execute(&state.db).await?;
            }
            Ok(Json(json!({ "ok": true })))
        }
        // `value` is "<flag> <on|off>"; with no value, the claim's flags are listed.
        "flag" => {
            let id = by_name(&state, server.id, &p.name).await?;
            let mut parts = p.value.split_whitespace();
            let (Some(flag), Some(state_word)) = (parts.next(), parts.next()) else {
                let stored: String = sqlx::query_scalar("SELECT flags FROM admin_claims WHERE id = ?").bind(&id).fetch_one(&state.db).await?;
                return Ok(Json(json!({ "ok": true, "flags": resolve_flags(&stored), "flag_catalog": catalog() })));
            };
            let allowed = match state_word.to_ascii_lowercase().as_str() {
                "on" | "true" | "allow" | "yes" => true,
                "off" | "false" | "deny" | "no" => false,
                _ => return Err(AppError::bad_request("Use on or off")),
            };
            let mut changes = serde_json::Map::new();
            changes.insert(flag.to_ascii_lowercase().replace('-', "_"), json!(allowed));
            clean_flags(&changes)?;
            let stored: String = sqlx::query_scalar("SELECT flags FROM admin_claims WHERE id = ?").bind(&id).fetch_one(&state.db).await?;
            let mut merged: serde_json::Map<String, Value> = serde_json::from_str(&stored).unwrap_or_default();
            merged.extend(changes);
            sqlx::query("UPDATE admin_claims SET flags = ? WHERE id = ?").bind(clean_flags(&merged)?).bind(&id).execute(&state.db).await?;
            Ok(Json(json!({ "ok": true })))
        }
        _ => Err(AppError::bad_request("Unknown action")),
    }
}
