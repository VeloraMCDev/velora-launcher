//! Per-server economy system, marketplace, transactions, and baltop.

use crate::state::RequestState as State;
use crate::auth::AuthUser;
use crate::error::{AppError, AppResult};
use crate::routes::servers::GameServer;
use crate::state::AppState;
use axum::extract::{Path};
use axum::Json;
use scopenet_shared::{BaltopEntry, EconomyTransaction, MarketListing, ServerEconomyBalance};
use serde::Deserialize;
use serde_json::Value;

pub(crate) fn storage_error(error: velora_experiences_storage::StorageError) -> AppError {
    match error {
        velora_experiences_storage::StorageError::InvalidOperationId => AppError::bad_request("Invalid operation ID"),
        velora_experiences_storage::StorageError::BadRequest(message) => AppError::bad_request(message),
        velora_experiences_storage::StorageError::Database(error) => error.into(),
        velora_experiences_storage::StorageError::Json(error) => error.into(),
    }
}
pub(crate) async fn begin_operation(
    state: &AppState,
    server_id: i64,
    operation_id: &str,
) -> AppResult<(sqlx::Transaction<'static, sqlx::Sqlite>, Option<Value>)> {
    velora_experiences_storage::begin_operation(&state.db, server_id, operation_id).await.map_err(storage_error)
}
pub(crate) async fn finish_operation(
    tx: sqlx::Transaction<'_, sqlx::Sqlite>,
    server_id: i64,
    operation_id: &str,
    response: Value,
) -> AppResult<Json<Value>> {
    velora_experiences_storage::finish_operation(tx, server_id, operation_id, response).await.map(Json).map_err(storage_error)
}
pub(crate) async fn ensure_balance(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, server_id: i64, uuid: &str, name: &str) -> AppResult<()> {
    let start = crate::progression::load(&mut **tx).await?.rules.starting_balance;
    velora_experiences_storage::ensure_balance(tx, server_id, uuid, name, start, || chrono::Utc::now().to_rfc3339())
        .await
        .map_err(storage_error)
}

#[derive(Deserialize)]
pub struct AdjustBalancePayload {
    pub uuid: String,
    pub username: String,
    pub delta: f64,
    pub operation_id: String,
    pub description: String,
}

pub async fn server_adjust_balance(
    GameServer(server): GameServer,
    State(state): State<AppState>,
    Json(p): Json<AdjustBalancePayload>,
) -> AppResult<Json<Value>> {
    if !p.delta.is_finite() || p.delta == 0.0 || p.delta.abs() > 1e12 {
        return Err(AppError::bad_request("Invalid amount"));
    }
    let (mut tx, previous) = begin_operation(&state, server.id, &p.operation_id).await?;
    if let Some(previous) = previous {
        return Ok(Json(previous));
    }
    ensure_balance(&mut tx, server.economy_id, &p.uuid, &p.username).await?;
    let balance: Option<f64> = sqlx::query_scalar("UPDATE server_economy SET balance = balance + ?, updated_at = ? WHERE server_id = ? AND uuid = ? AND balance + ? >= 0 RETURNING balance")
        .bind(p.delta).bind(chrono::Utc::now().to_rfc3339()).bind(server.economy_id).bind(&p.uuid).bind(p.delta).fetch_optional(&mut *tx).await?;
    let Some(balance) = balance else {
        return Err(AppError::bad_request("Insufficient funds"));
    };
    let (from, from_name, to, to_name) = if p.delta > 0.0 {
        ("server", "Server Shop", p.uuid.as_str(), p.username.as_str())
    } else {
        (p.uuid.as_str(), p.username.as_str(), "server", "Server Shop")
    };
    sqlx::query("INSERT INTO economy_transactions(server_id, from_uuid, from_name, to_uuid, to_name, amount, description, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(server.id).bind(from).bind(from_name).bind(to).bind(to_name).bind(p.delta.abs()).bind(&p.description).bind(chrono::Utc::now().to_rfc3339()).execute(&mut *tx).await?;
    finish_operation(tx, server.id, &p.operation_id, serde_json::json!({"ok": true, "balance": balance})).await
}

pub async fn server_market_read(GameServer(server): GameServer, State(state): State<AppState>) -> AppResult<Json<Vec<Value>>> {
    let rows: Vec<(i64, String, String, String, String, i32, f64, Option<String>, Option<String>, String, Option<String>, Option<f64>, Option<String>, i32)> = sqlx::query_as(
        "SELECT m.id, m.seller_uuid, m.seller_name, m.item_id, m.item_name, m.amount, m.price, m.item_data, g.tag, m.kind, m.ends_at, m.current_bid, m.bidder_name, m.bid_count
         FROM server_market m LEFT JOIN guilds g ON g.id = m.seller_guild_id WHERE m.server_id = ? ORDER BY m.id DESC LIMIT 45",
    )
    .bind(server.id)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(rows.into_iter().map(|(id, seller_uuid, seller, item_id, item_name, amount, price, data, guild, kind, ends_at, bid, bidder, bids)| {
        let min_next = crate::routes::auctions::min_next_bid(price, bid);
        serde_json::json!({"id":id,"seller_uuid":seller_uuid,"seller_name":seller,"seller_guild":guild,"item_id":item_id,"item_name":item_name,"amount":amount,"price":price,"item_data":data,
            "kind":kind,"ends_at":ends_at,"current_bid":bid,"bidder_name":bidder,"bid_count":bids,"min_next_bid":min_next})
    }).collect()))
}

