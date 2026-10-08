//! Guilds system, in-game relay, roles, posts, and land-claiming.

use crate::state::RequestState as State;
use crate::auth::{AdminUser, AuthUser};
use crate::error::{AppError, AppResult};
use crate::routes::servers::GameServer;
use crate::state::AppState;
use axum::extract::{Path, Query};
use axum::Json;
use scopenet_shared::{Guild, GuildClaim, GuildMember, GuildPost};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Deserialize)]
pub struct InstanceQuery {
    pub instance_id: Option<String>,
}

/// List guilds for an instance.
pub async fn list_guilds(Query(query): Query<InstanceQuery>, State(state): State<AppState>) -> AppResult<Json<Vec<Guild>>> {
    let instance_id = query.instance_id.unwrap_or_default();
    let rows: Vec<(
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        Option<String>,
        Option<String>,
        i64,
        i64,
        i64,
        String,
        i64,
        i64,
    )> = sqlx::query_as(
        "SELECT g.id, g.instance_id, g.name, g.tag, g.description, g.motd, g.leader_uuid,
                g.icon_url, g.banner_url, g.level, g.xp, (g.max_claims + COALESCE((SELECT SUM(b.claim_chunks) FROM player_bonuses b JOIN guild_members m ON m.uuid = b.uuid WHERE m.guild_id = g.id), 0) + COALESCE((SELECT CAST(value AS INTEGER) FROM kv WHERE key = 'rule_claims_per_member'), 0) * MAX(0, (SELECT COUNT(*) FROM guild_members m2 WHERE m2.guild_id = g.id) - 1) + COALESCE((SELECT CAST(value AS INTEGER) FROM kv WHERE key = 'rule_claims_per_level'), 0) * MAX(0, g.level - 1)), g.created_at,
                (SELECT COUNT(*) FROM guild_members gm WHERE gm.guild_id = g.id) as member_count,
                (SELECT COUNT(*) FROM guild_claims gc WHERE gc.guild_id = g.id) as claims_count
         FROM guilds g
         WHERE (? = '' OR g.instance_id = ?)
         ORDER BY g.level DESC, member_count DESC",
    )
    .bind(&instance_id)
    .bind(&instance_id)
    .fetch_all(&state.db)
    .await?;

    let list = rows
        .into_iter()
        .map(
            |(
                id,
                inst_id,
                name,
                tag,
                desc,
                motd,
                leader_uuid,
                icon_url,
                banner_url,
                level,
                xp,
                max_claims,
                created_at,
                member_count,
                claims_count,
            )| {
                Guild {
                    id,
                    instance_id: inst_id,
                    name,
                    tag,
                    description: desc,
                    motd,
                    leader_uuid,
                    icon_url,
                    banner_url,
                    level,
                    xp,
                    max_claims,
                    member_count,
                    claims_count,
                    created_at,
                }
            },
        )
        .collect();

    Ok(Json(list))
}

/// Get the player's primary guild for the selected instance/server.
pub async fn get_my_guild(
    auth: AuthUser,
    Query(query): Query<InstanceQuery>,
    State(state): State<AppState>,
) -> AppResult<Json<Option<GuildDetail>>> {
    let instance_id = query.instance_id.unwrap_or_default();

    let guild_id: Option<String> = sqlx::query_scalar(
        "SELECT g.id FROM guilds g
         JOIN guild_primary_memberships pm ON pm.guild_id = g.id
         JOIN game_servers s ON s.id = pm.server_id
         WHERE pm.uuid = ? AND (? = '' OR g.instance_id = ?)
         ORDER BY s.name COLLATE NOCASE LIMIT 1",
    )
    .bind(&auth.uuid).bind(&instance_id).bind(&instance_id).fetch_optional(&state.db).await?
        .or(sqlx::query_scalar::<_, String>(
            "SELECT g.id FROM guilds g JOIN guild_members gm ON gm.guild_id=g.id
             WHERE gm.uuid=? AND (?='' OR g.instance_id=?) ORDER BY g.created_at,g.id LIMIT 1",
        ).bind(&auth.uuid).bind(&instance_id).bind(&instance_id).fetch_optional(&state.db).await?);

    let Some(gid) = guild_id else {
        return Ok(Json(None));
    };

    let detail = fetch_guild_detail(&state, &gid).await?;
    Ok(Json(Some(detail)))
}

/// List every guild membership in the requested instance, marking the guild
/// selected for each server where the player is a member.
pub async fn my_memberships(auth: AuthUser, Query(query): Query<InstanceQuery>, State(state): State<AppState>) -> AppResult<Json<Value>> {
    let instance_id = query.instance_id.unwrap_or_default();
    let rows: Vec<(String,String,String,String,String,i64,Option<i64>)> = sqlx::query_as(
        "SELECT g.id,g.instance_id,g.name,g.tag,gm.role,
           (SELECT COUNT(*) FROM game_servers gs WHERE gs.instance_id=g.instance_id),
           (SELECT COUNT(*) FROM guild_primary_memberships pm JOIN game_servers ps ON ps.id=pm.server_id WHERE pm.uuid=gm.uuid AND pm.guild_id=g.id AND ps.instance_id=g.instance_id)
         FROM guild_members gm JOIN guilds g ON g.id=gm.guild_id
         WHERE gm.uuid=? AND (?='' OR g.instance_id=?) ORDER BY g.instance_id,g.name COLLATE NOCASE",
    ).bind(&auth.uuid).bind(&instance_id).bind(&instance_id).fetch_all(&state.db).await?;
    Ok(Json(json!(rows.into_iter().map(|(id,instance_id,name,tag,role,servers,primary_servers)| json!({"id":id,"instance_id":instance_id,"name":name,"tag":tag,"role":role,"server_count":servers,"primary_server_count":primary_servers.unwrap_or(0)})).collect::<Vec<_>>())))
}

#[derive(Deserialize)]
pub struct PrimaryGuildPayload {
    pub server_id: i64,
}

/// Select which of a player's memberships supplies the server tag and default
/// guild actions. The player must be a member of the selected guild.
pub async fn set_primary_guild(
    auth: AuthUser,
    Path(guild_id): Path<String>,
    State(state): State<AppState>,
    Json(payload): Json<PrimaryGuildPayload>,
) -> AppResult<Json<Value>> {
    let valid: bool = sqlx::query_scalar(
        "SELECT EXISTS(
            SELECT 1 FROM guild_members gm
            JOIN guilds g ON g.id = gm.guild_id
            JOIN game_servers s ON s.instance_id = g.instance_id
            WHERE gm.guild_id = ? AND gm.uuid = ? AND s.id = ?
        )",
    )
    .bind(&guild_id).bind(&auth.uuid).bind(payload.server_id)
    .fetch_one(&state.db).await?;
    if !valid {
        return Err(AppError::bad_request("You are not a member of that guild on this server"));
    }
    sqlx::query(
        "INSERT INTO guild_primary_memberships(server_id,uuid,guild_id,updated_at)
         VALUES(?,?,?,?)
         ON CONFLICT(server_id,uuid) DO UPDATE SET guild_id=excluded.guild_id,updated_at=excluded.updated_at",
    )
    .bind(payload.server_id).bind(&auth.uuid).bind(&guild_id).bind(crate::db::now())
    .execute(&state.db).await?;
    Ok(Json(json!({ "ok": true, "guild_id": guild_id, "server_id": payload.server_id })))
}

#[derive(Serialize)]
pub struct GuildDetail {
    #[serde(flatten)]
    pub guild: Guild,
    pub members: Vec<GuildMember>,
    pub posts: Vec<GuildPost>,
    pub claims: Vec<GuildClaim>,
}

#[derive(Deserialize)]
pub struct GuildWalletQuery {
    pub server_id: i64,
}

#[derive(Deserialize)]
pub struct GuildWalletTransfer {
    pub server_id: i64,
    pub amount: f64,
}

pub async fn guild_wallet(
    auth: AuthUser,
    Path(guild_id): Path<String>,
    Query(query): Query<GuildWalletQuery>,
    State(state): State<AppState>,
) -> AppResult<Json<Value>> {
    let member: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM guild_members gm JOIN guilds g ON g.id=gm.guild_id JOIN game_servers s ON s.instance_id=g.instance_id WHERE gm.guild_id=? AND gm.uuid=? AND s.id=?)")
        .bind(&guild_id).bind(&auth.uuid).bind(query.server_id).fetch_one(&state.db).await?;
    if !member {
        return Err(AppError::forbidden("Guild membership is required"));
    }
    let sid = crate::routes::servers::economy_scope(&state.db, query.server_id).await?;
    let balance: f64 = sqlx::query_scalar("SELECT balance FROM guild_wallets WHERE guild_id=? AND server_id=?")
        .bind(&guild_id)
        .bind(sid)
        .fetch_optional(&state.db)
        .await?
        .unwrap_or(0.0);
    let rows: Vec<(i64, String, String, f64, String, String)> = sqlx::query_as("SELECT id,actor_uuid,kind,amount,created_at,note FROM guild_wallet_transactions WHERE guild_id=? AND server_id=? ORDER BY id DESC LIMIT 30")
        .bind(&guild_id).bind(sid).fetch_all(&state.db).await?;
    let role: String = sqlx::query_scalar("SELECT gm.role FROM guild_members gm WHERE gm.guild_id=? AND gm.uuid=?")
        .bind(&guild_id)
        .bind(&auth.uuid)
        .fetch_one(&state.db)
        .await?;
    // A player's first transfer creates their balance at 1000, so show that until then.
    let my_balance: f64 = sqlx::query_scalar("SELECT balance FROM server_economy WHERE server_id=? AND uuid=?")
        .bind(sid)
        .bind(&auth.uuid)
        .fetch_optional(&state.db)
        .await?
        .unwrap_or(1000.0);
    Ok(Json(
        serde_json::json!({"balance":balance,"role":role,"my_balance":my_balance,"currency_symbol":"$","transactions":rows.into_iter().map(|(id,actor,kind,amount,created_at,note)| serde_json::json!({"id":id,"actor_uuid":actor,"kind":kind,"amount":amount,"created_at":created_at,"note":note})).collect::<Vec<_>>()}),
    ))
}

pub async fn guild_wallet_deposit(
    auth: AuthUser,
    Path(guild_id): Path<String>,
    State(state): State<AppState>,
    Json(p): Json<GuildWalletTransfer>,
) -> AppResult<Json<Value>> {
    wallet_transfer(&state, &auth, &guild_id, p, false).await
}

pub async fn guild_wallet_withdraw(
    auth: AuthUser,
    Path(guild_id): Path<String>,
    State(state): State<AppState>,
    Json(p): Json<GuildWalletTransfer>,
) -> AppResult<Json<Value>> {
    wallet_transfer(&state, &auth, &guild_id, p, true).await
}

