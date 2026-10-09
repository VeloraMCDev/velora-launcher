//! Friends, Direct Messages, Game Invites, Player Profiles, and Social Feed.

use crate::auth::{AuthUser, MaybeUser};
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::state::RequestState as State;
use axum::extract::{Path, Query};
use axum::Json;
use velora_shared::{DirectMessage, FriendInfo, GameInvite, UserPost, UserProfileView};
use serde::Deserialize;
use serde_json::Value;

// ---------------------------------------------------------------------------
// Friends System
// ---------------------------------------------------------------------------

pub async fn list_friends(auth: AuthUser, State(state): State<AppState>) -> AppResult<Json<Vec<FriendInfo>>> {
    let rows: Vec<(String, String, String, String, bool, Option<String>, i64)> = sqlx::query_as(
        "SELECT 
            (CASE WHEN f.user_uuid = ? THEN f.friend_uuid ELSE f.user_uuid END) as target_uuid,
            u.username,
            f.status,
            f.action_uuid,
            EXISTS(SELECT 1 FROM server_online so WHERE so.uuid = u.uuid) as online,
            (SELECT gs.name FROM server_online so JOIN game_servers gs ON gs.id = so.server_id WHERE so.uuid = u.uuid LIMIT 1) as playing_on,
            (SELECT COUNT(*) FROM direct_messages dm WHERE dm.recipient_uuid = ? AND dm.sender_uuid = u.uuid AND dm.is_read = 0) as unread
         FROM friendships f
         JOIN users u ON u.uuid = (CASE WHEN f.user_uuid = ? THEN f.friend_uuid ELSE f.user_uuid END)
         WHERE f.user_uuid = ? OR f.friend_uuid = ?
         ORDER BY online DESC, u.username ASC",
    )
    .bind(&auth.uuid)
    .bind(&auth.uuid)
    .bind(&auth.uuid)
    .bind(&auth.uuid)
    .bind(&auth.uuid)
    .fetch_all(&state.db)
    .await?;

    let list = rows
        .into_iter()
        .map(|(uuid, username, status, action_uuid, online, playing_on, unread)| {
            let final_status = if status == "accepted" {
                "accepted".to_string()
            } else if action_uuid == auth.uuid {
                "pending_outgoing".to_string()
            } else {
                "pending_incoming".to_string()
            };

            FriendInfo { uuid, username, status: final_status, online, playing_on, last_seen: None, unread }
        })
        .collect();

    Ok(Json(list))
}

#[derive(Deserialize)]
pub struct FriendRequestPayload {
    pub username: Option<String>,
    pub friend_username: Option<String>,
}

pub async fn send_friend_request(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<FriendRequestPayload>,
) -> AppResult<Json<Value>> {
    let name = payload
        .username
        .as_deref()
        .or(payload.friend_username.as_deref())
        .ok_or_else(|| AppError::bad_request("Username is required"))?
        .trim();

    let target: Option<(String, String)> =
        sqlx::query_as("SELECT uuid, username FROM users WHERE username = ? COLLATE NOCASE").bind(name).fetch_optional(&state.db).await?;

    let (target_uuid, target_name) = target.ok_or_else(|| AppError::not_found("Player not found"))?;

    if target_uuid == auth.uuid {
        return Err(AppError::bad_request("You cannot friend yourself"));
    }

    let now = chrono::Utc::now().to_rfc3339();

    // Check existing
    let existing: Option<(i64, String, String)> = sqlx::query_as(
        "SELECT id, status, action_uuid FROM friendships 
         WHERE (user_uuid = ? AND friend_uuid = ?) OR (user_uuid = ? AND friend_uuid = ?)",
    )
    .bind(&auth.uuid)
    .bind(&target_uuid)
    .bind(&target_uuid)
    .bind(&auth.uuid)
    .fetch_optional(&state.db)
    .await?;

    if let Some((_id, status, action)) = existing {
        if status == "accepted" {
            return Err(AppError::bad_request("Already friends with this player"));
        }
        if status == "pending" && action != auth.uuid {
            // Reciprocal request -> Auto-accept!
            sqlx::query("UPDATE friendships SET status = 'accepted', action_uuid = ?, updated_at = ? WHERE (user_uuid = ? AND friend_uuid = ?) OR (user_uuid = ? AND friend_uuid = ?)")
                .bind(&auth.uuid)
                .bind(&now)
                .bind(&auth.uuid)
                .bind(&target_uuid)
                .bind(&target_uuid)
                .bind(&auth.uuid)
                .execute(&state.platform_db)
                .await?;

            return Ok(Json(serde_json::json!({ "ok": true, "status": "accepted", "friend": target_name })));
        }
        return Err(AppError::bad_request("Friend request is already pending"));
    }

    sqlx::query(
        "INSERT INTO friendships (user_uuid, friend_uuid, status, action_uuid, created_at, updated_at)
         VALUES (?, ?, 'pending', ?, ?, ?)",
    )
    .bind(&auth.uuid)
    .bind(&target_uuid)
    .bind(&auth.uuid)
    .bind(&now)
    .bind(&now)
    .execute(&state.platform_db)
    .await?;

    Ok(Json(serde_json::json!({ "ok": true, "status": "pending_outgoing", "friend": target_name })))
}

