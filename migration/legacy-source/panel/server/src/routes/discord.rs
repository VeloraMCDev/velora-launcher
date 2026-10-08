//! Discord community features on top of account linking: role <-> group
//! sync, an invite link for launchers, and announcements and live boards posted by the bot into channels you pick by ID
//! (older setups that use a webhook address keep working).
//!
//! Everything here is opt-in and needs the bot token and server ID saved in
//! Settings. Group mappings only add membership in the chosen direction;
//! managed level-title roles are replaced when a player's rank changes.

use crate::auth::{AdminUser, AuthUser};
use crate::embeds;
use crate::error::{AppError, AppResult};
use crate::routes::connections::ConnectionsSettings;
use crate::state::AppState;
use crate::state::RequestState as State;
use crate::store;
use axum::extract::Path;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;

const API: &str = "https://discord.com/api/v10";
/// View Channel, Send Messages, Embed Links, Read Message History and Manage Roles.
const BOT_PERMISSIONS: u64 = 1024 + 2048 + 16384 + 65536 + 268_435_456;

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
#[serde(default)]
pub struct DiscordSettings {
    /// `off`, `discord_to_panel`, `panel_to_discord` or `both`.
    pub role_sync: String,
    /// Discord role used until the first mapped rank title is earned.
    pub base_role_id: String,
    /// Shown to players in the launcher.
    pub invite_url: String,
    /// The channel the bot posts announcements in. Empty (and no webhook) turns them off.
    pub channel_id: String,
    /// Older setups: announcements through a webhook address instead of the bot.
    pub webhook_url: String,
    pub notify_achievements: bool,
    pub notify_guilds: bool,
    pub notify_members: bool,
    /// The application's public key (Developer Portal → General Information); needed to verify slash-command requests.
    pub public_key: String,
}

impl Default for DiscordSettings {
    fn default() -> Self {
        Self {
            role_sync: "off".into(),
            base_role_id: String::new(),
            invite_url: String::new(),
            channel_id: String::new(),
            webhook_url: String::new(),
            notify_achievements: true,
            notify_guilds: true,
            notify_members: false,
            public_key: String::new(),
        }
    }
}

pub async fn load(state: &AppState) -> AppResult<DiscordSettings> {
    store::kv_get(state, "discord_settings").await
}

pub fn valid_webhook(url: &str) -> bool {
    url.is_empty()
        || [
            "https://discord.com/api/webhooks/",
            "https://discordapp.com/api/webhooks/",
            "https://ptb.discord.com/api/webhooks/",
            "https://canary.discord.com/api/webhooks/",
        ]
        .iter()
        .any(|p| url.starts_with(p))
}

/// A Discord snowflake as copied with Developer Mode ("Copy Channel ID"): 17 to 20 digits in practice.
pub fn valid_channel_id(id: &str) -> bool {
    id.is_empty() || ((15..=25).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_digit()))
}

pub fn valid_invite(url: &str) -> bool {
    url.is_empty()
        || ["https://discord.gg/", "https://discord.com/invite/", "https://discordapp.com/invite/"].iter().any(|p| url.starts_with(p))
}

fn view(s: &DiscordSettings) -> Value {
    json!({
        "role_sync": s.role_sync, "base_role_id": s.base_role_id, "invite_url": s.invite_url,
        "channel_id": s.channel_id, "webhook_set": !s.webhook_url.is_empty(),
        "notify_achievements": s.notify_achievements, "notify_guilds": s.notify_guilds, "notify_members": s.notify_members,
        "public_key": s.public_key,
    })
}