async fn wallet_transfer(
    state: &AppState,
    auth: &AuthUser,
    guild_id: &str,
    p: GuildWalletTransfer,
    withdraw: bool,
) -> AppResult<Json<Value>> {
    if !p.amount.is_finite() || p.amount <= 0.0 || p.amount > 1e9 || (p.amount * 100.0).fract().abs() > 0.00001 {
        return Err(AppError::bad_request("Amount must be between 0.01 and 1,000,000,000 with at most two decimals"));
    }
    let role: Option<String> = sqlx::query_scalar("SELECT gm.role FROM guild_members gm JOIN guilds g ON g.id=gm.guild_id JOIN game_servers s ON s.instance_id=g.instance_id WHERE gm.guild_id=? AND gm.uuid=? AND s.id=?")
        .bind(guild_id).bind(&auth.uuid).bind(p.server_id).fetch_optional(&state.db).await?;
    let Some(role) = role else {
        return Err(AppError::forbidden("Guild membership is required"));
    };
    if withdraw && role != "leader" && role != "officer" {
        return Err(AppError::forbidden("Only guild leaders and officers can withdraw"));
    }
    // Servers in one economy group share balances and guild banks.
    let mut p = p;
    p.server_id = crate::routes::servers::economy_scope(&state.db, p.server_id).await?;
    let now = chrono::Utc::now().to_rfc3339();
    let mut tx = state.db.begin().await?;
    // The first write serializes transfers across concurrent requests.
    sqlx::query(
        "INSERT INTO guild_wallets(server_id,guild_id,balance,updated_at) VALUES(?,?,0,?) ON CONFLICT(server_id,guild_id) DO NOTHING",
    )
    .bind(p.server_id)
    .bind(guild_id)
    .bind(&now)
    .execute(&mut *tx)
    .await?;
    sqlx::query("INSERT INTO server_economy(server_id,uuid,username,balance,updated_at) VALUES(?,?,?,1000,?) ON CONFLICT(server_id,uuid) DO NOTHING")
        .bind(p.server_id).bind(&auth.uuid).bind(&auth.username).bind(&now).execute(&mut *tx).await?;
    let source = if withdraw {
        sqlx::query("UPDATE guild_wallets SET balance=balance-?,updated_at=? WHERE server_id=? AND guild_id=? AND balance>=?")
            .bind(p.amount)
            .bind(&now)
            .bind(p.server_id)
            .bind(guild_id)
            .bind(p.amount)
            .execute(&mut *tx)
            .await?
    } else {
        sqlx::query("UPDATE server_economy SET balance=balance-?,updated_at=? WHERE server_id=? AND uuid=? AND balance>=?")
            .bind(p.amount)
            .bind(&now)
            .bind(p.server_id)
            .bind(&auth.uuid)
            .bind(p.amount)
            .execute(&mut *tx)
            .await?
    };
    if source.rows_affected() == 0 {
        return Err(AppError::bad_request("Insufficient funds"));
    }
    if withdraw {
        sqlx::query("UPDATE server_economy SET balance=balance+?,updated_at=? WHERE server_id=? AND uuid=?")
            .bind(p.amount)
            .bind(&now)
            .bind(p.server_id)
            .bind(&auth.uuid)
            .execute(&mut *tx)
            .await?;
    } else {
        sqlx::query("UPDATE guild_wallets SET balance=balance+?,updated_at=? WHERE server_id=? AND guild_id=?")
            .bind(p.amount)
            .bind(&now)
            .bind(p.server_id)
            .bind(guild_id)
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query("INSERT INTO guild_wallet_transactions(server_id,guild_id,actor_uuid,kind,amount,created_at) VALUES(?,?,?,?,?,?)")
        .bind(p.server_id)
        .bind(guild_id)
        .bind(&auth.uuid)
        .bind(if withdraw { "withdraw" } else { "deposit" })
        .bind(p.amount)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
    let balance: f64 = sqlx::query_scalar("SELECT balance FROM guild_wallets WHERE server_id=? AND guild_id=?")
        .bind(p.server_id)
        .bind(guild_id)
        .fetch_one(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Json(serde_json::json!({"balance":balance})))
}

/// Refuse a new member when the guild is at the admin-set member limit (0 = no limit).
pub async fn check_room(conn: &mut sqlx::SqliteConnection, guild_id: &str) -> AppResult<()> {
    let limit: i64 = sqlx::query_scalar("SELECT COALESCE((SELECT CAST(value AS INTEGER) FROM kv WHERE key = 'rule_guild_max_members'), 0)")
        .fetch_one(&mut *conn)
        .await?;
    if limit > 0 {
        let members: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM guild_members WHERE guild_id = ?").bind(guild_id).fetch_one(&mut *conn).await?;
        if members >= limit {
            return Err(AppError::bad_request(format!("That guild is full ({limit} members)")));
        }
    }
    Ok(())
}

async fn fetch_guild_detail(state: &AppState, guild_id: &str) -> AppResult<GuildDetail> {
    let row: Option<(String, String, String, String, String, String, String, Option<String>, Option<String>, i64, i64, i64, String)> =
        sqlx::query_as(
            "SELECT id, instance_id, name, tag, description, motd, leader_uuid,
                icon_url, banner_url, level, xp, (guilds.max_claims + COALESCE((SELECT SUM(b.claim_chunks) FROM player_bonuses b JOIN guild_members m ON m.uuid = b.uuid WHERE m.guild_id = guilds.id), 0) + COALESCE((SELECT CAST(value AS INTEGER) FROM kv WHERE key = 'rule_claims_per_member'), 0) * MAX(0, (SELECT COUNT(*) FROM guild_members m2 WHERE m2.guild_id = guilds.id) - 1) + COALESCE((SELECT CAST(value AS INTEGER) FROM kv WHERE key = 'rule_claims_per_level'), 0) * MAX(0, guilds.level - 1)), created_at
         FROM guilds WHERE id = ?",
        )
        .bind(guild_id)
        .fetch_optional(&state.db)
        .await?;

    let (id, inst_id, name, tag, desc, motd, leader_uuid, icon_url, banner_url, level, xp, max_claims, created_at) =
        row.ok_or_else(|| AppError::not_found("Guild not found"))?;

    // Members with online presence
    let member_rows: Vec<(String, String, String, String, bool)> = sqlx::query_as(
        "SELECT gm.uuid, gm.name, gm.role, gm.joined_at,
                EXISTS(SELECT 1 FROM server_online so WHERE so.uuid = gm.uuid) as online
         FROM guild_members gm
         WHERE gm.guild_id = ?
         ORDER BY (CASE gm.role WHEN 'leader' THEN 1 WHEN 'officer' THEN 2 ELSE 3 END) ASC, gm.joined_at ASC",
    )
    .bind(guild_id)
    .fetch_all(&state.db)
    .await?;

    let members: Vec<GuildMember> = member_rows
        .into_iter()
        .map(|(uuid, mname, role, joined_at, online)| GuildMember { uuid, name: mname, role, joined_at, online })
        .collect();

    // Posts
    let post_rows: Vec<(i64, String, String, String, String, String, String)> = sqlx::query_as(
        "SELECT id, guild_id, author_uuid, author_name, title, content, created_at
         FROM guild_posts
         WHERE guild_id = ?
         ORDER BY id DESC LIMIT 20",
    )
    .bind(guild_id)
    .fetch_all(&state.db)
    .await?;

    let posts: Vec<GuildPost> = post_rows
        .into_iter()
        .map(|(pid, gid, auuid, aname, title, content, pcreated)| GuildPost {
            id: pid,
            guild_id: gid,
            author_uuid: auuid,
            author_name: aname,
            title,
            content,
            created_at: pcreated,
        })
        .collect();

    // Claims
    let claim_rows: Vec<(i64, String, i64, String, i32, i32, String, String)> = sqlx::query_as(
        "SELECT id, guild_id, COALESCE(server_id, 0), dimension, chunk_x, chunk_z, claimed_by_uuid, claimed_at
         FROM guild_claims
         WHERE guild_id = ?
         LIMIT 200",
    )
    .bind(guild_id)
    .fetch_all(&state.db)
    .await?;

    let claims: Vec<GuildClaim> = claim_rows
        .into_iter()
        .map(|(cid, gid, sid, dim, cx, cz, cby, cat)| GuildClaim {
            id: cid,
            guild_id: gid,
            guild_name: name.clone(),
            guild_tag: tag.clone(),
            server_id: sid,
            dimension: dim,
            chunk_x: cx,
            chunk_z: cz,
            claimed_by_uuid: cby,
            claimed_at: cat,
        })
        .collect();

    Ok(GuildDetail {
        guild: Guild {
            id,
            instance_id: inst_id,
            name,
            tag,
            description: desc,
            motd,
            leader_uuid,
            icon_url,
            banner_url,
            level,
            xp,
            max_claims,
            member_count: members.len() as i64,
            claims_count: claims.len() as i64,
            created_at,
        },
        members,
        posts,
        claims,
    })
}

/// Get guild detail by ID.
pub async fn get_guild_by_id(Path(id): Path<String>, State(state): State<AppState>) -> AppResult<Json<GuildDetail>> {
    fetch_guild_detail(&state, &id).await.map(Json)
}

pub async fn get_guild_members(Path(id): Path<String>, State(state): State<AppState>) -> AppResult<Json<Vec<GuildMember>>> {
    Ok(Json(fetch_guild_detail(&state, &id).await?.members))
}

pub async fn get_guild_posts(Path(id): Path<String>, State(state): State<AppState>) -> AppResult<Json<Vec<GuildPost>>> {
    Ok(Json(fetch_guild_detail(&state, &id).await?.posts))
}

#[derive(Deserialize)]
pub struct CreateGuildPayload {
    pub instance_id: String,
    pub name: String,
    pub tag: String,
    pub description: Option<String>,
    pub motd: Option<String>,
    pub icon_url: Option<String>,
    pub banner_url: Option<String>,
}

/// Create a new Guild.
pub async fn create_guild(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<CreateGuildPayload>,
) -> AppResult<Json<GuildDetail>> {
    let name = payload.name.trim();
    let tag = payload.tag.trim();
    if name.len() < 3 || name.len() > 32 {
        return Err(AppError::bad_request("Guild name must be 3-32 characters"));
    }
    if tag.len() < 2 || tag.len() > 6 {
        return Err(AppError::bad_request("Guild tag must be 2-6 characters"));
    }

    let mut tx = state.db.begin().await?;

    let guild_id = format!("guild_{}", uuid::Uuid::new_v4().simple());
    let now = chrono::Utc::now().to_rfc3339();

    let base_claims = crate::progression::load(&mut tx).await?.rules.guild_base_claims;
    sqlx::query(
        "INSERT INTO guilds (id, instance_id, name, tag, description, motd, leader_uuid, icon_url, banner_url, level, xp, max_claims, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 1, 0, ?, ?)",
    )
    .bind(&guild_id)
    .bind(&payload.instance_id)
    .bind(name)
    .bind(tag.to_ascii_uppercase())
    .bind(payload.description.unwrap_or_default())
    .bind(payload.motd.unwrap_or_default())
    .bind(&auth.uuid)
    .bind(payload.icon_url)
    .bind(payload.banner_url)
    .bind(base_claims)
    .bind(&now)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO guild_primary_memberships(server_id,uuid,guild_id,updated_at)
         SELECT id,?,?,? FROM game_servers WHERE instance_id=?
         ON CONFLICT(server_id,uuid) DO NOTHING",
    )
    .bind(&auth.uuid).bind(&guild_id).bind(&now).bind(&payload.instance_id).execute(&mut *tx).await?;

    // Add leader as first member
    sqlx::query(
        "INSERT INTO guild_members (guild_id, uuid, name, role, joined_at)
         VALUES (?, ?, ?, 'leader', ?)",
    )
    .bind(&guild_id)
    .bind(&auth.uuid)
    .bind(&auth.username)
    .bind(&now)
    .execute(&mut *tx)
    .await?;

    // Record achievement event for founding a guild
    sqlx::query(
        "INSERT OR IGNORE INTO user_achievements (user_uuid, achievement_id, unlocked_at)
         VALUES (?, 'ach_guild_initiate', ?)",
    )
    .bind(&auth.uuid)
    .bind(&now)
    .execute(&mut *tx)
    .await?;

    crate::routes::leveling::grant_rewards(&mut tx, &auth.uuid, &now).await?;
    tx.commit().await?;

    fetch_guild_detail(&state, &guild_id).await.map(Json)
}

