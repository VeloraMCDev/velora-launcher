//! Rewards beyond XP and titles.
//!
//! An admin attaches a *bundle* of actions to a quest, an achievement or a rank milestone (a level reward). When a
//! player earns one, the source goes into `reward_queue`; [`process_queue`] then applies each action: money, claim
//! chunks and badges happen in the panel, while items, permissions, groups, commands and messages become
//! `reward_deliveries` that the player's game server collects (once the player is online) and acknowledges.

use crate::error::{AppError, AppResult};
use crate::state::AppState;
use serde_json::{json, Map, Value};
use sqlx::SqliteConnection;

pub const MAX_ACTIONS: usize = 20;
const SOURCES: [&str; 4] = ["quest", "achievement", "level_reward", "event"];

use crate::progression::AutoMoney;

pub fn check_source(kind: &str) -> AppResult<()> {
    if SOURCES.contains(&kind) {
        Ok(())
    } else {
        Err(AppError::bad_request("unknown reward source"))
    }
}

fn text(a: &Map<String, Value>, key: &str, max: usize) -> String {
    a.get(key).and_then(Value::as_str).map(|s| s.trim().chars().filter(|c| !c.is_control()).take(max).collect()).unwrap_or_default()
}

fn int(a: &Map<String, Value>, key: &str) -> Option<i64> {
    a.get(key).and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64)))
}

fn word(value: &str, extra: &str) -> bool {
    !value.is_empty() && value.chars().all(|c| c.is_ascii_alphanumeric() || extra.contains(c))
}

/// Validate and normalise an admin's list of actions. Unknown fields are dropped, so nothing unexpected is stored.
pub fn validate(actions: &Value) -> AppResult<Vec<Value>> {
    let list = actions.as_array().ok_or_else(|| AppError::bad_request("actions must be a list"))?;
    if list.len() > MAX_ACTIONS {
        return Err(AppError::bad_request(format!("at most {MAX_ACTIONS} rewards per bundle")));
    }
    let mut out = Vec::new();
    for raw in list {
        let a = raw.as_object().ok_or_else(|| AppError::bad_request("each reward must be an object"))?;
        let kind = text(a, "type", 24);
        let server = int(a, "server_id").filter(|s| *s > 0);
        let mut v = Map::new();
        v.insert("type".into(), json!(kind));
        if let Some(s) = server {
            v.insert("server_id".into(), json!(s));
        }
        match kind.as_str() {
            "money" => {
                let amount = a.get("amount").and_then(Value::as_f64).unwrap_or(0.0);
                if !amount.is_finite() || amount <= 0.0 || amount > 1e9 {
                    return Err(AppError::bad_request("money must be more than 0"));
                }
                v.insert("amount".into(), json!((amount * 100.0).round() / 100.0));
            }
            "item" => {
                let mut item = text(a, "item", 100).to_lowercase();
                if !item.contains(':') {
                    item = format!("minecraft:{item}");
                }
                let (ns, path) = item.split_once(':').unwrap_or(("", ""));
                if !word(ns, "_.-") || !word(path, "_.-/") {
                    return Err(AppError::bad_request("item must look like minecraft:diamond"));
                }
                let amount = int(a, "amount").unwrap_or(1);
                if !(1..=6400).contains(&amount) {
                    return Err(AppError::bad_request("item amount must be 1 to 6400"));
                }
                v.insert("item".into(), json!(item));
                v.insert("amount".into(), json!(amount));
            }
            "custom_item" => {
                let id = text(a, "custom", 32).to_lowercase();
                let amount = int(a, "amount").unwrap_or(1);
                if !word(&id, "_-") || !(1..=6400).contains(&amount) {
                    return Err(AppError::bad_request("Custom item needs an id and an amount from 1 to 6400"));
                }
                v.insert("custom".into(), json!(id));
                v.insert("amount".into(), json!(amount));
            }
            "permission" => {
                let node = text(a, "node", 100);
                if !word(&node, "_.*-:") {
                    return Err(AppError::bad_request("permission node has unusual characters"));
                }
                let minutes = int(a, "minutes").unwrap_or(0);
                if !(0..=5_256_000).contains(&minutes) {
                    return Err(AppError::bad_request("permission duration is out of range"));
                }
                v.insert("node".into(), json!(node));
                v.insert("value".into(), json!(a.get("value").and_then(Value::as_bool).unwrap_or(true)));
                v.insert("minutes".into(), json!(minutes));
            }
            "group" => {
                let group = text(a, "group", 64);
                if !word(&group, "_.-") {
                    return Err(AppError::bad_request("group name has unusual characters"));
                }
                v.insert("group".into(), json!(group));
            }
            "unlock" => {
                let key = text(a, "key", 80);
                if !word(&key, "_:-.") {
                    return Err(AppError::bad_request("choose a cosmetic from the list"));
                }
                v.insert("key".into(), json!(key));
            }
            "claim_chunks" => {
                let amount = int(a, "amount").unwrap_or(0);
                if !(1..=10_000).contains(&amount) {
                    return Err(AppError::bad_request("claim chunks must be 1 to 10000"));
                }
                v.insert("amount".into(), json!(amount));
            }
            "badge" => {
                let badge = text(a, "badge", 40);
                if badge.is_empty() {
                    return Err(AppError::bad_request("badge needs a name"));
                }
                v.insert("badge".into(), json!(badge));
            }
            "message" => {
                let message = text(a, "text", 200);
                if message.is_empty() {
                    return Err(AppError::bad_request("message can't be empty"));
                }
                v.insert("text".into(), json!(message));
            }
            "command" => {
                let command = text(a, "command", 256).trim_start_matches('/').to_string();
                if command.is_empty() {
                    return Err(AppError::bad_request("command can't be empty"));
                }
                v.insert("command".into(), json!(command));
            }
            _ => return Err(AppError::bad_request(format!("unknown reward type '{kind}'"))),
        }
        out.push(Value::Object(v));
    }
    Ok(out)
}