pub async fn accept_friend_request(
    auth: AuthUser,
    Path(target_uuid): Path<String>,
    State(state): State<AppState>,
) -> AppResult<Json<Value>> {
    let now = chrono::Utc::now().to_rfc3339();

    let res = sqlx::query(
        "UPDATE friendships SET status = 'accepted', updated_at = ?
         WHERE ((user_uuid = ? AND friend_uuid = ?) OR (user_uuid = ? AND friend_uuid = ?))
           AND status = 'pending' AND action_uuid <> ?",
    )
    .bind(&now)
    .bind(&auth.uuid)
    .bind(&target_uuid)
    .bind(&target_uuid)
    .bind(&auth.uuid)
    .bind(&auth.uuid)
    .execute(&state.platform_db)
    .await?;

    if res.rows_affected() == 0 {
        return Err(AppError::bad_request("No pending request from this user found"));
    }

    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn remove_friend(auth: AuthUser, Path(target_uuid): Path<String>, State(state): State<AppState>) -> AppResult<Json<Value>> {
    sqlx::query(
        "DELETE FROM friendships
         WHERE (user_uuid = ? AND friend_uuid = ?) OR (user_uuid = ? AND friend_uuid = ?)",
    )
    .bind(&auth.uuid)
    .bind(&target_uuid)
    .bind(&target_uuid)
    .bind(&auth.uuid)
    .execute(&state.platform_db)
    .await?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Deserialize)]
pub struct RespondFriendPayload {
    pub target_uuid: Option<String>,
    pub friend_uuid: Option<String>,
    pub accept: bool,
}

pub async fn respond_friend_request(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<RespondFriendPayload>,
) -> AppResult<Json<Value>> {
    let target = payload.target_uuid.or(payload.friend_uuid).ok_or_else(|| AppError::bad_request("Friend UUID is required"))?;

    if payload.accept {
        accept_friend_request(auth, Path(target), State(state)).await
    } else {
        remove_friend(auth, Path(target), State(state)).await
    }
}

// ---------------------------------------------------------------------------
// Direct Messaging (DMs)
// ---------------------------------------------------------------------------

#[derive(Deserialize, Default)]
pub struct DmQuery {
    pub before_id: Option<i64>,
    pub mark_read: Option<bool>,
}

pub async fn get_direct_messages(
    auth: AuthUser,
    Path(target_uuid): Path<String>,
    Query(q): Query<DmQuery>,
    State(state): State<AppState>,
) -> AppResult<Json<Vec<DirectMessage>>> {
    // FIX #13: Support optional before_id cursor for pagination beyond the first 100 messages.
    let mut rows: Vec<(i64, String, String, String, String, bool, String)> = if let Some(before) = q.before_id {
        sqlx::query_as(
            "SELECT id, sender_uuid, sender_name, recipient_uuid, content, is_read, created_at
             FROM direct_messages
             WHERE ((sender_uuid = ? AND recipient_uuid = ?) OR (sender_uuid = ? AND recipient_uuid = ?))
               AND id < ?
             ORDER BY id DESC LIMIT 100",
        )
        .bind(&auth.uuid)
        .bind(&target_uuid)
        .bind(&target_uuid)
        .bind(&auth.uuid)
        .bind(before)
        .fetch_all(&state.db)
        .await?
    } else {
        sqlx::query_as(
            "SELECT id, sender_uuid, sender_name, recipient_uuid, content, is_read, created_at
             FROM direct_messages
             WHERE (sender_uuid = ? AND recipient_uuid = ?) OR (sender_uuid = ? AND recipient_uuid = ?)
             ORDER BY id DESC LIMIT 100",
        )
        .bind(&auth.uuid)
        .bind(&target_uuid)
        .bind(&target_uuid)
        .bind(&auth.uuid)
        .fetch_all(&state.db)
        .await?
    };

    rows.reverse();

    // A sidebar preview must not clear unread messages in the full conversation.
    if q.mark_read.unwrap_or(true) {
        sqlx::query(
            "UPDATE direct_messages SET is_read = 1
         WHERE recipient_uuid = ? AND sender_uuid = ? AND is_read = 0",
        )
        .bind(&auth.uuid)
        .bind(&target_uuid)
        .execute(&state.platform_db)
        .await?;
    }

    let list = rows
        .into_iter()
        .map(|(id, suuid, sname, ruuid, content, is_read, created)| DirectMessage {
            id,
            sender_uuid: suuid,
            sender_name: sname,
            recipient_uuid: ruuid,
            content,
            is_read,
            created_at: created,
        })
        .collect();

    Ok(Json(list))
}

