//! Guild management from inside the game: the same rules, data and notifications the launcher uses, behind one endpoint so the
//! plugin and mod stay thin. Everything the player does here shows up in the launcher immediately, and the other way round.

use crate::state::RequestState as State;
use super::guilds::{create_join_request, decide_join_request, guild_can, kick_member, transfer_leadership};
use super::notifications;
use crate::error::{AppError, AppResult};
use crate::routes::servers::GameServer;
use crate::state::AppState;

use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
pub struct ManagePayload {
    pub uuid: String,
    #[serde(default)]
    pub name: String,
    pub action: String,
    /// A player or guild name, depending on the action.
    #[serde(default)]
    pub target: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub title: String,
}

struct Me {
    guild_id: String,
    guild_name: String,
    role: String,
}

async fn my_guild(state: &AppState, instance: &str, uuid: &str) -> AppResult<Option<Me>> {
    let row: Option<(String, String, String)> = sqlx::query_as(
        "SELECT g.id, g.name, gm.role FROM guild_members gm JOIN guilds g ON g.id = gm.guild_id WHERE gm.uuid = ? AND g.instance_id = ?",
    )
    .bind(uuid)
    .bind(instance)
    .fetch_optional(&state.db)
    .await?;
    Ok(row.map(|(guild_id, guild_name, role)| Me { guild_id, guild_name, role }))
}

async fn member_uuid(state: &AppState, guild_id: &str, name: &str) -> AppResult<String> {
    sqlx::query_scalar("SELECT uuid FROM guild_members WHERE guild_id = ? AND name = ? COLLATE NOCASE")
        .bind(guild_id)
        .bind(name.trim())
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::bad_request(format!("{} is not in your guild", name.trim())))
}

async fn find_guild(state: &AppState, instance: &str, text: &str) -> AppResult<Option<(String, String, String)>> {
    Ok(sqlx::query_as("SELECT id, name, tag FROM guilds WHERE instance_id = ?1 AND (name = ?2 COLLATE NOCASE OR tag = ?2 COLLATE NOCASE) LIMIT 1")
        .bind(instance)
        .bind(text.trim())
        .fetch_optional(&state.db)
        .await?)
}

fn clip(text: &str, max: usize, what: &str) -> AppResult<String> {
    let t = text.trim();
    if t.chars().count() > max {
        return Err(AppError::bad_request(format!("{what} can be up to {max} characters")));
    }
    if t.chars().any(|c| c.is_control() && c != '\n') {
        return Err(AppError::bad_request(format!("{what} can't contain control characters")));
    }
    Ok(t.to_string())
}