fn pretty_item(id: &str) -> String {
    let name = id.rsplit(':').next().unwrap_or(id).replace(['_', '/'], " ");
    name.split_whitespace()
        .map(|w| {
            let mut c = w.chars();
            c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// A short, player-facing line for one action (commands stay vague on purpose).
pub fn describe(a: &Value) -> String {
    let n = |k: &str| a[k].as_i64().unwrap_or(0);
    match a["type"].as_str().unwrap_or("") {
        "money" => format!("${:.2}", a["amount"].as_f64().unwrap_or(0.0)),
        "item" => format!("{} x{}", pretty_item(a["item"].as_str().unwrap_or("")), n("amount")),
        "custom_item" => format!("{} x{}", pretty_item(a["custom"].as_str().unwrap_or("")), n("amount")),
        "permission" => {
            let m = n("minutes");
            let span = if m == 0 {
                String::new()
            } else if m % 1440 == 0 {
                format!(" for {} day{}", m / 1440, if m / 1440 == 1 { "" } else { "s" })
            } else if m % 60 == 0 {
                format!(" for {}h", m / 60)
            } else {
                format!(" for {m}m")
            };
            format!("Permission: {}{}", a["node"].as_str().unwrap_or(""), span)
        }
        "group" => format!("Group: {}", a["group"].as_str().unwrap_or("")),
        "unlock" => format!("Unlocks: {}", pretty_item(a["key"].as_str().unwrap_or(""))),
        "claim_chunks" => format!("+{} claim chunks", n("amount")),
        "badge" => format!("Badge: {}", a["badge"].as_str().unwrap_or("")),
        "message" => "A message".to_string(),
        _ => "A special reward".to_string(),
    }
}

#[cfg(test)]
mod event_reward_tests {
    use super::validate;
    use serde_json::json;

    #[test]
    fn event_reward_actions_use_the_standard_reward_contract() {
        let actions = validate(&json!([{"type":"money","amount":125.5}])).unwrap();
        assert_eq!(actions[0]["amount"], 125.5);
        assert!(validate(&json!([{"type":"command","command":""}])).is_err());
    }
}

// ---- default money, scaled by difficulty ----------------------------------------------------------------------------

/// A flat amount ending in 5 or 0, never less than 5.
pub fn round5(x: f64) -> f64 {
    ((x / 5.0).round() * 5.0).max(5.0)
}

/// A quest's default pay: its XP is the difficulty it was given, a daily is worth less per XP than a weekly.
pub fn quest_money(a: &AutoMoney, period: &str, xp: i64) -> f64 {
    let rate = match period {
        "daily" => a.quest_daily,
        "weekly" => a.quest_weekly,
        _ => a.quest_other,
    };
    round5(xp.max(0) as f64 * rate * a.scale)
}

pub fn achievement_money(a: &AutoMoney, xp: i64) -> f64 {
    round5(xp.max(0) as f64 * a.achievement * a.scale)
}

/// What reaching `level` pays: it grows steeply, so a rank in the hundreds is worth chasing.
pub fn level_money(a: &AutoMoney, level: i64) -> f64 {
    round5(a.level_base * (level.max(1) as f64).powf(a.level_exponent) * a.scale)
}

/// A rank milestone (title, badge...) on top of the level's own money; server ranks are worth half.
pub fn milestone_money(a: &AutoMoney, level: i64, server: bool) -> f64 {
    round5(level_money(a, level) * a.milestone * if server { 0.5 } else { 1.0 })
}

/// Give every quest, achievement and rank milestone a money reward when it has none, scaled by its difficulty. Amounts the
/// admin set themselves are left alone; the ones this wrote (marked `auto`) follow the current rates. Returns how many
/// bundles were written. Safe to run as often as you like.
pub async fn apply_defaults(state: &AppState) -> AppResult<usize> {
    apply_defaults_to(&state.db).await
}

pub async fn apply_defaults_to(db: &sqlx::SqlitePool) -> AppResult<usize> {
    let rules = crate::progression::load_pool(db).await?.rules.auto_money;
    if !rules.enabled {
        // Switched off: the automatic amounts go away again (amounts the admin set stay).
        let rows: Vec<(String, String, String)> = sqlx::query_as("SELECT source_type, source_id, actions FROM reward_bundles WHERE actions LIKE '%\"auto\"%'").fetch_all(db).await?;
        let mut removed = 0;
        for (kind, id, raw) in rows {
            let mut actions: Vec<Value> = serde_json::from_str(&raw).unwrap_or_default();
            let before = actions.len();
            actions.retain(|a| !(a["type"] == "money" && a["auto"] == true));
            if actions.len() == before {
                continue;
            }
            if actions.is_empty() {
                sqlx::query("DELETE FROM reward_bundles WHERE source_type = ? AND source_id = ?").bind(&kind).bind(&id).execute(db).await?;
            } else {
                sqlx::query("UPDATE reward_bundles SET actions = ? WHERE source_type = ? AND source_id = ?").bind(serde_json::to_string(&actions)?).bind(&kind).bind(&id).execute(db).await?;
            }
            removed += 1;
        }
        return Ok(removed);
    }
    let mut targets: Vec<(&'static str, String, f64)> = Vec::new();
    let quests: Vec<(String, String, i64)> = sqlx::query_as("SELECT id, period, xp_reward FROM quests").fetch_all(db).await?;
    targets.extend(quests.into_iter().map(|(id, period, xp)| ("quest", id, quest_money(&rules, &period, xp))));
    let achievements: Vec<(String, i64)> = sqlx::query_as("SELECT id, xp_reward FROM achievements").fetch_all(db).await?;
    targets.extend(achievements.into_iter().map(|(id, xp)| ("achievement", id, achievement_money(&rules, xp))));
    let milestones: Vec<(i64, String, i64)> = sqlx::query_as("SELECT id, level_type, level_req FROM level_rewards").fetch_all(db).await?;
    targets.extend(milestones.into_iter().map(|(id, scope, level)| ("level_reward", id.to_string(), milestone_money(&rules, level, scope == "server"))));
    let mut written = 0;
    let mut tx = db.begin().await?;
    for (kind, id, amount) in targets {
        let mut actions = bundle_stored(&mut tx, kind, &id).await?;
        let at = actions.iter().position(|a| a["type"] == "money" && a["server_id"].is_null());
        match at {
            Some(i) if actions[i]["auto"] != true => continue, // the admin's own amount
            Some(i) => {
                if actions[i]["amount"].as_f64() == Some(amount) {
                    continue;
                }
                actions[i]["amount"] = json!(amount);
            }
            None => actions.push(json!({ "type": "money", "amount": amount, "auto": true })),
        }
        sqlx::query("INSERT INTO reward_bundles (source_type, source_id, actions) VALUES (?, ?, ?) ON CONFLICT(source_type, source_id) DO UPDATE SET actions = excluded.actions")
            .bind(kind).bind(&id).bind(serde_json::to_string(&actions)?).execute(&mut *tx).await?;
        written += 1;
    }
    tx.commit().await?;
    Ok(written)
}

/// The actions saved for a source (no defaults mixed in).
async fn bundle_stored(conn: &mut SqliteConnection, kind: &str, id: &str) -> AppResult<Vec<Value>> {
    let raw: Option<String> = sqlx::query_scalar("SELECT actions FROM reward_bundles WHERE source_type = ? AND source_id = ?")
        .bind(kind)
        .bind(id)
        .fetch_optional(&mut *conn)
        .await?;
    Ok(raw.and_then(|r| serde_json::from_str::<Vec<Value>>(&r).ok()).unwrap_or_default())
}

/// Queue the money for reaching a level (when default money is on).
pub async fn enqueue_level(conn: &mut SqliteConnection, uuid: &str, level: i64) -> AppResult<()> {
    if !crate::progression::load(&mut *conn).await?.rules.auto_money.enabled {
        return Ok(());
    }
    sqlx::query("INSERT INTO reward_queue (uuid, source_type, source_id, created_at) VALUES (?, 'level', ?, ?)")
        .bind(uuid).bind(level.to_string()).bind(crate::db::now()).execute(&mut *conn).await?;
    Ok(())
}

pub async fn bundle(conn: &mut SqliteConnection, kind: &str, id: &str) -> AppResult<Vec<Value>> {
    if kind == "level" {
        let a = crate::progression::load(&mut *conn).await?.rules.auto_money;
        return Ok(vec![json!({ "type": "money", "amount": level_money(&a, id.parse().unwrap_or(1)) })]);
    }
    if kind == "event" {
        let raw: Option<String> = sqlx::query_scalar("SELECT reward_data FROM community_events WHERE id = ?")
            .bind(id).fetch_optional(&mut *conn).await?;
        return Ok(raw.and_then(|r| serde_json::from_str::<Vec<Value>>(&r).ok()).unwrap_or_default());
    }
    let raw: Option<String> = sqlx::query_scalar("SELECT actions FROM reward_bundles WHERE source_type = ? AND source_id = ?")
        .bind(kind)
        .bind(id)
        .fetch_optional(&mut *conn)
        .await?;
    Ok(raw.and_then(|r| serde_json::from_str::<Vec<Value>>(&r).ok()).unwrap_or_default())
}

/// Queue a bundle for a player who just earned its source. Does nothing when the source has no bundle.
pub async fn enqueue(conn: &mut SqliteConnection, uuid: &str, kind: &str, id: &str) -> AppResult<()> {
    let has: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM reward_bundles WHERE source_type = ? AND source_id = ? AND actions <> '[]')")
            .bind(kind)
            .bind(id)
            .fetch_one(&mut *conn)
            .await?;
    if has {
        sqlx::query("INSERT INTO reward_queue (uuid, source_type, source_id, created_at) VALUES (?, ?, ?, ?)")
            .bind(uuid)
            .bind(kind)
            .bind(id)
            .bind(crate::db::now())
            .execute(&mut *conn)
            .await?;
    }
    Ok(())
}

async fn credit(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    uuid: &str,
    name: &str,
    amount: f64,
    server: Option<i64>,
    note: &str,
) -> AppResult<()> {
    // One credit per economy: servers sharing a group share balances, so they must not be paid twice.
    let servers: Vec<i64> = match server {
        Some(id) => vec![id],
        None => sqlx::query_scalar("SELECT id FROM game_servers ORDER BY id").fetch_all(&mut **tx).await?,
    };
    let mut economies = Vec::new();
    for id in servers {
        let e: i64 = sqlx::query_scalar(
            "SELECT COALESCE((SELECT MIN(b.id) FROM game_servers b WHERE b.economy_group <> '' AND b.economy_group = a.economy_group COLLATE NOCASE), a.id) FROM game_servers a WHERE a.id = ?",
        )
        .bind(id)
        .fetch_optional(&mut **tx)
        .await?
        .unwrap_or(id);
        if !economies.contains(&e) {
            economies.push(e);
        }
    }
    let now = chrono::Utc::now().to_rfc3339();
    for economy in economies {
        crate::routes::economy::ensure_balance(tx, economy, uuid, name).await?;
        sqlx::query("UPDATE server_economy SET balance = balance + ?, updated_at = ? WHERE server_id = ? AND uuid = ?")
            .bind(amount)
            .bind(&now)
            .bind(economy)
            .bind(uuid)
            .execute(&mut **tx)
            .await?;
        sqlx::query("INSERT INTO economy_transactions(server_id, from_uuid, from_name, to_uuid, to_name, amount, description, created_at) VALUES (?, 'server', 'Rewards', ?, ?, ?, ?, ?)")
            .bind(economy).bind(uuid).bind(name).bind(amount).bind(note).bind(&now).execute(&mut **tx).await?;
    }
    Ok(())
}

async fn source_label(conn: &mut SqliteConnection, kind: &str, id: &str) -> String {
    let q = match kind {
        "quest" => "SELECT title FROM quests WHERE id = ?",
        "achievement" => "SELECT title FROM achievements WHERE id = ?",
        "event" => "SELECT title FROM community_events WHERE id = ?",
        "level" => return format!("Reached level {id}"),
        _ => "SELECT reward_name FROM level_rewards WHERE CAST(id AS TEXT) = ?",
    };
    sqlx::query_scalar::<_, String>(q).bind(id).fetch_optional(&mut *conn).await.ok().flatten().unwrap_or_else(|| id.to_string())
}

/// Apply every queued bundle. Safe to call often; each queue row is handled exactly once.
pub async fn process_queue(state: &AppState) -> AppResult<usize> {
    let rows: Vec<(i64, String, String, String)> =
        sqlx::query_as("SELECT id, uuid, source_type, source_id FROM reward_queue ORDER BY id LIMIT 200").fetch_all(&state.db).await?;
    let mut done = 0;
    for (queue_id, uuid, kind, id) in rows {
        let mut tx = state.db.begin().await?;
        let gone: Option<i64> =
            sqlx::query_scalar("DELETE FROM reward_queue WHERE id = ? RETURNING id").bind(queue_id).fetch_optional(&mut *tx).await?;
        if gone.is_none() {
            continue; // another request got there first
        }
        let name: String =
            sqlx::query_scalar("SELECT username FROM users WHERE uuid = ?").bind(&uuid).fetch_optional(&mut *tx).await?.unwrap_or_default();
        if name.is_empty() {
            tx.commit().await?;
            continue;
        }
        let label = source_label(&mut tx, &kind, &id).await;
        let source = format!("{kind}:{id}");
        let now = crate::db::now();
        for action in bundle(&mut tx, &kind, &id).await? {
            let server = action["server_id"].as_i64();
            match action["type"].as_str().unwrap_or("") {
                "money" => {
                    credit(&mut tx, &uuid, &name, action["amount"].as_f64().unwrap_or(0.0), server, &format!("Reward: {label}")).await?
                }
                "claim_chunks" => {
                    sqlx::query("INSERT INTO player_bonuses (uuid, claim_chunks) VALUES (?, ?) ON CONFLICT(uuid) DO UPDATE SET claim_chunks = claim_chunks + excluded.claim_chunks")
                        .bind(&uuid).bind(action["amount"].as_i64().unwrap_or(0)).execute(&mut *tx).await?;
                }
                "unlock" => {
                    // Copy the cosmetic template into the player's collection (once; a second reward of the same key does nothing).
                    sqlx::query(
                        "INSERT OR IGNORE INTO player_unlocks(uuid, unlock_key, unlock_type, source_type, source_id, unlocked_at, equipped, metadata)
                         SELECT ?, key, type, ?, ?, ?, 0, metadata FROM cosmetic_templates WHERE key = ?",
                    )
                    .bind(&uuid).bind(&kind).bind(&id).bind(&now).bind(action["key"].as_str().unwrap_or(""))
                    .execute(&mut *tx)
                    .await?;
                }
                "badge" => {
                    let raw: String = sqlx::query_scalar("SELECT badges FROM user_levels WHERE uuid = ?")
                        .bind(&uuid)
                        .fetch_optional(&mut *tx)
                        .await?
                        .unwrap_or_else(|| "[]".into());
                    let mut badges: Vec<String> = serde_json::from_str(&raw).unwrap_or_default();
                    let badge = action["badge"].as_str().unwrap_or("").to_string();
                    if !badge.is_empty() && !badges.contains(&badge) {
                        badges.push(badge);
                        sqlx::query("INSERT INTO user_levels(uuid, badges, updated_at) VALUES (?, ?, ?) ON CONFLICT(uuid) DO UPDATE SET badges = excluded.badges")
                            .bind(&uuid).bind(serde_json::to_string(&badges)?).bind(&now).execute(&mut *tx).await?;
                    }
                }
                other => {
                    let mut payload = action.clone();
                    if other == "custom_item" {
                        let spec: Option<String> = sqlx::query_scalar("SELECT spec FROM custom_items WHERE id = ?")
                            .bind(action["custom"].as_str().unwrap_or(""))
                            .fetch_optional(&mut *tx)
                            .await?;
                        if let Some(spec) = spec {
                            payload["spec"] = serde_json::from_str(&spec)?;
                        }
                    }
                    payload["reason"] = json!(label);
                    sqlx::query(
                        "INSERT INTO reward_deliveries (uuid, server_id, kind, payload, source, created_at) VALUES (?, ?, ?, ?, ?, ?)",
                    )
                    .bind(&uuid)
                    .bind(server)
                    .bind(other)
                    .bind(payload.to_string())
                    .bind(&source)
                    .bind(&now)
                    .execute(&mut *tx)
                    .await?;
                }
            }
        }
        tx.commit().await?;
        done += 1;
    }
    Ok(done)
}

/// Deliveries this game server should carry out now: for players online here, not yet done, and not already out for delivery.
pub async fn poll(state: &AppState, server_id: i64) -> AppResult<Vec<Value>> {
    process_queue(state).await?;
    let mut tx = state.db.begin().await?;
    let stale = (chrono::Utc::now() - chrono::Duration::seconds(120)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let rows: Vec<(i64, String, String, String, String)> = sqlx::query_as(
        "SELECT d.id, d.uuid, COALESCE(u.username, ''), d.kind, d.payload FROM reward_deliveries d
         LEFT JOIN users u ON u.uuid = d.uuid
         WHERE d.delivered_at IS NULL AND d.attempts < 8 AND (d.server_id IS NULL OR d.server_id = ?)
           AND (d.sent_at IS NULL OR d.sent_at < ?)
           AND EXISTS (SELECT 1 FROM server_online o WHERE o.server_id = ? AND o.uuid = d.uuid)
         ORDER BY d.id LIMIT 50",
    )
    .bind(server_id)
    .bind(&stale)
    .bind(server_id)
    .fetch_all(&mut *tx)
    .await?;
    let now = crate::db::now();
    for (id, ..) in &rows {
        sqlx::query("UPDATE reward_deliveries SET sent_at = ?, attempts = attempts + 1 WHERE id = ?")
            .bind(&now)
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(rows.into_iter().map(|(id, uuid, name, kind, payload)| json!({ "id": id, "uuid": uuid, "name": name, "kind": kind, "payload": serde_json::from_str::<Value>(&payload).unwrap_or(json!({})) })).collect())
}

/// The game server reports what it did. Failures are retried a few times, then kept in the log with the reason.
pub async fn ack(state: &AppState, done: &[i64], failed: &[(i64, String)]) -> AppResult<()> {
    let now = crate::db::now();
    let mut tx = state.db.begin().await?;
    for id in done {
        sqlx::query("UPDATE reward_deliveries SET delivered_at = ?, error = NULL WHERE id = ?")
            .bind(&now)
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }
    for (id, error) in failed {
        let error: String = error.chars().filter(|c| !c.is_control()).take(200).collect();
        // A refused delivery (e.g. commands switched off on that server) is retried a few times, then left for the admin to see.
        sqlx::query("UPDATE reward_deliveries SET error = ?, sent_at = ? WHERE id = ? AND delivered_at IS NULL")
            .bind(error)
            .bind(&now)
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}