pub async fn get_settings(_: AdminUser, State(state): State<AppState>) -> AppResult<Json<Value>> {
    let conn: ConnectionsSettings = store::kv_get(&state, "connections_settings").await?;
    let linked: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM account_connections WHERE provider = 'discord'").fetch_one(&state.db).await?;
    let mut v = view(&load(&state).await?);
    v["bot_ready"] = json!(!conn.discord_bot_token.is_empty() && !conn.discord_guild_id.is_empty());
    v["bot_token_set"] = json!(!conn.discord_bot_token.is_empty());
    // Where to add the bot to a server: it needs to see and post (with embeds) in the channels, and manage roles for role sync.
    v["bot_invite_url"] = if conn.discord_client_id.is_empty() {
        Value::Null
    } else {
        json!(format!("https://discord.com/oauth2/authorize?client_id={}&scope=bot&permissions={BOT_PERMISSIONS}", conn.discord_client_id))
    };
    v["linked_accounts"] = json!(linked);
    Ok(Json(v))
}

#[derive(Deserialize)]
pub struct SettingsInput {
    role_sync: String,
    #[serde(default)]
    base_role_id: String,
    invite_url: String,
    /// The announcement channel's ID. Empty clears it.
    #[serde(default)]
    channel_id: String,
    /// Older setups only. Omitted or empty keeps the saved webhook; `"-"` clears it.
    #[serde(default)]
    webhook_url: String,
    notify_achievements: bool,
    notify_guilds: bool,
    notify_members: bool,
    /// Leave out to keep the saved key; an empty string clears it.
    #[serde(default)]
    public_key: Option<String>,
}

pub async fn put_settings(_: AdminUser, State(state): State<AppState>, Json(i): Json<SettingsInput>) -> AppResult<Json<Value>> {
    if !["off", "discord_to_panel", "panel_to_discord", "both"].contains(&i.role_sync.as_str()) {
        return Err(AppError::bad_request("unknown role sync mode"));
    }
    if !i.base_role_id.is_empty() && (!i.base_role_id.bytes().all(|b| b.is_ascii_digit()) || i.base_role_id.len() > 24) {
        return Err(AppError::bad_request("Base Discord role ID must contain digits only"));
    }
    let old = load(&state).await?;
    let webhook = match i.webhook_url.trim() {
        "" => old.webhook_url.clone(),
        "-" => String::new(),
        w => w.to_string(),
    };
    let invite = i.invite_url.trim().to_string();
    if !valid_webhook(&webhook) {
        return Err(AppError::bad_request("that is not a Discord webhook address"));
    }
    let channel_id = i.channel_id.trim().to_string();
    if !valid_channel_id(&channel_id) {
        return Err(AppError::bad_request(
            "a channel ID is a long number: in Discord turn on Developer Mode, then right click the channel and choose Copy Channel ID",
        ));
    }
    if !valid_invite(&invite) {
        return Err(AppError::bad_request("invite links look like https://discord.gg/yourcode"));
    }
    let public_key = match i.public_key.as_deref().map(str::trim) {
        None => old.public_key.clone(),
        Some(k) if k.is_empty() => String::new(),
        Some(k) if k.len() == 64 && k.bytes().all(|b| b.is_ascii_hexdigit()) => k.to_lowercase(),
        Some(_) => {
            return Err(AppError::bad_request(
                "The public key is 64 hex characters: copy it from Developer Portal → your application → General Information",
            ))
        }
    };
    let next = DiscordSettings {
        role_sync: i.role_sync,
        base_role_id: i.base_role_id,
        invite_url: invite,
        channel_id,
        webhook_url: webhook,
        notify_achievements: i.notify_achievements,
        notify_guilds: i.notify_guilds,
        notify_members: i.notify_members,
        public_key: public_key,
    };
    store::kv_set(&state, "discord_settings", &next).await?;
    Ok(Json(view(&next)))
}

/// What launchers show: the invite link, and whether the account is linked.
pub async fn public_info(State(state): State<AppState>, AuthUser(user): AuthUser) -> AppResult<Json<Value>> {
    let s = load(&state).await?;
    let linked: Option<(String, String)> =
        sqlx::query_as("SELECT provider_id, display_name FROM account_connections WHERE user_id = ? AND provider = 'discord'")
            .bind(user.id)
            .fetch_optional(&state.db)
            .await?;
    Ok(Json(json!({ "invite_url": s.invite_url, "linked": linked.is_some(), "display_name": linked.map(|l| l.1) })))
}