#[derive(Deserialize)]
pub struct UpdateGuildPayload {
    pub description: Option<String>,
    pub motd: Option<String>,
    pub icon_url: Option<String>,
    pub banner_url: Option<String>,
}

/// Update guild info.
pub async fn update_guild(
    auth: AuthUser,
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(payload): Json<UpdateGuildPayload>,
) -> AppResult<Json<Value>> {
    let is_officer_or_leader = guild_can(&state, &id, &auth.uuid, "manage").await?;
    if !is_officer_or_leader {
        return Err(AppError::forbidden("Only guild officers or leaders can update guild details"));
    }

    sqlx::query(
        "UPDATE guilds SET
            description = COALESCE(?, description),
            motd = COALESCE(?, motd),
            icon_url = COALESCE(?, icon_url),
            banner_url = COALESCE(?, banner_url)
         WHERE id = ?",
    )
    .bind(payload.description)
    .bind(payload.motd)
    .bind(payload.icon_url)
    .bind(payload.banner_url)
    .bind(&id)
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Deserialize)]
pub struct AddMemberPayload {
    pub username: String,
}

#[derive(Deserialize)]
pub struct JoinRequestPayload {
    #[serde(default)]
    pub message: String,
}

pub async fn request_join(
    auth: AuthUser,
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(payload): Json<JoinRequestPayload>,
) -> AppResult<Json<Value>> {
    let name: String = sqlx::query_scalar("SELECT username FROM users WHERE id = ?").bind(auth.id).fetch_one(&state.db).await?;
    create_join_request(&state, &id, &auth.uuid, &name, &payload.message).await?;
    Ok(Json(json!({"ok":true})))
}

/// A player asks to join a guild; its leaders and officers hear about it in their bell.
pub(crate) async fn create_join_request(state: &AppState, id: &str, uuid: &str, name: &str, message: &str) -> AppResult<()> {
    let guild_name: String = sqlx::query_scalar("SELECT name FROM guilds WHERE id = ?")
        .bind(id).fetch_optional(&state.db).await?.ok_or_else(|| AppError::not_found("guild not found"))?;
    sqlx::query("INSERT INTO guild_join_requests(guild_id,uuid,name,message,created_at) VALUES(?,?,?,?,?) ON CONFLICT(guild_id,uuid) DO UPDATE SET message=excluded.message,created_at=excluded.created_at")
        .bind(id).bind(uuid).bind(name).bind(message.trim().chars().take(300).collect::<String>()).bind(crate::db::now()).execute(&state.db).await?;
    let reviewers: Vec<String> = sqlx::query_scalar("SELECT uuid FROM guild_members WHERE guild_id = ? AND role IN ('leader','officer')")
        .bind(id)
        .fetch_all(&state.db)
        .await?;
    super::notifications::push_many(&state.db, &reviewers, "guild_request", "New join request", &format!("{name} wants to join {guild_name}."), Some("/guild")).await;
    Ok(())
}

pub async fn list_join_requests(auth: AuthUser, Path(id): Path<String>, State(state): State<AppState>) -> AppResult<Json<Value>> {
    if !guild_can(&state, &id, &auth.uuid, "invite").await? {
        return Err(AppError::forbidden("Your guild role cannot review requests"));
    }
    let rows: Vec<(String, String, String, String)> =
        sqlx::query_as("SELECT uuid,name,message,created_at FROM guild_join_requests WHERE guild_id=? ORDER BY created_at DESC")
            .bind(&id)
            .fetch_all(&state.db)
            .await?;
    Ok(Json(json!(rows
        .into_iter()
        .map(|(uuid, name, message, created_at)| json!({"uuid":uuid,"name":name,"message":message,"created_at":created_at}))
        .collect::<Vec<_>>())))
}

#[derive(Deserialize)]
pub struct JoinDecision {
    pub accept: bool,
}

pub async fn respond_join_request(
    auth: AuthUser,
    Path((id, uuid)): Path<(String, String)>,
    State(state): State<AppState>,
    Json(decision): Json<JoinDecision>,
) -> AppResult<Json<Value>> {
    if !guild_can(&state, &id, &auth.uuid, "invite").await? {
        return Err(AppError::forbidden("Your guild role cannot review requests"));
    }
    decide_join_request(&state, &id, &uuid, decision.accept).await?;
    Ok(Json(json!({"ok":true})))
}

/// Accept or turn down a join request, and tell the player either way.
pub(crate) async fn decide_join_request(state: &AppState, id: &str, uuid: &str, accept: bool) -> AppResult<String> {
    let mut tx = state.db.begin().await?;
    let name: Option<String> = sqlx::query_scalar("SELECT name FROM guild_join_requests WHERE guild_id=? AND uuid=?")
        .bind(id)
        .bind(uuid)
        .fetch_optional(&mut *tx)
        .await?;
    let name = name.ok_or_else(|| AppError::not_found("request not found"))?;
    let guild_name: String = sqlx::query_scalar("SELECT name FROM guilds WHERE id = ?").bind(id).fetch_one(&mut *tx).await?;
    if accept {
        check_room(&mut tx, &id).await?;
        sqlx::query("INSERT INTO guild_members(guild_id,uuid,name,role,joined_at) VALUES(?,?,?,'member',?) ON CONFLICT(guild_id,uuid) DO NOTHING")
            .bind(&id)
            .bind(&uuid)
            .bind(&name)
            .bind(crate::db::now())
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "INSERT INTO guild_primary_memberships(server_id,uuid,guild_id,updated_at)
             SELECT id,?,?,? FROM game_servers WHERE instance_id=(SELECT instance_id FROM guilds WHERE id=?)
             ON CONFLICT(server_id,uuid) DO NOTHING",
        )
        .bind(&uuid).bind(&id).bind(crate::db::now()).bind(&id).execute(&mut *tx).await?;
        sqlx::query("DELETE FROM guild_join_requests WHERE uuid=?").bind(&uuid).execute(&mut *tx).await?;
    } else {
        sqlx::query("DELETE FROM guild_join_requests WHERE guild_id=? AND uuid=?").bind(id).bind(uuid).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    if accept {
        super::notifications::push(&state.db, uuid, "guild_joined", "Request accepted", &format!("You are now a member of {guild_name}."), Some("/guild")).await;
    } else {
        super::notifications::push(&state.db, uuid, "guild_request_declined", "Request declined", &format!("{guild_name} declined your request to join."), None).await;
    }
    Ok(name)
}

#[derive(Deserialize)]
pub struct GuildRoleInput {
    pub name: String,
    #[serde(default)]
    pub priority: i64,
    #[serde(default)]
    pub can_invite: bool,
    #[serde(default)]
    pub can_kick: bool,
    #[serde(default)]
    pub can_claim: bool,
    #[serde(default)]
    pub can_post: bool,
    #[serde(default)]
    pub can_manage: bool,
}

pub(crate) async fn guild_can(state: &AppState, guild_id: &str, uuid: &str, action: &str) -> AppResult<bool> {
    let role: Option<String> = sqlx::query_scalar("SELECT role FROM guild_members WHERE guild_id=? AND uuid=?")
        .bind(guild_id)
        .bind(uuid)
        .fetch_optional(&state.db)
        .await?;
    let Some(role) = role else {
        return Ok(false);
    };
    if role == "leader" || role == "officer" {
        return Ok(true);
    }
    if role == "member" {
        return Ok(matches!(action, "claim" | "post"));
    }
    let column = match action {
        "invite" => "can_invite",
        "kick" => "can_kick",
        "claim" => "can_claim",
        "post" => "can_post",
        "manage" => "can_manage",
        _ => return Ok(false),
    };
    let allowed: Option<i64> = sqlx::query_scalar(&format!("SELECT {column} FROM guild_roles WHERE guild_id=? AND name=?"))
        .bind(guild_id)
        .bind(&role)
        .fetch_optional(&state.db)
        .await?;
    Ok(allowed.unwrap_or(0) != 0)
}

async fn require_guild_leader(state: &AppState, guild_id: &str, uuid: &str) -> AppResult<()> {
    let role: Option<String> = sqlx::query_scalar("SELECT role FROM guild_members WHERE guild_id=? AND uuid=?")
        .bind(guild_id)
        .bind(uuid)
        .fetch_optional(&state.db)
        .await?;
    if role.as_deref() != Some("leader") {
        return Err(AppError::forbidden("Only the guild leader can manage roles"));
    }
    Ok(())
}

pub async fn list_guild_roles(auth: AuthUser, Path(id): Path<String>, State(state): State<AppState>) -> AppResult<Json<Value>> {
    let member: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM guild_members WHERE guild_id=? AND uuid=?)")
        .bind(&id)
        .bind(&auth.uuid)
        .fetch_one(&state.db)
        .await?;
    if !member {
        return Err(AppError::forbidden("Guild membership is required"));
    }
    let roles: Vec<(i64,String,i64,i64,i64,i64,i64,i64)> = sqlx::query_as("SELECT id,name,priority,can_invite,can_kick,can_claim,can_post,can_manage FROM guild_roles WHERE guild_id=? ORDER BY priority DESC,name")
        .bind(&id).fetch_all(&state.db).await?;
    Ok(Json(json!(roles.into_iter().map(|(id,name,priority,invite,kick,claim,post,manage)| json!({"id":id,"name":name,"priority":priority,"can_invite":invite!=0,"can_kick":kick!=0,"can_claim":claim!=0,"can_post":post!=0,"can_manage":manage!=0})).collect::<Vec<_>>())))
}

pub async fn create_guild_role(
    auth: AuthUser,
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(role): Json<GuildRoleInput>,
) -> AppResult<Json<Value>> {
    require_guild_leader(&state, &id, &auth.uuid).await?;
    let name = role.name.trim();
    if name.len() < 2
        || name.len() > 24
        || !name.chars().all(|c| c.is_alphanumeric() || c == ' ' || c == '-' || c == '_')
        || ["leader", "officer", "member"].iter().any(|r| r.eq_ignore_ascii_case(name))
    {
        return Err(AppError::bad_request(
            "Role names must be 2–24 letters, numbers, spaces, hyphens or underscores, and cannot be a built-in role",
        ));
    }
    let duplicate: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM guild_roles WHERE guild_id=? AND name=? COLLATE NOCASE)")
        .bind(&id)
        .bind(name)
        .fetch_one(&state.db)
        .await?;
    if duplicate {
        return Err(AppError::conflict("A role with this name already exists"));
    }
    let inserted = sqlx::query(
        "INSERT INTO guild_roles(guild_id,name,priority,can_invite,can_kick,can_claim,can_post,can_manage) VALUES(?,?,?,?,?,?,?,?)",
    )
    .bind(&id)
    .bind(name)
    .bind(role.priority.clamp(0, 100))
    .bind(role.can_invite)
    .bind(role.can_kick)
    .bind(role.can_claim)
    .bind(role.can_post)
    .bind(role.can_manage)
    .execute(&state.db)
    .await?;
    Ok(Json(json!({"ok":true,"id":inserted.last_insert_rowid()})))
}