// ---------------------------------------------------------------------------
// Launcher & Public Economy API
// ---------------------------------------------------------------------------

/// Get current user's economy balances across all connected game servers.
pub async fn get_my_balances(auth: AuthUser, State(state): State<AppState>) -> AppResult<Json<Vec<ServerEconomyBalance>>> {
    let rows: Vec<(i64, String, f64)> = sqlx::query_as(
        "SELECT se.server_id, gs.name, se.balance
         FROM server_economy se
         JOIN game_servers gs ON gs.id = se.server_id
         WHERE se.uuid = ?
         ORDER BY se.balance DESC",
    )
    .bind(&auth.uuid)
    .fetch_all(&state.db)
    .await?;

    let list = rows
        .into_iter()
        .map(|(sid, sname, bal)| ServerEconomyBalance { server_id: sid, server_name: sname, balance: bal, currency_symbol: "$".into() })
        .collect();

    Ok(Json(list))
}

/// The signed-in player's balance on one server (servers that share an economy share the balance). `null` until they have one.
pub async fn get_my_balance_on(auth: AuthUser, State(state): State<AppState>, Path(server_id): Path<i64>) -> AppResult<Json<Value>> {
    let server = crate::routes::servers::get_server(&state, server_id).await?;
    let scope = crate::routes::servers::economy_scope(&state.db, server.id).await?;
    let balance: Option<f64> = sqlx::query_scalar("SELECT balance FROM server_economy WHERE server_id = ? AND uuid = ?")
        .bind(scope)
        .bind(&auth.uuid)
        .fetch_optional(&state.db)
        .await?;
    Ok(Json(serde_json::json!({ "server_id": server.id, "server_name": server.name, "balance": balance, "currency_symbol": "$" })))
}

/// Get current user's recent transactions across all servers.
pub async fn get_my_transactions(auth: AuthUser, State(state): State<AppState>) -> AppResult<Json<Vec<EconomyTransaction>>> {
    let rows: Vec<(i64, i64, String, String, String, String, String, f64, String, String)> = sqlx::query_as(
        "SELECT et.id, et.server_id, gs.name, et.from_uuid, et.from_name, et.to_uuid, et.to_name, et.amount, et.description, et.created_at
         FROM economy_transactions et
         JOIN game_servers gs ON gs.id = et.server_id
         WHERE et.from_uuid = ? OR et.to_uuid = ?
         ORDER BY et.id DESC
         LIMIT 50",
    )
    .bind(&auth.uuid)
    .bind(&auth.uuid)
    .fetch_all(&state.db)
    .await?;

    let list = rows
        .into_iter()
        .map(|(id, sid, sname, fuuid, fname, tuuid, tname, amount, desc, created)| EconomyTransaction {
            id,
            server_id: sid,
            server_name: Some(sname),
            from_uuid: fuuid,
            from_name: fname,
            to_uuid: tuuid,
            to_name: tname,
            amount,
            description: desc,
            created_at: created,
        })
        .collect();

    Ok(Json(list))
}