#[derive(Deserialize)]
pub struct SendMessagePayload {
    pub content: String,
}

pub async fn send_direct_message(
    auth: AuthUser,
    Path(target_uuid): Path<String>,
    State(state): State<AppState>,
    Json(payload): Json<SendMessagePayload>,
) -> AppResult<Json<DirectMessage>> {
    let content = payload.content.trim();
    if content.is_empty() {
        return Err(AppError::bad_request("Message cannot be empty"));
    }

    if target_uuid == auth.uuid {
        return Err(AppError::bad_request("You can't message yourself"));
    }
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE uuid = ? AND status = 'active')")
        .bind(&target_uuid)
        .fetch_one(&state.db)
        .await?;
    if !exists {
        return Err(AppError::not_found("Player not found"));
    }
    if content.chars().count() > 1000 {
        return Err(AppError::bad_request("Messages can be up to 1000 characters"));
    }

    let now = chrono::Utc::now().to_rfc3339();

    let id: i64 = sqlx::query_scalar(
        "INSERT INTO direct_messages (sender_uuid, sender_name, recipient_uuid, content, is_read, created_at)
         VALUES (?, ?, ?, ?, 0, ?)
         RETURNING id",
    )
    .bind(&auth.uuid)
    .bind(&auth.username)
    .bind(&target_uuid)
    .bind(content)
    .bind(&now)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(DirectMessage {
        id,
        sender_uuid: auth.uuid.clone(),
        sender_name: auth.username.clone(),
        recipient_uuid: target_uuid,
        content: content.to_string(),
        is_read: false,
        created_at: now,
    }))
}

// ---------------------------------------------------------------------------
// Game Invites
// ---------------------------------------------------------------------------

pub async fn list_my_game_invites(auth: AuthUser, State(state): State<AppState>) -> AppResult<Json<Vec<GameInvite>>> {
    let rows: Vec<(String, String, String, String, String, String, Option<i64>, Option<String>, String, String, String)> = sqlx::query_as(
        "SELECT gi.id, gi.sender_uuid, gi.sender_name, gi.recipient_uuid,
                gi.instance_id, i.name as instance_name,
                gi.server_id, gs.name as server_name,
                gi.status, gi.created_at, gi.expires_at
         FROM game_invites gi
         JOIN instances i ON i.id = gi.instance_id
         LEFT JOIN game_servers gs ON gs.id = gi.server_id
         WHERE gi.recipient_uuid = ? AND gi.status = 'pending' AND gi.expires_at > ?
         ORDER BY gi.created_at DESC",
    )
    .bind(&auth.uuid)
    .bind(chrono::Utc::now().to_rfc3339())
    .fetch_all(&state.db)
    .await?;

    let list = rows
        .into_iter()
        .map(|(id, suuid, sname, ruuid, iid, iname, sid, sname_opt, status, cat, eat)| GameInvite {
            id,
            sender_uuid: suuid,
            sender_name: sname,
            recipient_uuid: ruuid,
            instance_id: iid,
            instance_name: iname,
            server_id: sid,
            server_name: sname_opt,
            status,
            created_at: cat,
            expires_at: eat,
        })
        .collect();

    Ok(Json(list))
}

#[derive(Deserialize)]
pub struct SendInvitePayload {
    pub recipient_uuid: String,
    pub instance_id: String,
    pub server_id: Option<i64>,
}

