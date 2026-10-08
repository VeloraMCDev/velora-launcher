//! Guild claim rules: what guild leaders and officers let happen on their land.
//!
//! Visitors (players outside the guild) are held back by default exactly as before; a guild can open its land up (let visitors build,
//! use doors or open chests) or tighten it (no PVP, no mob spawning). Members are never affected by the visitor rules. Server admins
//! decide which rules guilds may change (`guild_flag_policy`); the rest stay at their defaults. The values ride along in the claim
//! index, so the game plugin enforces them with the same code it uses for admin claims.

use crate::state::RequestState as State;
use crate::auth::{AdminUser, AuthUser};
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use axum::extract::{Path};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeSet;

/// `(id, label, help, group, default)`. `true` means allowed. The defaults match how guild land has always behaved.
pub const GUILD_FLAGS: &[(&str, &str, &str, &str, bool)] = &[
    ("build", "Visitors can build", "Players outside your guild can place and break blocks and use buckets on your land.", "Visitors", false),
    ("interact", "Visitors can use doors & buttons", "Doors, trapdoors, buttons, levers, pressure plates and beds.", "Visitors", false),
    ("containers", "Visitors can open chests", "Chests, barrels, furnaces, hoppers, shulker boxes and similar.", "Visitors", false),
    ("entry", "Visitors can walk in", "Turn this off to keep everyone outside your guild off your land.", "Visitors", true),
    ("pvp", "Player vs player", "Players can hurt each other on your land. Off makes it a peaceful zone for everyone.", "Combat", true),
    ("mob_spawning", "Mobs spawn", "Mobs spawn naturally, from spawners and from eggs.", "Mobs", true),
    ("mob_griefing", "Mob griefing", "Endermen, ravagers and silverfish can change blocks.", "Mobs", false),
    ("explosions", "Explosions break blocks", "TNT, creepers and beds can destroy blocks. Risky: this includes visitors' TNT.", "World", false),
    ("fire_spread", "Fire spreads", "Fire can start, burn blocks and spread.", "World", false),
    ("fluid_flow", "Fluids flow in", "Water and lava from outside can flow onto your land.", "World", false),
];

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Policy {
    /// Rules guilds may change. `None` (never saved) means all of them.
    pub editable: Option<Vec<String>>,
}

impl Policy {
    fn allows(&self, id: &str) -> bool {
        self.editable.as_ref().is_none_or(|list| list.iter().any(|f| f == id))
    }
}

pub async fn policy(state: &AppState) -> AppResult<Policy> {
    crate::store::kv_get(state, "guild_flag_policy").await
}

/// Every rule with its value: the guild's choice where guilds may choose and it has one, otherwise the default.
pub fn resolve(stored: &str, policy: &Policy) -> serde_json::Map<String, Value> {
    let saved: serde_json::Map<String, Value> = serde_json::from_str(stored).unwrap_or_default();
    GUILD_FLAGS
        .iter()
        .map(|(id, .., default)| {
            let value = if policy.allows(id) { saved.get(*id).and_then(Value::as_bool).unwrap_or(*default) } else { *default };
            (id.to_string(), json!(value))
        })
        .collect()
}

fn catalog(policy: &Policy) -> Vec<Value> {
    GUILD_FLAGS
        .iter()
        .map(|(id, label, help, group, default)| json!({ "id": id, "label": label, "help": help, "group": group, "default": default, "editable": policy.allows(id) }))
        .collect()
}

async fn view(state: &AppState, guild_id: &str, can_edit: bool) -> AppResult<Value> {
    let stored: Option<String> = sqlx::query_scalar("SELECT claim_flags FROM guilds WHERE id = ?").bind(guild_id).fetch_optional(&state.db).await?;
    let stored = stored.ok_or_else(|| AppError::not_found("Guild not found"))?;
    let policy = policy(state).await?;
    Ok(json!({ "flags": resolve(&stored, &policy), "catalog": catalog(&policy), "can_edit": can_edit }))
}

pub async fn get(user: AuthUser, State(state): State<AppState>, Path(id): Path<String>) -> AppResult<Json<Value>> {
    let member: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM guild_members WHERE guild_id = ? AND uuid = ?)")
        .bind(&id)
        .bind(&user.uuid)
        .fetch_one(&state.db)
        .await?;
    if !member {
        return Err(AppError::forbidden("Only members can see their guild's land rules"));
    }
    let can_edit = super::guilds::guild_can(&state, &id, &user.uuid, "manage").await?;
    Ok(Json(view(&state, &id, can_edit).await?))
}