/// Get top richest players for a specific server.
pub async fn get_server_baltop(Path(server_id): Path<i64>, State(state): State<AppState>) -> AppResult<Json<Vec<BaltopEntry>>> {
    let rows: Vec<(String, String, f64)> = sqlx::query_as(
        "SELECT uuid, username, balance
         FROM server_economy
         WHERE server_id = ?
         ORDER BY balance DESC
         LIMIT 25",
    )
    .bind(server_id)
    .fetch_all(&state.db)
    .await?;

    let list =
        rows.into_iter().enumerate().map(|(i, (uuid, username, balance))| BaltopEntry { rank: i + 1, uuid, username, balance }).collect();

    Ok(Json(list))
}

/// Get active marketplace listings for a server.
pub async fn get_server_market(Path(server_id): Path<i64>, State(state): State<AppState>) -> AppResult<Json<Vec<MarketListing>>> {
    let rows: Vec<(i64, i64, String, String, String, String, i32, f64, String, Option<String>, String, Option<String>, Option<f64>, Option<String>, i32)> = sqlx::query_as(
        "SELECT m.id, m.server_id, m.seller_uuid, m.seller_name, m.item_id, m.item_name, m.amount, m.price, m.created_at, g.tag,
                m.kind, m.ends_at, m.current_bid, m.bidder_name, m.bid_count
         FROM server_market m LEFT JOIN guilds g ON g.id = m.seller_guild_id
         WHERE m.server_id = ?
         ORDER BY m.id DESC
         LIMIT 100",
    )
    .bind(server_id)
    .fetch_all(&state.db)
    .await?;

    let list = rows
        .into_iter()
        .map(|(id, sid, suuid, sname, iid, iname, amt, price, created, guild_tag, kind, ends_at, current_bid, bidder_name, bid_count)| MarketListing {
            seller_guild_tag: guild_tag,
            kind,
            ends_at,
            current_bid,
            bidder_name,
            bid_count,
            id,
            server_id: sid,
            seller_uuid: suuid,
            seller_name: sname,
            item_id: iid,
            item_name: iname,
            amount: amt,
            price,
            created_at: created,
        })
        .collect();

    Ok(Json(list))
}

// ---------------------------------------------------------------------------
// Server Integration Relay Endpoints
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct ServerBalanceQuery {
    pub uuid: String,
    pub username: Option<String>,
}

/// A player's balance on a server (0 until they have an account there).
pub async fn balance_for(state: &AppState, server_id: i64, uuid: &str) -> AppResult<f64> {
    Ok(sqlx::query_scalar("SELECT balance FROM server_economy WHERE server_id = ? AND uuid = ?")
        .bind(server_id)
        .bind(uuid)
        .fetch_optional(&state.db)
        .await?
        .unwrap_or(0.0))
}

pub async fn server_get_balance(
    GameServer(server): GameServer,
    State(state): State<AppState>,
    Json(payload): Json<ServerBalanceQuery>,
) -> AppResult<Json<Value>> {
    let now = chrono::Utc::now().to_rfc3339();
    let username = payload.username.unwrap_or_else(|| "Player".into());

    let bal_opt: Option<f64> = sqlx::query_scalar("SELECT balance FROM server_economy WHERE server_id = ? AND uuid = ?")
        .bind(server.economy_id)
        .bind(&payload.uuid)
        .fetch_optional(&state.db)
        .await?;

    let balance = match bal_opt {
        Some(b) => b,
        None => {
            let starting_balance = crate::progression::load_pool(&state.db).await?.rules.starting_balance;
            sqlx::query(
                "INSERT INTO server_economy (server_id, uuid, username, balance, updated_at)
                 VALUES (?, ?, ?, ?, ?)
                 ON CONFLICT(server_id, uuid) DO UPDATE SET username = excluded.username",
            )
            .bind(server.economy_id)
            .bind(&payload.uuid)
            .bind(&username)
            .bind(starting_balance)
            .bind(&now)
            .execute(&state.db)
            .await?;
            starting_balance
        }
    };

    Ok(Json(serde_json::json!({
        "server_id": server.id,
        "uuid": payload.uuid,
        "balance": balance,
        "currency_symbol": "$"
    })))
}

