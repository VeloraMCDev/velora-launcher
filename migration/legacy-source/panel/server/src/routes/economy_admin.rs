//! The admin side of the economy: stats, player and guild balances, and market / auction listings.
//!
//! Balances and guild wallets live on the server's economy scope (servers in one economy group share them); listings and the
//! ledger are keyed by the server itself, exactly like the launcher market does.

use crate::state::RequestState as State;
use super::economy::ensure_balance;
use super::guild_bank;
use super::notifications;
use crate::auth::AdminUser;
use crate::error::{AppError, AppResult};
use crate::routes::servers::{economy_scope, get_server, ServerRow};
use crate::state::AppState;
use axum::extract::{Path, Query};
use axum::Json;
use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::SqliteConnection;
use std::collections::BTreeMap;

const LIMIT: f64 = 1e12;
/// Ledger parties that are not players.
const PSEUDO: &str = "('server','casino','auction','admin')";
/// Every server that shares the economy of server `?1`.
const GROUP: &str = "(SELECT id FROM game_servers WHERE id = ?1 OR (economy_group <> '' AND economy_group = (SELECT economy_group FROM game_servers WHERE id = ?1) COLLATE NOCASE))";

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

async fn scoped(state: &AppState, id: i64) -> AppResult<ServerRow> {
    let mut server = get_server(state, id).await?;
    server.economy_id = economy_scope(&state.db, server.id).await?;
    Ok(server)
}