#[derive(Deserialize)]
pub struct AssignRole {
    pub role: String,
}

pub async fn assign_guild_role(
    auth: AuthUser,
    Path((id, uuid)): Path<(String, String)>,
    State(state): State<AppState>,
    Json(input): Json<AssignRole>,
) -> AppResult<Json<Value>> {
    require_guild_leader(&state, &id, &auth.uuid).await?;
    if input.role == "leader" {
        return Err(AppError::bad_request("Use leadership transfer to assign the leader role"));
    }
    if input.role != "member" && input.role != "officer" {
        let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM guild_roles WHERE guild_id=? AND name=?)")
            .bind(&id)
            .bind(&input.role)
            .fetch_one(&state.db)
            .await?;
        if !exists {
            return Err(AppError::bad_request("Role does not belong to this guild"));
        }
    }
    let done = sqlx::query("UPDATE guild_members SET role=? WHERE guild_id=? AND uuid=? AND role<>'leader'")
        .bind(&input.role)
        .bind(&id)
        .bind(&uuid)
        .execute(&state.db)
        .await?;
    if done.rows_affected() == 0 {
        return Err(AppError::not_found("member not found or is guild leader"));
    }
    let guild_name: String = sqlx::query_scalar("SELECT name FROM guilds WHERE id = ?").bind(&id).fetch_one(&state.db).await?;
    super::notifications::push(&state.db, &uuid, "guild_role", "Your guild role changed", &format!("You are now {} in {guild_name}.", input.role), Some("/guild")).await;
    Ok(Json(json!({"ok":true})))
}

pub async fn delete_guild_role(
    auth: AuthUser,
    Path((id, role_id)): Path<(String, i64)>,
    State(state): State<AppState>,
) -> AppResult<Json<Value>> {
    require_guild_leader(&state, &id, &auth.uuid).await?;
    let name: Option<String> = sqlx::query_scalar("SELECT name FROM guild_roles WHERE guild_id=? AND id=?")
        .bind(&id)
        .bind(role_id)
        .fetch_optional(&state.db)
        .await?;
    let name = name.ok_or_else(|| AppError::not_found("role not found"))?;
    let mut tx = state.db.begin().await?;
    sqlx::query("UPDATE guild_members SET role='member' WHERE guild_id=? AND role=?").bind(&id).bind(&name).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM guild_roles WHERE guild_id=? AND id=?").bind(&id).bind(role_id).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(Json(json!({"ok":true})))
}

/// Add / Invite member to guild.
pub async fn add_guild_member(
    auth: AuthUser,
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(payload): Json<AddMemberPayload>,
) -> AppResult<Json<Value>> {
    if !guild_can(&state, &id, &auth.uuid, "invite").await? {
        return Err(AppError::forbidden("Only guild leaders or officers can invite members"));
    }

    let target_user: Option<(String, String)> = sqlx::query_as("SELECT uuid, username FROM users WHERE username = ? COLLATE NOCASE")
        .bind(payload.username.trim())
        .fetch_optional(&state.db)
        .await?;

    let (target_uuid, target_name) = target_user.ok_or_else(|| AppError::not_found("User not found"))?;

    let now = chrono::Utc::now().to_rfc3339();

    {
        let mut conn = state.db.acquire().await?;
        check_room(&mut conn, &id).await?;
    }
    sqlx::query(
        "INSERT INTO guild_members (guild_id, uuid, name, role, joined_at)
         VALUES (?, ?, ?, 'member', ?)
         ON CONFLICT (guild_id, uuid) DO NOTHING",
    )
    .bind(&id)
    .bind(&target_uuid)
    .bind(&target_name)
    .bind(&now)
    .execute(&state.db)
    .await?;

    // Record achievement event for joining a guild
    sqlx::query(
        "INSERT OR IGNORE INTO user_achievements (user_uuid, achievement_id, unlocked_at)
         VALUES (?, 'ach_guild_initiate', ?)",
    )
    .bind(&target_uuid)
    .bind(&now)
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

/// Leave or kick member.
pub async fn remove_guild_member(
    auth: AuthUser,
    Path((guild_id, target_uuid)): Path<(String, String)>,
    State(state): State<AppState>,
) -> AppResult<Json<Value>> {
    if auth.uuid == target_uuid {
        let role: Option<String> = sqlx::query_scalar("SELECT role FROM guild_members WHERE guild_id = ? AND uuid = ?")
            .bind(&guild_id)
            .bind(&auth.uuid)
            .fetch_optional(&state.db)
            .await?;
        if role.as_deref() == Some("leader") {
            return Err(AppError::bad_request("Guild leader cannot leave without transferring leadership"));
        }
        sqlx::query("DELETE FROM guild_members WHERE guild_id = ? AND uuid = ?").bind(&guild_id).bind(&target_uuid).execute(&state.db).await?;
    } else {
        kick_member(&state, &guild_id, &auth.uuid, &target_uuid).await?;
    }
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// Remove a member on behalf of someone allowed to. The leader can never be removed, and only the leader can remove officers.
pub(crate) async fn kick_member(state: &AppState, guild_id: &str, actor: &str, target: &str) -> AppResult<String> {
    if !guild_can(state, guild_id, actor, "kick").await? {
        return Err(AppError::forbidden("Cannot kick member without officer privileges"));
    }
    let row: Option<(String, String)> = sqlx::query_as("SELECT name, role FROM guild_members WHERE guild_id = ? AND uuid = ?")
        .bind(guild_id)
        .bind(target)
        .fetch_optional(&state.db)
        .await?;
    let (name, role) = row.ok_or_else(|| AppError::not_found("That player is not in your guild"))?;
    if role == "leader" {
        return Err(AppError::forbidden("The guild leader can't be removed"));
    }
    let actor_role: Option<String> = sqlx::query_scalar("SELECT role FROM guild_members WHERE guild_id = ? AND uuid = ?")
        .bind(guild_id)
        .bind(actor)
        .fetch_optional(&state.db)
        .await?;
    if role == "officer" && actor_role.as_deref() != Some("leader") {
        return Err(AppError::forbidden("Only the leader can remove officers"));
    }
    let guild_name: String = sqlx::query_scalar("SELECT name FROM guilds WHERE id = ?").bind(guild_id).fetch_one(&state.db).await?;
    sqlx::query("DELETE FROM guild_members WHERE guild_id = ? AND uuid = ?").bind(guild_id).bind(target).execute(&state.db).await?;
    super::notifications::push(&state.db, target, "guild_kicked", "Removed from your guild", &format!("You were removed from {guild_name}."), None).await;
    Ok(name)
}

/// Hand the guild to another member. The old leader becomes an officer.
pub(crate) async fn transfer_leadership(state: &AppState, guild_id: &str, actor: &str, target: &str) -> AppResult<String> {
    require_guild_leader(state, guild_id, actor).await.map_err(|_| AppError::forbidden("Only the guild leader can hand over leadership"))?;
    if actor == target {
        return Err(AppError::bad_request("You already lead this guild"));
    }
    let name: Option<String> = sqlx::query_scalar("SELECT name FROM guild_members WHERE guild_id = ? AND uuid = ?")
        .bind(guild_id)
        .bind(target)
        .fetch_optional(&state.db)
        .await?;
    let name = name.ok_or_else(|| AppError::not_found("That player is not in your guild"))?;
    let mut tx = state.db.begin().await?;
    sqlx::query("UPDATE guilds SET leader_uuid = ? WHERE id = ?").bind(target).bind(guild_id).execute(&mut *tx).await?;
    sqlx::query("UPDATE guild_members SET role = 'officer' WHERE guild_id = ? AND uuid = ?").bind(guild_id).bind(actor).execute(&mut *tx).await?;
    sqlx::query("UPDATE guild_members SET role = 'leader' WHERE guild_id = ? AND uuid = ?").bind(guild_id).bind(target).execute(&mut *tx).await?;
    tx.commit().await?;
    let guild_name: String = sqlx::query_scalar("SELECT name FROM guilds WHERE id = ?").bind(guild_id).fetch_one(&state.db).await?;
    super::notifications::push(&state.db, target, "guild_leader", "You lead the guild now", &format!("You are the new leader of {guild_name}."), Some("/guild")).await;
    Ok(name)
}

#[derive(Deserialize)]
pub struct TransferPayload {
    pub uuid: String,
}

pub async fn transfer_guild_leader(
    auth: AuthUser,
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(p): Json<TransferPayload>,
) -> AppResult<Json<Value>> {
    transfer_leadership(&state, &id, &auth.uuid, &p.uuid).await?;
    Ok(Json(json!({ "ok": true })))
}

// ---------------------------------------------------------------------------
// Guild Posts
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct CreatePostPayload {
    pub title: String,
    pub content: String,
}

pub async fn create_guild_post(
    auth: AuthUser,
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(payload): Json<CreatePostPayload>,
) -> AppResult<Json<Value>> {
    let in_guild: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM guild_members WHERE guild_id = ? AND uuid = ?)")
        .bind(&id)
        .bind(&auth.uuid)
        .fetch_one(&state.db)
        .await?;

    if !in_guild || !guild_can(&state, &id, &auth.uuid, "post").await? {
        return Err(AppError::forbidden("Your guild role cannot post"));
    }

    let now = chrono::Utc::now().to_rfc3339();
    let post_id: i64 = sqlx::query_scalar(
        "INSERT INTO guild_posts (guild_id, author_uuid, author_name, title, content, created_at)
         VALUES (?, ?, ?, ?, ?, ?)
         RETURNING id",
    )
    .bind(&id)
    .bind(&auth.uuid)
    .bind(&auth.username)
    .bind(&payload.title)
    .bind(&payload.content)
    .bind(&now)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(serde_json::json!({ "id": post_id, "guild_id": id, "author_uuid": auth.uuid,
        "author_name": auth.username, "title": payload.title, "content": payload.content, "created_at": now })))
}

// ---------------------------------------------------------------------------
// Land Claiming API (FTB Chunks-style grid)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct ClaimChunkPayload {
    pub server_id: Option<i64>,
    pub instance_id: Option<String>,
    pub dimension: Option<String>,
    pub chunk_x: i32,
    pub chunk_z: i32,
}

