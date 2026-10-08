//! The market in the launcher: browse listings and auctions, see what sold, buy and bid. Purchases go to the player's vault in
//! game, so it works whether or not they are online.

use crate::state::RequestState as State;
use super::auctions::{bid_core, cancel_core, min_next_bid, BidPayload, CancelPayload};
use super::economy::{market_buy, MarketBuyPayload};
use crate::auth::AuthUser;
use crate::error::AppResult;
use crate::routes::servers::{economy_scope, get_server, ServerRow};
use crate::state::AppState;
use axum::extract::{Path};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

async fn server_of(state: &AppState, id: i64) -> AppResult<ServerRow> {
    let mut server = get_server(state, id).await?;
    server.economy_id = economy_scope(&state.db, server.id).await?;
    Ok(server)
}

fn op() -> String {
    format!("launcher-{}", uuid::Uuid::new_v4())
}

/// Everything for sale on a server.
pub async fn listings(auth: AuthUser, State(state): State<AppState>, Path(server_id): Path<i64>) -> AppResult<Json<Value>> {
    let server = server_of(&state, server_id).await?;
    let rows: Vec<(i64, String, String, Option<String>, String, String, i32, f64, String, String, Option<String>, Option<f64>, Option<String>, Option<String>, i32)> = sqlx::query_as(
        "SELECT m.id, m.seller_uuid, m.seller_name, g.tag, m.item_id, m.item_name, m.amount, m.price, m.created_at, m.kind, m.ends_at, m.current_bid, m.bidder_name, m.bidder_uuid, m.bid_count
         FROM server_market m LEFT JOIN guilds g ON g.id = m.seller_guild_id WHERE m.server_id = ? ORDER BY m.id DESC LIMIT 300",
    )
    .bind(server.id)
    .fetch_all(&state.db)
    .await?;
    let balance: Option<f64> = sqlx::query_scalar("SELECT balance FROM server_economy WHERE server_id = ? AND uuid = ?").bind(server.economy_id).bind(&auth.uuid).fetch_optional(&state.db).await?;
    let list: Vec<Value> = rows
        .into_iter()
        .map(|(id, seller_uuid, seller_name, guild, item_id, item_name, amount, price, created_at, kind, ends_at, bid, bidder_name, bidder_uuid, bids)| {
            json!({
                "id": id, "seller_name": seller_name, "seller_guild": guild, "item_id": item_id, "item_name": item_name, "amount": amount,
                "price": price, "created_at": created_at, "kind": kind, "ends_at": ends_at, "current_bid": bid, "bidder_name": bidder_name, "bid_count": bids,
                "min_next_bid": min_next_bid(price, bid), "mine": seller_uuid == auth.uuid, "leading": bidder_uuid.as_deref() == Some(auth.uuid.as_str()),
            })
        })
        .collect();
    Ok(Json(json!({
        "server": { "id": server.id, "name": server.name }, "balance": balance, "listings": list,
    })))
}

/// My listings' results: what sold, what I won or bought, and what is waiting for me.
pub async fn mine(auth: AuthUser, State(state): State<AppState>, Path(server_id): Path<i64>) -> AppResult<Json<Value>> {
    let server = server_of(&state, server_id).await?;
    let history: Vec<(String, String, f64, String, String)> = sqlx::query_as(
        "SELECT from_uuid, to_uuid, amount, description, created_at FROM economy_transactions
         WHERE server_id = ?1 AND (to_uuid = ?2 OR from_uuid = ?2)
           AND (description LIKE 'Market purchase:%' OR description LIKE 'Auction sale:%' OR description LIKE 'Auction bid:%')
         ORDER BY id DESC LIMIT 60",
    )
    .bind(server.id)
    .bind(&auth.uuid)
    .fetch_all(&state.db)
    .await?;
    let history: Vec<Value> = history
        .into_iter()
        .map(|(from, to, amount, description, at)| {
            let sold = to == auth.uuid && !description.starts_with("Auction bid:");
            let kind = if sold { "sold" } else if description.starts_with("Auction bid:") { "bid" } else { "bought" };
            json!({ "kind": kind, "amount": amount, "description": description, "at": at, "other": if sold { from } else { to } })
        })
        .collect();
    let waiting: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM market_mailbox WHERE server_id = ? AND uuid = ?").bind(server.id).bind(&auth.uuid).fetch_one(&state.db).await?;
    let waiting_items: Vec<(String, i32, String)> =
        sqlx::query_as("SELECT item_name, amount, note FROM market_mailbox WHERE server_id = ? AND uuid = ? ORDER BY id DESC LIMIT 20").bind(server.id).bind(&auth.uuid).fetch_all(&state.db).await?;
    Ok(Json(json!({
        "history": history, "waiting": waiting,
        "waiting_items": waiting_items.into_iter().map(|(n, a, note)| json!({ "item_name": n, "amount": a, "note": note })).collect::<Vec<_>>(),
    })))
}

#[derive(Deserialize)]
pub struct ListingRef {
    pub listing_id: i64,
    pub amount: Option<f64>,
}

pub async fn buy(auth: AuthUser, State(state): State<AppState>, Path(server_id): Path<i64>, Json(p): Json<ListingRef>) -> AppResult<Json<Value>> {
    let server = server_of(&state, server_id).await?;
    let payload = MarketBuyPayload { operation_id: op(), listing_id: p.listing_id, buyer_uuid: auth.uuid.clone(), buyer_name: auth.username.clone(), as_guild: false };
    market_buy(&state, &server, payload, true).await
}

pub async fn bid(auth: AuthUser, State(state): State<AppState>, Path(server_id): Path<i64>, Json(p): Json<ListingRef>) -> AppResult<Json<Value>> {
    let server = server_of(&state, server_id).await?;
    bid_core(&state, &server, BidPayload { operation_id: op(), listing_id: p.listing_id, bidder_uuid: auth.uuid.clone(), bidder_name: auth.username.clone(), amount: p.amount }).await
}

pub async fn cancel(auth: AuthUser, State(state): State<AppState>, Path(server_id): Path<i64>, Json(p): Json<ListingRef>) -> AppResult<Json<Value>> {
    let server = server_of(&state, server_id).await?;
    cancel_core(&state, &server, CancelPayload { operation_id: op(), listing_id: p.listing_id, uuid: auth.uuid.clone() }, true).await
}
