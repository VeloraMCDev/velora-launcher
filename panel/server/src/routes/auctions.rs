//! Market auctions. A listing is either *buy now* (the original instant sale) or an *auction*: bidders' money is held while
//! they lead, the previous leader is refunded when outbid, a late bid extends the clock, and when time runs out the seller is
//! paid and the winner's item waits in a mailbox (`/market claim`). Unsold items go back to the seller the same way.

use crate::state::RequestState as State;
use super::economy::{begin_operation, ensure_balance, finish_operation};
use super::notifications;
use crate::error::{AppError, AppResult};
use crate::routes::servers::GameServer;
use crate::state::AppState;

use axum::Json;
use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::SqliteConnection;

pub const MAX_HOURS: i64 = 168;
const ANTI_SNIPE_SECS: i64 = 120;

pub fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// The lowest bid that is accepted next: the starting price, then 5% (at least 1) above the leading bid.
pub fn min_next_bid(start: f64, current: Option<f64>) -> f64 {
    match current {
        None => round2(start),
        Some(c) => round2(c + (c * 0.05).max(1.0)),
    }
}

async fn log_tx(conn: &mut SqliteConnection, server_id: i64, from: (&str, &str), to: (&str, &str), amount: f64, description: &str) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO economy_transactions (server_id, from_uuid, from_name, to_uuid, to_name, amount, description, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(server_id)
    .bind(from.0)
    .bind(from.1)
    .bind(to.0)
    .bind(to.1)
    .bind(amount)
    .bind(description)
    .bind(Utc::now().to_rfc3339())
    .execute(&mut *conn)
    .await?;
    Ok(())
}

