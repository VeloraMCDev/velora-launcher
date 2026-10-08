//! Discord slash commands (`/status`, `/players`, `/quests`, `/stats`, `/leaderboard`, `/guild`, `/guilds`).
//!
//! Discord calls the panel over HTTPS for each use, so there is no always-on gateway connection to keep alive. Every request
//! is signed; ones that fail the check are refused. Set the application's *Interactions Endpoint URL* to
//! `<panel address>/api/v1/discord/interactions?instance=<instance ID>`, save the public key in Settings, then press **Register commands**.

use crate::state::RequestState as State;
use crate::auth::AdminUser;
use crate::error::{AppError, AppResult};
use crate::routes::{discord, quests};
use crate::state::AppState;
use axum::body::Bytes;

use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::{json, Value};

const COLOR: u32 = 0x6366f1;

/// Slash command definitions registered with Discord.
pub fn definitions() -> Value {
    let player_opt = |required: bool, description: &str| json!({ "type": 3, "name": "player", "description": description, "required": required, "autocomplete": true });
    json!([
        { "name": "status", "description": "Is the server online? Players, version and performance", "type": 1 },
        { "name": "players", "description": "Who is online right now", "type": 1 },
        { "name": "quests", "description": "See a player's daily and weekly quests", "type": 1, "options": [player_opt(true, "The player to look at")] },
        { "name": "stats", "description": "A player's level, playtime, guild and more", "type": 1, "options": [player_opt(true, "The player to look at")] },
        { "name": "leaderboard", "description": "The top players", "type": 1, "options": [{
            "type": 3, "name": "by", "description": "What to rank by", "required": false,
            "choices": [{ "name": "Level", "value": "level" }, { "name": "Playtime", "value": "playtime" }, { "name": "Player kills", "value": "kills" }, { "name": "Blocks broken", "value": "blocks" }, { "name": "Balance", "value": "balance" }]
        }] },
        { "name": "guild", "description": "Look up a guild", "type": 1, "options": [{ "type": 3, "name": "name", "description": "Guild name or tag", "required": true, "autocomplete": true }] },
        { "name": "guilds", "description": "The biggest guilds", "type": 1 },
    ])
}

/// Checks the `X-Signature-Ed25519` / `X-Signature-Timestamp` pair Discord puts on every interaction.
pub fn verify(public_key_hex: &str, signature_hex: &str, timestamp: &str, body: &[u8]) -> bool {
    let (Ok(key), Ok(sig)) = (hex::decode(public_key_hex), hex::decode(signature_hex)) else { return false };
    let mut message = Vec::with_capacity(timestamp.len() + body.len());
    message.extend_from_slice(timestamp.as_bytes());
    message.extend_from_slice(body);
    ring::signature::UnparsedPublicKey::new(&ring::signature::ED25519, key).verify(&message, &sig).is_ok()
}

fn reply(embed: Value) -> Value {
    json!({ "type": 4, "data": { "embeds": [embed] } })
}

fn notice(text: &str) -> Value {
    json!({ "type": 4, "data": { "content": text, "flags": 64 } })
}

fn cut(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        s.chars().take(n.saturating_sub(1)).collect::<String>() + "…"
    }
}

fn option<'a>(interaction: &'a Value, name: &str) -> Option<&'a str> {
    interaction["data"]["options"].as_array()?.iter().find(|o| o["name"] == name)?["value"].as_str()
}

fn hours(secs: i64) -> String {
    if secs >= 3600 {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    } else {
        format!("{}m", secs / 60)
    }
}

