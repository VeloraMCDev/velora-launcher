//! Things you can do from the map: ask a player to teleport, message them in game, and invite them to your guild.
//!
//! Teleports and messages are queued for the game server (`server_actions`), which collects them every few seconds
//! and carries them out with its normal rules. Guild invitations are real invitations: the other player accepts
//! or declines, in the launcher or in game.

use crate::state::RequestState as State;
use crate::auth::AuthUser;
use crate::error::{AppError, AppResult};
use crate::routes::servers::{get_server, GameServer};
use crate::state::AppState;
use axum::extract::{Path};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::SqliteConnection;

const ACTION_BURST: i64 = 6;
const ACTION_WINDOW_SECS: i64 = 30;

fn ago(secs: i64) -> String {
    (chrono::Utc::now() - chrono::Duration::seconds(secs)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

async fn online_on(conn: &mut SqliteConnection, server_id: i64, uuid: &str) -> AppResult<bool> {
    Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM server_online WHERE server_id = ? AND uuid = ?)")
        .bind(server_id)
        .bind(uuid)
        .fetch_one(&mut *conn)
        .await?)
}

/// Keep only printable characters, collapse whitespace, and cut to `max`.
fn clean(text: &str, max: usize) -> String {
    text.chars()
        .filter(|c| c.is_whitespace() || !c.is_control())
        .map(|c| if c.is_whitespace() { ' ' } else { c })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max)
        .collect()
}

#[derive(Deserialize)]
pub struct MapAction {
    /// `tpa` or `message`.
    pub action: String,
    pub target_uuid: String,
    #[serde(default)]
    pub text: String,
}

/// `POST /servers/{id}/map/actions`: a signed-in player acts on another player they clicked on the map.
pub async fn map_action(
    auth: AuthUser,
    Path(server_id): Path<i64>,
    State(state): State<AppState>,
    Json(a): Json<MapAction>,
) -> AppResult<Json<Value>> {
    get_server(&state, server_id).await?;
    if a.target_uuid == auth.uuid {
        return Err(AppError::bad_request("That's you"));
    }
    let mut conn = state.db.acquire().await?;
    let active: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE uuid = ? AND status = 'active')")
        .bind(&a.target_uuid)
        .fetch_one(&mut *conn)
        .await?;
    if !active {
        return Err(AppError::not_found("Player not found"));
    }
    let recent: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM server_actions WHERE from_uuid = ? AND created_at > ?")
        .bind(&auth.uuid)
        .bind(ago(ACTION_WINDOW_SECS))
        .fetch_one(&mut *conn)
        .await?;
    if recent >= ACTION_BURST {
        return Err(AppError::new(axum::http::StatusCode::TOO_MANY_REQUESTS, "Slow down a little and try again"));
    }
    if !online_on(&mut conn, server_id, &a.target_uuid).await? {
        return Err(AppError::bad_request("They aren't online on this server right now"));
    }
    let text = match a.action.as_str() {
        "tpa" => {
            if !online_on(&mut conn, server_id, &auth.uuid).await? {
                return Err(AppError::bad_request("Join the server first: a teleport request needs you in game"));
            }
            String::new()
        }
        "message" => {
            let text = clean(&a.text, 200);
            if text.is_empty() {
                return Err(AppError::bad_request("Write a message first"));
            }
            text
        }
        _ => return Err(AppError::bad_request("unknown action")),
    };
    sqlx::query(
        "INSERT INTO server_actions (server_id, kind, from_uuid, from_name, to_uuid, text, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(server_id)
    .bind(&a.action)
    .bind(&auth.uuid)
    .bind(&auth.username)
    .bind(&a.target_uuid)
    .bind(&text)
    .bind(crate::db::now())
    .execute(&mut *conn)
    .await?;
    Ok(Json(json!({ "ok": true })))
}