async fn credit(conn: &mut SqliteConnection, economy_id: i64, uuid: &str, name: &str, amount: f64) -> AppResult<()> {
    let start = crate::progression::load(&mut *conn).await?.rules.starting_balance;
    sqlx::query("INSERT INTO server_economy(server_id, uuid, username, balance, updated_at) VALUES (?, ?, ?, ?, ?) ON CONFLICT(server_id, uuid) DO NOTHING")
        .bind(economy_id).bind(uuid).bind(name).bind(start).bind(Utc::now().to_rfc3339()).execute(&mut *conn).await?;
    sqlx::query("UPDATE server_economy SET balance = balance + ?, updated_at = ? WHERE server_id = ? AND uuid = ?")
        .bind(amount).bind(Utc::now().to_rfc3339()).bind(economy_id).bind(uuid).execute(&mut *conn).await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Bidding
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct BidPayload {
    pub operation_id: String,
    pub listing_id: i64,
    pub bidder_uuid: String,
    pub bidder_name: String,
    /// Leave out to bid the minimum.
    pub amount: Option<f64>,
}

pub async fn server_bid(GameServer(server): GameServer, State(state): State<AppState>, Json(p): Json<BidPayload>) -> AppResult<Json<Value>> {
    bid_core(&state, &server, p).await
}

pub(crate) async fn bid_core(state: &AppState, server: &crate::routes::servers::ServerRow, p: BidPayload) -> AppResult<Json<Value>> {
    let (mut tx, previous) = begin_operation(state, server.id, &p.operation_id).await?;
    if let Some(previous) = previous {
        return Ok(Json(previous));
    }
    let row: Option<(String, String, f64, Option<f64>, Option<String>, Option<String>, Option<String>, String, i32)> = sqlx::query_as(
        "SELECT seller_uuid, item_name, price, current_bid, bidder_uuid, bidder_name, ends_at, kind, amount FROM server_market WHERE id = ? AND server_id = ?",
    )
    .bind(p.listing_id)
    .bind(server.id)
    .fetch_optional(&mut *tx)
    .await?;
    let Some((seller_uuid, item_name, start, current, prev_uuid, prev_name, ends_at, kind, amount)) = row else {
        return Err(AppError::not_found("Listing not found"));
    };
    if kind != "auction" {
        return Err(AppError::bad_request("That listing is a buy-now sale, not an auction"));
    }
    let now = Utc::now();
    let end: DateTime<Utc> = ends_at.as_deref().and_then(|e| DateTime::parse_from_rfc3339(e).ok()).map(|d| d.with_timezone(&Utc)).unwrap_or(now);
    if end <= now {
        return Err(AppError::bad_request("This auction has already ended"));
    }
    if seller_uuid == p.bidder_uuid {
        return Err(AppError::forbidden("You can't bid on your own auction"));
    }
    let min = min_next_bid(start, current);
    let bid = round2(p.amount.unwrap_or(min));
    if !bid.is_finite() || bid > 1e12 {
        return Err(AppError::bad_request("Invalid bid"));
    }
    if bid + 1e-9 < min {
        return Err(AppError::bad_request(format!("The minimum bid is ${min:.2}")));
    }
    let economy = server.economy_id;
    // Give the leader their money back (this may be the bidder raising their own bid), then take the new bid.
    if let (Some(c), Some(u)) = (current, prev_uuid.as_deref()) {
        credit(&mut tx, economy, u, prev_name.as_deref().unwrap_or("Player"), c).await?;
        log_tx(&mut tx, server.id, ("auction", "Auction escrow"), (u, prev_name.as_deref().unwrap_or("Player")), c, &format!("Auction refund: {item_name}")).await?;
    }
    ensure_balance(&mut tx, economy, &p.bidder_uuid, &p.bidder_name).await?;
    let balance: Option<f64> = sqlx::query_scalar(
        "UPDATE server_economy SET balance = balance - ?, updated_at = ? WHERE server_id = ? AND uuid = ? AND balance >= ? RETURNING balance",
    )
    .bind(bid)
    .bind(now.to_rfc3339())
    .bind(economy)
    .bind(&p.bidder_uuid)
    .bind(bid)
    .fetch_optional(&mut *tx)
    .await?;
    let Some(balance) = balance else {
        return Err(AppError::bad_request("Insufficient funds for that bid"));
    };
    log_tx(&mut tx, server.id, (&p.bidder_uuid, &p.bidder_name), ("auction", "Auction escrow"), bid, &format!("Auction bid: {amount}x {item_name}")).await?;
    // A bid in the last moments buys everyone else a little time to answer.
    let new_end = if (end - now).num_seconds() < ANTI_SNIPE_SECS { now + Duration::seconds(ANTI_SNIPE_SECS) } else { end };
    sqlx::query("UPDATE server_market SET current_bid = ?, bidder_uuid = ?, bidder_name = ?, bid_count = bid_count + 1, ends_at = ? WHERE id = ?")
        .bind(bid)
        .bind(&p.bidder_uuid)
        .bind(&p.bidder_name)
        .bind(new_end.to_rfc3339())
        .bind(p.listing_id)
        .execute(&mut *tx)
        .await?;
    let outbid = prev_uuid.filter(|u| *u != p.bidder_uuid);
    let response = json!({
        "ok": true, "current_bid": bid, "ends_at": new_end.to_rfc3339(), "min_next": min_next_bid(start, Some(bid)), "new_balance": balance,
        "message": format!("You lead the auction for {item_name} with ${bid:.2}."),
    });
    let out = finish_operation(tx, server.id, &p.operation_id, response).await?;
    if let Some(u) = outbid {
        notifications::push(&state.db, &u, "auction_outbid", "You were outbid",
            &format!("{} bid ${bid:.2} on {item_name}. Your ${:.2} was refunded.", p.bidder_name, current.unwrap_or(0.0)), None).await;
    }
    notifications::push(&state.db, &seller_uuid, "auction_bid", "New bid on your auction", &format!("{} bid ${bid:.2} on {item_name}.", p.bidder_name), None).await;
    Ok(out)
}

// ---------------------------------------------------------------------------
// Cancelling and the mailbox
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct CancelPayload {
    pub operation_id: String,
    pub listing_id: i64,
    pub uuid: String,
}

fn item_json(id: &str, name: &str, amount: i32, data: &Option<String>) -> Value {
    json!({ "item_id": id, "item_name": name, "amount": amount, "item_data": data })
}

/// Take your own listing down. Auctions can only be cancelled while nobody has bid.
pub async fn server_cancel(GameServer(server): GameServer, State(state): State<AppState>, Json(p): Json<CancelPayload>) -> AppResult<Json<Value>> {
    cancel_core(&state, &server, p, false).await
}

/// `to_vault`: from the launcher the item goes to the player's vault instead of coming back in the response.
pub(crate) async fn cancel_core(state: &AppState, server: &crate::routes::servers::ServerRow, p: CancelPayload, to_vault: bool) -> AppResult<Json<Value>> {
    let (mut tx, previous) = begin_operation(state, server.id, &p.operation_id).await?;
    if let Some(previous) = previous {
        return Ok(Json(previous));
    }
    let row: Option<(String, String, String, i32, Option<String>, String, i32, Option<String>)> = sqlx::query_as(
        "SELECT seller_uuid, item_id, item_name, amount, item_data, kind, bid_count, seller_guild_id FROM server_market WHERE id = ? AND server_id = ?",
    )
    .bind(p.listing_id)
    .bind(server.id)
    .fetch_optional(&mut *tx)
    .await?;
    let Some((seller, item_id, item_name, amount, data, kind, bids, guild)) = row else {
        return Err(AppError::not_found("Listing not found"));
    };
    if seller != p.uuid {
        return Err(AppError::forbidden("That isn't your listing"));
    }
    if kind == "auction" && bids > 0 {
        return Err(AppError::bad_request("Someone has already bid, so this auction can't be cancelled"));
    }
    if guild.is_some() {
        return Err(AppError::bad_request("Guild listings are managed by the guild's leaders and officers; ask them to cancel it"));
    }
    sqlx::query("DELETE FROM server_market WHERE id = ?").bind(p.listing_id).execute(&mut *tx).await?;
    if to_vault {
        sqlx::query("INSERT INTO market_mailbox (server_id, uuid, item_id, item_name, amount, item_data, note, created_at, to_vault) VALUES (?, ?, ?, ?, ?, ?, 'returned', ?, 1)")
            .bind(server.id).bind(&p.uuid).bind(&item_id).bind(&item_name).bind(amount).bind(&data).bind(Utc::now().to_rfc3339())
            .execute(&mut *tx).await?;
    }
    let items = if to_vault { json!([]) } else { json!([item_json(&item_id, &item_name, amount, &data)]) };
    let response = json!({ "ok": true, "items": items, "message": if to_vault { format!("Listing cancelled; {item_name} is waiting in your vault.") } else { format!("Listing cancelled; {item_name} returned to you.") } });
    finish_operation(tx, server.id, &p.operation_id, response).await
}

#[derive(Deserialize)]
pub struct MailboxPayload {
    #[serde(default)]
    pub operation_id: String,
    pub uuid: String,
}

/// How many things are waiting for a player.
pub async fn server_mailbox_count(GameServer(server): GameServer, State(state): State<AppState>, Json(p): Json<MailboxPayload>) -> AppResult<Json<Value>> {
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM market_mailbox WHERE server_id = ? AND uuid = ? AND (leased_until IS NULL OR leased_until < ?)").bind(server.id).bind(&p.uuid).bind(Utc::now().to_rfc3339()).fetch_one(&state.db).await?;
    Ok(Json(json!({ "waiting": n })))
}

/// Hand over everything in the mailbox (auction wins, returned items). Safe to retry with the same operation id.
pub async fn server_mailbox_claim(GameServer(server): GameServer, State(state): State<AppState>, Json(p): Json<MailboxPayload>) -> AppResult<Json<Value>> {
    let (mut tx, previous) = begin_operation(&state, server.id, &p.operation_id).await?;
    if let Some(previous) = previous {
        return Ok(Json(previous));
    }
    let rows: Vec<(String, String, i32, Option<String>)> =
        sqlx::query_as("DELETE FROM market_mailbox WHERE server_id = ? AND uuid = ? AND (leased_until IS NULL OR leased_until < ?) RETURNING item_id, item_name, amount, item_data")
            .bind(server.id)
            .bind(&p.uuid)
            .bind(Utc::now().to_rfc3339())
            .fetch_all(&mut *tx)
            .await?;
    let message = if rows.is_empty() { "Your mailbox is empty.".to_string() } else { format!("Collected {} item stack{} from your mailbox.", rows.len(), if rows.len() == 1 { "" } else { "s" }) };
    let items: Vec<Value> = rows.iter().map(|(i, n, a, d)| item_json(i, n, *a, d)).collect();
    finish_operation(tx, server.id, &p.operation_id, json!({ "ok": true, "items": items, "message": message })).await
}

// ---------------------------------------------------------------------------
// Settlement (scheduled task)
// ---------------------------------------------------------------------------

/// Closes every auction whose time is up. Returns a one-line summary for the scheduled-tasks page.
pub async fn settle_due(state: &AppState) -> AppResult<String> {
    let now = Utc::now().to_rfc3339();
    let due: Vec<(i64, i64)> = sqlx::query_as("SELECT id, server_id FROM server_market WHERE kind = 'auction' AND ends_at IS NOT NULL AND ends_at <= ? ORDER BY ends_at LIMIT 100")
        .bind(&now)
        .fetch_all(&state.db)
        .await?;
    let (mut sold, mut unsold) = (0, 0);
    for (id, server_id) in due {
        match settle_one(state, id, server_id).await {
            Ok(true) => sold += 1,
            Ok(false) => unsold += 1,
            Err(e) => tracing::warn!("auction {id} could not be settled: {}", e.message),
        }
    }
    Ok(format!("{sold} sold, {unsold} returned unsold"))
}

async fn settle_one(state: &AppState, id: i64, server_id: i64) -> AppResult<bool> {
    let economy = crate::routes::servers::economy_scope(&state.db, server_id).await?;
    let mut tx = state.db.begin().await?;
    // Deleting the row first makes settling idempotent: a second pass finds nothing.
    let row: Option<(String, String, String, String, i32, Option<String>, Option<f64>, Option<String>, Option<String>, Option<String>)> = sqlx::query_as(
        "DELETE FROM server_market WHERE id = ? AND server_id = ? AND kind = 'auction'
         RETURNING seller_uuid, seller_name, item_id, item_name, amount, item_data, current_bid, bidder_uuid, bidder_name, seller_guild_id",
    )
    .bind(id)
    .bind(server_id)
    .fetch_optional(&mut *tx)
    .await?;
    let Some((seller_uuid, seller_name, item_id, item_name, amount, data, bid, winner, winner_name, guild)) = row else { return Ok(false) };
    let now = Utc::now().to_rfc3339();
    let mailbox = |uuid: String, note: &'static str| {
        let (item_id, item_name, data, now) = (item_id.clone(), item_name.clone(), data.clone(), now.clone());
        async move {
            Ok::<_, AppError>((uuid, note, item_id, item_name, data, now))
        }
    };
    match (winner, bid) {
        (Some(winner), Some(price)) => {
            let winner_name = winner_name.unwrap_or_else(|| "Player".into());
            // The seller is paid: the guild bank for a guild listing that still has its guild, otherwise the player.
            let guild_target: Option<(String, String)> = match &guild {
                Some(g) => sqlx::query_as("SELECT id, tag FROM guilds WHERE id = ?").bind(g).fetch_optional(&mut *tx).await?,
                None => None,
            };
            let (to_uuid, to_name) = if let Some((gid, tag)) = &guild_target {
                super::guild_bank::adjust_wallet(&mut tx, economy, gid, price).await?;
                super::guild_bank::log(&mut tx, economy, gid, &seller_uuid, "sale", price, &format!("Auction: {amount}x {item_name}")).await?;
                (format!("guild:{gid}"), format!("[{tag}] guild bank"))
            } else {
                credit(&mut tx, economy, &seller_uuid, &seller_name, price).await?;
                (seller_uuid.clone(), seller_name.clone())
            };
            log_tx(&mut tx, server_id, ("auction", "Auction escrow"), (&to_uuid, &to_name), price, &format!("Auction sale: {amount}x {item_name} to {winner_name}")).await?;
            let (uuid, note, item_id, item_name2, data, now) = mailbox(winner.clone(), "won").await?;
            sqlx::query("INSERT INTO market_mailbox (server_id, uuid, item_id, item_name, amount, item_data, note, created_at, to_vault) VALUES (?, ?, ?, ?, ?, ?, ?, ?, 1)")
                .bind(server_id).bind(uuid).bind(item_id).bind(item_name2).bind(amount).bind(data).bind(note).bind(now)
                .execute(&mut *tx).await?;
            tx.commit().await?;
            notifications::push(&state.db, &winner, "auction_won", "You won an auction!",
                &format!("{item_name} for ${price:.2}. Collect it in game with /market claim."), None).await;
            if guild_target.is_none() {
                notifications::push(&state.db, &seller_uuid, "auction_sold", "Your auction sold", &format!("{item_name} sold to {winner_name} for ${price:.2}."), None).await;
            }
            Ok(true)
        }
        _ => {
            let (uuid, note, item_id, item_name2, data, now) = mailbox(seller_uuid.clone(), "unsold").await?;
            sqlx::query("INSERT INTO market_mailbox (server_id, uuid, item_id, item_name, amount, item_data, note, created_at, to_vault) VALUES (?, ?, ?, ?, ?, ?, ?, ?, 1)")
                .bind(server_id).bind(uuid).bind(item_id).bind(item_name2).bind(amount).bind(data).bind(note).bind(now)
                .execute(&mut *tx).await?;
            tx.commit().await?;
            notifications::push(&state.db, &seller_uuid, "auction_unsold", "Your auction ended without bids",
                &format!("{item_name} is waiting for you. Collect it in game with /market claim."), None).await;
            Ok(false)
        }
    }
}