// ---------------------------------------------------------------------------
// Role mapping
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct RoleMapping {
    pub discord_role: String,
}

pub async fn set_role_mapping(
    _: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(m): Json<RoleMapping>,
) -> AppResult<Json<Value>> {
    let role = m.discord_role.trim();
    if !role.bytes().all(|b| b.is_ascii_digit()) || role.len() > 24 {
        return Err(AppError::bad_request("Discord role IDs are numbers"));
    }
    let done = if state.instance_id.is_some() {
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM platform.groups WHERE id=?)").bind(id).fetch_one(&state.db).await?;
        if !exists {
            return Err(AppError::not_found("group not found"));
        }
        sqlx::query("INSERT INTO experience_group_links(group_id,discord_role) VALUES(?,?) ON CONFLICT(group_id) DO UPDATE SET discord_role=excluded.discord_role")
                .bind(id).bind(role).execute(&state.db).await?
    } else {
        sqlx::query("UPDATE groups SET discord_role=? WHERE id=?").bind(role).bind(id).execute(&state.platform_db).await?
    };
    if done.rows_affected() == 0 {
        return Err(AppError::not_found("group not found"));
    }
    Ok(Json(json!({ "ok": true })))
}

/// What to change for one person. Only ever additions.
#[derive(Debug, Default, PartialEq)]
pub struct Plan {
    pub add_groups: Vec<i64>,
    pub add_roles: Vec<String>,
}

/// `mapping` is (group id, discord role id); `member_roles` are the Discord roles
/// the person has; `groups` the panel groups they are in.
pub fn plan(mode: &str, mapping: &[(i64, String)], member_roles: &[String], groups: &[i64]) -> Plan {
    let mut p = Plan::default();
    let from_discord = matches!(mode, "discord_to_panel" | "both");
    let to_discord = matches!(mode, "panel_to_discord" | "both");
    for (group, role) in mapping.iter().filter(|(_, r)| !r.is_empty()) {
        let has_role = member_roles.contains(role);
        let in_group = groups.contains(group);
        if from_discord && has_role && !in_group {
            p.add_groups.push(*group);
        }
        if to_discord && in_group && !has_role {
            p.add_roles.push(role.clone());
        }
    }
    p
}

/// Discord's API address. Only tests and self-hosted API proxies ever change it (`SCOPENET_DISCORD_API`).
pub(crate) fn api_base() -> String {
    std::env::var("SCOPENET_DISCORD_API")
        .ok()
        .filter(|v| v.starts_with("http"))
        .unwrap_or_else(|| API.to_string())
        .trim_end_matches('/')
        .to_string()
}

/// A request to the Discord API as the bot. `path` starts with a slash, e.g. `/channels/1/messages`.
pub(crate) fn bot_request(state: &AppState, method: reqwest::Method, path: String, token: &str) -> reqwest::RequestBuilder {
    state
        .http
        .request(method, format!("{}{path}", api_base()))
        .header("Authorization", format!("Bot {token}"))
        .timeout(Duration::from_secs(10))
}

/// Where a message is delivered: a channel (posted by the bot) or, for older setups, a webhook address.
pub enum Dest {
    Channel { id: String, token: String },
    Webhook(String),
}

/// What Discord's refusal means in plain words, for the person setting this up.
fn explain(status: u16, what: &str) -> AppError {
    AppError::bad_request(match status {
        401 => "Discord rejected the bot token. Check it under Settings → Discord (Bot token).".to_string(),
        403 => format!("The bot isn't allowed to {what}. Add it to your server (Settings → Discord has an invite link) and give it View Channel, Send Messages and Embed Links in that channel."),
        404 => format!("Discord can't find that {}. Check the channel ID, and that the bot is in the server.", if what.contains("message") { "message or channel" } else { "channel" }),
        429 => "Discord is rate limiting the bot. It will retry on the next update.".to_string(),
        s => format!("Discord refused the request (HTTP {s})."),
    })
}