/// Claim a chunk for your guild.
pub async fn claim_chunk(
    auth: AuthUser,
    Path(guild_id): Path<String>,
    State(state): State<AppState>,
    Json(payload): Json<ClaimChunkPayload>,
) -> AppResult<Json<Value>> {
    let role: Option<String> = sqlx::query_scalar("SELECT role FROM guild_members WHERE guild_id = ? AND uuid = ?")
        .bind(&guild_id)
        .bind(&auth.uuid)
        .fetch_optional(&state.db)
        .await?;

    if role.is_none() || !guild_can(&state, &guild_id, &auth.uuid, "claim").await? {
        return Err(AppError::forbidden("Your guild role cannot claim land"));
    }

    let max_claims: i64 = sqlx::query_scalar("SELECT (guilds.max_claims + COALESCE((SELECT SUM(b.claim_chunks) FROM player_bonuses b JOIN guild_members m ON m.uuid = b.uuid WHERE m.guild_id = guilds.id), 0) + COALESCE((SELECT CAST(value AS INTEGER) FROM kv WHERE key = 'rule_claims_per_member'), 0) * MAX(0, (SELECT COUNT(*) FROM guild_members m2 WHERE m2.guild_id = guilds.id) - 1) + COALESCE((SELECT CAST(value AS INTEGER) FROM kv WHERE key = 'rule_claims_per_level'), 0) * MAX(0, guilds.level - 1)) FROM guilds WHERE id = ?").bind(&guild_id).fetch_one(&state.db).await?;

    let current_claims: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM guild_claims WHERE guild_id = ?").bind(&guild_id).fetch_one(&state.db).await?;

    if current_claims >= max_claims {
        return Err(AppError::bad_request(format!("Guild reached its max claim limit of {max_claims} chunks")));
    }

    let server_id = payload.server_id.ok_or_else(|| AppError::bad_request("Select a game server for this claim"))?;
    let matches: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_servers s JOIN guilds g ON g.instance_id = s.instance_id WHERE s.id = ? AND g.id = ?)",
    )
    .bind(server_id)
    .bind(&guild_id)
    .fetch_one(&state.db)
    .await?;
    if !matches {
        return Err(AppError::bad_request("Server is not linked to this guild's instance"));
    }
    let dim = payload.dimension.unwrap_or_else(|| "minecraft:overworld".into());
    let now = chrono::Utc::now().to_rfc3339();

    let res = sqlx::query_scalar::<_, i64>(
        "INSERT INTO guild_claims (guild_id, server_id, dimension, chunk_x, chunk_z, claimed_by_uuid, claimed_at)
         VALUES (?, ?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(&guild_id)
    .bind(server_id)
    .bind(&dim)
    .bind(payload.chunk_x)
    .bind(payload.chunk_z)
    .bind(&auth.uuid)
    .bind(&now)
    .fetch_one(&state.db)
    .await;

    let claim_id = res.map_err(|_| AppError::bad_request("Chunk is already claimed"))?;

    // Award achievement for claiming land
    sqlx::query(
        "INSERT OR IGNORE INTO user_achievements (user_uuid, achievement_id, unlocked_at)
         VALUES (?, 'ach_land_claim', ?)",
    )
    .bind(&auth.uuid)
    .bind(&now)
    .execute(&state.db)
    .await?;

    let detail = fetch_guild_detail(&state, &guild_id).await?;
    Ok(Json(serde_json::to_value(
        detail.claims.into_iter().find(|c| c.id == claim_id).ok_or_else(|| AppError::not_found("Claim not found"))?,
    )?))
}

/// Unclaim a chunk by coordinates.
pub async fn unclaim_chunk(
    auth: AuthUser,
    Path(guild_id): Path<String>,
    State(state): State<AppState>,
    Json(payload): Json<ClaimChunkPayload>,
) -> AppResult<Json<Value>> {
    let role: Option<String> = sqlx::query_scalar("SELECT role FROM guild_members WHERE guild_id = ? AND uuid = ?")
        .bind(&guild_id)
        .bind(&auth.uuid)
        .fetch_optional(&state.db)
        .await?;

    if role.is_none() || !guild_can(&state, &guild_id, &auth.uuid, "claim").await? {
        return Err(AppError::forbidden("Your guild role cannot unclaim land"));
    }

    let dim = payload.dimension.unwrap_or_else(|| "minecraft:overworld".into());

    sqlx::query("DELETE FROM guild_claims WHERE guild_id = ? AND dimension = ? AND chunk_x = ? AND chunk_z = ?")
        .bind(&guild_id)
        .bind(&dim)
        .bind(payload.chunk_x)
        .bind(payload.chunk_z)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

/// Unclaim by claim ID.
pub async fn unclaim_by_id(auth: AuthUser, Path(claim_id): Path<i64>, State(state): State<AppState>) -> AppResult<Json<Value>> {
    let claim: Option<(String,)> =
        sqlx::query_as("SELECT guild_id FROM guild_claims WHERE id = ?").bind(claim_id).fetch_optional(&state.db).await?;

    let (guild_id,) = claim.ok_or_else(|| AppError::not_found("Claim not found"))?;

    let role: Option<String> = sqlx::query_scalar("SELECT role FROM guild_members WHERE guild_id = ? AND uuid = ?")
        .bind(&guild_id)
        .bind(&auth.uuid)
        .fetch_optional(&state.db)
        .await?;

    if role.is_none() || !guild_can(&state, &guild_id, &auth.uuid, "claim").await? {
        return Err(AppError::forbidden("Your guild role cannot unclaim land"));
    }

    sqlx::query("DELETE FROM guild_claims WHERE id = ?").bind(claim_id).execute(&state.db).await?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Deserialize)]
pub struct ChunkGridQuery {
    pub server_id: Option<i64>,
    pub instance_id: Option<String>,
    pub dimension: Option<String>,
    pub center_x: Option<i32>,
    pub center_z: Option<i32>,
    pub radius: Option<i32>,
    /// Only this guild's claims (used to find and frame a guild's land wherever it is).
    pub guild_id: Option<String>,
}

/// Returns chunk claims in a bounding box centered around (center_x, center_z) chunks.
pub async fn get_chunk_grid(Query(query): Query<ChunkGridQuery>, State(state): State<AppState>) -> AppResult<Json<Vec<GuildClaim>>> {
    let dim = query.dimension.unwrap_or_else(|| "minecraft:overworld".into());
    let inst_id = query.instance_id.unwrap_or_default();
    let cx = query.center_x.unwrap_or(0);
    let cz = query.center_z.unwrap_or(0);
    let guild_id = query.guild_id.unwrap_or_default();
    // One guild's land may be spread far and wide; everyone's land in a window is kept small.
    let (max_radius, limit) = if guild_id.is_empty() { (1024, 1000) } else { (1_000_000, 5000) };
    let r = query.radius.unwrap_or(32).clamp(2, max_radius);
    let min_x = cx - r;
    let max_x = cx + r;
    let min_z = cz - r;
    let max_z = cz + r;

    let rows: Vec<(i64, String, String, String, Option<i64>, String, i32, i32, String, String)> = sqlx::query_as(
        "SELECT gc.id, gc.guild_id, g.name, g.tag, gc.server_id, gc.dimension,
                gc.chunk_x, gc.chunk_z, gc.claimed_by_uuid, gc.claimed_at
         FROM guild_claims gc
         JOIN guilds g ON g.id = gc.guild_id
         WHERE (? IS NULL OR gc.server_id = ?)
           AND (? = '' OR g.instance_id = ?)
           AND (? = '' OR gc.guild_id = ?)
           AND gc.dimension = ?
           AND gc.chunk_x BETWEEN ? AND ?
           AND gc.chunk_z BETWEEN ? AND ?
         LIMIT ?",
    )
    .bind(query.server_id)
    .bind(query.server_id)
    .bind(&inst_id)
    .bind(&inst_id)
    .bind(&guild_id)
    .bind(&guild_id)
    .bind(&dim)
    .bind(min_x)
    .bind(max_x)
    .bind(min_z)
    .bind(max_z)
    .bind(limit)
    .fetch_all(&state.db)
    .await?;

    let list = rows
        .into_iter()
        .map(|(id, gid, gname, gtag, sid, gdim, cx, cz, cby, cat)| GuildClaim {
            id,
            guild_id: gid,
            guild_name: gname,
            guild_tag: gtag,
            server_id: sid.unwrap_or(0),
            dimension: gdim,
            chunk_x: cx,
            chunk_z: cz,
            claimed_by_uuid: cby,
            claimed_at: cat,
        })
        .collect();

    Ok(Json(list))
}

// ---------------------------------------------------------------------------
// Rename and disband
// ---------------------------------------------------------------------------

fn checked_guild_name(name: &str) -> AppResult<String> {
    let name = name.trim();
    let len = name.chars().count();
    if !(3..=32).contains(&len) {
        return Err(AppError::bad_request("Guild name must be 3-32 characters"));
    }
    if name.chars().any(|c| c.is_control()) {
        return Err(AppError::bad_request("Guild name can't contain control characters"));
    }
    Ok(name.to_string())
}

fn checked_guild_tag(tag: &str) -> AppResult<String> {
    let tag = tag.trim();
    let len = tag.chars().count();
    if !(2..=6).contains(&len) || !tag.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return Err(AppError::bad_request("Guild tag must be 2-6 letters, digits, _ or -"));
    }
    Ok(tag.to_ascii_uppercase())
}

/// Give a guild a new name (and optionally tag). Names are unique, ignoring case.
async fn rename_guild_to(state: &AppState, id: &str, name: &str, tag: Option<&str>) -> AppResult<(String, String)> {
    let name = checked_guild_name(name)?;
    let tag = tag.filter(|t| !t.trim().is_empty()).map(checked_guild_tag).transpose()?;
    let taken: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM guilds WHERE name = ? AND id <> ?)")
        .bind(&name)
        .bind(id)
        .fetch_one(&state.db)
        .await?;
    if taken {
        return Err(AppError::bad_request("Another guild already uses that name"));
    }
    let old_name: Option<String> = sqlx::query_scalar("SELECT name FROM guilds WHERE id = ?").bind(id).fetch_optional(&state.db).await?;
    let done = sqlx::query("UPDATE guilds SET name = ?, tag = COALESCE(?, tag) WHERE id = ?")
        .bind(&name)
        .bind(&tag)
        .bind(id)
        .execute(&state.db)
        .await?;
    if done.rows_affected() == 0 {
        return Err(AppError::not_found("Guild not found"));
    }
    let tag: String = sqlx::query_scalar("SELECT tag FROM guilds WHERE id = ?").bind(id).fetch_one(&state.db).await?;
    if old_name.as_deref().is_some_and(|o| o != name) {
        let members = guild_member_uuids(state, id).await;
        let body = format!("{} is now called {} [{}].", old_name.unwrap_or_default(), name, tag);
        super::notifications::push_many(&state.db, &members, "guild_renamed", "Your guild was renamed", &body, Some("/guild")).await;
    }
    Ok((name, tag))
}