pub async fn send_game_invite(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<SendInvitePayload>,
) -> AppResult<Json<Value>> {
    let friends: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM friendships WHERE status = 'accepted'
           AND ((user_uuid = ?1 AND friend_uuid = ?2) OR (user_uuid = ?2 AND friend_uuid = ?1)))",
    )
    .bind(&auth.uuid)
    .bind(&payload.recipient_uuid)
    .fetch_one(&state.db)
    .await?;
    if !friends {
        return Err(AppError::forbidden("You can only invite friends"));
    }
    let instance: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM instances WHERE id = ?)").bind(&payload.instance_id).fetch_one(&state.db).await?;
    if !instance {
        return Err(AppError::not_found("Instance not found"));
    }
    let invite_id = format!("inv_{}", uuid::Uuid::new_v4().simple());
    let now = chrono::Utc::now();
    let expires = now + chrono::Duration::minutes(30);

    sqlx::query(
        "INSERT INTO game_invites (id, sender_uuid, sender_name, recipient_uuid, instance_id, server_id, status, created_at, expires_at)
         VALUES (?, ?, ?, ?, ?, ?, 'pending', ?, ?)",
    )
    .bind(&invite_id)
    .bind(&auth.uuid)
    .bind(&auth.username)
    .bind(payload.recipient_uuid)
    .bind(payload.instance_id)
    .bind(payload.server_id)
    .bind(now.to_rfc3339())
    .bind(expires.to_rfc3339())
    .execute(&state.platform_db)
    .await?;

    Ok(Json(serde_json::json!({ "id": invite_id, "ok": true })))
}

#[derive(Deserialize)]
pub struct RespondInvitePayload {
    pub action: String, // "accept" or "decline"
}

pub async fn respond_game_invite(
    auth: AuthUser,
    Path(invite_id): Path<String>,
    State(state): State<AppState>,
    Json(payload): Json<RespondInvitePayload>,
) -> AppResult<Json<Value>> {
    let status = if payload.action == "accept" { "accepted" } else { "declined" };

    sqlx::query("UPDATE game_invites SET status = ? WHERE id = ? AND recipient_uuid = ?")
        .bind(status)
        .bind(&invite_id)
        .bind(&auth.uuid)
        .execute(&state.platform_db)
        .await?;

    Ok(Json(serde_json::json!({ "ok": true, "status": status })))
}

// ---------------------------------------------------------------------------
// Viewable Player Profiles
// ---------------------------------------------------------------------------

