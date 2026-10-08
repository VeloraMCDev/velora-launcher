//! The guild bank as seen from the game server: check the balance, move
//! money between a player and the guild, credit shop sales to the guild, and
//! pay other guilds. Every mutating call is idempotent on `operation_id`.

use crate::state::RequestState as State;
use crate::error::{AppError, AppResult};
use crate::routes::economy::{begin_operation, ensure_balance, finish_operation};
use crate::routes::servers::{GameServer, ServerRow};
use crate::state::AppState;

use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::SqliteConnection;

/// A player's place in the guild that plays on this server's instance.
pub struct Membership {
    pub guild_id: String,
    pub name: String,
    pub tag: String,
    pub role: String,
}

impl Membership {
    pub fn can_spend(&self) -> bool {
        matches!(self.role.as_str(), "leader" | "officer")
    }
}

pub async fn membership(conn: &mut SqliteConnection, server: &ServerRow, uuid: &str) -> AppResult<Option<Membership>> {
    let row: Option<(String, String, String, String)> = sqlx::query_as(
        "SELECT g.id, g.name, g.tag, gm.role FROM guild_members gm JOIN guilds g ON g.id = gm.guild_id
         WHERE gm.uuid = ? AND g.instance_id = ?",
    )
    .bind(uuid)
    .bind(&server.instance_id)
    .fetch_optional(&mut *conn)
    .await?;
    Ok(row.map(|(guild_id, name, tag, role)| Membership { guild_id, name, tag, role }))
}

pub fn valid_amount(amount: f64) -> AppResult<()> {
    if !amount.is_finite() || !(0.01..=1e9).contains(&amount) || (amount * 100.0 - (amount * 100.0).round()).abs() > 1e-6 {
        return Err(AppError::bad_request("Amount must be between 0.01 and 1,000,000,000 with at most two decimals"));
    }
    Ok(())
}