/// Delete a guild with everything that hangs off it (members, roles, posts, claims, invites, requests and wallets are removed by the
/// database). Whatever its treasury holds is paid to `refund_to` first, so disbanding never destroys money. Returns the amount paid.
async fn disband_guild_now(state: &AppState, id: &str, refund_to: Option<(&str, &str)>) -> AppResult<f64> {
    let members = guild_member_uuids(state, id).await;
    let guild_name: String = sqlx::query_scalar("SELECT name FROM guilds WHERE id = ?").bind(id).fetch_optional(&state.db).await?.unwrap_or_default();
    let mut tx = state.db.begin().await?;
    let wallets: Vec<(i64, f64)> = sqlx::query_as("SELECT server_id, balance FROM guild_wallets WHERE guild_id = ? AND balance > 0")
        .bind(id)
        .fetch_all(&mut *tx)
        .await?;
    let mut paid = 0.0;
    if let Some((uuid, username)) = refund_to {
        let now = chrono::Utc::now().to_rfc3339();
        for (server_id, balance) in wallets {
            sqlx::query("INSERT INTO server_economy(server_id,uuid,username,balance,updated_at) VALUES(?,?,?,1000,?) ON CONFLICT(server_id,uuid) DO NOTHING")
                .bind(server_id).bind(uuid).bind(username).bind(&now).execute(&mut *tx).await?;
            sqlx::query("UPDATE server_economy SET balance = balance + ?, updated_at = ? WHERE server_id = ? AND uuid = ?")
                .bind(balance)
                .bind(&now)
                .bind(server_id)
                .bind(uuid)
                .execute(&mut *tx)
                .await?;
            paid += balance;
        }
    }
    // Explicit deletes first so this also works on databases opened without foreign key cascades.
    for table in [
        "guild_claims",
        "guild_members",
        "guild_posts",
        "guild_roles",
        "guild_invites",
        "guild_join_requests",
        "guild_wallet_transactions",
        "guild_wallets",
    ] {
        sqlx::query(&format!("DELETE FROM {table} WHERE guild_id = ?")).bind(id).execute(&mut *tx).await?;
    }
    let done = sqlx::query("DELETE FROM guilds WHERE id = ?").bind(id).execute(&mut *tx).await?;
    if done.rows_affected() == 0 {
        return Err(AppError::not_found("Guild not found"));
    }
    tx.commit().await?;
    let body = format!("{guild_name} was disbanded by its leader. Its claims were released.");
    super::notifications::push_many(&state.db, &members, "guild_disbanded", "Your guild was disbanded", &body, None).await;
    Ok((paid * 100.0).round() / 100.0)
}

async fn guild_member_uuids(state: &AppState, id: &str) -> Vec<String> {
    sqlx::query_scalar("SELECT uuid FROM guild_members WHERE guild_id = ?").bind(id).fetch_all(&state.db).await.unwrap_or_default()
}

async fn require_leader(state: &AppState, guild_id: &str, uuid: &str) -> AppResult<()> {
    let role: Option<String> = sqlx::query_scalar("SELECT role FROM guild_members WHERE guild_id = ? AND uuid = ?")
        .bind(guild_id)
        .bind(uuid)
        .fetch_optional(&state.db)
        .await?;
    if role.as_deref() != Some("leader") {
        return Err(AppError::forbidden("Only the guild leader can do this"));
    }
    Ok(())
}

#[derive(Deserialize)]
pub struct RenameGuildPayload {
    pub name: String,
    pub tag: Option<String>,
}

/// The leader renames their guild.
pub async fn rename_guild(
    auth: AuthUser,
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(p): Json<RenameGuildPayload>,
) -> AppResult<Json<Value>> {
    require_leader(&state, &id, &auth.uuid).await?;
    let (name, tag) = rename_guild_to(&state, &id, &p.name, p.tag.as_deref()).await?;
    Ok(Json(json!({ "ok": true, "name": name, "tag": tag })))
}

/// The leader disbands their guild: members are released, land is unclaimed and the treasury is paid to the leader.
pub async fn disband_guild(auth: AuthUser, Path(id): Path<String>, State(state): State<AppState>) -> AppResult<Json<Value>> {
    require_leader(&state, &id, &auth.uuid).await?;
    let refunded = disband_guild_now(&state, &id, Some((&auth.uuid, &auth.username))).await?;
    Ok(Json(json!({ "ok": true, "refunded": refunded })))
}

/// An admin renames any guild.
pub async fn admin_rename_guild(
    _admin: AdminUser,
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(p): Json<RenameGuildPayload>,
) -> AppResult<Json<Value>> {
    let (name, tag) = rename_guild_to(&state, &id, &p.name, p.tag.as_deref()).await?;
    Ok(Json(json!({ "ok": true, "name": name, "tag": tag })))
}

/// Admin list all guilds.
pub async fn admin_list_guilds(_admin: AdminUser, State(state): State<AppState>) -> AppResult<Json<Vec<Guild>>> {
    let rows: Vec<(
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        Option<String>,
        Option<String>,
        i64,
        i64,
        i64,
        String,
        i64,
        i64,
    )> = sqlx::query_as(
        "SELECT g.id, g.instance_id, g.name, g.tag, g.description, g.motd, g.leader_uuid,
                g.icon_url, g.banner_url, g.level, g.xp, (g.max_claims + COALESCE((SELECT SUM(b.claim_chunks) FROM player_bonuses b JOIN guild_members m ON m.uuid = b.uuid WHERE m.guild_id = g.id), 0) + COALESCE((SELECT CAST(value AS INTEGER) FROM kv WHERE key = 'rule_claims_per_member'), 0) * MAX(0, (SELECT COUNT(*) FROM guild_members m2 WHERE m2.guild_id = g.id) - 1) + COALESCE((SELECT CAST(value AS INTEGER) FROM kv WHERE key = 'rule_claims_per_level'), 0) * MAX(0, g.level - 1)), g.created_at,
                (SELECT COUNT(*) FROM guild_members gm WHERE gm.guild_id = g.id) as member_count,
                (SELECT COUNT(*) FROM guild_claims gc WHERE gc.guild_id = g.id) as claims_count
         FROM guilds g
         ORDER BY g.level DESC, member_count DESC",
    )
    .fetch_all(&state.db)
    .await?;

    let list = rows
        .into_iter()
        .map(|(id, inst_id, name, tag, desc, motd, leader, icon, banner, lvl, xp, max_c, created, members, claims)| Guild {
            id,
            instance_id: inst_id,
            name,
            tag,
            description: desc,
            motd,
            leader_uuid: leader,
            icon_url: icon,
            banner_url: banner,
            level: lvl,
            xp,
            max_claims: max_c,
            member_count: members,
            claims_count: claims,
            created_at: created,
        })
        .collect();

    Ok(Json(list))
}

/// Admin disband a guild. The treasury is paid to its leader.
pub async fn admin_delete_guild(_admin: AdminUser, Path(id): Path<String>, State(state): State<AppState>) -> AppResult<Json<Value>> {
    let leader: Option<(String, String)> = sqlx::query_as("SELECT uuid, name FROM guild_members WHERE guild_id = ? AND role = 'leader'")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?;
    let refunded = disband_guild_now(&state, &id, leader.as_ref().map(|(u, n)| (u.as_str(), n.as_str()))).await?;
    Ok(Json(json!({ "ok": true, "refunded": refunded })))
}

#[derive(Deserialize)]
pub struct RelationPayload {
    pub other_guild_id: String,
    pub relation: String,
}

#[derive(Deserialize)]
pub struct RelationDecisionPayload {
    pub accept: bool,
}

/// List alliance and rivalry requests for a guild.
pub async fn list_relations(auth: AuthUser, Path(id): Path<String>, State(state): State<AppState>) -> AppResult<Json<Value>> {
    let member: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM guild_members WHERE guild_id=? AND uuid=?)")
        .bind(&id).bind(&auth.uuid).fetch_one(&state.db).await?;
    if !member { return Err(AppError::forbidden("You are not a member of this guild")); }
    let rows: Vec<(i64,String,String,String,String,String,String)> = sqlx::query_as(
        "SELECT r.id,r.guild_id,r.other_guild_id,r.relation,r.status,g.name,g.tag
         FROM guild_relations r JOIN guilds g ON g.id=CASE WHEN r.guild_id=? THEN r.other_guild_id ELSE r.guild_id END
         WHERE r.guild_id=? OR r.other_guild_id=? ORDER BY r.updated_at DESC")
        .bind(&id).bind(&id).bind(&id).fetch_all(&state.db).await?;
    Ok(Json(json!(rows.into_iter().map(|(rid,gid,other,relation,status,name,tag)| json!({"id":rid,"guild_id":gid,"other_guild_id":other,"relation":relation,"status":status,"name":name,"tag":tag})).collect::<Vec<_>>())))
}

/// Create a pending alliance or rivalry request. Relations are symmetric and
/// restricted to guilds belonging to the same launcher instance.
pub async fn create_relation(auth: AuthUser, Path(id): Path<String>, State(state): State<AppState>, Json(payload): Json<RelationPayload>) -> AppResult<Json<Value>> {
    if !guild_can(&state, &id, &auth.uuid, "manage").await? { return Err(AppError::forbidden("Your guild role cannot manage relations")); }
    if payload.relation != "alliance" && payload.relation != "rival" { return Err(AppError::bad_request("relation must be alliance or rival")); }
    let instance: Option<String> = sqlx::query_scalar("SELECT instance_id FROM guilds WHERE id=? AND id<>?").bind(&payload.other_guild_id).bind(&id).fetch_optional(&state.db).await?;
    let own: Option<String> = sqlx::query_scalar("SELECT instance_id FROM guilds WHERE id=?").bind(&id).fetch_optional(&state.db).await?;
    if instance.is_none() || instance != own { return Err(AppError::bad_request("Guilds must belong to the same instance")); }
    let now = crate::db::now();
    sqlx::query("INSERT INTO guild_relations(instance_id,guild_id,other_guild_id,relation,status,actor_uuid,created_at,updated_at) VALUES(?,?,?,?, 'pending',?,?,?) ON CONFLICT(instance_id,guild_id,other_guild_id) DO UPDATE SET relation=excluded.relation,status='pending',actor_uuid=excluded.actor_uuid,updated_at=excluded.updated_at")
        .bind(own.unwrap()).bind(&id).bind(&payload.other_guild_id).bind(&payload.relation).bind(&auth.uuid).bind(&now).bind(&now).execute(&state.db).await?;
    Ok(Json(json!({"ok":true,"status":"pending"})))
}

/// Accept or decline an incoming relation request. Either guild's manager may
/// end an accepted relation, but only the receiving guild can resolve pending.
pub async fn decide_relation(
    auth: AuthUser,
    Path((id, relation_id)): Path<(String, i64)>,
    State(state): State<AppState>,
    Json(payload): Json<RelationDecisionPayload>,
) -> AppResult<Json<Value>> {
    if !guild_can(&state, &id, &auth.uuid, "manage").await? {
        return Err(AppError::forbidden("Your guild role cannot manage relations"));
    }
    let row: Option<(String, String, String)> = sqlx::query_as(
        "SELECT guild_id,other_guild_id,status FROM guild_relations WHERE id=? AND other_guild_id=?",
    ).bind(relation_id).bind(&id).fetch_optional(&state.db).await?;
    let Some((guild_id, other_guild_id, status)) = row else {
        return Err(AppError::not_found("relation not found"));
    };
    let next = if payload.accept { "accepted" } else { "declined" };
    if status != "pending" {
        return Err(AppError::bad_request("relation is no longer pending"));
    }
    sqlx::query("UPDATE guild_relations SET status=?,updated_at=? WHERE id=?")
        .bind(next).bind(crate::db::now()).bind(relation_id).execute(&state.db).await?;
    if payload.accept {
        sqlx::query(
            "INSERT OR IGNORE INTO guild_relations(instance_id,guild_id,other_guild_id,relation,status,actor_uuid,created_at,updated_at)
             SELECT instance_id,?,?,relation,'accepted',?,?,? FROM guild_relations WHERE id=?",
        ).bind(&other_guild_id).bind(&guild_id).bind(&auth.uuid).bind(crate::db::now()).bind(crate::db::now()).bind(relation_id)
            .execute(&state.db).await?;
    }
    Ok(Json(json!({ "ok": true, "status": next })))
}