/// `POST /api/server/v1/actions/poll`: the game server collects what is waiting for it (each action is handed out once).
pub async fn poll_actions(GameServer(server): GameServer, State(state): State<AppState>) -> AppResult<Json<Value>> {
    let mut tx = state.db.begin().await?;
    // Stale ones (a server that was offline) are dropped rather than replayed minutes later.
    sqlx::query("UPDATE server_actions SET taken_at = ? WHERE server_id = ? AND taken_at IS NULL AND created_at < ?")
        .bind(crate::db::now())
        .bind(server.id)
        .bind(ago(120))
        .execute(&mut *tx)
        .await?;
    let rows: Vec<(i64, String, String, String, String, String)> = sqlx::query_as(
        "SELECT id, kind, from_uuid, from_name, to_uuid, text FROM server_actions WHERE server_id = ? AND taken_at IS NULL ORDER BY id LIMIT 50",
    )
    .bind(server.id)
    .fetch_all(&mut *tx)
    .await?;
    let now = crate::db::now();
    for (id, ..) in &rows {
        sqlx::query("UPDATE server_actions SET taken_at = ? WHERE id = ?").bind(&now).bind(id).execute(&mut *tx).await?;
    }
    // Old handled rows are only useful for rate limiting.
    sqlx::query("DELETE FROM server_actions WHERE taken_at IS NOT NULL AND created_at < ?").bind(ago(3600)).execute(&mut *tx).await?;
    tx.commit().await?;
    let actions: Vec<Value> = rows
        .into_iter()
        .map(|(id, kind, from_uuid, from_name, to_uuid, text)| json!({ "id": id, "kind": kind, "from_uuid": from_uuid, "from_name": from_name, "to_uuid": to_uuid, "text": text }))
        .collect();
    Ok(Json(json!({ "actions": actions })))
}