pub async fn get_player_profile(
    MaybeUser(viewer): MaybeUser,
    Path(uuid): Path<String>,
    State(state): State<AppState>,
) -> AppResult<Json<UserProfileView>> {
    let user_row: Option<(String, String)> =
        sqlx::query_as("SELECT uuid, username FROM users WHERE uuid = ?").bind(&uuid).fetch_optional(&state.db).await?;

    let (puuid, username) = user_row.ok_or_else(|| AppError::not_found("Player profile not found"))?;

    let prof_row: Option<(String, Option<String>, Option<String>, Option<String>)> =
        sqlx::query_as("SELECT bio, banner_url, custom_badge, featured_achievement_id FROM user_profiles WHERE uuid = ?")
            .bind(&puuid)
            .fetch_optional(&state.db)
            .await?;

    let (bio, banner_url, custom_badge, feat_ach_id) = prof_row.unwrap_or((String::new(), None, None, None));

    // Levels
    let levels = crate::routes::leveling::get_user_levels_by_uuid(&state, &puuid).await?;

    // Featured achievement if set
    let featured_achievement = if let Some(fid) = feat_ach_id {
        let ach_list = crate::routes::achievements::get_achievements_for_user(&state, &puuid, false).await?;
        ach_list.into_iter().find(|a| a.id == fid)
    } else {
        None
    };

    // Unlocked achievements count
    let achievements_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM user_achievements WHERE user_uuid = ?").bind(&puuid).fetch_one(&state.db).await?;

    // Recent posts
    let post_rows: Vec<(i64, String, String, String, Option<String>, i64, bool, String)> = sqlx::query_as(
        "SELECT p.id, p.user_uuid, p.author_name, p.content, p.image_url, p.likes_count,
                EXISTS(SELECT 1 FROM user_post_likes l WHERE l.post_id = p.id AND l.user_uuid = ?),
                p.created_at
         FROM user_posts p WHERE p.user_uuid = ? ORDER BY p.id DESC LIMIT 10",
    )
    .bind(viewer.as_ref().map(|user| user.uuid.as_str()).unwrap_or(""))
    .bind(&puuid)
    .fetch_all(&state.db)
    .await?;

    let posts = post_rows
        .into_iter()
        .map(|(id, u_uuid, aname, content, img, likes, liked_by_me, cat)| UserPost {
            id,
            user_uuid: u_uuid,
            author_name: aname,
            content,
            image_url: img,
            likes_count: likes,
            liked_by_me,
            created_at: cat,
        })
        .collect();

    let achievements = crate::routes::achievements::get_achievements_for_user(&state, &puuid, true).await?;
    let friends_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM friendships WHERE status = 'accepted' AND (user_uuid = ? OR friend_uuid = ?)")
            .bind(&puuid)
            .bind(&puuid)
            .fetch_one(&state.db)
            .await?;
    let created_at: String = sqlx::query_scalar("SELECT created_at FROM users WHERE uuid = ?").bind(&puuid).fetch_one(&state.db).await?;
    let stats: Option<crate::routes::servers::AggregatedStatRow> = sqlx::query_as(
        "SELECT uuid, name, SUM(playtime_secs) playtime_secs, SUM(joins) joins, SUM(deaths) deaths,
         SUM(player_kills) player_kills, SUM(mob_kills) mob_kills, SUM(blocks_broken) blocks_broken,
         SUM(blocks_placed) blocks_placed, SUM(messages) messages, MIN(first_seen) first_seen, MAX(last_seen) last_seen
         FROM player_stats WHERE uuid = ? GROUP BY uuid",
    )
    .bind(&puuid)
    .fetch_optional(&state.db)
    .await?;
    let extras = profile_extras(&state, &puuid, viewer.as_ref().map(|v| v.uuid.as_str())).await?;
    Ok(Json(UserProfileView {
        online: extras.online,
        last_seen: extras.last_seen,
        guild: extras.guild,
        rank: extras.rank,
        favorite_server: extras.favorite_server,
        wealth: extras.wealth,
        recent_activity: extras.recent_activity,
        mutual_friends: extras.mutual_friends,
        relationship: extras.relationship,
        accent_color: extras.accent_color,
        discord_linked: extras.discord_linked,
        level_info: levels.clone(),
        badges: levels.badges.clone(),
        achievements,
        friends_count,
        created_at,
        stats: stats.map(|s| serde_json::to_value(s).unwrap()),
        uuid: puuid,
        username,
        bio,
        banner_url,
        custom_badge,
        title: levels.title,
        global_level: levels.global_level,
        global_xp: levels.global_xp,
        server_levels: levels.server_levels,
        featured_achievement,
        achievements_count: achievements_count as usize,
        posts,
    }))
}

#[derive(Default)]
struct ProfileExtras {
    online: bool,
    last_seen: Option<String>,
    guild: Option<velora_shared::ProfileGuild>,
    rank: Option<velora_shared::ProfileRank>,
    favorite_server: Option<String>,
    wealth: f64,
    recent_activity: Vec<velora_shared::ProfileActivity>,
    mutual_friends: Vec<velora_shared::MutualFriend>,
    relationship: String,
    accent_color: Option<String>,
    discord_linked: bool,
}