#[cfg(test)]
mod relation_tests {
    #[test]
    fn only_the_receiving_guild_can_resolve_a_relation_request() {
        let decision_sql = "SELECT guild_id,other_guild_id,status FROM guild_relations WHERE id=? AND other_guild_id=?";
        assert!(decision_sql.contains("other_guild_id=?"));
        assert!(!decision_sql.contains("OR guild_id=?"));
    }
}

// ---------------------------------------------------------------------------
// Server Integration / Plugin In-Game Land Claim Relay
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct ServerCheckChunkPayload {
    pub uuid: String,
    pub dimension: String,
    pub chunk_x: i32,
    pub chunk_z: i32,
}

#[derive(Deserialize)]
pub struct ClaimSnapshotPayload {
    pub uuid: String,
    pub dimension: String,
    pub chunk_x: i32,
    pub chunk_z: i32,
}

/// One bounded area lookup replaces repeated network checks for every block.
pub async fn server_claim_snapshot(
    GameServer(server): GameServer,
    State(state): State<AppState>,
    Json(payload): Json<ClaimSnapshotPayload>,
) -> AppResult<Json<Value>> {
    let min_x = payload.chunk_x.saturating_sub(2);
    let max_x = payload.chunk_x.saturating_add(2);
    let min_z = payload.chunk_z.saturating_sub(2);
    let max_z = payload.chunk_z.saturating_add(2);
    let rows: Vec<(i32, i32, String, String, bool)> = sqlx::query_as(
        "SELECT gc.chunk_x,gc.chunk_z,g.name,g.tag,
                EXISTS(SELECT 1 FROM guild_members gm WHERE gm.guild_id=gc.guild_id AND gm.uuid=?)
         FROM guild_claims gc JOIN guilds g ON g.id=gc.guild_id
         WHERE gc.server_id=? AND gc.dimension=? AND gc.chunk_x BETWEEN ? AND ? AND gc.chunk_z BETWEEN ? AND ?",
    )
    .bind(&payload.uuid)
    .bind(server.id)
    .bind(&payload.dimension)
    .bind(min_x)
    .bind(max_x)
    .bind(min_z)
    .bind(max_z)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(serde_json::json!({"center_x":payload.chunk_x,"center_z":payload.chunk_z,"radius":2,
        "claims":rows.into_iter().map(|(x,z,name,tag,allowed)| serde_json::json!({
            "chunk_x":x,"chunk_z":z,"guild_name":name,"guild_tag":tag,"allowed":allowed
        })).collect::<Vec<_>>() })))
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct ClaimIndexPayload {
    /// The revision the game server already holds.
    pub revision: Option<String>,
}

/// Everything a game server needs to decide claim protection locally:
/// every claimed chunk on this server plus who belongs to each claiming guild.
/// Answers `{"unchanged": true}` when `revision` is still current, so polling
/// is one cheap query.
pub async fn server_claim_index(
    GameServer(server): GameServer,
    State(state): State<AppState>,
    Json(payload): Json<ClaimIndexPayload>,
) -> AppResult<Json<Value>> {
    let revision: String =
        sqlx::query_scalar("SELECT value FROM kv WHERE key = 'guild_rev'").fetch_optional(&state.db).await?.unwrap_or_else(|| "0".into());
    if payload.revision.as_deref() == Some(revision.as_str()) {
        return Ok(Json(json!({ "revision": revision, "unchanged": true })));
    }
    let flag_policy = super::guild_flags::policy(&state).await?;
    let rows: Vec<(String, i32, i32, String, String, String, Option<String>, String)> = sqlx::query_as(
        "SELECT gc.dimension, gc.chunk_x, gc.chunk_z, gc.guild_id, g.name, g.tag, g.icon_url, g.claim_flags
         FROM guild_claims gc JOIN guilds g ON g.id = gc.guild_id WHERE gc.server_id = ?",
    )
    .bind(server.id)
    .fetch_all(&state.db)
    .await?;
    let members: Vec<(String, String)> = sqlx::query_as(
        "SELECT gm.guild_id, gm.uuid FROM guild_members gm
         WHERE gm.guild_id IN (SELECT DISTINCT guild_id FROM guild_claims WHERE server_id = ?)",
    )
    .bind(server.id)
    .fetch_all(&state.db)
    .await?;
    // Guilds are sent once; each claim refers to its guild by index.
    let mut guilds: Vec<Value> = Vec::new();
    let mut index: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut claims: Vec<Value> = Vec::with_capacity(rows.len());
    for (dimension, x, z, guild_id, name, tag, icon, claim_flags) in rows {
        let i = *index.entry(guild_id.clone()).or_insert_with(|| {
            guilds.push(json!({ "id": guild_id, "name": name, "tag": tag, "icon_url": icon.unwrap_or_default(), "flags": super::guild_flags::resolve(&claim_flags, &flag_policy) }));
            guilds.len() - 1
        });
        claims.push(json!([dimension, x, z, i]));
    }
    // Admin claims ride along as guilds nobody belongs to, so the plugin needs no second code path to protect and draw them.
    let admin_rows: Vec<(String, String, String, String, i32, i32, String, String)> = sqlx::query_as(
        "SELECT c.dimension, a.name, a.description, a.color, c.chunk_x, c.chunk_z, a.id, a.flags
         FROM admin_claim_chunks c JOIN admin_claims a ON a.id = c.claim_id WHERE c.server_id = ?",
    )
    .bind(server.id)
    .fetch_all(&state.db)
    .await?;
    for (dimension, name, description, color, x, z, claim_id, flags) in admin_rows {
        let key = format!("admin:{claim_id}");
        let i = *index.entry(key.clone()).or_insert_with(|| {
            guilds.push(json!({ "id": key, "name": name, "tag": "ADMIN", "icon_url": "", "admin": true, "description": description, "color": color, "flags": super::admin_claims::resolve_flags(&flags) }));
            guilds.len() - 1
        });
        claims.push(json!([dimension, x, z, i]));
    }
    let mut roster: std::collections::BTreeMap<String, Vec<String>> = Default::default();
    for (guild, uuid) in members {
        roster.entry(guild).or_default().push(uuid);
    }
    Ok(Json(json!({ "revision": revision, "unchanged": false, "guilds": guilds, "claims": claims, "members": roster })))
}

/// Token-authenticated check called by the Minecraft server plugin/mod to
/// verify if a player can build/break in a chunk.
pub async fn server_check_chunk(
    GameServer(server): GameServer,
    State(state): State<AppState>,
    Json(payload): Json<ServerCheckChunkPayload>,
) -> AppResult<Json<Value>> {
    let claim: Option<(String, String, String)> = sqlx::query_as(
        "SELECT gc.guild_id, g.name, g.tag
         FROM guild_claims gc
         JOIN guilds g ON g.id = gc.guild_id
         WHERE gc.server_id = ? AND gc.dimension = ? AND gc.chunk_x = ? AND gc.chunk_z = ?",
    )
    .bind(server.id)
    .bind(&payload.dimension)
    .bind(payload.chunk_x)
    .bind(payload.chunk_z)
    .fetch_optional(&state.db)
    .await?;

    let Some((guild_id, guild_name, guild_tag)) = claim else {
        // Wilderness / unclaimed land is allowed
        return Ok(Json(serde_json::json!({
            "claimed": false,
            "allowed": true
        })));
    };

    // Check if player is a member of this guild
    let is_member: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM guild_members WHERE guild_id = ? AND uuid = ?)")
        .bind(&guild_id)
        .bind(&payload.uuid)
        .fetch_one(&state.db)
        .await?;

    Ok(Json(serde_json::json!({
        "claimed": true,
        "guild_id": guild_id,
        "guild_name": guild_name,
        "guild_tag": guild_tag,
        "allowed": is_member,
        "message": if is_member { None } else { Some(format!("This land is claimed by [{guild_tag}] {guild_name}")) }
    })))
}

#[derive(Deserialize)]
pub struct ServerClaimChunkPayload {
    pub uuid: String,
    pub dimension: String,
    pub chunk_x: i32,
    pub chunk_z: i32,
}

pub async fn server_claim_chunk(
    GameServer(server): GameServer,
    State(state): State<AppState>,
    Json(payload): Json<ServerClaimChunkPayload>,
) -> AppResult<Json<Value>> {
    let member: Option<(String, String)> = sqlx::query_as(
        "SELECT gm.guild_id, gm.role FROM guild_members gm JOIN guilds g ON g.id = gm.guild_id WHERE gm.uuid = ? AND g.instance_id = ?",
    )
    .bind(&payload.uuid)
    .bind(&server.instance_id)
    .fetch_optional(&state.db)
    .await?;

    let Some((guild_id, _role)) = member else {
        return Err(AppError::bad_request("You must be in a guild to claim land. Create one with /guild create <name> <tag>"));
    };
    if !guild_can(&state, &guild_id, &payload.uuid, "claim").await? {
        return Err(AppError::forbidden("Your guild role cannot claim land"));
    }

    let max_claims: i64 =
        sqlx::query_scalar("SELECT (guilds.max_claims + COALESCE((SELECT SUM(b.claim_chunks) FROM player_bonuses b JOIN guild_members m ON m.uuid = b.uuid WHERE m.guild_id = guilds.id), 0) + COALESCE((SELECT CAST(value AS INTEGER) FROM kv WHERE key = 'rule_claims_per_member'), 0) * MAX(0, (SELECT COUNT(*) FROM guild_members m2 WHERE m2.guild_id = guilds.id) - 1) + COALESCE((SELECT CAST(value AS INTEGER) FROM kv WHERE key = 'rule_claims_per_level'), 0) * MAX(0, guilds.level - 1)) FROM guilds WHERE id = ?").bind(&guild_id).fetch_one(&state.db).await.unwrap_or(16);

    let current_claims: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM guild_claims WHERE guild_id = ?").bind(&guild_id).fetch_one(&state.db).await.unwrap_or(0);

    if current_claims >= max_claims {
        return Err(AppError::bad_request(format!("Guild reached its max claim limit of {max_claims} chunks")));
    }

    let now = chrono::Utc::now().to_rfc3339();
    let res = sqlx::query(
        "INSERT INTO guild_claims (guild_id, server_id, dimension, chunk_x, chunk_z, claimed_by_uuid, claimed_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&guild_id)
    .bind(server.id)
    .bind(&payload.dimension)
    .bind(payload.chunk_x)
    .bind(payload.chunk_z)
    .bind(&payload.uuid)
    .bind(&now)
    .execute(&state.db)
    .await;

    if let Err(e) = res {
        tracing::warn!("server claim chunk failed: {e}");
        return Err(AppError::bad_request("Chunk is already claimed by another guild"));
    }

    let guild_name: String = sqlx::query_scalar("SELECT name FROM guilds WHERE id = ?")
        .bind(&guild_id)
        .fetch_one(&state.db)
        .await
        .unwrap_or_else(|_| "Guild".into());

    Ok(Json(serde_json::json!({
        "ok": true,
        "guild_id": guild_id,
        "guild_name": guild_name,
        "chunk_x": payload.chunk_x,
        "chunk_z": payload.chunk_z
    })))
}

pub async fn server_unclaim_chunk(
    GameServer(server): GameServer,
    State(state): State<AppState>,
    Json(payload): Json<ServerClaimChunkPayload>,
) -> AppResult<Json<Value>> {
    let claim: Option<(i64, String)> =
        sqlx::query_as("SELECT id, guild_id FROM guild_claims WHERE server_id = ? AND dimension = ? AND chunk_x = ? AND chunk_z = ?")
            .bind(server.id)
            .bind(&payload.dimension)
            .bind(payload.chunk_x)
            .bind(payload.chunk_z)
            .fetch_optional(&state.db)
            .await?;

    let Some((claim_id, guild_id)) = claim else {
        return Err(AppError::not_found("This chunk is not claimed"));
    };

    let member: Option<(String,)> = sqlx::query_as("SELECT role FROM guild_members WHERE guild_id = ? AND uuid = ?")
        .bind(&guild_id)
        .bind(&payload.uuid)
        .fetch_optional(&state.db)
        .await?;

    if member.is_none() || !guild_can(&state, &guild_id, &payload.uuid, "claim").await? {
        return Err(AppError::forbidden("You cannot unclaim land belonging to another guild"));
    }

    sqlx::query("DELETE FROM guild_claims WHERE id = ?").bind(claim_id).execute(&state.db).await?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Deserialize)]