#[derive(Deserialize)]
pub struct FlagsBody {
    flags: serde_json::Map<String, Value>,
}

/// Validate and store changes to a guild's rules. The caller has already checked the player may manage the guild.
pub async fn change(state: &AppState, guild_id: &str, flags: &serde_json::Map<String, Value>) -> AppResult<()> {
    let policy = policy(state).await?;
    let stored: Option<String> = sqlx::query_scalar("SELECT claim_flags FROM guilds WHERE id = ?").bind(guild_id).fetch_optional(&state.db).await?;
    let mut merged: serde_json::Map<String, Value> = serde_json::from_str(&stored.ok_or_else(|| AppError::not_found("Guild not found"))?).unwrap_or_default();
    for (key, value) in flags {
        if !GUILD_FLAGS.iter().any(|f| f.0 == key) {
            return Err(AppError::bad_request(format!("Unknown rule \"{key}\"")));
        }
        if !policy.allows(key) {
            return Err(AppError::bad_request("The server admins manage that rule, so guilds can't change it"));
        }
        merged.insert(key.clone(), json!(value.as_bool().ok_or_else(|| AppError::bad_request("Rules are on or off"))?));
    }
    sqlx::query("UPDATE guilds SET claim_flags = ? WHERE id = ?").bind(Value::Object(merged).to_string()).bind(guild_id).execute(&state.db).await?;
    Ok(())
}

pub async fn put(user: AuthUser, State(state): State<AppState>, Path(id): Path<String>, Json(b): Json<FlagsBody>) -> AppResult<Json<Value>> {
    if !super::guilds::guild_can(&state, &id, &user.uuid, "manage").await? {
        return Err(AppError::forbidden("Only the leader, officers and roles that can manage the guild may change land rules"));
    }
    change(&state, &id, &b.flags).await?;
    Ok(Json(view(&state, &id, true).await?))
}

/// The rules as a game server shows them: `{"ok", "can_edit", "rules": [{id, label, help, group, value, default, editable}]}`.
pub async fn for_game(state: &AppState, guild_id: &str, can_edit: bool) -> AppResult<Value> {
    let v = view(state, guild_id, can_edit).await?;
    let rules: Vec<Value> = v["catalog"]
        .as_array()
        .map(|c| c.iter().map(|r| json!({ "id": r["id"], "label": r["label"], "group": r["group"], "default": r["default"], "editable": r["editable"], "value": v["flags"][r["id"].as_str().unwrap_or("")] })).collect())
        .unwrap_or_default();
    Ok(json!({ "ok": true, "can_edit": can_edit, "rules": rules }))
}

// ---------------------------------------------------------------------------
// Admin: which rules guilds may change
// ---------------------------------------------------------------------------

pub async fn admin_get(_: AdminUser, State(state): State<AppState>) -> AppResult<Json<Value>> {
    let policy = policy(&state).await?;
    Ok(Json(json!({ "catalog": catalog(&policy) })))
}

#[derive(Deserialize)]
pub struct PolicyBody {
    editable: Vec<String>,
}

pub async fn admin_put(_: AdminUser, State(state): State<AppState>, Json(b): Json<PolicyBody>) -> AppResult<Json<Value>> {
    let known: BTreeSet<&str> = GUILD_FLAGS.iter().map(|f| f.0).collect();
    if let Some(bad) = b.editable.iter().find(|f| !known.contains(f.as_str())) {
        return Err(AppError::bad_request(format!("Unknown rule \"{bad}\"")));
    }
    let policy = Policy { editable: Some(b.editable) };
    crate::store::kv_set(&state, "guild_flag_policy", &policy).await?;
    // Game servers re-read the claim index when this changes.
    sqlx::query("UPDATE kv SET value = CAST(value AS INTEGER) + 1 WHERE key = 'guild_rev'").execute(&state.db).await?;
    Ok(Json(json!({ "catalog": catalog(&policy) })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_keep_guild_land_protected() {
        let all = resolve("{}", &Policy::default());
        assert_eq!(all["build"], false);
        assert_eq!(all["entry"], true);
        assert_eq!(all["explosions"], false);
        assert_eq!(all.len(), GUILD_FLAGS.len());
    }

    #[test]
    fn locked_rules_ignore_what_a_guild_saved() {
        let policy = Policy { editable: Some(vec!["pvp".into()]) };
        let r = resolve(r#"{"pvp":false,"build":true}"#, &policy);
        assert_eq!(r["pvp"], false, "pvp is the guild's call");
        assert_eq!(r["build"], false, "build is locked to its default");
    }
}