/// Queue a line of chat for a player on every server where they are online.
pub async fn tell_player(conn: &mut SqliteConnection, to_uuid: &str, text: &str) -> AppResult<()> {
    let servers: Vec<i64> =
        sqlx::query_scalar("SELECT server_id FROM server_online WHERE uuid = ?").bind(to_uuid).fetch_all(&mut *conn).await?;
    for id in servers {
        sqlx::query("INSERT INTO server_actions (server_id, kind, to_uuid, text, created_at) VALUES (?, 'notify', ?, ?, ?)")
            .bind(id)
            .bind(to_uuid)
            .bind(text)
            .bind(crate::db::now())
            .execute(&mut *conn)
            .await?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Guild invitations
// ---------------------------------------------------------------------------

/// Invite `target_uuid` to `guild_id` on behalf of `inviter_uuid` (who must lead or officiate it).
pub async fn create_invite(
    state: &AppState,
    inviter_uuid: &str,
    inviter_name: &str,
    guild_id: &str,
    target_uuid: &str,
) -> AppResult<Value> {
    if inviter_uuid == target_uuid {
        return Err(AppError::bad_request("You can't invite yourself"));
    }
    let mut conn = state.db.acquire().await?;
    let role: Option<String> = sqlx::query_scalar("SELECT role FROM guild_members WHERE guild_id = ? AND uuid = ?")
        .bind(guild_id)
        .bind(inviter_uuid)
        .fetch_optional(&mut *conn)
        .await?;
    let allowed = match role.as_deref() {
        Some("leader" | "officer") => true,
        Some(custom) => {
            sqlx::query_scalar::<_, i64>("SELECT can_invite FROM guild_roles WHERE guild_id=? AND name=?")
                .bind(guild_id)
                .bind(custom)
                .fetch_optional(&mut *conn)
                .await?
                .unwrap_or(0)
                != 0
        }
        None => false,
    };
    if !allowed {
        return Err(AppError::forbidden("Your guild role cannot invite players"));
    }
    let target: Option<String> = sqlx::query_scalar("SELECT username FROM users WHERE uuid = ? AND status = 'active'")
        .bind(target_uuid)
        .fetch_optional(&mut *conn)
        .await?;
    let target_name = target.ok_or_else(|| AppError::not_found("Player not found"))?;
    let (name, tag, instance): (String, String, String) =
        sqlx::query_as("SELECT name, tag, instance_id FROM guilds WHERE id = ?").bind(guild_id).fetch_one(&mut *conn).await?;
    let taken: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM guild_members gm JOIN guilds g ON g.id = gm.guild_id WHERE gm.uuid = ? AND g.instance_id = ?)",
    )
    .bind(target_uuid)
    .bind(&instance)
    .fetch_one(&mut *conn)
    .await?;
    if taken {
        return Err(AppError::bad_request(format!("{target_name} is already in a guild here")));
    }
    let recent: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM guild_invites WHERE inviter_uuid = ? AND created_at > ?")
        .bind(inviter_uuid)
        .bind(ago(60))
        .fetch_one(&mut *conn)
        .await?;
    if recent >= 10 {
        return Err(AppError::new(axum::http::StatusCode::TOO_MANY_REQUESTS, "Too many invitations; wait a minute"));
    }
    // One open invitation per guild and player.
    sqlx::query("DELETE FROM guild_invites WHERE guild_id = ? AND target_uuid = ? AND status = 'pending'")
        .bind(guild_id)
        .bind(target_uuid)
        .execute(&mut *conn)
        .await?;
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO guild_invites (guild_id, inviter_uuid, inviter_name, target_uuid, created_at) VALUES (?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(guild_id)
    .bind(inviter_uuid)
    .bind(inviter_name)
    .bind(target_uuid)
    .bind(crate::db::now())
    .fetch_one(&mut *conn)
    .await?;
    tell_player(&mut conn, target_uuid, &format!("\u{a7}e{inviter_name} \u{a7}ainvited you to join \u{a7}6[{tag}] {name}\u{a7}a. \u{a7}7Type \u{a7}e/guild accept \u{a7}7or \u{a7}c/guild decline\u{a7}7, or answer in the launcher.")).await?;
    Ok(json!({ "ok": true, "id": id, "player": target_name }))
}

#[derive(Deserialize)]
pub struct InvitePayload {
    pub uuid: String,
}

pub async fn send_invite(
    auth: AuthUser,
    Path(guild_id): Path<String>,
    State(state): State<AppState>,
    Json(p): Json<InvitePayload>,
) -> AppResult<Json<Value>> {
    Ok(Json(create_invite(&state, &auth.uuid, &auth.username, &guild_id, &p.uuid).await?))
}

fn invite_json(row: (i64, String, String, String, String, String, String, String)) -> Value {
    let (id, guild_id, name, tag, icon, inviter, at, inviter_uuid) = row;
    json!({ "id": id, "guild_id": guild_id, "guild_name": name, "guild_tag": tag, "icon_url": icon, "inviter": inviter, "inviter_uuid": inviter_uuid, "created_at": at })
}

async fn pending_for(state: &AppState, uuid: &str) -> AppResult<Vec<Value>> {
    let rows: Vec<(i64, String, String, String, String, String, String, String)> = sqlx::query_as(
        "SELECT i.id, i.guild_id, g.name, g.tag, COALESCE(g.icon_url, ''), i.inviter_name, i.created_at, i.inviter_uuid
         FROM guild_invites i JOIN guilds g ON g.id = i.guild_id WHERE i.target_uuid = ? AND i.status = 'pending' ORDER BY i.id DESC LIMIT 20",
    )
    .bind(uuid)
    .fetch_all(&state.db)
    .await?;
    Ok(rows.into_iter().map(invite_json).collect())
}

pub async fn my_invites(auth: AuthUser, State(state): State<AppState>) -> AppResult<Json<Value>> {
    Ok(Json(json!(pending_for(&state, &auth.uuid).await?)))
}

/// Accept or decline. `invite` is an id, or empty to take the newest open one.
pub async fn respond(state: &AppState, uuid: &str, name: &str, invite: Option<i64>, tag: Option<&str>, accept: bool) -> AppResult<Value> {
    let mut tx = state.db.begin().await?;
    let row: Option<(i64, String)> = match (invite, tag) {
        (Some(id), _) => {
            sqlx::query_as("SELECT id, guild_id FROM guild_invites WHERE id = ? AND target_uuid = ? AND status = 'pending'")
                .bind(id)
                .bind(uuid)
                .fetch_optional(&mut *tx)
                .await?
        }
        (None, Some(tag)) => {
            sqlx::query_as(
                "SELECT i.id, i.guild_id FROM guild_invites i JOIN guilds g ON g.id = i.guild_id
             WHERE i.target_uuid = ? AND i.status = 'pending' AND g.tag = ? COLLATE NOCASE ORDER BY i.id DESC LIMIT 1",
            )
            .bind(uuid)
            .bind(tag)
            .fetch_optional(&mut *tx)
            .await?
        }
        (None, None) => {
            sqlx::query_as("SELECT id, guild_id FROM guild_invites WHERE target_uuid = ? AND status = 'pending' ORDER BY id DESC LIMIT 1")
                .bind(uuid)
                .fetch_optional(&mut *tx)
                .await?
        }
    };
    let Some((id, guild_id)) = row else { return Err(AppError::not_found("You have no open guild invitation")) };
    sqlx::query("UPDATE guild_invites SET status = ? WHERE id = ?")
        .bind(if accept { "accepted" } else { "declined" })
        .bind(id)
        .execute(&mut *tx)
        .await?;
    let (gname, gtag): (String, String) =
        sqlx::query_as("SELECT name, tag FROM guilds WHERE id = ?").bind(&guild_id).fetch_one(&mut *tx).await?;
    if accept {
        crate::routes::guilds::check_room(&mut tx, &guild_id).await?;
        let res = sqlx::query("INSERT INTO guild_members (guild_id, uuid, name, role, joined_at) VALUES (?, ?, ?, 'member', ?)")
            .bind(&guild_id)
            .bind(uuid)
            .bind(name)
            .bind(crate::db::now())
            .execute(&mut *tx)
            .await;
        if res.is_err() {
            // The database refuses a second guild on the same instance.
            return Err(AppError::bad_request("You're already in a guild here. Leave it first."));
        }
        sqlx::query("INSERT OR IGNORE INTO user_achievements (user_uuid, achievement_id, unlocked_at) VALUES (?, 'ach_guild_initiate', ?)")
            .bind(uuid)
            .bind(crate::db::now())
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(json!({ "ok": true, "accepted": accept, "guild": gname, "tag": gtag }))
}

pub async fn accept_invite(auth: AuthUser, Path(id): Path<i64>, State(state): State<AppState>) -> AppResult<Json<Value>> {
    Ok(Json(respond(&state, &auth.uuid, &auth.username, Some(id), None, true).await?))
}

pub async fn decline_invite(auth: AuthUser, Path(id): Path<i64>, State(state): State<AppState>) -> AppResult<Json<Value>> {
    Ok(Json(respond(&state, &auth.uuid, &auth.username, Some(id), None, false).await?))
}

// ---- the same, asked by a game server on a player's behalf ---------------------------------

#[derive(Deserialize)]
pub struct ServerInvite {
    /// The player asking (inviter, or the invited player when answering).
    pub uuid: String,
    #[serde(default)]
    pub target: String,
    #[serde(default)]
    pub tag: String,
    #[serde(default)]
    pub accept: bool,
}

pub async fn server_invite_send(
    GameServer(server): GameServer,
    State(state): State<AppState>,
    Json(p): Json<ServerInvite>,
) -> AppResult<Json<Value>> {
    let inviter: Option<String> =
        sqlx::query_scalar("SELECT username FROM users WHERE uuid = ?").bind(&p.uuid).fetch_optional(&state.db).await?;
    let inviter = inviter.ok_or_else(|| AppError::not_found("Account not found"))?;
    let target: Option<String> = sqlx::query_scalar("SELECT uuid FROM users WHERE username = ? COLLATE NOCASE")
        .bind(p.target.trim())
        .fetch_optional(&state.db)
        .await?;
    let target = target.ok_or_else(|| AppError::not_found("No player with that name"))?;
    let guild: Option<String> = sqlx::query_scalar(
        "SELECT g.id FROM guild_members gm JOIN guilds g ON g.id = gm.guild_id WHERE gm.uuid = ? AND g.instance_id = ? AND gm.role IN ('leader', 'officer')",
    )
    .bind(&p.uuid)
    .bind(&server.instance_id)
    .fetch_optional(&state.db)
    .await?;
    let guild = guild.ok_or_else(|| AppError::forbidden("You must lead or officiate a guild to invite players"))?;
    Ok(Json(create_invite(&state, &p.uuid, &inviter, &guild, &target).await?))
}

pub async fn server_invites(
    GameServer(_): GameServer,
    State(state): State<AppState>,
    Json(p): Json<ServerInvite>,
) -> AppResult<Json<Value>> {
    Ok(Json(json!({ "invites": pending_for(&state, &p.uuid).await? })))
}

pub async fn server_invite_respond(
    GameServer(_): GameServer,
    State(state): State<AppState>,
    Json(p): Json<ServerInvite>,
) -> AppResult<Json<Value>> {
    let name: Option<String> =
        sqlx::query_scalar("SELECT username FROM users WHERE uuid = ?").bind(&p.uuid).fetch_optional(&state.db).await?;
    let name = name.ok_or_else(|| AppError::not_found("Account not found"))?;
    let tag = if p.tag.trim().is_empty() { None } else { Some(p.tag.trim().to_string()) };
    Ok(Json(respond(&state, &p.uuid, &name, None, tag.as_deref(), p.accept).await?))
}