/// Everything on a profile beyond the basics: presence, guild, rank, wealth,
/// recent activity and how the viewer is connected to the player.
async fn profile_extras(state: &AppState, uuid: &str, viewer: Option<&str>) -> AppResult<ProfileExtras> {
    let db = &state.db;
    let online: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM server_online WHERE uuid = ?)").bind(uuid).fetch_one(db).await?;
    let last_seen: Option<String> =
        sqlx::query_scalar("SELECT MAX(last_seen) FROM player_stats WHERE uuid = ?").bind(uuid).fetch_one(db).await?;

    let guild: Option<(String, String, String, String)> = sqlx::query_as(
        "SELECT g.id, g.name, g.tag, gm.role FROM guild_members gm JOIN guilds g ON g.id = gm.guild_id WHERE gm.uuid = ? ORDER BY gm.joined_at LIMIT 1",
    )
    .bind(uuid)
    .fetch_optional(db)
    .await?;

    // LuckPerms stays authoritative: this is only what a server last reported.
    let rank: Option<(String, String, String, String)> = sqlx::query_as(
        "SELECT r.primary_group, r.display, r.prefix, gs.name FROM player_ranks r JOIN game_servers gs ON gs.id = r.server_id
         WHERE r.uuid = ? AND r.primary_group <> '' ORDER BY r.weight DESC, r.updated_at DESC LIMIT 1",
    )
    .bind(uuid)
    .fetch_optional(db)
    .await?;

    let favorite_server: Option<String> = sqlx::query_scalar(
        "SELECT gs.name FROM player_stats ps JOIN game_servers gs ON gs.id = ps.server_id WHERE ps.uuid = ? AND ps.playtime_secs > 0 ORDER BY ps.playtime_secs DESC LIMIT 1",
    )
    .bind(uuid)
    .fetch_optional(db)
    .await?;

    // Servers that share an economy group hold one balance between them, so count each pool once.
    let wealth: f64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(se.balance), 0.0) FROM server_economy se WHERE se.uuid = ? AND se.server_id IN (
             SELECT COALESCE((SELECT MIN(b.id) FROM game_servers b WHERE b.economy_group <> '' AND b.economy_group = a.economy_group COLLATE NOCASE), a.id) FROM game_servers a)",
    )
    .bind(uuid)
    .fetch_one(db)
    .await?;

    // Only moments worth showing; chat and commands stay private.
    let events: Vec<(String, Option<String>, String, String)> = sqlx::query_as(
        "SELECT e.kind, e.detail, gs.name, e.created_at FROM server_events e JOIN game_servers gs ON gs.id = e.server_id
         WHERE e.uuid = ? AND e.kind IN ('join', 'death', 'advancement', 'kill') ORDER BY e.id DESC LIMIT 8",
    )
    .bind(uuid)
    .fetch_all(db)
    .await?;
    let unlocked: Vec<(String, String)> = sqlx::query_as(
        "SELECT a.title, ua.unlocked_at FROM user_achievements ua JOIN achievements a ON a.id = ua.achievement_id WHERE ua.user_uuid = ? ORDER BY ua.unlocked_at DESC LIMIT 8",
    )
    .bind(uuid)
    .fetch_all(db)
    .await?;
    let mut recent_activity: Vec<velora_shared::ProfileActivity> = events
        .into_iter()
        .map(|(kind, detail, server, at)| {
            let detail = detail.unwrap_or_default();
            let text = match kind.as_str() {
                "join" => format!("Joined {server}"),
                "death" => format!("Died on {server}"),
                "kill" => {
                    format!("Defeated {}", if detail.is_empty() { "a foe".into() } else { detail.chars().take(40).collect::<String>() })
                }
                _ => format!(
                    "Earned {}",
                    if detail.is_empty() { "an advancement".into() } else { detail.chars().take(60).collect::<String>() }
                ),
            };
            velora_shared::ProfileActivity { kind, text, at }
        })
        .chain(unlocked.into_iter().map(|(title, at)| velora_shared::ProfileActivity {
            kind: "achievement".into(),
            text: format!("Unlocked {title}"),
            at,
        }))
        .collect();
    recent_activity.sort_by(|a, b| b.at.cmp(&a.at));
    recent_activity.truncate(8);

    let (relationship, mutual_friends) = match viewer {
        Some(v) if v == uuid => ("self".to_string(), vec![]),
        Some(v) => {
            let row: Option<(String, String)> = sqlx::query_as(
                "SELECT status, action_uuid FROM friendships WHERE (user_uuid = ? AND friend_uuid = ?) OR (user_uuid = ? AND friend_uuid = ?)",
            )
            .bind(v).bind(uuid).bind(uuid).bind(v)
            .fetch_optional(db)
            .await?;
            let relationship = match row {
                Some((s, _)) if s == "accepted" => "accepted",
                Some((_, action)) if action == v => "pending_outgoing",
                Some(_) => "pending_incoming",
                None => "none",
            }
            .to_string();
            let mutual: Vec<(String, String)> = sqlx::query_as(
                "WITH mine AS (SELECT CASE WHEN user_uuid = ?1 THEN friend_uuid ELSE user_uuid END AS f FROM friendships WHERE status = 'accepted' AND (user_uuid = ?1 OR friend_uuid = ?1)),
                      theirs AS (SELECT CASE WHEN user_uuid = ?2 THEN friend_uuid ELSE user_uuid END AS f FROM friendships WHERE status = 'accepted' AND (user_uuid = ?2 OR friend_uuid = ?2))
                 SELECT u.uuid, u.username FROM users u WHERE u.uuid IN (SELECT f FROM mine INTERSECT SELECT f FROM theirs) ORDER BY u.username COLLATE NOCASE LIMIT 12",
            )
            .bind(v)
            .bind(uuid)
            .fetch_all(db)
            .await?;
            (relationship, mutual.into_iter().map(|(uuid, username)| velora_shared::MutualFriend { uuid, username }).collect())
        }
        None => ("none".to_string(), vec![]),
    };

    let accent_color: Option<String> =
        sqlx::query_scalar("SELECT accent_color FROM user_profiles WHERE uuid = ?").bind(uuid).fetch_optional(db).await?.flatten();
    let discord_linked: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM account_connections ac JOIN users u ON u.id = ac.user_id WHERE u.uuid = ? AND ac.provider = 'discord')")
        .bind(uuid)
        .fetch_one(db)
        .await?;

    Ok(ProfileExtras {
        online,
        last_seen,
        guild: guild.map(|(id, name, tag, role)| velora_shared::ProfileGuild { id, name, tag, role }),
        rank: rank.map(|(primary, display, prefix, server_name)| velora_shared::ProfileRank {
            display: if display.is_empty() { primary } else { display },
            prefix,
            server_name,
        }),
        favorite_server,
        wealth,
        recent_activity,
        mutual_friends,
        relationship,
        accent_color,
        discord_linked,
    })
}