// ---------------------------------------------------------------------------
// Vault deliveries: the game server fetches waiting items, puts them in each player's vault, then confirms.
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct VaultPull {
    #[serde(default)]
    pub limit: Option<i64>,
}

/// Hand out items waiting for a vault. They are leased for ten minutes: `/market claim` skips them meanwhile, and they come back
/// round if the server never confirms.
pub async fn server_vault_pull(GameServer(server): GameServer, State(state): State<AppState>, Json(p): Json<VaultPull>) -> AppResult<Json<Value>> {
    let now = Utc::now();
    let lease = (now + Duration::minutes(10)).to_rfc3339();
    let rows: Vec<(i64, String, String, String, i32, Option<String>, String)> = sqlx::query_as(
        "UPDATE market_mailbox SET leased_until = ? WHERE id IN (
             SELECT id FROM market_mailbox WHERE server_id = ? AND to_vault = 1 AND (leased_until IS NULL OR leased_until < ?) ORDER BY id LIMIT ?)
         RETURNING id, uuid, item_id, item_name, amount, item_data, note",
    )
    .bind(&lease)
    .bind(server.id)
    .bind(now.to_rfc3339())
    .bind(p.limit.unwrap_or(25).clamp(1, 100))
    .fetch_all(&state.db)
    .await?;
    Ok(Json(json!({ "deliveries": rows.into_iter().map(|(id, uuid, item_id, item_name, amount, data, note)| json!({
        "id": id, "uuid": uuid, "item_id": item_id, "item_name": item_name, "amount": amount, "item_data": data, "note": note })).collect::<Vec<_>>() })))
}