#[derive(Deserialize)]
pub struct ServerTransferPayload {
    pub from_uuid: String,
    pub from_name: String,
    pub to_uuid: String,
    pub to_name: String,
    pub amount: f64,
    pub description: Option<String>,
}

pub async fn server_transfer(
    GameServer(server): GameServer,
    State(state): State<AppState>,
    Json(payload): Json<ServerTransferPayload>,
) -> AppResult<Json<Value>> {
    if !payload.amount.is_finite() || payload.amount <= 0.0 {
        return Err(AppError::bad_request("Transfer amount must be positive"));
    }

    let mut tx = state.db.begin().await?;
    let now = chrono::Utc::now().to_rfc3339();

    // Check sender balance
    let from_bal: Option<f64> = sqlx::query_scalar("SELECT balance FROM server_economy WHERE server_id = ? AND uuid = ?")
        .bind(server.economy_id)
        .bind(&payload.from_uuid)
        .fetch_optional(&mut *tx)
        .await?;

    let current_from_bal = from_bal.unwrap_or(0.0);
    if current_from_bal < payload.amount {
        return Err(AppError::bad_request("Insufficient funds"));
    }

    let new_from_bal = current_from_bal - payload.amount;
    sqlx::query("UPDATE server_economy SET balance = ?, updated_at = ? WHERE server_id = ? AND uuid = ?")
        .bind(new_from_bal)
        .bind(&now)
        .bind(server.economy_id)
        .bind(&payload.from_uuid)
        .execute(&mut *tx)
        .await?;

    // Update receiver
    sqlx::query(
        "INSERT INTO server_economy (server_id, uuid, username, balance, updated_at)
         VALUES (?, ?, ?, ?, ?)
         ON CONFLICT (server_id, uuid) DO UPDATE SET
             balance = balance + excluded.balance,
             username = excluded.username,
             updated_at = excluded.updated_at",
    )
    .bind(server.economy_id)
    .bind(&payload.to_uuid)
    .bind(&payload.to_name)
    .bind(payload.amount)
    .bind(&now)
    .execute(&mut *tx)
    .await?;

    let desc = payload.description.unwrap_or_else(|| format!("Payment to {}", payload.to_name));
    sqlx::query(
        "INSERT INTO economy_transactions (server_id, from_uuid, from_name, to_uuid, to_name, amount, description, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(server.id)
    .bind(&payload.from_uuid)
    .bind(&payload.from_name)
    .bind(&payload.to_uuid)
    .bind(&payload.to_name)
    .bind(payload.amount)
    .bind(&desc)
    .bind(&now)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(serde_json::json!({
        "ok": true,
        "from_balance": new_from_bal
    })))
}

#[derive(Deserialize)]
pub struct ServerSyncBalancePayload {
    pub uuid: String,
    pub username: String,
    pub new_balance: f64,
}