pub async fn interactions(State(state): State<AppState>, headers: HeaderMap, body: Bytes) -> Response {
    let settings = match discord::load(&state).await {
        Ok(s) => s,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let sig = headers.get("x-signature-ed25519").and_then(|v| v.to_str().ok()).unwrap_or("");
    let ts = headers.get("x-signature-timestamp").and_then(|v| v.to_str().ok()).unwrap_or("");
    if settings.public_key.is_empty() || !verify(&settings.public_key, sig, ts, &body) {
        return (StatusCode::UNAUTHORIZED, "invalid request signature").into_response();
    }
    let Ok(interaction) = serde_json::from_slice::<Value>(&body) else { return StatusCode::BAD_REQUEST.into_response() };
    let answer = match interaction["type"].as_i64() {
        Some(1) => json!({ "type": 1 }),
        Some(2) => handle_command(&state, &interaction).await.unwrap_or_else(|e| notice(&format!("⚠️ {}", e.message))),
        Some(4) => autocomplete(&state, &interaction).await.unwrap_or_else(|_| json!({ "type": 8, "data": { "choices": [] } })),
        _ => notice("That isn't supported yet."),
    };
    Json(answer).into_response()
}

async fn autocomplete(state: &AppState, interaction: &Value) -> AppResult<Value> {
    let focused = interaction["data"]["options"].as_array().and_then(|o| o.iter().find(|x| x["focused"] == true)).cloned().unwrap_or(Value::Null);
    let typed = focused["value"].as_str().unwrap_or("").trim().to_lowercase().replace(['%', '_'], "");
    let like = format!("{typed}%");
    let names: Vec<String> = if focused["name"] == "name" {
        sqlx::query_scalar("SELECT name FROM guilds WHERE name LIKE ?1 OR tag LIKE ?1 ORDER BY name LIMIT 25").bind(&like).fetch_all(&state.db).await?
    } else {
        sqlx::query_scalar(
            "SELECT n FROM (SELECT username AS n FROM users WHERE username LIKE ?1 UNION SELECT name AS n FROM player_stats WHERE name LIKE ?1) ORDER BY n COLLATE NOCASE LIMIT 25",
        )
        .bind(&like)
        .fetch_all(&state.db)
        .await?
    };
    Ok(json!({ "type": 8, "data": { "choices": names.into_iter().map(|n| json!({ "name": cut(&n, 100), "value": cut(&n, 100) })).collect::<Vec<_>>() } }))
}

async fn handle_command(state: &AppState, interaction: &Value) -> AppResult<Value> {
    match interaction["data"]["name"].as_str().unwrap_or("") {
        "status" => status(state).await,
        "players" => players(state).await,
        "quests" => quests_embed(state, option(interaction, "player").unwrap_or("")).await,
        "stats" => stats(state, option(interaction, "player").unwrap_or("")).await,
        "leaderboard" => leaderboard(state, option(interaction, "by").unwrap_or("level")).await,
        "guild" => guild(state, option(interaction, "name").unwrap_or("")).await,
        "guilds" => guilds(state).await,
        _ => Ok(notice("I don't know that command. An admin may need to register commands again.")),
    }
}

async fn status(state: &AppState) -> AppResult<Value> {
    let rows = crate::routes::servers::status_rows(state).await?;
    if rows.is_empty() {
        return Ok(notice("No game servers are set up yet."));
    }
    let online = rows.iter().filter(|s| s.online).count();
    let mut fields = Vec::new();
    for s in &rows {
        let version = if s.version.is_empty() { s.software.clone() } else { format!("{} {}", s.software, s.version) };
        let seen = s.last_seen.as_deref().and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok()).map(|t| format!("<t:{}:R>", t.timestamp())).unwrap_or_else(|| "never".into());
        fields.push(json!({
            "name": format!("{} {}", if s.online { "🟢" } else { "🔴" }, cut(&s.name, 80)),
            "value": if s.online {
                format!("**{}**/{} players · TPS **{}**\n{}", s.players, if s.max_players > 0 { s.max_players.to_string() } else { "–".into() }, s.tps.map(|t| format!("{t:.1}")).unwrap_or_else(|| "–".into()), cut(&version, 60))
            } else {
                format!("Offline · last seen {seen}")
            },
            "inline": true,
        }));
    }
    let total: i64 = rows.iter().map(|s| s.players).sum();
    Ok(reply(json!({
        "title": if online == rows.len() { "✅ All servers online" } else if online == 0 { "🔴 Servers offline" } else { "⚠️ Some servers are offline" },
        "description": format!("**{total}** player{} online across **{online}/{}** server{}", if total == 1 { "" } else { "s" }, rows.len(), if rows.len() == 1 { "" } else { "s" }),
        "color": COLOR, "fields": fields, "timestamp": chrono::Utc::now().to_rfc3339(),
    })))
}