#[derive(Deserialize)]
pub struct VaultAck {
    pub ids: Vec<i64>,
    /// Deliveries the vault had no room for: they go back in line instead of being removed.
    #[serde(default)]
    pub failed: Vec<i64>,
}

pub async fn server_vault_ack(GameServer(server): GameServer, State(state): State<AppState>, Json(p): Json<VaultAck>) -> AppResult<Json<Value>> {
    let mut tx = state.db.begin().await?;
    for id in &p.ids {
        sqlx::query("DELETE FROM market_mailbox WHERE id = ? AND server_id = ?").bind(id).bind(server.id).execute(&mut *tx).await?;
    }
    // A full vault: leave the item in the mailbox for `/market claim` instead of retrying it forever.
    let mut stuck: Vec<(String, String)> = Vec::new();
    for id in &p.failed {
        let row: Option<(String, String)> = sqlx::query_as("SELECT uuid, item_name FROM market_mailbox WHERE id = ? AND server_id = ?").bind(id).bind(server.id).fetch_optional(&mut *tx).await?;
        sqlx::query("UPDATE market_mailbox SET to_vault = 0, leased_until = NULL WHERE id = ? AND server_id = ?").bind(id).bind(server.id).execute(&mut *tx).await?;
        stuck.extend(row);
    }
    tx.commit().await?;
    for (uuid, item) in stuck {
        notifications::push(&state.db, &uuid, "market_vault_full", "Your vault is full", &format!("{item} is waiting in your market mailbox. Make room, then use /market claim."), None).await;
    }
    Ok(Json(json!({ "ok": true })))
}