pub async fn server_sync_balance(
    GameServer(server): GameServer,
    State(state): State<AppState>,
    Json(payload): Json<ServerSyncBalancePayload>,
) -> AppResult<Json<Value>> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO server_economy (server_id, uuid, username, balance, updated_at)
         VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(server_id, uuid) DO UPDATE SET
             balance = excluded.balance,
             username = excluded.username,
             updated_at = excluded.updated_at",
    )
    .bind(server.economy_id)
    .bind(&payload.uuid)
    .bind(&payload.username)
    .bind(payload.new_balance)
    .bind(&now)
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn server_baltop(GameServer(server): GameServer, State(state): State<AppState>) -> AppResult<Json<Vec<BaltopEntry>>> {
    let rows: Vec<(String, String, f64)> = sqlx::query_as(
        "SELECT uuid, username, balance
         FROM server_economy
         WHERE server_id = ?
         ORDER BY balance DESC
         LIMIT 20",
    )
    .bind(server.economy_id)
    .fetch_all(&state.db)
    .await?;

    let list =
        rows.into_iter().enumerate().map(|(i, (uuid, username, balance))| BaltopEntry { rank: i + 1, uuid, username, balance }).collect();

    Ok(Json(list))
}

#[derive(Deserialize)]
pub struct MarketListPayload {
    pub operation_id: String,
    pub item_data: Option<String>,
    pub seller_uuid: String,
    pub seller_name: String,
    pub item_id: String,
    pub item_name: String,
    pub amount: i32,
    pub price: f64,
    /// Sell on behalf of the seller's guild: the sale is paid into its bank.
    #[serde(default)]
    pub as_guild: bool,
    /// `buy_now` (default) or `auction`; for an auction `price` is the starting bid.
    #[serde(default)]
    pub kind: String,
    /// Auction length; defaults to a day.
    #[serde(default)]
    pub duration_hours: Option<i64>,
}

pub async fn server_market_list(
    GameServer(server): GameServer,
    State(state): State<AppState>,
    Json(payload): Json<MarketListPayload>,
) -> AppResult<Json<Value>> {
    if !payload.price.is_finite() || payload.price <= 0.0 || payload.amount <= 0 || payload.amount > 64 {
        return Err(AppError::bad_request("Invalid listing"));
    }
    let (mut tx, previous) = begin_operation(&state, server.id, &payload.operation_id).await?;
    if let Some(previous) = previous {
        return Ok(Json(previous));
    }
    let limit = crate::progression::load(&mut tx).await?.rules.market_max_listings;
    if limit > 0 {
        let mine: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM server_market WHERE server_id = ? AND seller_uuid = ?")
            .bind(server.id)
            .bind(&payload.seller_uuid)
            .fetch_one(&mut *tx)
            .await?;
        if mine >= limit {
            return Err(AppError::bad_request(format!("You can have at most {limit} market listings at once")));
        }
    }
    let now = chrono::Utc::now().to_rfc3339();
    let guild_id = if payload.as_guild {
        let m = crate::routes::guild_bank::membership(&mut tx, &server, &payload.seller_uuid)
            .await?
            .ok_or_else(|| AppError::forbidden("You are not in a guild"))?;
        if !m.can_spend() {
            return Err(AppError::forbidden("Only guild leaders and officers can sell for the guild"));
        }
        Some(m.guild_id)
    } else {
        None
    };
    let auction = match payload.kind.as_str() {
        "" | "buy_now" => false,
        "auction" => true,
        _ => return Err(AppError::bad_request("Listing type must be buy_now or auction")),
    };
    let hours = payload.duration_hours.unwrap_or(24);
    if auction && !(1..=crate::routes::auctions::MAX_HOURS).contains(&hours) {
        return Err(AppError::bad_request(format!("Auctions run for 1 to {} hours", crate::routes::auctions::MAX_HOURS)));
    }
    let ends_at = auction.then(|| (chrono::Utc::now() + chrono::Duration::hours(hours)).to_rfc3339());
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO server_market (server_id, seller_uuid, seller_name, item_id, item_name, amount, price, created_at, item_data, seller_guild_id, kind, ends_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         RETURNING id",
    )
    .bind(server.id)
    .bind(payload.seller_uuid)
    .bind(payload.seller_name)
    .bind(payload.item_id)
    .bind(payload.item_name)
    .bind(payload.amount)
    .bind(payload.price)
    .bind(&now)
    .bind(&payload.item_data)
    .bind(&guild_id)
    .bind(if auction { "auction" } else { "buy_now" })
    .bind(&ends_at)
    .fetch_one(&mut *tx)
    .await?;

    finish_operation(tx, server.id, &payload.operation_id, serde_json::json!({ "id": id, "ok": true, "kind": if auction { "auction" } else { "buy_now" }, "ends_at": ends_at })).await
}