impl Dest {
    /// The destination a feature should use: its own channel, else its own webhook, else the main announcement settings.
    /// `None` means nothing is set up at all.
    pub async fn resolve(state: &AppState, channel_id: &str, webhook_url: &str) -> AppResult<Option<Dest>> {
        let main = load(state).await?;
        let (channel, webhook) = if channel_id.trim().is_empty() && webhook_url.trim().is_empty() {
            (main.channel_id, main.webhook_url)
        } else {
            (channel_id.trim().to_string(), webhook_url.trim().to_string())
        };
        if !channel.is_empty() {
            let conn: ConnectionsSettings = store::kv_get(state, "connections_settings").await?;
            if conn.discord_bot_token.is_empty() {
                return Err(AppError::bad_request("save the Discord bot token under Settings → Discord first"));
            }
            return Ok(Some(Dest::Channel { id: channel, token: conn.discord_bot_token }));
        }
        if !webhook.is_empty() && valid_webhook(&webhook) {
            return Ok(Some(Dest::Webhook(webhook)));
        }
        Ok(None)
    }

    fn body(&self, body: &Value) -> Value {
        let mut body = body.clone();
        if matches!(self, Dest::Channel { .. }) {
            // A webhook's custom name and picture don't exist for bot messages.
            if let Some(o) = body.as_object_mut() {
                o.remove("username");
                o.remove("avatar_url");
            }
        }
        body
    }

    /// Post a message and return its id (so it can be edited later).
    pub async fn send(&self, state: &AppState, body: &Value) -> AppResult<String> {
        let req = match self {
            Dest::Channel { id, token } => bot_request(state, reqwest::Method::POST, format!("/channels/{id}/messages"), token),
            Dest::Webhook(url) => state.http.post(format!("{url}?wait=true")).timeout(Duration::from_secs(10)),
        };
        let resp =
            req.json(&self.body(body)).send().await.map_err(|e| AppError::bad_request(format!("Discord could not be reached: {e}")))?;
        if !resp.status().is_success() {
            return Err(explain(resp.status().as_u16(), "post in that channel"));
        }
        let v: Value = resp.json().await.map_err(|_| AppError::bad_request("Discord sent an unexpected answer"))?;
        Ok(v["id"].as_str().unwrap_or_default().to_string())
    }

    /// Replace a message's content. `Ok(false)` means the message no longer exists (it was deleted in Discord).
    pub async fn edit(&self, state: &AppState, message_id: &str, body: &Value) -> AppResult<bool> {
        let req = match self {
            Dest::Channel { id, token } => {
                bot_request(state, reqwest::Method::PATCH, format!("/channels/{id}/messages/{message_id}"), token)
            }
            Dest::Webhook(url) => state.http.patch(format!("{url}/messages/{message_id}")).timeout(Duration::from_secs(10)),
        };
        let resp =
            req.json(&self.body(body)).send().await.map_err(|e| AppError::bad_request(format!("Discord could not be reached: {e}")))?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(false);
        }
        if !resp.status().is_success() {
            return Err(explain(resp.status().as_u16(), "edit its message"));
        }
        Ok(true)
    }
}