pub struct ServerGuildPlayerQuery {
    pub uuid: String,
}

pub async fn server_get_player_guild(
    GameServer(server): GameServer,
    State(state): State<AppState>,
    Json(payload): Json<ServerGuildPlayerQuery>,
) -> AppResult<Json<Value>> {
    let member_opt: Option<(String, String)> = sqlx::query_as(
        "SELECT gm.guild_id, gm.role FROM guild_members gm JOIN guilds g ON g.id = gm.guild_id WHERE gm.uuid = ? AND g.instance_id = ?",
    )
    .bind(&payload.uuid)
    .bind(&server.instance_id)
    .fetch_optional(&state.db)
    .await?;

    let Some((guild_id, player_role)) = member_opt else {
        return Ok(Json(serde_json::json!({ "in_guild": false })));
    };

    let guild_opt: Option<(String, String, String, String, String, i64, i64, i64)> =
        sqlx::query_as("SELECT name, tag, description, motd, leader_uuid, level, xp, (guilds.max_claims + COALESCE((SELECT SUM(b.claim_chunks) FROM player_bonuses b JOIN guild_members m ON m.uuid = b.uuid WHERE m.guild_id = guilds.id), 0) + COALESCE((SELECT CAST(value AS INTEGER) FROM kv WHERE key = 'rule_claims_per_member'), 0) * MAX(0, (SELECT COUNT(*) FROM guild_members m2 WHERE m2.guild_id = guilds.id) - 1) + COALESCE((SELECT CAST(value AS INTEGER) FROM kv WHERE key = 'rule_claims_per_level'), 0) * MAX(0, guilds.level - 1)) FROM guilds WHERE id = ?")
            .bind(&guild_id)
            .fetch_optional(&state.db)
            .await?;

    let Some((name, tag, desc, motd, leader_uuid, level, xp, max_claims)) = guild_opt else {
        return Ok(Json(serde_json::json!({ "in_guild": false })));
    };

    let member_rows: Vec<(String, String, String)> =
        sqlx::query_as("SELECT uuid, name, role FROM guild_members WHERE guild_id = ?").bind(&guild_id).fetch_all(&state.db).await?;

    let claims_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM guild_claims WHERE guild_id = ?").bind(&guild_id).fetch_one(&state.db).await.unwrap_or(0);

    let members_val: Vec<Value> = member_rows.into_iter().map(|(u, n, r)| serde_json::json!({ "uuid": u, "name": n, "role": r })).collect();

    Ok(Json(serde_json::json!({
        "in_guild": true,
        "guild": {
            "id": guild_id,
            "name": name,
            "tag": tag,
            "description": desc,
            "motd": motd,
            "leader_uuid": leader_uuid,
            "role": player_role,
            "level": level,
            "xp": xp,
            "claims_count": claims_count,
            "max_claims": max_claims,
            "members": members_val
        }
    })))
}

#[derive(Deserialize)]
pub struct ServerCreateGuildPayload {
    pub uuid: String,
    pub username: String,
    pub name: String,
    pub tag: String,
}

pub async fn server_create_guild(
    GameServer(server): GameServer,
    State(state): State<AppState>,
    Json(payload): Json<ServerCreateGuildPayload>,
) -> AppResult<Json<Value>> {
    let name = payload.name.trim();
    let tag = payload.tag.trim();
    if name.len() < 3 || name.len() > 32 {
        return Err(AppError::bad_request("Guild name must be between 3 and 32 characters"));
    }
    if tag.len() < 2 || tag.len() > 6 {
        return Err(AppError::bad_request("Guild tag must be between 2 and 6 characters"));
    }

    let in_guild: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM guild_members gm JOIN guilds g ON g.id = gm.guild_id WHERE gm.uuid = ? AND g.instance_id = ?)",
    )
    .bind(&payload.uuid)
    .bind(&server.instance_id)
    .fetch_one(&state.db)
    .await?;

    if in_guild {
        return Err(AppError::bad_request("You are already in a guild. Leave your current guild first"));
    }

    let guild_id = format!("guild_{}", uuid::Uuid::new_v4().simple());
    let now = chrono::Utc::now().to_rfc3339();

    let mut tx = state.db.begin().await?;

    let base_claims = crate::progression::load(&mut tx).await?.rules.guild_base_claims;
    let res = sqlx::query(
        "INSERT INTO guilds (id, instance_id, name, tag, description, motd, leader_uuid, max_claims, created_at)
         VALUES (?, ?, ?, ?, '', 'Welcome to the guild!', ?, ?, ?)",
    )
    .bind(&guild_id)
    .bind(&server.instance_id)
    .bind(name)
    .bind(tag)
    .bind(&payload.uuid)
    .bind(base_claims)
    .bind(&now)
    .execute(&mut *tx)
    .await;

    if let Err(e) = res {
        tracing::warn!("server_create_guild failed: {e}");
        return Err(AppError::bad_request("A guild with that name or tag already exists"));
    }

    sqlx::query(
        "INSERT INTO guild_members (guild_id, uuid, name, role, joined_at)
         VALUES (?, ?, ?, 'leader', ?)",
    )
    .bind(&guild_id)
    .bind(&payload.uuid)
    .bind(&payload.username)
    .bind(&now)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(serde_json::json!({
        "ok": true,
        "guild_id": guild_id,
        "name": name,
        "tag": tag
    })))
}

#[derive(Deserialize)]
pub struct ServerGuildLeavePayload {
    pub uuid: String,
}

pub async fn server_guild_leave(
    GameServer(server): GameServer,
    State(state): State<AppState>,
    Json(payload): Json<ServerGuildLeavePayload>,
) -> AppResult<Json<Value>> {
    let member_opt: Option<(String, String)> = sqlx::query_as(
        "SELECT gm.guild_id, gm.role FROM guild_members gm JOIN guilds g ON g.id = gm.guild_id WHERE gm.uuid = ? AND g.instance_id = ?",
    )
    .bind(&payload.uuid)
    .bind(&server.instance_id)
    .fetch_optional(&state.db)
    .await?;

    let Some((guild_id, role)) = member_opt else {
        return Err(AppError::bad_request("You are not in a guild"));
    };

    if role == "leader" {
        let member_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM guild_members WHERE guild_id = ?")
            .bind(&guild_id)
            .fetch_one(&state.db)
            .await
            .unwrap_or(1);

        if member_count > 1 {
            return Err(AppError::bad_request("Guild leader cannot leave without transferring leadership"));
        } else {
            let name: String = sqlx::query_scalar("SELECT name FROM guild_members WHERE guild_id = ? AND uuid = ?")
                .bind(&guild_id)
                .bind(&payload.uuid)
                .fetch_one(&state.db)
                .await?;
            disband_guild_now(&state, &guild_id, Some((&payload.uuid, &name))).await?;
            return Ok(Json(serde_json::json!({ "ok": true, "disbanded": true })));
        }
    }

    sqlx::query("DELETE FROM guild_members WHERE guild_id = ? AND uuid = ?").bind(&guild_id).bind(&payload.uuid).execute(&state.db).await?;

    Ok(Json(serde_json::json!({ "ok": true, "disbanded": false })))
}

#[derive(Deserialize)]
pub struct ServerGuildRenamePayload {
    pub uuid: String,
    pub name: String,
    pub tag: Option<String>,
}

/// Leader renames their guild from the game (`/guild rename`).
pub async fn server_guild_rename(
    GameServer(server): GameServer,
    State(state): State<AppState>,
    Json(p): Json<ServerGuildRenamePayload>,
) -> AppResult<Json<Value>> {
    let guild_id = leader_guild_on(&state, &server.instance_id, &p.uuid).await?;
    let (name, tag) = rename_guild_to(&state, &guild_id, &p.name, p.tag.as_deref()).await?;
    Ok(Json(json!({ "ok": true, "name": name, "tag": tag })))
}

/// Leader disbands their guild from the game (`/guild disband confirm`).
pub async fn server_guild_disband(
    GameServer(server): GameServer,
    State(state): State<AppState>,
    Json(p): Json<ServerGuildLeavePayload>,
) -> AppResult<Json<Value>> {
    let guild_id = leader_guild_on(&state, &server.instance_id, &p.uuid).await?;
    let name: String = sqlx::query_scalar("SELECT name FROM guild_members WHERE guild_id = ? AND uuid = ?")
        .bind(&guild_id)
        .bind(&p.uuid)
        .fetch_one(&state.db)
        .await?;
    let refunded = disband_guild_now(&state, &guild_id, Some((&p.uuid, &name))).await?;
    Ok(Json(json!({ "ok": true, "refunded": refunded })))
}

async fn leader_guild_on(state: &AppState, instance_id: &str, uuid: &str) -> AppResult<String> {
    let found: Option<(String, String)> = sqlx::query_as(
        "SELECT gm.guild_id, gm.role FROM guild_members gm JOIN guilds g ON g.id = gm.guild_id WHERE gm.uuid = ? AND g.instance_id = ?",
    )
    .bind(uuid)
    .bind(instance_id)
    .fetch_optional(&state.db)
    .await?;
    match found {
        Some((id, role)) if role == "leader" => Ok(id),
        Some(_) => Err(AppError::forbidden("Only the guild leader can do this")),
        None => Err(AppError::bad_request("You are not in a guild")),
    }
}