pub async fn server_guild_manage(GameServer(server): GameServer, State(state): State<AppState>, Json(p): Json<ManagePayload>) -> AppResult<Json<Value>> {
    let instance = server.instance_id.as_str();
    let me = my_guild(&state, instance, &p.uuid).await?;
    let need = |m: &Option<Me>| -> AppResult<()> { m.as_ref().map(|_| ()).ok_or_else(|| AppError::bad_request("You are not in a guild")) };

    match p.action.as_str() {
        "list" => {
            let rows: Vec<(String, String, String, i64, i64, i64)> = sqlx::query_as(
                "SELECT g.name, g.tag, g.description, (SELECT COUNT(*) FROM guild_members m WHERE m.guild_id = g.id),
                        (SELECT COUNT(*) FROM guild_claims c WHERE c.guild_id = g.id), g.level
                 FROM guilds g WHERE g.instance_id = ? ORDER BY 4 DESC, g.name LIMIT 20",
            )
            .bind(instance)
            .fetch_all(&state.db)
            .await?;
            Ok(Json(json!({ "ok": true, "guilds": rows.into_iter().map(|(name, tag, desc, members, claims, level)| json!({"name":name,"tag":tag,"description":desc,"members":members,"claims":claims,"level":level})).collect::<Vec<_>>() })))
        }
        "info" => {
            let (id, member_role) = if p.target.trim().is_empty() {
                need(&me)?;
                let m = me.as_ref().unwrap();
                (m.guild_id.clone(), Some(m.role.clone()))
            } else {
                let (id, _, _) = find_guild(&state, instance, &p.target).await?.ok_or_else(|| AppError::bad_request(format!("No guild called \"{}\"", p.target.trim())))?;
                let role = me.as_ref().filter(|m| m.guild_id == id).map(|m| m.role.clone());
                (id, role)
            };
            let g: (String, String, String, String, String, i64, String) =
                sqlx::query_as("SELECT name, tag, description, motd, leader_uuid, level, created_at FROM guilds WHERE id = ?").bind(&id).fetch_one(&state.db).await?;
            let leader: Option<String> = sqlx::query_scalar("SELECT name FROM guild_members WHERE guild_id = ? AND uuid = ?").bind(&id).bind(&g.4).fetch_optional(&state.db).await?;
            let members: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM guild_members WHERE guild_id = ?").bind(&id).fetch_one(&state.db).await?;
            let claims: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM guild_claims WHERE guild_id = ?").bind(&id).fetch_one(&state.db).await?;
            Ok(Json(json!({ "ok": true, "name": g.0, "tag": g.1, "description": g.2, "motd": g.3, "leader": leader, "level": g.5,
                "members": members, "claims": claims, "created_at": g.6, "your_role": member_role })))
        }
        "flags" => {
            need(&me)?;
            let m = me.as_ref().unwrap();
            let can_edit = guild_can(&state, &m.guild_id, &p.uuid, "manage").await?;
            Ok(Json(super::guild_flags::for_game(&state, &m.guild_id, can_edit).await?))
        }
        "flag" => {
            need(&me)?;
            let m = me.as_ref().unwrap();
            if !guild_can(&state, &m.guild_id, &p.uuid, "manage").await? {
                return Err(AppError::forbidden("Only the leader, officers and roles that can manage the guild may change land rules"));
            }
            let value = match p.text.trim().to_lowercase().as_str() {
                "on" | "true" | "allow" | "yes" => true,
                "off" | "false" | "deny" | "no" => false,
                _ => return Err(AppError::bad_request("Say on or off")),
            };
            let rule = p.target.trim().to_lowercase();
            let mut one = serde_json::Map::new();
            one.insert(rule.clone(), json!(value));
            super::guild_flags::change(&state, &m.guild_id, &one).await?;
            Ok(Json(json!({ "ok": true, "message": format!("{rule} is now {} for {}.", if value { "on" } else { "off" }, m.guild_name) })))
        }
        "join" => {
            if me.is_some() {
                return Err(AppError::bad_request("Leave your current guild before joining another"));
            }
            let (id, name, _) = find_guild(&state, instance, &p.target).await?.ok_or_else(|| AppError::bad_request(format!("No guild called \"{}\"", p.target.trim())))?;
            create_join_request(&state, &id, &p.uuid, &p.name, &p.text).await?;
            Ok(Json(json!({ "ok": true, "message": format!("Request sent to {name}. Its leaders have been told.") })))
        }
        "requests" => {
            need(&me)?;
            let m = me.as_ref().unwrap();
            if !guild_can(&state, &m.guild_id, &p.uuid, "invite").await? {
                return Err(AppError::forbidden("Your guild role cannot review requests"));
            }
            let rows: Vec<(String, String)> = sqlx::query_as("SELECT name, message FROM guild_join_requests WHERE guild_id = ? ORDER BY created_at").bind(&m.guild_id).fetch_all(&state.db).await?;
            Ok(Json(json!({ "ok": true, "requests": rows.into_iter().map(|(n, msg)| json!({"name": n, "message": msg})).collect::<Vec<_>>() })))
        }
        "accept" | "deny" => {
            need(&me)?;
            let m = me.as_ref().unwrap();
            if !guild_can(&state, &m.guild_id, &p.uuid, "invite").await? {
                return Err(AppError::forbidden("Your guild role cannot review requests"));
            }
            let uuid: Option<String> = sqlx::query_scalar("SELECT uuid FROM guild_join_requests WHERE guild_id = ? AND name = ? COLLATE NOCASE")
                .bind(&m.guild_id).bind(p.target.trim()).fetch_optional(&state.db).await?;
            let uuid = uuid.ok_or_else(|| AppError::bad_request(format!("{} has no pending request", p.target.trim())))?;
            let name = decide_join_request(&state, &m.guild_id, &uuid, p.action == "accept").await?;
            Ok(Json(json!({ "ok": true, "message": if p.action == "accept" { format!("{name} joined {}.", m.guild_name) } else { format!("Declined {name}'s request.") } })))
        }
        "kick" => {
            need(&me)?;
            let m = me.as_ref().unwrap();
            let target = member_uuid(&state, &m.guild_id, &p.target).await?;
            if target == p.uuid {
                return Err(AppError::bad_request("Use /guild leave to leave your own guild"));
            }
            let name = kick_member(&state, &m.guild_id, &p.uuid, &target).await?;
            Ok(Json(json!({ "ok": true, "message": format!("{name} was removed from {}.", m.guild_name) })))
        }
        "promote" | "demote" | "role" => {
            need(&me)?;
            let m = me.as_ref().unwrap();
            if m.role != "leader" {
                return Err(AppError::forbidden("Only the guild leader can change roles"));
            }
            let target = member_uuid(&state, &m.guild_id, &p.target).await?;
            let current: String = sqlx::query_scalar("SELECT role FROM guild_members WHERE guild_id = ? AND uuid = ?").bind(&m.guild_id).bind(&target).fetch_one(&state.db).await?;
            if current == "leader" {
                return Err(AppError::bad_request("That is the guild leader"));
            }
            let role = match p.action.as_str() {
                "promote" => "officer".to_string(),
                "demote" => "member".to_string(),
                _ => {
                    let wanted = p.text.trim().to_string();
                    let known: Option<String> = if ["member", "officer"].contains(&wanted.to_lowercase().as_str()) {
                        Some(wanted.to_lowercase())
                    } else {
                        sqlx::query_scalar("SELECT name FROM guild_roles WHERE guild_id = ? AND name = ? COLLATE NOCASE").bind(&m.guild_id).bind(&wanted).fetch_optional(&state.db).await?
                    };
                    known.ok_or_else(|| AppError::bad_request(format!("Your guild has no role called \"{wanted}\". See /guild roles")))?
                }
            };
            sqlx::query("UPDATE guild_members SET role = ? WHERE guild_id = ? AND uuid = ?").bind(&role).bind(&m.guild_id).bind(&target).execute(&state.db).await?;
            notifications::push(&state.db, &target, "guild_role", "Your guild role changed", &format!("You are now {role} in {}.", m.guild_name), Some("/guild")).await;
            Ok(Json(json!({ "ok": true, "message": format!("{} is now {role}.", p.target.trim()) })))
        }
        "roles" => {
            need(&me)?;
            let m = me.as_ref().unwrap();
            let rows: Vec<(String, i64, i64, i64, i64, i64)> = sqlx::query_as("SELECT name, can_invite, can_kick, can_claim, can_post, can_manage FROM guild_roles WHERE guild_id = ? ORDER BY priority DESC, name").bind(&m.guild_id).fetch_all(&state.db).await?;
            Ok(Json(json!({ "ok": true, "roles": rows.into_iter().map(|(n, i, k, c, po, ma)| json!({"name":n,"can_invite":i!=0,"can_kick":k!=0,"can_claim":c!=0,"can_post":po!=0,"can_manage":ma!=0})).collect::<Vec<_>>() })))
        }
        "transfer" => {
            need(&me)?;
            let m = me.as_ref().unwrap();
            let target = member_uuid(&state, &m.guild_id, &p.target).await?;
            let name = transfer_leadership(&state, &m.guild_id, &p.uuid, &target).await?;
            Ok(Json(json!({ "ok": true, "message": format!("{name} now leads {}. You are an officer.", m.guild_name) })))
        }
        "motd" | "desc" => {
            need(&me)?;
            let m = me.as_ref().unwrap();
            if !guild_can(&state, &m.guild_id, &p.uuid, "manage").await? {
                return Err(AppError::forbidden("Only guild officers or leaders can update guild details"));
            }
            if p.action == "motd" {
                let text = clip(&p.text, 200, "The message of the day")?;
                sqlx::query("UPDATE guilds SET motd = ? WHERE id = ?").bind(&text).bind(&m.guild_id).execute(&state.db).await?;
                Ok(Json(json!({ "ok": true, "message": if text.is_empty() { "Message of the day cleared.".to_string() } else { "Message of the day updated.".to_string() } })))
            } else {
                let text = clip(&p.text, 500, "The description")?;
                sqlx::query("UPDATE guilds SET description = ? WHERE id = ?").bind(&text).bind(&m.guild_id).execute(&state.db).await?;
                Ok(Json(json!({ "ok": true, "message": "Guild description updated." })))
            }
        }
        "post" => {
            need(&me)?;
            let m = me.as_ref().unwrap();
            if !guild_can(&state, &m.guild_id, &p.uuid, "post").await? {
                return Err(AppError::forbidden("Your guild role cannot post"));
            }
            let title = clip(&p.title, 80, "The title")?;
            let content = clip(&p.text, 2000, "The post")?;
            if title.is_empty() || content.is_empty() {
                return Err(AppError::bad_request("A post needs a title and some text"));
            }
            sqlx::query("INSERT INTO guild_posts (guild_id, author_uuid, author_name, title, content, created_at) VALUES (?, ?, ?, ?, ?, ?)")
                .bind(&m.guild_id).bind(&p.uuid).bind(&p.name).bind(&title).bind(&content).bind(chrono::Utc::now().to_rfc3339()).execute(&state.db).await?;
            let members: Vec<String> = sqlx::query_scalar("SELECT uuid FROM guild_members WHERE guild_id = ? AND uuid <> ?").bind(&m.guild_id).bind(&p.uuid).fetch_all(&state.db).await?;
            notifications::push_many(&state.db, &members, "guild_post", &format!("{}: {title}", m.guild_name), &format!("{} posted an announcement.", p.name), Some("/guild")).await;
            Ok(Json(json!({ "ok": true, "message": "Posted to your guild's board." })))
        }
        "posts" => {
            need(&me)?;
            let m = me.as_ref().unwrap();
            let rows: Vec<(String, String, String, String)> = sqlx::query_as("SELECT title, content, author_name, created_at FROM guild_posts WHERE guild_id = ? ORDER BY id DESC LIMIT 5").bind(&m.guild_id).fetch_all(&state.db).await?;
            Ok(Json(json!({ "ok": true, "posts": rows.into_iter().map(|(t, c, a, at)| json!({"title":t,"content":c,"author":a,"created_at":at})).collect::<Vec<_>>() })))
        }
        _ => Err(AppError::bad_request("Unknown guild action")),
    }
}