/// The text channels of the Discord server, for a picker (so nobody has to hunt for IDs).
pub async fn channels(_: AdminUser, State(state): State<AppState>) -> AppResult<Json<Value>> {
    let c: ConnectionsSettings = store::kv_get(&state, "connections_settings").await?;
    if c.discord_bot_token.is_empty() || c.discord_guild_id.is_empty() {
        return Err(AppError::bad_request("save the Discord bot token and server ID under Settings → Discord first"));
    }
    let resp = bot_request(&state, reqwest::Method::GET, format!("/guilds/{}/channels", c.discord_guild_id), &c.discord_bot_token)
        .send()
        .await
        .map_err(|_| AppError::bad_request("could not reach Discord"))?;
    if !resp.status().is_success() {
        return Err(match resp.status().as_u16() {
            401 => explain(401, ""),
            404 => AppError::bad_request("Discord can't find that server. Check the server ID, and that the bot has been added to it."),
            s => explain(s, "list the server's channels"),
        });
    }
    let list: Vec<Value> = resp.json().await.map_err(|_| AppError::bad_request("unexpected Discord response"))?;
    let category = |id: &Value| list.iter().find(|c| c["id"] == *id).and_then(|c| c["name"].as_str()).unwrap_or("").to_string();
    let mut out: Vec<(i64, i64, Value)> = list
        .iter()
        // 0 = text, 5 = announcement
        .filter(|c| matches!(c["type"].as_i64(), Some(0) | Some(5)))
        .map(|c| {
            let parent = category(&c["parent_id"]);
            (
                list.iter().find(|p| p["id"] == c["parent_id"]).and_then(|p| p["position"].as_i64()).unwrap_or(-1),
                c["position"].as_i64().unwrap_or(0),
                json!({ "id": c["id"], "name": c["name"], "category": parent }),
            )
        })
        .collect();
    out.sort_by_key(|(cat, pos, _)| (*cat, *pos));
    Ok(Json(json!(out.into_iter().map(|(_, _, v)| v).collect::<Vec<_>>())))
}

pub async fn roles(_: AdminUser, State(state): State<AppState>) -> AppResult<Json<Value>> {
    let c: ConnectionsSettings = store::kv_get(&state, "connections_settings").await?;
    if c.discord_bot_token.is_empty() || c.discord_guild_id.is_empty() {
        return Err(AppError::bad_request("save a Discord bot token and server ID in Settings first"));
    }
    let resp = bot_request(&state, reqwest::Method::GET, format!("/guilds/{}/roles", c.discord_guild_id), &c.discord_bot_token)
        .send()
        .await
        .map_err(|_| AppError::bad_request("could not reach Discord"))?;
    if !resp.status().is_success() {
        return Err(AppError::bad_request("Discord refused the bot token or server ID"));
    }
    let list: Vec<Value> = resp.json().await.map_err(|_| AppError::bad_request("unexpected Discord response"))?;
    let mut out: Vec<Value> = list
        .into_iter()
        .filter(|r| r["name"] != "@everyone" && !r["managed"].as_bool().unwrap_or(false))
        .map(|r| json!({ "id": r["id"], "name": r["name"], "color": r["color"], "position": r["position"] }))
        .collect();
    out.sort_by_key(|r| std::cmp::Reverse(r["position"].as_i64().unwrap_or(0)));
    Ok(Json(json!(out)))
}

#[derive(Default, Serialize)]
pub struct SyncReport {
    pub checked: u32,
    pub not_in_server: u32,
    pub groups_added: u32,
    pub roles_added: u32,
    pub roles_removed: u32,
    pub failed: u32,
}