pub async fn ensure_wallet(conn: &mut SqliteConnection, server_id: i64, guild_id: &str) -> AppResult<()> {
    sqlx::query("INSERT INTO guild_wallets(server_id, guild_id, balance, updated_at) VALUES (?, ?, 0, ?) ON CONFLICT(server_id, guild_id) DO NOTHING")
        .bind(server_id)
        .bind(guild_id)
        .bind(chrono::Utc::now().to_rfc3339())
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// Move `delta` into (positive) or out of (negative) a guild wallet. Fails if it would go below zero.
pub async fn adjust_wallet(conn: &mut SqliteConnection, server_id: i64, guild_id: &str, delta: f64) -> AppResult<f64> {
    ensure_wallet(conn, server_id, guild_id).await?;
    let balance: Option<f64> = sqlx::query_scalar(
        "UPDATE guild_wallets SET balance = balance + ?, updated_at = ? WHERE server_id = ? AND guild_id = ? AND balance + ? >= 0 RETURNING balance",
    )
    .bind(delta)
    .bind(chrono::Utc::now().to_rfc3339())
    .bind(server_id)
    .bind(guild_id)
    .bind(delta)
    .fetch_optional(&mut *conn)
    .await?;
    balance.ok_or_else(|| AppError::bad_request("The guild bank doesn't have enough funds"))
}

pub async fn log(
    conn: &mut SqliteConnection,
    server_id: i64,
    guild_id: &str,
    actor: &str,
    kind: &str,
    amount: f64,
    note: &str,
) -> AppResult<()> {
    sqlx::query("INSERT INTO guild_wallet_transactions(server_id, guild_id, actor_uuid, kind, amount, note, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)")
        .bind(server_id)
        .bind(guild_id)
        .bind(actor)
        .bind(kind)
        .bind(amount)
        .bind(note.chars().filter(|c| !c.is_control()).take(120).collect::<String>())
        .bind(chrono::Utc::now().to_rfc3339())
        .execute(&mut *conn)
        .await?;
    Ok(())
}

async fn player_balance(conn: &mut SqliteConnection, server_id: i64, uuid: &str) -> AppResult<f64> {
    Ok(sqlx::query_scalar("SELECT balance FROM server_economy WHERE server_id = ? AND uuid = ?")
        .bind(server_id)
        .bind(uuid)
        .fetch_optional(&mut *conn)
        .await?
        .unwrap_or(0.0))
}

#[derive(Deserialize)]
pub struct Who {
    pub uuid: String,
}

/// Bank summary for `/guild bank`: who the player's guild is, their role, the balance and recent activity.
pub async fn server_bank_info(GameServer(server): GameServer, State(state): State<AppState>, Json(p): Json<Who>) -> AppResult<Json<Value>> {
    let mut conn = state.db.acquire().await?;
    let Some(m) = membership(&mut conn, &server, &p.uuid).await? else {
        return Ok(Json(json!({ "guild": null })));
    };
    let balance: f64 = sqlx::query_scalar("SELECT balance FROM guild_wallets WHERE server_id = ? AND guild_id = ?")
        .bind(server.economy_id)
        .bind(&m.guild_id)
        .fetch_optional(&mut *conn)
        .await?
        .unwrap_or(0.0);
    let recent: Vec<(String, String, f64, String, String)> = sqlx::query_as(
        "SELECT t.kind, COALESCE((SELECT gm.name FROM guild_members gm WHERE gm.guild_id = t.guild_id AND gm.uuid = t.actor_uuid), 'Former member'), t.amount, t.note, t.created_at
         FROM guild_wallet_transactions t WHERE t.server_id = ? AND t.guild_id = ? ORDER BY t.id DESC LIMIT 8",
    )
    .bind(server.economy_id)
    .bind(&m.guild_id)
    .fetch_all(&mut *conn)
    .await?;
    Ok(Json(json!({
        "guild": { "id": m.guild_id, "name": m.name, "tag": m.tag },
        "role": m.role,
        "can_spend": m.can_spend(),
        "balance": balance,
        "my_balance": player_balance(&mut conn, server.economy_id, &p.uuid).await?,
        "recent": recent.into_iter().map(|(kind, who, amount, note, at)| json!({ "kind": kind, "who": who, "amount": amount, "note": note, "at": at })).collect::<Vec<_>>(),
    })))
}

#[derive(Deserialize)]
pub struct Transfer {
    pub operation_id: String,
    pub uuid: String,
    pub username: String,
    pub amount: f64,
    /// `deposit` (player → guild) or `withdraw` (guild → player, officers and leaders).
    pub action: String,
}

pub async fn server_bank_transfer(
    GameServer(server): GameServer,
    State(state): State<AppState>,
    Json(p): Json<Transfer>,
) -> AppResult<Json<Value>> {
    valid_amount(p.amount)?;
    let withdraw = match p.action.as_str() {
        "deposit" => false,
        "withdraw" => true,
        _ => return Err(AppError::bad_request("action must be deposit or withdraw")),
    };
    let (mut tx, previous) = begin_operation(&state, server.id, &p.operation_id).await?;
    if let Some(previous) = previous {
        return Ok(Json(previous));
    }
    let m = membership(&mut tx, &server, &p.uuid).await?.ok_or_else(|| AppError::forbidden("You are not in a guild"))?;
    if withdraw && !m.can_spend() {
        return Err(AppError::forbidden("Only guild leaders and officers can withdraw"));
    }
    ensure_balance(&mut tx, server.economy_id, &p.uuid, &p.username).await?;
    let (guild_delta, player_delta) = if withdraw { (-p.amount, p.amount) } else { (p.amount, -p.amount) };
    let moved: Option<f64> = sqlx::query_scalar(
        "UPDATE server_economy SET balance = balance + ?, updated_at = ? WHERE server_id = ? AND uuid = ? AND balance + ? >= 0 RETURNING balance",
    )
    .bind(player_delta)
    .bind(chrono::Utc::now().to_rfc3339())
    .bind(server.economy_id)
    .bind(&p.uuid)
    .bind(player_delta)
    .fetch_optional(&mut *tx)
    .await?;
    let my_balance = moved.ok_or_else(|| AppError::bad_request("Insufficient funds"))?;
    let balance = adjust_wallet(&mut tx, server.economy_id, &m.guild_id, guild_delta).await?;
    log(&mut tx, server.economy_id, &m.guild_id, &p.uuid, &p.action, p.amount, "").await?;
    let message = if withdraw {
        format!("Withdrew ${:.2} from [{}] bank.", p.amount, m.tag)
    } else {
        format!("Deposited ${:.2} into [{}] bank.", p.amount, m.tag)
    };
    finish_operation(
        tx,
        server.id,
        &p.operation_id,
        json!({ "ok": true, "balance": balance, "my_balance": my_balance, "message": message }),
    )
    .await
}

#[derive(Deserialize)]
pub struct Credit {
    pub operation_id: String,
    pub uuid: String,
    pub username: String,
    pub amount: f64,
    #[serde(default)]
    pub description: String,
}

/// Shop sales made with `/guild sell`: the proceeds go to the guild bank instead of the seller.
pub async fn server_bank_credit(
    GameServer(server): GameServer,
    State(state): State<AppState>,
    Json(p): Json<Credit>,
) -> AppResult<Json<Value>> {
    valid_amount(p.amount)?;
    let (mut tx, previous) = begin_operation(&state, server.id, &p.operation_id).await?;
    if let Some(previous) = previous {
        return Ok(Json(previous));
    }
    let m = membership(&mut tx, &server, &p.uuid).await?.ok_or_else(|| AppError::forbidden("You are not in a guild"))?;
    let balance = adjust_wallet(&mut tx, server.economy_id, &m.guild_id, p.amount).await?;
    let note = if p.description.is_empty() { "Items sold to the server shop" } else { &p.description };
    log(&mut tx, server.economy_id, &m.guild_id, &p.uuid, "sale", p.amount, note).await?;
    let message = format!("Sold for ${:.2}, paid into the [{}] bank.", p.amount, m.tag);
    finish_operation(tx, server.id, &p.operation_id, json!({ "ok": true, "balance": balance, "message": message })).await
}

#[derive(Deserialize)]
pub struct Pay {
    pub operation_id: String,
    pub uuid: String,
    /// Tag of the receiving guild.
    pub to_tag: String,
    pub amount: f64,
    #[serde(default)]
    pub note: String,
}

/// Guild-to-guild payment (leaders and officers).
pub async fn server_bank_pay(GameServer(server): GameServer, State(state): State<AppState>, Json(p): Json<Pay>) -> AppResult<Json<Value>> {
    valid_amount(p.amount)?;
    let (mut tx, previous) = begin_operation(&state, server.id, &p.operation_id).await?;
    if let Some(previous) = previous {
        return Ok(Json(previous));
    }
    let m = membership(&mut tx, &server, &p.uuid).await?.ok_or_else(|| AppError::forbidden("You are not in a guild"))?;
    if !m.can_spend() {
        return Err(AppError::forbidden("Only guild leaders and officers can spend guild money"));
    }
    let target: Option<(String, String, String)> =
        sqlx::query_as("SELECT id, name, tag FROM guilds WHERE instance_id = ? AND tag = ? COLLATE NOCASE AND id <> ?")
            .bind(&server.instance_id)
            .bind(p.to_tag.trim())
            .bind(&m.guild_id)
            .fetch_optional(&mut *tx)
            .await?;
    let (target_id, target_name, target_tag) = target.ok_or_else(|| AppError::not_found("No other guild with that tag"))?;
    let balance = adjust_wallet(&mut tx, server.economy_id, &m.guild_id, -p.amount).await?;
    adjust_wallet(&mut tx, server.economy_id, &target_id, p.amount).await?;
    let note = p.note.chars().take(60).collect::<String>();
    log(&mut tx, server.economy_id, &m.guild_id, &p.uuid, "transfer_out", p.amount, &format!("to [{target_tag}] {target_name} {note}"))
        .await?;
    log(&mut tx, server.economy_id, &target_id, &p.uuid, "transfer_in", p.amount, &format!("from [{}] {} {note}", m.tag, m.name)).await?;
    let message = format!("Paid ${:.2} from [{}] to [{}] {}.", p.amount, m.tag, target_tag, target_name);
    finish_operation(tx, server.id, &p.operation_id, json!({ "ok": true, "balance": balance, "message": message })).await
}