fn like(q: &str) -> String {
    let mut out = String::from("%");
    for c in q.trim().chars() {
        if matches!(c, '%' | '_' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('%');
    out
}

fn clean(s: Option<&str>, fallback: &str) -> String {
    let s: String = s.unwrap_or("").chars().filter(|c| !c.is_control()).take(100).collect();
    let s = s.trim();
    if s.is_empty() {
        fallback.to_string()
    } else {
        s.to_string()
    }
}

fn check_amount(amount: f64, allow_zero: bool) -> AppResult<f64> {
    if !amount.is_finite() || amount.abs() > LIMIT || amount < 0.0 || (!allow_zero && round2(amount) <= 0.0) {
        return Err(AppError::bad_request("Amount must be a positive number (at most 1,000,000,000,000)"));
    }
    Ok(round2(amount))
}

/// The change an adjustment makes, given the current balance.
fn delta_for(mode: &str, amount: f64, current: f64) -> AppResult<f64> {
    match mode {
        "add" => Ok(check_amount(amount, false)?),
        "remove" => Ok(-check_amount(amount, false)?),
        "set" => Ok(round2(check_amount(amount, true)? - current)),
        _ => Err(AppError::bad_request("mode must be add, remove or set")),
    }
}

#[derive(Deserialize)]
pub struct ServerQuery {
    pub server_id: i64,
}

// ---------------------------------------------------------------------------
// Overview
// ---------------------------------------------------------------------------

pub async fn overview(_admin: AdminUser, State(state): State<AppState>, Query(q): Query<ServerQuery>) -> AppResult<Json<Value>> {
    let server = scoped(&state, q.server_id).await?;
    let (eco, sid) = (server.economy_id, server.id);
    let db = &state.db;
    let now = Utc::now();

    let mut balances: Vec<(String, String, f64)> =
        sqlx::query_as("SELECT username, uuid, balance FROM server_economy WHERE server_id = ? ORDER BY balance")
            .bind(eco)
            .fetch_all(db)
            .await?;
    let accounts = balances.len();
    let circulation = round2(balances.iter().map(|b| b.2).sum());
    let median = match accounts {
        0 => 0.0,
        n if n % 2 == 1 => balances[n / 2].2,
        n => (balances[n / 2 - 1].2 + balances[n / 2].2) / 2.0,
    };
    let richest = balances.pop().map(|(name, uuid, balance)| json!({ "name": name, "uuid": uuid, "balance": balance }));
    let (wallets_total, wallets_count): (f64, i64) =
        sqlx::query_as("SELECT COALESCE(SUM(balance),0.0), COUNT(*) FROM guild_wallets WHERE server_id = ?")
            .bind(eco)
            .fetch_one(db)
            .await?;

    let today = now.format("%Y-%m-%d").to_string();
    let d7 = (now - Duration::days(7)).format("%Y-%m-%dT%H:%M:%S").to_string();
    let d30 = (now - Duration::days(30)).format("%Y-%m-%dT%H:%M:%S").to_string();
    let vol_sql =
        format!("SELECT COALESCE(SUM(amount),0.0), COUNT(*) FROM economy_transactions WHERE server_id IN {GROUP} AND created_at >= ?2");
    let (v_today, tx_today): (f64, i64) = sqlx::query_as(&vol_sql).bind(sid).bind(&today).fetch_one(db).await?;
    let (v7, tx7): (f64, i64) = sqlx::query_as(&vol_sql).bind(sid).bind(&d7).fetch_one(db).await?;
    let (v30, _): (f64, i64) = sqlx::query_as(&vol_sql).bind(sid).bind(&d30).fetch_one(db).await?;

    let since14 = (now - Duration::days(13)).format("%Y-%m-%d").to_string();
    let days: Vec<(String, f64, i64)> = sqlx::query_as(&format!(
        "SELECT substr(created_at,1,10), COALESCE(SUM(amount),0.0), COUNT(*) FROM economy_transactions WHERE server_id IN {GROUP} AND created_at >= ?2 GROUP BY 1"
    )).bind(sid).bind(&since14).fetch_all(db).await?;
    let by_day: BTreeMap<String, (f64, i64)> = days.into_iter().map(|(d, v, c)| (d, (v, c))).collect();
    let daily: Vec<Value> = (0..14)
        .map(|i| {
            let day = (now - Duration::days(13 - i)).format("%Y-%m-%d").to_string();
            let (v, c) = by_day.get(&day).copied().unwrap_or((0.0, 0));
            json!({ "day": day, "volume": round2(v), "count": c })
        })
        .collect();

    let earners: Vec<(String, String, f64)> = sqlx::query_as(&format!(
        "SELECT MAX(to_name), to_uuid, SUM(amount) FROM economy_transactions WHERE server_id IN {GROUP} AND created_at >= ?2 AND to_uuid NOT IN {PSEUDO} AND to_uuid NOT LIKE 'guild:%' AND description NOT LIKE 'Admin:%'
         GROUP BY to_uuid ORDER BY SUM(amount) DESC LIMIT 5"
    )).bind(sid).bind(&d7).fetch_all(db).await?;
    let spenders: Vec<(String, String, f64)> = sqlx::query_as(&format!(
        "SELECT MAX(from_name), from_uuid, SUM(amount) FROM economy_transactions WHERE server_id IN {GROUP} AND created_at >= ?2 AND from_uuid NOT IN {PSEUDO} AND from_uuid NOT LIKE 'guild:%' AND description NOT LIKE 'Admin:%'
         GROUP BY from_uuid ORDER BY SUM(amount) DESC LIMIT 5"
    )).bind(sid).bind(&d7).fetch_all(db).await?;
    let top = |rows: Vec<(String, String, f64)>| -> Vec<Value> {
        rows.into_iter().map(|(name, uuid, amount)| json!({ "name": name, "uuid": uuid, "amount": round2(amount) })).collect()
    };

    let (fixed, auctions, value): (i64, i64, f64) = sqlx::query_as(
        "SELECT COALESCE(SUM(kind <> 'auction'),0), COALESCE(SUM(kind = 'auction'),0), COALESCE(SUM(CASE WHEN kind = 'auction' THEN COALESCE(current_bid, price) ELSE price END),0.0) FROM server_market WHERE server_id = ?",
    ).bind(sid).fetch_one(db).await?;
    let ending_soon: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM server_market WHERE server_id = ? AND kind = 'auction' AND ends_at > ? AND ends_at <= ?")
            .bind(sid)
            .bind(now.to_rfc3339())
            .bind((now + Duration::hours(24)).to_rfc3339())
            .fetch_one(db)
            .await?;
    let sales7: f64 = sqlx::query_scalar(&format!(
        "SELECT COALESCE(SUM(amount),0.0) FROM economy_transactions WHERE server_id IN {GROUP} AND created_at >= ?2 AND (description LIKE 'Market%' OR description LIKE 'Auction sale%')"
    )).bind(sid).bind(&d7).fetch_one(db).await?;

    let mut out = json!({
        "server": { "id": server.id, "name": server.name },
        "totals": { "circulation": circulation, "accounts": accounts, "average": if accounts > 0 { round2(circulation / accounts as f64) } else { 0.0 }, "median": round2(median),
                    "richest": richest, "guild_wallets_total": round2(wallets_total), "guild_wallets_count": wallets_count },
        "volume": { "today": round2(v_today), "last_7d": round2(v7), "last_30d": round2(v30), "tx_today": tx_today, "tx_7d": tx7 },
        "daily": daily,
        "top_earners": top(earners),
        "top_spenders": top(spenders),
        "market": { "fixed_listings": fixed, "auctions": auctions, "total_value": round2(value), "ending_soon": ending_soon, "sales_7d_volume": round2(sales7) },
    });
    let casino: Option<(f64, f64)> = sqlx::query_as(&format!(
        "SELECT COALESCE(SUM(bet),0.0), COALESCE(SUM(payout),0.0) FROM casino_rounds WHERE server_id IN {GROUP} AND created_at >= ?2"
    ))
    .bind(sid)
    .bind(&d7)
    .fetch_optional(db)
    .await
    .ok()
    .flatten();
    if let Some((wagered, paid)) = casino {
        out["casino"] = json!({ "wagered_7d": round2(wagered), "paid_7d": round2(paid), "house_net_7d": round2(wagered - paid) });
    }
    Ok(Json(out))
}

// ---------------------------------------------------------------------------
// Players
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct PlayersQuery {
    pub server_id: i64,
    #[serde(default)]
    pub q: String,
    pub sort: Option<String>,
    pub dir: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn players(_admin: AdminUser, State(state): State<AppState>, Query(p): Query<PlayersQuery>) -> AppResult<Json<Value>> {
    let server = scoped(&state, p.server_id).await?;
    let col = match p.sort.as_deref() {
        Some("name") => "username COLLATE NOCASE",
        Some("updated") => "updated_at",
        _ => "balance",
    };
    let dir = if p.dir.as_deref() == Some("asc") { "ASC" } else { "DESC" };
    let pattern = like(&p.q);
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM server_economy WHERE server_id = ? AND username LIKE ? ESCAPE '\\'")
        .bind(server.economy_id)
        .bind(&pattern)
        .fetch_one(&state.db)
        .await?;
    let rows: Vec<(String, String, f64, String)> = sqlx::query_as(&format!(
        "SELECT uuid, username, balance, updated_at FROM server_economy WHERE server_id = ? AND username LIKE ? ESCAPE '\\' ORDER BY {col} {dir}, uuid LIMIT ? OFFSET ?"
    ))
    .bind(server.economy_id).bind(&pattern).bind(p.limit.unwrap_or(50).clamp(1, 200)).bind(p.offset.unwrap_or(0).max(0))
    .fetch_all(&state.db).await?;
    Ok(Json(json!({
        "total": total,
        "rows": rows.into_iter().map(|(uuid, username, balance, updated_at)| json!({ "uuid": uuid, "username": username, "balance": balance, "updated_at": updated_at })).collect::<Vec<_>>(),
    })))
}

#[derive(Deserialize)]
pub struct AdjustPayload {
    pub server_id: i64,
    pub uuid: Option<String>,
    pub guild_id: Option<String>,
    pub username: Option<String>,
    pub mode: String,
    pub amount: f64,
    pub reason: Option<String>,
}

pub async fn adjust_player(_admin: AdminUser, State(state): State<AppState>, Json(p): Json<AdjustPayload>) -> AppResult<Json<Value>> {
    let server = scoped(&state, p.server_id).await?;
    let uuid = p
        .uuid
        .as_deref()
        .map(str::trim)
        .filter(|u| !u.is_empty() && u.len() <= 64)
        .ok_or_else(|| AppError::bad_request("A player is required"))?;
    let reason = clean(p.reason.as_deref(), "adjustment");
    let known: Option<String> = sqlx::query_scalar("SELECT username FROM server_economy WHERE server_id = ? AND uuid = ?")
        .bind(server.economy_id)
        .bind(uuid)
        .fetch_optional(&state.db)
        .await?;
    let name = match known {
        Some(n) => n,
        None => {
            let from_user: Option<String> =
                sqlx::query_scalar("SELECT username FROM users WHERE uuid = ?").bind(uuid).fetch_optional(&state.db).await?;
            clean(p.username.as_deref().or(from_user.as_deref()), "Player")
        }
    };
    let mut tx = state.db.begin().await?;
    ensure_balance(&mut tx, server.economy_id, uuid, &name).await?;
    let current: f64 = sqlx::query_scalar("SELECT balance FROM server_economy WHERE server_id = ? AND uuid = ?")
        .bind(server.economy_id)
        .bind(uuid)
        .fetch_one(&mut *tx)
        .await?;
    let delta = delta_for(&p.mode, p.amount, current)?;
    let balance = if delta == 0.0 {
        current
    } else {
        let balance: Option<f64> = sqlx::query_scalar("UPDATE server_economy SET balance = round(balance + ?1, 2), updated_at = ?2 WHERE server_id = ?3 AND uuid = ?4 AND balance + ?1 >= 0 RETURNING balance")
            .bind(delta).bind(Utc::now().to_rfc3339()).bind(server.economy_id).bind(uuid).fetch_optional(&mut *tx).await?;
        let Some(balance) = balance else { return Err(AppError::bad_request("Insufficient funds")) };
        let (from, from_name, to, to_name) =
            if delta > 0.0 { ("server", "Admin", uuid, name.as_str()) } else { (uuid, name.as_str(), "server", "Admin") };
        sqlx::query("INSERT INTO economy_transactions(server_id, from_uuid, from_name, to_uuid, to_name, amount, description, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(server.id).bind(from).bind(from_name).bind(to).bind(to_name).bind(delta.abs()).bind(format!("Admin: {reason}")).bind(Utc::now().to_rfc3339())
            .execute(&mut *tx).await?;
        balance
    };
    tx.commit().await?;
    if delta != 0.0 {
        let body = if delta > 0.0 {
            format!("An admin added ${:.2} to your balance on {}. Reason: {reason}. New balance: ${balance:.2}.", delta, server.name)
        } else {
            format!("An admin removed ${:.2} from your balance on {}. Reason: {reason}. New balance: ${balance:.2}.", -delta, server.name)
        };
        notifications::push(&state.db, uuid, "economy_adjust", "Your balance was changed", &body, None).await;
    }
    Ok(Json(json!({ "ok": true, "balance": balance, "delta": delta })))
}

// ---------------------------------------------------------------------------
// Guilds
// ---------------------------------------------------------------------------

pub async fn guilds(_admin: AdminUser, State(state): State<AppState>, Query(q): Query<ServerQuery>) -> AppResult<Json<Value>> {
    let server = scoped(&state, q.server_id).await?;
    let rows: Vec<(String, String, String, String, i64, f64)> = sqlx::query_as(
        "SELECT g.id, g.name, g.tag, g.leader_uuid, (SELECT COUNT(*) FROM guild_members m WHERE m.guild_id = g.id), COALESCE(w.balance, 0.0)
         FROM guilds g LEFT JOIN guild_wallets w ON w.guild_id = g.id AND w.server_id = ? WHERE g.instance_id = ? ORDER BY COALESCE(w.balance, 0.0) DESC, g.name",
    )
    .bind(server.economy_id).bind(&server.instance_id).fetch_all(&state.db).await?;
    Ok(Json(json!(rows.into_iter().map(|(id, name, tag, leader_uuid, members, balance)| json!({ "id": id, "name": name, "tag": tag, "leader_uuid": leader_uuid, "members": members, "member_count": members, "balance": balance })).collect::<Vec<_>>())))
}

pub async fn adjust_guild(admin: AdminUser, State(state): State<AppState>, Json(p): Json<AdjustPayload>) -> AppResult<Json<Value>> {
    let server = scoped(&state, p.server_id).await?;
    let gid = p.guild_id.as_deref().map(str::trim).filter(|g| !g.is_empty()).ok_or_else(|| AppError::bad_request("A guild is required"))?;
    let reason = clean(p.reason.as_deref(), "adjustment");
    let mut tx = state.db.begin().await?;
    let guild: Option<(String, String)> = sqlx::query_as("SELECT leader_uuid, tag FROM guilds WHERE id = ? AND instance_id = ?")
        .bind(gid)
        .bind(&server.instance_id)
        .fetch_optional(&mut *tx)
        .await?;
    let Some((leader, tag)) = guild else { return Err(AppError::not_found("Guild not found on this server")) };
    guild_bank::ensure_wallet(&mut tx, server.economy_id, gid).await?;
    let current: f64 = sqlx::query_scalar("SELECT balance FROM guild_wallets WHERE server_id = ? AND guild_id = ?")
        .bind(server.economy_id)
        .bind(gid)
        .fetch_one(&mut *tx)
        .await?;
    let delta = delta_for(&p.mode, p.amount, current)?;
    let balance = if delta == 0.0 {
        current
    } else {
        let b = guild_bank::adjust_wallet(&mut tx, server.economy_id, gid, delta)
            .await
            .map_err(|_| AppError::bad_request("Insufficient funds"))?;
        guild_bank::log(&mut tx, server.economy_id, gid, &admin.uuid, "admin", delta, &format!("Admin: {reason}")).await?;
        b
    };
    tx.commit().await?;
    if delta != 0.0 {
        let verb = if delta > 0.0 { "added to" } else { "removed from" };
        notifications::push(
            &state.db,
            &leader,
            "economy_adjust",
            "Guild bank changed",
            &format!("An admin {verb} the [{tag}] guild bank: ${:.2}. Reason: {reason}. New balance: ${balance:.2}.", delta.abs()),
            None,
        )
        .await;
    }
    Ok(Json(json!({ "ok": true, "balance": balance, "delta": delta })))
}

// ---------------------------------------------------------------------------
// Market
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct MarketQuery {
    pub server_id: i64,
    pub kind: Option<String>,
    #[serde(default)]
    pub q: String,
    pub sort: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

type ListingRow = (
    i64,
    String,
    String,
    Option<String>,
    String,
    String,
    i32,
    f64,
    String,
    String,
    Option<String>,
    Option<f64>,
    Option<String>,
    i32,
    String,
);

pub async fn market(_admin: AdminUser, State(state): State<AppState>, Query(p): Query<MarketQuery>) -> AppResult<Json<Value>> {
    let server = scoped(&state, p.server_id).await?;
    let kind = match p.kind.as_deref() {
        Some("buy_now") => "AND m.kind <> 'auction'",
        Some("auction") => "AND m.kind = 'auction'",
        _ => "",
    };
    let order = match p.sort.as_deref() {
        Some("price") => "COALESCE(m.current_bid, m.price) DESC, m.id DESC",
        Some("ending") => "(m.kind <> 'auction'), m.ends_at, m.id DESC",
        _ => "m.id DESC",
    };
    let pattern = like(&p.q);
    let filter = format!("FROM server_market m LEFT JOIN guilds g ON g.id = m.seller_guild_id WHERE m.server_id = ? {kind} AND (m.item_name LIKE ?2 ESCAPE '\\' OR m.seller_name LIKE ?2 ESCAPE '\\' OR m.item_id LIKE ?2 ESCAPE '\\')");
    let total: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) {filter}")).bind(server.id).bind(&pattern).fetch_one(&state.db).await?;
    let rows: Vec<ListingRow> = sqlx::query_as(&format!(
        "SELECT m.id, m.seller_uuid, m.seller_name, g.tag, m.item_id, m.item_name, m.amount, m.price, m.created_at, m.kind, m.ends_at, m.current_bid, m.bidder_name, m.bid_count, COALESCE(m.bidder_uuid,'') {filter} ORDER BY {order} LIMIT ?3 OFFSET ?4"
    ))
    .bind(server.id).bind(&pattern).bind(p.limit.unwrap_or(100).clamp(1, 300)).bind(p.offset.unwrap_or(0).max(0))
    .fetch_all(&state.db).await?;
    Ok(Json(json!({
        "total": total,
        "rows": rows.into_iter().map(|(id, seller_uuid, seller_name, seller_guild, item_id, item_name, amount, price, created_at, kind, ends_at, current_bid, bidder_name, bid_count, bidder_uuid)| json!({
            "id": id, "seller_uuid": seller_uuid, "seller_name": seller_name, "seller_guild": seller_guild, "item_id": item_id, "item_name": item_name, "amount": amount,
            "price": price, "kind": kind, "ends_at": ends_at, "current_bid": current_bid, "bidder_name": bidder_name, "bidder_uuid": bidder_uuid, "bid_count": bid_count, "created_at": created_at,
        })).collect::<Vec<_>>(),
    })))
}

#[derive(Deserialize)]
pub struct ListingAction {
    pub server_id: i64,
    pub reason: Option<String>,
    pub hours: Option<i64>,
    pub price: Option<f64>,
}

async fn credit(conn: &mut SqliteConnection, eco: i64, uuid: &str, name: &str, amount: f64) -> AppResult<()> {
    ensure_balance_conn(conn, eco, uuid, name).await?;
    sqlx::query("UPDATE server_economy SET balance = balance + ?, updated_at = ? WHERE server_id = ? AND uuid = ?")
        .bind(amount)
        .bind(Utc::now().to_rfc3339())
        .bind(eco)
        .bind(uuid)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

async fn ensure_balance_conn(conn: &mut SqliteConnection, eco: i64, uuid: &str, name: &str) -> AppResult<()> {
    let start = crate::progression::load(&mut *conn).await?.rules.starting_balance;
    sqlx::query("INSERT INTO server_economy(server_id, uuid, username, balance, updated_at) VALUES (?, ?, ?, ?, ?) ON CONFLICT(server_id, uuid) DO NOTHING")
        .bind(eco).bind(uuid).bind(name).bind(start).bind(Utc::now().to_rfc3339()).execute(&mut *conn).await?;
    Ok(())
}

pub async fn remove_listing(
    _admin: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(p): Json<ListingAction>,
) -> AppResult<Json<Value>> {
    let server = scoped(&state, p.server_id).await?;
    let reason = clean(p.reason.as_deref(), "removed by admin");
    let mut tx = state.db.begin().await?;
    // Deleting first makes this idempotent and takes the write lock before any money moves.
    let row: Option<(String, String, String, i32, Option<String>, Option<f64>, Option<String>, Option<String>, String)> = sqlx::query_as(
        "DELETE FROM server_market WHERE id = ? AND server_id = ? RETURNING seller_uuid, item_id, item_name, amount, item_data, current_bid, bidder_uuid, bidder_name, kind",
    )
    .bind(id).bind(server.id).fetch_optional(&mut *tx).await?;
    let Some((seller, item_id, item_name, amount, data, bid, bidder, bidder_name, kind)) = row else {
        return Err(AppError::not_found("Listing not found"));
    };
    let now = Utc::now().to_rfc3339();
    sqlx::query("INSERT INTO market_mailbox (server_id, uuid, item_id, item_name, amount, item_data, note, created_at, to_vault) VALUES (?, ?, ?, ?, ?, ?, 'removed by admin', ?, 1)")
        .bind(server.id).bind(&seller).bind(&item_id).bind(&item_name).bind(amount).bind(&data).bind(&now).execute(&mut *tx).await?;
    let mut refunded = None;
    if let (true, Some(bid), Some(bidder)) = (kind == "auction", bid, bidder.as_deref()) {
        let bname = bidder_name.clone().unwrap_or_else(|| "Player".into());
        credit(&mut tx, server.economy_id, bidder, &bname, bid).await?;
        sqlx::query("INSERT INTO economy_transactions(server_id, from_uuid, from_name, to_uuid, to_name, amount, description, created_at) VALUES (?, 'auction', 'Auction escrow', ?, ?, ?, ?, ?)")
            .bind(server.id).bind(bidder).bind(&bname).bind(bid).bind(format!("Auction refund: {item_name}")).bind(&now).execute(&mut *tx).await?;
        refunded = Some((bidder.to_string(), bid));
    }
    tx.commit().await?;
    notifications::push(
        &state.db,
        &seller,
        "market_removed",
        "Your listing was removed",
        &format!("An admin removed your listing of {item_name}. Reason: {reason}. The item is waiting for you."),
        None,
    )
    .await;
    if let Some((b, amt)) = &refunded {
        notifications::push(
            &state.db,
            b,
            "auction_refund",
            "Auction cancelled",
            &format!("An admin removed the auction for {item_name}. Your ${amt:.2} bid was refunded."),
            None,
        )
        .await;
    }
    Ok(Json(json!({ "ok": true, "refunded": refunded.map(|r| r.1) })))
}

pub async fn extend_auction(
    _admin: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(p): Json<ListingAction>,
) -> AppResult<Json<Value>> {
    let hours = p.hours.unwrap_or(0);
    if !(1..=super::auctions::MAX_HOURS).contains(&hours) {
        return Err(AppError::bad_request("Extend by 1 to 168 hours"));
    }
    let server = scoped(&state, p.server_id).await?;
    let mut tx = state.db.begin().await?;
    let row: Option<(String, Option<String>, String, String)> =
        sqlx::query_as("SELECT kind, ends_at, seller_uuid, item_name FROM server_market WHERE id = ? AND server_id = ?")
            .bind(id)
            .bind(server.id)
            .fetch_optional(&mut *tx)
            .await?;
    let Some((kind, ends, seller, item)) = row else { return Err(AppError::not_found("Listing not found")) };
    if kind != "auction" {
        return Err(AppError::bad_request("Only auctions have an end time"));
    }
    let now = Utc::now();
    let end = ends.as_deref().and_then(|e| DateTime::parse_from_rfc3339(e).ok()).map(|d| d.with_timezone(&Utc)).unwrap_or(now).max(now);
    let new_end = (end + Duration::hours(hours)).to_rfc3339();
    sqlx::query("UPDATE server_market SET ends_at = ? WHERE id = ?").bind(&new_end).bind(id).execute(&mut *tx).await?;
    tx.commit().await?;
    notifications::push(
        &state.db,
        &seller,
        "auction_extended",
        "Your auction was extended",
        &format!("An admin extended your auction of {item} by {hours}h."),
        None,
    )
    .await;
    Ok(Json(json!({ "ok": true, "ends_at": new_end })))
}

pub async fn set_price(
    _admin: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(p): Json<ListingAction>,
) -> AppResult<Json<Value>> {
    let price = check_amount(p.price.unwrap_or(-1.0), false)?;
    let server = scoped(&state, p.server_id).await?;
    let mut tx = state.db.begin().await?;
    let row: Option<(String, i32)> = sqlx::query_as("SELECT kind, bid_count FROM server_market WHERE id = ? AND server_id = ?")
        .bind(id)
        .bind(server.id)
        .fetch_optional(&mut *tx)
        .await?;
    let Some((kind, bids)) = row else { return Err(AppError::not_found("Listing not found")) };
    if bids > 0 {
        return Err(AppError::bad_request("This listing already has bids"));
    }
    let _ = kind;
    sqlx::query("UPDATE server_market SET price = ? WHERE id = ?").bind(price).bind(id).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(Json(json!({ "ok": true, "price": price })))
}

// ---------------------------------------------------------------------------
// Ledger
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct LedgerQuery {
    pub server_id: i64,
    #[serde(default)]
    pub q: String,
    pub limit: Option<i64>,
    pub before_id: Option<i64>,
}

pub async fn transactions(_admin: AdminUser, State(state): State<AppState>, Query(p): Query<LedgerQuery>) -> AppResult<Json<Value>> {
    let server = scoped(&state, p.server_id).await?;
    let pattern = like(&p.q);
    let rows: Vec<(i64, String, String, String, String, f64, String, String)> = sqlx::query_as(&format!(
        "SELECT id, from_uuid, from_name, to_uuid, to_name, amount, description, created_at FROM economy_transactions
         WHERE server_id IN {GROUP} AND id < ?2 AND (from_name LIKE ?3 ESCAPE '\\' OR to_name LIKE ?3 ESCAPE '\\' OR description LIKE ?3 ESCAPE '\\')
         ORDER BY id DESC LIMIT ?4"
    ))
    .bind(server.id).bind(p.before_id.unwrap_or(i64::MAX)).bind(&pattern).bind(p.limit.unwrap_or(100).clamp(1, 300))
    .fetch_all(&state.db).await?;
    let next = rows.last().map(|r| r.0);
    Ok(Json(json!({
        "rows": rows.into_iter().map(|(id, from_uuid, from_name, to_uuid, to_name, amount, description, created_at)| json!({
            "id": id, "from_uuid": from_uuid, "from_name": from_name, "to_uuid": to_uuid, "to_name": to_name, "amount": amount, "description": description, "created_at": created_at })).collect::<Vec<_>>(),
        "next_before_id": next,
    })))
}

// ---------------------------------------------------------------------------
// Bank accounts: players who have none, and accounts that sit under the wrong player
// ---------------------------------------------------------------------------

/// Everyone who counts as a player of the economy scope `?1`: active panel accounts, and anyone a server in the group has seen.
const PLAYERS: &str = "SELECT uuid, MAX(name) AS name FROM (
    SELECT uuid, username AS name FROM users WHERE status = 'active'
    UNION ALL
    SELECT uuid, name FROM player_stats WHERE name <> '' AND server_id IN (SELECT id FROM game_servers WHERE id = ?1 OR (economy_group <> '' AND economy_group = (SELECT economy_group FROM game_servers WHERE id = ?1) COLLATE NOCASE))
) GROUP BY uuid";

pub async fn missing_accounts(db: &sqlx::SqlitePool, scope: i64) -> AppResult<Vec<(String, String)>> {
    Ok(sqlx::query_as(&format!("SELECT p.uuid, p.name FROM ({PLAYERS}) p WHERE NOT EXISTS (SELECT 1 FROM server_economy e WHERE e.server_id = ?1 AND e.uuid = p.uuid) ORDER BY p.name COLLATE NOCASE"))
        .bind(scope)
        .fetch_all(db)
        .await?)
}

/// Accounts that belong to nobody the panel knows (typically the same person under an old or offline-mode UUID).
pub async fn orphan_accounts(db: &sqlx::SqlitePool, scope: i64) -> AppResult<Vec<(String, String, f64)>> {
    Ok(sqlx::query_as(&format!("SELECT e.uuid, e.username, e.balance FROM server_economy e WHERE e.server_id = ?1 AND NOT EXISTS (SELECT 1 FROM ({PLAYERS}) p WHERE p.uuid = e.uuid) ORDER BY e.username COLLATE NOCASE"))
        .bind(scope)
        .fetch_all(db)
        .await?)
}

/// Hand an existing account (with its balance and history) to another UUID.
async fn move_account(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, scope: i64, from: &str, to: &str, name: &str) -> AppResult<()> {
    sqlx::query("UPDATE server_economy SET uuid = ?1, username = ?2, updated_at = ?3 WHERE server_id = ?4 AND uuid = ?5")
        .bind(to).bind(name).bind(Utc::now().to_rfc3339()).bind(scope).bind(from).execute(&mut **tx).await?;
    for column in ["from_uuid", "to_uuid"] {
        sqlx::query(&format!("UPDATE economy_transactions SET {column} = ?2 WHERE {column} = ?3 AND server_id IN {GROUP}"))
            .bind(scope).bind(to).bind(from).execute(&mut **tx).await?;
    }
    Ok(())
}

/// Give every player without an account one (at the starting balance), first re-linking an unclaimed account with the same
/// name. Safe to run any time and as often as you like. Returns (created, re-linked).
pub async fn repair_scope(state: &AppState, scope: i64, link_by_name: bool) -> AppResult<(usize, usize)> {
    let missing = missing_accounts(&state.db, scope).await?;
    if missing.is_empty() {
        return Ok((0, 0));
    }
    let start = crate::progression::load_pool(&state.db).await?.rules.starting_balance;
    let mut orphans = if link_by_name { orphan_accounts(&state.db, scope).await? } else { vec![] };
    let (mut created, mut linked) = (0, 0);
    let mut tx = state.db.begin().await?;
    for (uuid, name) in missing {
        if let Some(i) = orphans.iter().position(|o| o.1.eq_ignore_ascii_case(&name)) {
            let (old, _, _) = orphans.remove(i);
            move_account(&mut tx, scope, &old, &uuid, &name).await?;
            linked += 1;
        } else {
            sqlx::query("INSERT INTO server_economy(server_id, uuid, username, balance, updated_at) VALUES (?, ?, ?, ?, ?) ON CONFLICT(server_id, uuid) DO NOTHING")
                .bind(scope).bind(&uuid).bind(&name).bind(start).bind(Utc::now().to_rfc3339()).execute(&mut *tx).await?;
            created += 1;
        }
    }
    tx.commit().await?;
    Ok((created, linked))
}

/// The scheduled job: repair every economy the panel has.
pub async fn repair_all(state: &AppState) -> AppResult<String> {
    let ids: Vec<i64> = sqlx::query_scalar("SELECT id FROM game_servers ORDER BY id").fetch_all(&state.db).await?;
    let mut scopes = std::collections::BTreeSet::new();
    for id in ids {
        scopes.insert(economy_scope(&state.db, id).await?);
    }
    let (mut created, mut linked) = (0, 0);
    for scope in scopes {
        let (c, l) = repair_scope(state, scope, true).await?;
        created += c;
        linked += l;
    }
    Ok(format!("{created} accounts created, {linked} re-linked"))
}

/// `GET /economy/accounts/missing`: who has no account, and which accounts are unclaimed (so one can be handed to a player).
pub async fn missing(_admin: AdminUser, State(state): State<AppState>, Query(q): Query<ServerQuery>) -> AppResult<Json<Value>> {
    let server = scoped(&state, q.server_id).await?;
    let missing = missing_accounts(&state.db, server.economy_id).await?;
    let orphans = orphan_accounts(&state.db, server.economy_id).await?;
    let suggest = |name: &str| orphans.iter().find(|o| o.1.eq_ignore_ascii_case(name)).map(|o| o.0.clone());
    Ok(Json(json!({
        "total": missing.len(),
        "missing": missing.iter().take(200).map(|(uuid, name)| json!({ "uuid": uuid, "name": name, "suggested_from": suggest(name) })).collect::<Vec<_>>(),
        "orphans": orphans.iter().take(200).map(|(uuid, name, balance)| json!({ "uuid": uuid, "username": name, "balance": balance })).collect::<Vec<_>>(),
    })))
}

#[derive(Deserialize)]
pub struct RepairPayload {
    pub server_id: i64,
    /// Re-link an unclaimed account with the same name instead of opening a new one (default on).
    #[serde(default = "yes")]
    pub link_by_name: bool,
}
fn yes() -> bool {
    true
}

/// `POST /economy/accounts/repair`: open an account for every player who has none.
pub async fn repair(_admin: AdminUser, State(state): State<AppState>, Json(p): Json<RepairPayload>) -> AppResult<Json<Value>> {
    let server = scoped(&state, p.server_id).await?;
    let (created, linked) = repair_scope(&state, server.economy_id, p.link_by_name).await?;
    let remaining = missing_accounts(&state.db, server.economy_id).await?.len();
    Ok(Json(json!({ "ok": true, "created": created, "linked": linked, "remaining": remaining })))
}

#[derive(Deserialize)]
pub struct AssignPayload {
    pub server_id: i64,
    pub uuid: String,
    /// Take over this unclaimed account (balance and history move to the player).
    pub from_uuid: Option<String>,
    /// Opening balance for a brand-new account (the starting balance when left out).
    pub balance: Option<f64>,
}

/// `POST /economy/accounts/assign`: give one player an account, new or taken over from an unclaimed one.
pub async fn assign(_admin: AdminUser, State(state): State<AppState>, Json(p): Json<AssignPayload>) -> AppResult<Json<Value>> {
    let server = scoped(&state, p.server_id).await?;
    let scope = server.economy_id;
    let uuid = p.uuid.trim();
    let name: Option<String> = sqlx::query_scalar(&format!("SELECT name FROM ({PLAYERS}) WHERE uuid = ?2")).bind(scope).bind(uuid).fetch_optional(&state.db).await?;
    let name = name.ok_or_else(|| AppError::bad_request("That player is not known to the panel"))?;
    let has: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM server_economy WHERE server_id = ? AND uuid = ?)").bind(scope).bind(uuid).fetch_one(&state.db).await?;
    if has {
        return Err(AppError::conflict("That player already has an account"));
    }
    let mut tx = state.db.begin().await?;
    let balance = if let Some(from) = p.from_uuid.as_deref().filter(|f| !f.trim().is_empty()) {
        let found: Option<f64> = sqlx::query_scalar("SELECT balance FROM server_economy WHERE server_id = ? AND uuid = ?").bind(scope).bind(from).fetch_optional(&mut *tx).await?;
        let Some(balance) = found else { return Err(AppError::not_found("That account no longer exists")) };
        move_account(&mut tx, scope, from, uuid, &name).await?;
        balance
    } else {
        let start = match p.balance {
            Some(b) => check_amount(b, true)?,
            None => crate::progression::load_pool(&state.db).await?.rules.starting_balance,
        };
        sqlx::query("INSERT INTO server_economy(server_id, uuid, username, balance, updated_at) VALUES (?, ?, ?, ?, ?)")
            .bind(scope).bind(uuid).bind(&name).bind(start).bind(Utc::now().to_rfc3339()).execute(&mut *tx).await?;
        start
    };
    tx.commit().await?;
    Ok(Json(json!({ "ok": true, "uuid": uuid, "username": name, "balance": balance })))
}