#[derive(Deserialize)]
pub struct MarketBuyPayload {
    pub operation_id: String,
    pub listing_id: i64,
    pub buyer_uuid: String,
    pub buyer_name: String,
    /// Pay from the buyer's guild bank (leaders and officers) instead of their own balance.
    #[serde(default)]
    pub as_guild: bool,
}

pub async fn server_market_buy(
    GameServer(server): GameServer,
    State(state): State<AppState>,
    Json(payload): Json<MarketBuyPayload>,
) -> AppResult<Json<Value>> {
    market_buy(&state, &server, payload, false).await
}

/// Buy a listing. From the game the item is handed over in the response; from the launcher (`to_vault`) it is queued for the
/// player's vault instead, so it arrives whether or not they are online.
pub(crate) async fn market_buy(state: &AppState, server: &crate::routes::servers::ServerRow, payload: MarketBuyPayload, to_vault: bool) -> AppResult<Json<Value>> {
    let (mut tx, previous) = begin_operation(state, server.id, &payload.operation_id).await?;
    if let Some(previous) = previous {
        return Ok(Json(previous));
    }
    let kind: Option<String> = sqlx::query_scalar("SELECT kind FROM server_market WHERE id = ? AND server_id = ?")
        .bind(payload.listing_id)
        .bind(server.id)
        .fetch_optional(&mut *tx)
        .await?;
    if kind.as_deref() == Some("auction") {
        return Err(AppError::bad_request("That listing is an auction. Place a bid with /market bid"));
    }
    let listing: Option<(String, String, String, String, i32, f64, Option<String>, Option<String>)> = sqlx::query_as(
        "DELETE FROM server_market
         WHERE id = ? AND server_id = ?
         RETURNING seller_uuid, seller_name, item_id, item_name, amount, price, item_data, seller_guild_id
",
    )
    .bind(payload.listing_id)
    .bind(server.id)
    .fetch_optional(&mut *tx)
    .await?;

    let Some((seller_uuid, seller_name, item_id, item_name, amount, price, item_data, seller_guild)) = listing else {
        return Err(AppError::not_found("Listing not found"));
    };
    // Guild sales only pay the guild if it still exists; otherwise the lister is paid.
    let seller_guild: Option<(String, String)> = match seller_guild {
        Some(id) => sqlx::query_as("SELECT id, tag FROM guilds WHERE id = ?").bind(id).fetch_optional(&mut *tx).await?,
        None => None,
    };
    let buyer_guild = if payload.as_guild {
        let m = crate::routes::guild_bank::membership(&mut tx, server, &payload.buyer_uuid)
            .await?
            .ok_or_else(|| AppError::forbidden("You are not in a guild"))?;
        if !m.can_spend() {
            return Err(AppError::forbidden("Only guild leaders and officers can spend guild money"));
        }
        Some(m)
    } else {
        None
    };

    ensure_balance(&mut tx, server.economy_id, &payload.buyer_uuid, &payload.buyer_name).await?;
    let now = chrono::Utc::now().to_rfc3339();

    // Buyer pays: their own balance, or their guild's bank.
    let new_balance = if let Some(m) = &buyer_guild {
        let left = crate::routes::guild_bank::adjust_wallet(&mut tx, server.economy_id, &m.guild_id, -price).await?;
        crate::routes::guild_bank::log(
            &mut tx,
            server.economy_id,
            &m.guild_id,
            &payload.buyer_uuid,
            "purchase",
            price,
            &format!("Market: {amount}x {item_name}"),
        )
        .await?;
        left
    } else {
        let buyer_bal: f64 = sqlx::query_scalar("SELECT balance FROM server_economy WHERE server_id = ? AND uuid = ?")
            .bind(server.economy_id)
            .bind(&payload.buyer_uuid)
            .fetch_optional(&mut *tx)
            .await?
            .unwrap_or(0.0);
        if buyer_bal < price {
            return Err(AppError::bad_request("Insufficient funds"));
        }
        sqlx::query("UPDATE server_economy SET balance = balance - ?, updated_at = ? WHERE server_id = ? AND uuid = ?")
            .bind(price)
            .bind(&now)
            .bind(server.economy_id)
            .bind(&payload.buyer_uuid)
            .execute(&mut *tx)
            .await?;
        buyer_bal - price
    };

    // Seller is paid: the guild bank for guild listings, otherwise the player.
    let (to_uuid, to_name) = if let Some((guild_id, tag)) = &seller_guild {
        crate::routes::guild_bank::adjust_wallet(&mut tx, server.economy_id, guild_id, price).await?;
        crate::routes::guild_bank::log(
            &mut tx,
            server.economy_id,
            guild_id,
            &seller_uuid,
            "sale",
            price,
            &format!("Market: {amount}x {item_name}"),
        )
        .await?;
        (format!("guild:{guild_id}"), format!("[{tag}] guild bank"))
    } else {
        // A seller without an account first gets the usual starting balance, then the sale on top.
        ensure_balance(&mut tx, server.economy_id, &seller_uuid, &seller_name).await?;
        sqlx::query("UPDATE server_economy SET balance = balance + ?, username = ?, updated_at = ? WHERE server_id = ? AND uuid = ?")
            .bind(price)
            .bind(&seller_name)
            .bind(&now)
            .bind(server.economy_id)
            .bind(&seller_uuid)
            .execute(&mut *tx)
            .await?;
        (seller_uuid.clone(), seller_name.clone())
    };

    // Delete listing
    sqlx::query("DELETE FROM server_market WHERE id = ?").bind(payload.listing_id).execute(&mut *tx).await?;

    // Log transaction
    let desc = format!("Market purchase: {}x {} from {}", amount, item_name, seller_name);
    sqlx::query(
        "INSERT INTO economy_transactions (server_id, from_uuid, from_name, to_uuid, to_name, amount, description, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(server.id)
    .bind(match &buyer_guild {
        Some(m) => format!("guild:{}", m.guild_id),
        None => payload.buyer_uuid.clone(),
    })
    .bind(match &buyer_guild {
        Some(m) => format!("[{}] guild bank", m.tag),
        None => payload.buyer_name.clone(),
    })
    .bind(&to_uuid)
    .bind(&to_name)
    .bind(price)
    .bind(&desc)
    .bind(&now)
    .execute(&mut *tx)
    .await?;

    if to_vault {
        sqlx::query("INSERT INTO market_mailbox (server_id, uuid, item_id, item_name, amount, item_data, note, created_at, to_vault) VALUES (?, ?, ?, ?, ?, ?, 'bought', ?, 1)")
            .bind(server.id)
            .bind(&payload.buyer_uuid)
            .bind(&item_id)
            .bind(&item_name)
            .bind(amount)
            .bind(&item_data)
            .bind(&now)
            .execute(&mut *tx)
            .await?;
    }
    let seller_for_bell = if seller_guild.is_none() { Some(seller_uuid.clone()) } else { None };
    let buyer_name = payload.buyer_name.clone();
    let out = finish_operation(
        tx,
        server.id,
        &payload.operation_id,
        serde_json::json!({
            "ok": true,
            "item_id": item_id,
            "item_data": if to_vault { Value::Null } else { serde_json::json!(item_data) },
            "item_name": item_name,
            "amount": amount,
            "price": price,
            "new_balance": new_balance,
            "to_vault": to_vault,
            "message": if to_vault { format!("Bought {amount}x {item_name} for ${price:.2}. It is waiting in your vault.") } else { format!("Bought {amount}x {item_name} for ${price:.2}.") },
        }),
    )
    .await?;
    if let Some(seller) = seller_for_bell {
        super::notifications::push(&state.db, &seller, "market_sold", "Your item sold", &format!("{buyer_name} bought {amount}x {item_name} for ${price:.2}."), None).await;
    }
    Ok(out)
}