#[derive(Deserialize)]
pub struct UpdateProfilePayload {
    pub accent_color: Option<String>,
    pub bio: Option<String>,
    pub banner_url: Option<String>,
    pub custom_badge: Option<String>,
    pub featured_achievement_id: Option<String>,
}

pub async fn update_my_profile(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<UpdateProfilePayload>,
) -> AppResult<Json<Value>> {
    let now = chrono::Utc::now().to_rfc3339();
    let payload_accent = payload.accent_color.clone();

    sqlx::query(
        "INSERT INTO user_profiles (uuid, bio, banner_url, custom_badge, featured_achievement_id, updated_at)
         VALUES (?, ?, ?, ?, ?, ?)
         ON CONFLICT (uuid) DO UPDATE SET
            bio = COALESCE(?, bio),
            banner_url = COALESCE(?, banner_url),
            custom_badge = COALESCE(?, custom_badge),
            featured_achievement_id = COALESCE(?, featured_achievement_id),
            updated_at = excluded.updated_at",
    )
    .bind(&auth.uuid)
    .bind(payload.bio.clone().unwrap_or_default())
    .bind(payload.banner_url.clone())
    .bind(payload.custom_badge.clone())
    .bind(payload.featured_achievement_id.clone())
    .bind(&now)
    .bind(payload.bio)
    .bind(payload.banner_url)
    .bind(payload.custom_badge)
    .bind(payload.featured_achievement_id)
    .execute(&state.platform_db)
    .await?;

    if let Some(color) = payload_accent {
        let color = color.trim().to_lowercase();
        let valid = color.is_empty() || (color.len() == 7 && color.starts_with('#') && color[1..].chars().all(|c| c.is_ascii_hexdigit()));
        if !valid {
            return Err(AppError::bad_request("accent colour must look like #8b6cff"));
        }
        sqlx::query("UPDATE user_profiles SET accent_color = ? WHERE uuid = ?")
            .bind(if color.is_empty() { None } else { Some(color) })
            .bind(&auth.uuid)
            .execute(&state.platform_db)
            .await?;
    }

    Ok(Json(serde_json::json!({ "ok": true })))
}

// ---------------------------------------------------------------------------
// Social Posts Feed
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct CreatePostInput {
    pub content: String,
    pub image_url: Option<String>,
}

pub async fn create_user_post(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<CreatePostInput>,
) -> AppResult<Json<UserPost>> {
    let content = payload.content.trim();
    if content.is_empty() {
        return Err(AppError::bad_request("Post content cannot be empty"));
    }

    let now = chrono::Utc::now().to_rfc3339();

    let id: i64 = sqlx::query_scalar(
        "INSERT INTO user_posts (user_uuid, author_name, content, image_url, likes_count, created_at)
         VALUES (?, ?, ?, ?, 0, ?)
         RETURNING id",
    )
    .bind(&auth.uuid)
    .bind(&auth.username)
    .bind(content)
    .bind(payload.image_url.as_deref())
    .bind(&now)
    // Profiles and posts belong to the shared account store. Instance pools
    // expose these tables through read-only views of the platform database.
    .fetch_one(&state.platform_db)
    .await?;

    Ok(Json(UserPost {
        id,
        user_uuid: auth.uuid.clone(),
        author_name: auth.username.clone(),
        content: content.to_string(),
        image_url: payload.image_url,
        likes_count: 0,
        liked_by_me: false,
        created_at: now,
    }))
}