/// Sync every linked account. Safe to run repeatedly.
pub async fn sync_all(state: &AppState) -> AppResult<SyncReport> {
    let settings = load(state).await?;
    let mut report = SyncReport::default();
    if settings.role_sync == "off" {
        return Ok(report);
    }
    let c: ConnectionsSettings = store::kv_get(state, "connections_settings").await?;
    if c.discord_bot_token.is_empty() || c.discord_guild_id.is_empty() {
        return Err(AppError::bad_request("save a Discord bot token and server ID in Settings first"));
    }
    let mapping: Vec<(i64, String)> =
        sqlx::query_as("SELECT id, discord_role FROM groups WHERE discord_role <> ''").fetch_all(&state.db).await?;
    let rank_roles: Vec<(i64, String)> = sqlx::query_as("SELECT level_req, json_extract(reward_data, '$.discord_role_id') FROM level_rewards WHERE level_type='global' AND reward_type='title' AND json_extract(reward_data, '$.discord_role_id') IS NOT NULL AND json_extract(reward_data, '$.discord_role_id') <> '' ORDER BY level_req DESC")
        .fetch_all(&state.db).await?;
    if mapping.is_empty() && rank_roles.is_empty() && settings.base_role_id.is_empty() {
        return Ok(report);
    }
    let linked: Vec<(i64, String, String)> = sqlx::query_as(
        "SELECT c.user_id, c.provider_id, u.uuid FROM account_connections c JOIN users u ON u.id=c.user_id WHERE c.provider = 'discord'",
    )
    .fetch_all(&state.db)
    .await?;
    for (user_id, discord_id, uuid) in linked {
        report.checked += 1;
        let resp =
            bot_request(state, reqwest::Method::GET, format!("/guilds/{}/members/{discord_id}", c.discord_guild_id), &c.discord_bot_token)
                .send()
                .await;
        let resp = match resp {
            Ok(r) => r,
            Err(_) => {
                report.failed += 1;
                continue;
            }
        };
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            report.not_in_server += 1;
            continue;
        }
        if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            // Discord asked us to slow down; the next run finishes the rest.
            report.failed += 1;
            break;
        }
        if !resp.status().is_success() {
            report.failed += 1;
            continue;
        }
        let Ok(member) = resp.json::<Value>().await else {
            report.failed += 1;
            continue;
        };
        let roles: Vec<String> =
            member["roles"].as_array().map(|a| a.iter().filter_map(|r| r.as_str().map(String::from)).collect()).unwrap_or_default();
        let groups: Vec<i64> =
            sqlx::query_scalar("SELECT group_id FROM user_groups WHERE user_id = ?").bind(user_id).fetch_all(&state.db).await?;
        let p = plan(&settings.role_sync, &mapping, &roles, &groups);
        let xp: i64 =
            sqlx::query_scalar("SELECT global_xp FROM user_levels WHERE uuid=?").bind(&uuid).fetch_optional(&state.db).await?.unwrap_or(0);
        let level = crate::progression::load_pool(&state.db).await?.level_from_xp(xp).0;
        let chosen = rank_roles
            .iter()
            .find(|(required, _)| *required <= level)
            .map(|(_, role)| role.clone())
            .or_else(|| (!settings.base_role_id.is_empty()).then(|| settings.base_role_id.clone()));
        for g in p.add_groups {
            sqlx::query("INSERT OR IGNORE INTO user_groups (user_id, group_id) VALUES (?, ?)")
                .bind(user_id)
                .bind(g)
                .execute(&state.platform_db)
                .await?;
            report.groups_added += 1;
        }
        // A level title takes precedence over mapped panel groups on Discord.
        let wanted_roles = if let Some(role) = &chosen {
            if roles.contains(role) {
                Vec::new()
            } else {
                vec![role.clone()]
            }
        } else {
            p.add_roles
        };
        for role in wanted_roles {
            let done = bot_request(
                state,
                reqwest::Method::PUT,
                format!("/guilds/{}/members/{discord_id}/roles/{role}", c.discord_guild_id),
                &c.discord_bot_token,
            )
            .header("Content-Length", "0")
            .send()
            .await;
            match done {
                Ok(r) if r.status().is_success() => report.roles_added += 1,
                _ => report.failed += 1,
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        if let Some(chosen) = chosen {
            let mut managed: Vec<String> = rank_roles.iter().map(|(_, role)| role.clone()).collect();
            if !settings.base_role_id.is_empty() {
                managed.push(settings.base_role_id.clone());
            }
            managed.sort();
            managed.dedup();
            for role in managed.into_iter().filter(|r| r != &chosen && roles.contains(r)) {
                let done = bot_request(
                    state,
                    reqwest::Method::DELETE,
                    format!("/guilds/{}/members/{discord_id}/roles/{role}", c.discord_guild_id),
                    &c.discord_bot_token,
                )
                .send()
                .await;
                match done {
                    Ok(r) if r.status().is_success() => report.roles_removed += 1,
                    _ => report.failed += 1,
                }
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
        }
        tokio::time::sleep(Duration::from_millis(120)).await;
    }
    Ok(report)
}

pub async fn sync_now(_: AdminUser, State(state): State<AppState>) -> AppResult<Json<SyncReport>> {
    if load(&state).await?.role_sync == "off" {
        return Err(AppError::bad_request("turn on a role sync mode first"));
    }
    Ok(Json(sync_all(&state).await?))
}

// ---------------------------------------------------------------------------
// Announcements
// ---------------------------------------------------------------------------

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct Cursor {
    achievements: String,
    guilds: String,
    members: String,
}

fn clip(s: &str) -> String {
    s.chars().take(200).collect()
}

/// Post what happened since the last look. The first run starts from "now".
pub async fn announce(state: &AppState) -> AppResult<u32> {
    let s = load(state).await?;
    if s.channel_id.is_empty() && (s.webhook_url.is_empty() || !valid_webhook(&s.webhook_url)) {
        return Ok(0);
    }
    let now = crate::db::now();
    let mut cursor: Cursor = store::kv_get(state, "discord_cursor").await?;
    if cursor.achievements.is_empty() {
        cursor = Cursor { achievements: now.clone(), guilds: now.clone(), members: now.clone() };
        store::kv_set(state, "discord_cursor", &cursor).await?;
        return Ok(0);
    }
    let mut sent = 0;
    if s.notify_achievements {
        let rows: Vec<(String, String, String, String, i64, String, String)> = sqlx::query_as(
            "SELECT ua.user_uuid, COALESCE(u.username, ua.user_uuid), a.title, a.description, a.xp_reward, a.category, ua.unlocked_at FROM user_achievements ua
             JOIN achievements a ON a.id = ua.achievement_id LEFT JOIN users u ON u.uuid = ua.user_uuid
             WHERE ua.unlocked_at > ? ORDER BY ua.unlocked_at LIMIT 10",
        )
        .bind(&cursor.achievements)
        .fetch_all(&state.db)
        .await?;
        for (uuid, who, title, desc, xp, category, at) in rows {
            let mut vars = embeds::player_vars(state, &uuid, &who).await;
            for (k, v) in [
                ("achievement", embeds::plain(&title)),
                ("achievement_description", embeds::plain(&desc)),
                ("xp", xp.to_string()),
                ("category", embeds::plain(&category)),
            ] {
                vars.insert(k.into(), v);
            }
            if embeds::announce_event(state, "achievement", &vars).await.unwrap_or(false) {
                sent += 1;
            }
            cursor.achievements = at;
        }
    }
    if s.notify_guilds {
        let rows: Vec<(String, String)> =
            sqlx::query_as("SELECT id, created_at FROM guilds WHERE created_at > ? ORDER BY created_at LIMIT 10")
                .bind(&cursor.guilds)
                .fetch_all(&state.db)
                .await?;
        for (id, at) in rows {
            let vars = embeds::guild_vars(state, &id).await;
            if embeds::announce_event(state, "guild", &vars).await.unwrap_or(false) {
                sent += 1;
            }
            cursor.guilds = at;
        }
    }
    if s.notify_members {
        let rows: Vec<(String, String, String)> = sqlx::query_as(
            "SELECT uuid, username, created_at FROM users WHERE status = 'active' AND created_at > ? ORDER BY created_at LIMIT 10",
        )
        .bind(&cursor.members)
        .fetch_all(&state.db)
        .await?;
        for (uuid, name, at) in rows {
            let mut vars = embeds::player_vars(state, &uuid, &name).await;
            let total: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE status = 'active'").fetch_one(&state.db).await.unwrap_or(0);
            vars.insert("members_total".into(), total.to_string());
            if embeds::announce_event(state, "member", &vars).await.unwrap_or(false) {
                sent += 1;
            }
            cursor.members = at;
        }
    }
    store::kv_set(state, "discord_cursor", &cursor).await?;
    Ok(sent)
}

/// Send a test announcement to the configured channel, so setup problems show up here and not silently later.
pub async fn test_announce(_: AdminUser, State(state): State<AppState>) -> AppResult<Json<Value>> {
    let Some(dest) = Dest::resolve(&state, "", "").await? else {
        return Err(AppError::bad_request("enter the announcement channel ID and save first"));
    };
    let body = json!({ "username": "SCOPENET", "allowed_mentions": { "parse": [] },
        "embeds": [{ "title": "SCOPENET is connected", "description": "Announcements will appear in this channel.", "color": 0x8b6cff }] });
    dest.send(&state, &body).await?;
    Ok(Json(json!({ "ok": true })))
}

/// Background loop: announcements every 30 s, role sync every 15 min.
pub fn spawn_worker(state: AppState) {
    tokio::spawn(async move {
        let mut tick = 0u64;
        let mut live_last = std::collections::BTreeMap::new();
        loop {
            tokio::time::sleep(Duration::from_secs(30)).await;
            if state.db.is_closed() {
                return;
            }
            let started = std::time::Instant::now();
            match announce(&state).await {
                Ok(0) => {}
                Ok(n) => crate::scheduler::record(&state, "discord_announce", started, &Ok(format!("posted {n} announcements"))).await,
                Err(e) => {
                    tracing::debug!("discord announce: {e:?}");
                    crate::scheduler::record(&state, "discord_announce", started, &Err(e.message)).await;
                }
            }
            embeds::tick_live(&state, &mut live_last).await;
            tick += 1;
            if tick % 30 == 0 {
                let started = std::time::Instant::now();
                match sync_all(&state).await {
                    Ok(r) => {
                        let msg = format!("checked {} accounts, {} roles added, {} removed", r.checked, r.roles_added, r.roles_removed);
                        crate::scheduler::record(&state, "discord_role_sync", started, &Ok(msg)).await;
                    }
                    Err(e) => {
                        tracing::debug!("discord sync: {e:?}");
                        // "turn on a role sync mode" is a setting, not a fault.
                        if e.status != axum::http::StatusCode::BAD_REQUEST {
                            crate::scheduler::record(&state, "discord_role_sync", started, &Err(e.message)).await;
                        }
                    }
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map() -> Vec<(i64, String)> {
        vec![(1, "100".into()), (2, "200".into()), (3, String::new())]
    }

    #[test]
    fn off_changes_nothing() {
        assert_eq!(plan("off", &map(), &["100".into()], &[2]), Plan::default());
    }

    #[test]
    fn discord_roles_add_panel_groups() {
        let p = plan("discord_to_panel", &map(), &["100".into(), "999".into()], &[]);
        assert_eq!(p, Plan { add_groups: vec![1], add_roles: vec![] });
    }

    #[test]
    fn panel_groups_add_discord_roles_and_unmapped_are_ignored() {
        let p = plan("panel_to_discord", &map(), &[], &[2, 3]);
        assert_eq!(p, Plan { add_groups: vec![], add_roles: vec!["200".into()] });
    }

    #[test]
    fn both_never_removes() {
        let p = plan("both", &map(), &["100".into()], &[2]);
        assert_eq!(p, Plan { add_groups: vec![1], add_roles: vec!["200".into()] });
        // Already in sync: nothing to do.
        assert_eq!(plan("both", &map(), &["100".into()], &[1]), Plan::default());
    }

    #[test]
    fn webhook_and_invite_addresses_are_checked() {
        assert!(valid_webhook("https://discord.com/api/webhooks/1/abc"));
        assert!(!valid_webhook("https://evil.example/api/webhooks/1"));
        assert!(valid_invite("https://discord.gg/abc"));
        assert!(!valid_invite("http://discord.gg/abc"));
    }
}