async fn players(state: &AppState) -> AppResult<Value> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT s.name, o.name FROM server_online o JOIN game_servers s ON s.id = o.server_id ORDER BY s.id, o.name COLLATE NOCASE",
    )
    .fetch_all(&state.db)
    .await?;
    if rows.is_empty() {
        return Ok(reply(json!({ "title": "👥 Nobody is online", "description": "Be the first to hop on!", "color": COLOR })));
    }
    let mut by_server: Vec<(String, Vec<String>)> = Vec::new();
    for (server, player) in rows {
        match by_server.last_mut() {
            Some((s, list)) if *s == server => list.push(player),
            _ => by_server.push((server, vec![player])),
        }
    }
    let fields: Vec<Value> = by_server.iter().map(|(s, p)| json!({ "name": format!("{} ({})", cut(s, 60), p.len()), "value": cut(&p.join(", "), 1000), "inline": false })).collect();
    let total: usize = by_server.iter().map(|(_, p)| p.len()).sum();
    Ok(reply(json!({ "title": format!("👥 {total} online"), "color": COLOR, "fields": fields })))
}

async fn quests_embed(state: &AppState, who: &str) -> AppResult<Value> {
    let (uuid, name) = quests::find_player(state, who).await?.ok_or_else(|| AppError::not_found(format!("I couldn't find a player called \"{}\".", who.trim())))?;
    let q = quests::player_summary(state, &uuid, &name).await?;
    let section = |key: &str| -> String {
        let list = q[key].as_array().cloned().unwrap_or_default();
        if list.is_empty() {
            return "*None right now*".into();
        }
        list.iter()
            .map(|x| {
                let (p, t) = (x["progress"].as_i64().unwrap_or(0), x["target"].as_i64().unwrap_or(1).max(1));
                let filled = ((10 * p) / t).clamp(0, 10) as usize;
                let state = if x["claimed"] == true { "✅ claimed".to_string() } else if x["completed"] == true { "🎁 ready to claim".to_string() } else { format!("{p}/{t}") };
                format!("`{}{}` **{}** · {} · +{} XP", "▰".repeat(filled), "▱".repeat(10 - filled), cut(x["title"].as_str().unwrap_or(""), 60), state, x["xp"].as_i64().unwrap_or(0))
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let stamp = |key: &str| q[key].as_str().and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok()).map(|t| format!("resets <t:{}:R>", t.timestamp())).unwrap_or_default();
    Ok(reply(json!({
        "title": format!("📜 {}'s quests", name), "color": COLOR,
        "thumbnail": { "url": crate::embeds::avatar(state, &uuid) },
        "fields": [
            { "name": format!("☀️ Daily · {}", stamp("daily_resets")), "value": cut(&section("daily"), 1024) },
            { "name": format!("📅 Weekly · {}", stamp("weekly_resets")), "value": cut(&section("weekly"), 1024) },
        ],
    })))
}

async fn stats(state: &AppState, who: &str) -> AppResult<Value> {
    let (uuid, name) = quests::find_player(state, who).await?.ok_or_else(|| AppError::not_found(format!("I couldn't find a player called \"{}\".", who.trim())))?;
    let level: Option<(i64, i64, Option<String>)> = sqlx::query_as("SELECT global_level, global_xp, title FROM user_levels WHERE uuid = ?").bind(&uuid).fetch_optional(&state.db).await?;
    let totals: (i64, i64, i64, i64, i64) = sqlx::query_as(
        "SELECT COALESCE(SUM(playtime_secs),0), COALESCE(SUM(player_kills),0), COALESCE(SUM(deaths),0), COALESCE(SUM(mob_kills),0), COALESCE(SUM(blocks_broken),0) FROM player_stats WHERE uuid = ?",
    )
    .bind(&uuid)
    .fetch_one(&state.db)
    .await?;
    let guild: Option<(String, String, String)> = sqlx::query_as("SELECT g.name, g.tag, m.role FROM guild_members m JOIN guilds g ON g.id = m.guild_id WHERE m.uuid = ? LIMIT 1").bind(&uuid).fetch_optional(&state.db).await?;
    let achievements: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM user_achievements WHERE user_uuid = ?").bind(&uuid).fetch_one(&state.db).await?;
    let online: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM server_online WHERE uuid = ?)").bind(&uuid).fetch_one(&state.db).await?;
    let (lvl, xp, title) = level.unwrap_or((1, 0, None));
    Ok(reply(json!({
        "title": format!("{} {}", if online { "🟢" } else { "⚫" }, name),
        "description": title.map(|t| format!("*{}*", cut(&t, 80))).unwrap_or_default(),
        "color": COLOR, "thumbnail": { "url": crate::embeds::avatar(state, &uuid) },
        "fields": [
            { "name": "Level", "value": format!("**{lvl}** · {xp} XP"), "inline": true },
            { "name": "Playtime", "value": hours(totals.0), "inline": true },
            { "name": "Achievements", "value": achievements.to_string(), "inline": true },
            { "name": "Combat", "value": format!("{} kills · {} deaths · {} mobs", totals.1, totals.2, totals.3), "inline": false },
            { "name": "Blocks broken", "value": totals.4.to_string(), "inline": true },
            { "name": "Guild", "value": guild.map(|(n, t, r)| format!("[{t}] {n} ({r})")).unwrap_or_else(|| "None".into()), "inline": true },
        ],
    })))
}

async fn leaderboard(state: &AppState, by: &str) -> AppResult<Value> {
    let (title, rows): (&str, Vec<(String, String)>) = match by {
        "balance" => {
            let sid: i64 = sqlx::query_scalar("SELECT COALESCE(MIN(id), 0) FROM game_servers").fetch_one(&state.db).await?;
            let economy = crate::routes::servers::economy_scope(&state.db, sid).await?;
            let list: Vec<(String, f64)> = sqlx::query_as("SELECT username, balance FROM server_economy WHERE server_id = ? ORDER BY balance DESC LIMIT 10").bind(economy).fetch_all(&state.db).await?;
            ("💰 Richest players", list.into_iter().map(|(n, b)| (n, format!("${b:.2}"))).collect())
        }
        other => {
            let (label, order) = match other {
                "playtime" => ("⏱️ Most playtime", "playtime"),
                "kills" => ("⚔️ Most player kills", "kills"),
                "blocks" => ("⛏️ Most blocks broken", "blocks"),
                _ => ("⭐ Highest level", "COALESCE(l.global_xp, 0)"),
            };
            let sql = format!(
                "SELECT COALESCE(u.username, MAX(s.name)), SUM(s.playtime_secs) AS playtime, SUM(s.player_kills) AS kills, SUM(s.blocks_broken) AS blocks, COALESCE(l.global_level, 1)
                 FROM player_stats s LEFT JOIN users u ON u.uuid = s.uuid LEFT JOIN user_levels l ON l.uuid = s.uuid GROUP BY s.uuid ORDER BY {order} DESC LIMIT 10"
            );
            let list: Vec<(String, i64, i64, i64, i64)> = sqlx::query_as(&sql).fetch_all(&state.db).await?;
            (label, list.into_iter().map(|(n, p, k, b, l)| (n, match other { "playtime" => hours(p), "kills" => format!("{k} kills"), "blocks" => format!("{b} blocks"), _ => format!("level {l}") })).collect())
        }
    };
    if rows.is_empty() {
        return Ok(notice("Nothing to rank yet."));
    }
    let medals = ["🥇", "🥈", "🥉"];
    let text = rows.iter().enumerate().map(|(i, (n, v))| format!("{} **{}** · {}", medals.get(i).map(|m| m.to_string()).unwrap_or_else(|| format!("`{}`", i + 1)), cut(n, 40), v)).collect::<Vec<_>>().join("\n");
    Ok(reply(json!({ "title": title, "description": text, "color": COLOR })))
}

async fn guild(state: &AppState, text: &str) -> AppResult<Value> {
    let g: Option<(String, String, String, String, String, i64, String)> = sqlx::query_as(
        "SELECT id, name, tag, description, leader_uuid, level, created_at FROM guilds WHERE name = ?1 COLLATE NOCASE OR tag = ?1 COLLATE NOCASE LIMIT 1",
    )
    .bind(text.trim())
    .fetch_optional(&state.db)
    .await?;
    let (id, name, tag, desc, leader_uuid, level, created) = g.ok_or_else(|| AppError::not_found(format!("There's no guild called \"{}\".", text.trim())))?;
    let leader: Option<String> = sqlx::query_scalar("SELECT name FROM guild_members WHERE guild_id = ? AND uuid = ?").bind(&id).bind(&leader_uuid).fetch_optional(&state.db).await?;
    let members: Vec<String> = sqlx::query_scalar("SELECT name FROM guild_members WHERE guild_id = ? ORDER BY CASE role WHEN 'leader' THEN 0 WHEN 'officer' THEN 1 ELSE 2 END, name COLLATE NOCASE LIMIT 25").bind(&id).fetch_all(&state.db).await?;
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM guild_members WHERE guild_id = ?").bind(&id).fetch_one(&state.db).await?;
    let claims: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM guild_claims WHERE guild_id = ?").bind(&id).fetch_one(&state.db).await?;
    Ok(reply(json!({
        "title": format!("🛡️ [{tag}] {name}"), "description": cut(&desc, 400), "color": COLOR,
        "fields": [
            { "name": "Leader", "value": leader.unwrap_or_else(|| "?".into()), "inline": true },
            { "name": "Level", "value": level.to_string(), "inline": true },
            { "name": "Land", "value": format!("{claims} chunks"), "inline": true },
            { "name": format!("Members ({total})"), "value": cut(&members.join(", "), 1000) },
        ],
        "footer": { "text": format!("Founded {}", created.get(..10).unwrap_or(&created)) },
    })))
}

async fn guilds(state: &AppState) -> AppResult<Value> {
    let rows: Vec<(String, String, i64, i64, i64)> = sqlx::query_as(
        "SELECT g.name, g.tag, g.level, (SELECT COUNT(*) FROM guild_members m WHERE m.guild_id = g.id), (SELECT COUNT(*) FROM guild_claims c WHERE c.guild_id = g.id)
         FROM guilds g ORDER BY g.level DESC, g.xp DESC, 4 DESC LIMIT 10",
    )
    .fetch_all(&state.db)
    .await?;
    if rows.is_empty() {
        return Ok(notice("No guilds yet."));
    }
    let text = rows.iter().enumerate().map(|(i, (n, t, l, m, c))| format!("`{}` **[{}] {}** · level {l} · {m} members · {c} chunks", i + 1, t, cut(n, 40))).collect::<Vec<_>>().join("\n");
    Ok(reply(json!({ "title": "🛡️ Top guilds", "description": text, "color": COLOR })))
}

/// Register (or refresh) the slash commands with the bot's server. Guild commands show up immediately.
pub async fn register(_: AdminUser, State(state): State<AppState>) -> AppResult<Json<Value>> {
    let conn = crate::routes::connections::settings(&state).await?;
    if conn.discord_bot_token.is_empty() || conn.discord_guild_id.is_empty() || conn.discord_client_id.is_empty() {
        return Err(AppError::bad_request("Save the Discord application (client) ID, the bot token and the server ID in Settings first"));
    }
    let settings = discord::load(&state).await?;
    if settings.public_key.is_empty() {
        return Err(AppError::bad_request("Save the application's public key first (Developer Portal → General Information)"));
    }
    let path = format!("/applications/{}/guilds/{}/commands", conn.discord_client_id, conn.discord_guild_id);
    let resp = discord::bot_request(&state, reqwest::Method::PUT, path, &conn.discord_bot_token)
        .json(&definitions())
        .send()
        .await
        .map_err(|e| AppError::bad_request(format!("Could not reach Discord: {e}")))?;
    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let detail = resp.text().await.unwrap_or_default();
        return Err(AppError::bad_request(match status {
            401 => "Discord rejected the bot token.".to_string(),
            403 | 404 => "Discord refused. Add the bot to your server with the applications.commands scope (use the invite link in Settings), and check the application ID and server ID.".to_string(),
            s => format!("Discord refused the request (HTTP {s}): {}", cut(&detail, 200)),
        }));
    }
    let count = definitions().as_array().map(|a| a.len()).unwrap_or(0);
    Ok(Json(json!({ "ok": true, "commands": count })))
}