pub async fn delete_user_post(auth: AuthUser, Path(post_id): Path<i64>, State(state): State<AppState>) -> AppResult<Json<Value>> {
    sqlx::query("DELETE FROM user_posts WHERE id = ? AND user_uuid = ?").bind(post_id).bind(&auth.uuid).execute(&state.platform_db).await?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn like_user_post(auth: AuthUser, Path(post_id): Path<i64>, State(state): State<AppState>) -> AppResult<Json<Value>> {
    let mut tx = state.platform_db.begin().await?;
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM user_posts WHERE id = ?)").bind(post_id).fetch_one(&mut *tx).await?;
    if !exists {
        return Err(AppError::not_found("Post not found"));
    }
    let added = sqlx::query("INSERT OR IGNORE INTO user_post_likes (post_id, user_uuid) VALUES (?, ?)")
        .bind(post_id)
        .bind(&auth.uuid)
        .execute(&mut *tx)
        .await?
        .rows_affected()
        > 0;
    if added {
        sqlx::query("UPDATE user_posts SET likes_count = likes_count + 1 WHERE id = ?").bind(post_id).execute(&mut *tx).await?;
    }
    tx.commit().await?;

    Ok(Json(serde_json::json!({ "ok": true, "liked": added })))
}

// ---------------------------------------------------------------------------
// Members Search & Discovery
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct SearchMembersQuery {
    pub q: Option<String>,
}

pub async fn search_members(
    auth: AuthUser,
    Query(query): Query<SearchMembersQuery>,
    State(state): State<AppState>,
) -> AppResult<Json<Vec<velora_shared::MemberProfile>>> {
    // `%` and `_` are LIKE wildcards; players shouldn't be able to inject them.
    let q = query.q.unwrap_or_default().trim().to_lowercase().replace(['%', '_', '\\'], "");
    let pattern = format!("%{q}%");

    let rows: Vec<(
        String,
        String,
        String,
        String,
        Option<String>,
        Option<i64>,
        Option<String>,
        i64,
        Option<String>,
        bool,
        Option<String>,
        Option<String>,
    )> = sqlx::query_as(
        "SELECT u.uuid, u.username, u.role, u.status, u.skin_hash,
                    ul.global_level, ul.title,
                    COALESCE((SELECT SUM(ps.playtime_secs) FROM player_stats ps WHERE ps.uuid = u.uuid), 0),
                    (SELECT MAX(ps.last_seen) FROM player_stats ps WHERE ps.uuid = u.uuid),
                    EXISTS(SELECT 1 FROM server_online so WHERE so.uuid = u.uuid),
                    f.status, f.action_uuid
             FROM users u
             LEFT JOIN user_levels ul ON ul.uuid = u.uuid
             LEFT JOIN friendships f ON (f.user_uuid = ?1 AND f.friend_uuid = u.uuid) OR (f.friend_uuid = ?1 AND f.user_uuid = u.uuid)
             WHERE u.status = 'active' AND u.uuid <> ?1 AND (?2 = '' OR LOWER(u.username) LIKE ?3)
             ORDER BY (CASE WHEN LOWER(u.username) = ?2 THEN 0 ELSE 1 END), 10 DESC, u.username COLLATE NOCASE
             LIMIT 60",
    )
    .bind(&auth.uuid)
    .bind(&q)
    .bind(&pattern)
    .fetch_all(&state.db)
    .await?;

    let list = rows
        .into_iter()
        .map(|(uuid, username, role, status, skin_hash, glvl, title, playtime, last_seen, online, friend_status, action)| {
            let friendship_status = match (friend_status.as_deref(), action.as_deref()) {
                (Some("accepted"), _) => "accepted",
                (Some(_), Some(a)) if a == auth.uuid => "pending_outgoing",
                (Some(_), _) => "pending_incoming",
                _ => "none",
            }
            .to_string();
            velora_shared::MemberProfile {
                uuid,
                username,
                role,
                status,
                skin_url: skin_hash.map(|h| format!("/textures/{h}")),
                global_level: glvl.unwrap_or(1),
                title,
                playtime_secs: playtime,
                last_seen,
                online,
                is_friend: friendship_status == "accepted",
                friendship_status,
            }
        })
        .collect();

    Ok(Json(list))
}
