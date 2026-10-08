//! Buy orders: a player asks for a number of an item at a total price. The money is taken at once and held in escrow. Anyone
//! else can pick the order up from the board (which reserves it for a while so nobody snipes it), gather the items and submit
//! them in game. The moment the items are in, they land in the buyer's vault and the escrow is released to whoever filled it.
//!
//! Escrow, delivery and payment happen in one transaction with an idempotency row (like the market), so an order can never be
//! filled twice, paid twice, or pay out without delivering.

use crate::state::RequestState as State;
use super::economy::{begin_operation, finish_operation};
use super::ledger::{balance_of, credit, debit, normalize_item, pretty, round2, Tx, ORDERS};
use super::notifications;
use crate::auth::AuthUser;
use crate::error::{AppError, AppResult};
use crate::routes::servers::{economy_scope, get_server, GameServer, ServerRow};
use crate::state::AppState;
use axum::extract::{Path};
use axum::Json;
use chrono::{Duration, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

// ---- settings --------------------------------------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Config {
    pub enabled: bool,
    pub max_open_per_player: u32,
    pub min_total: f64,
    pub max_total: f64,
    pub max_amount: u32,
    /// An order nobody fills comes down after this long and the escrow goes back to the buyer.
    pub expire_hours: u32,
    /// How long picking an order up reserves it for the player who took it.
    pub claim_minutes: u32,
    pub max_claims_per_player: u32,
    /// Share of the payment the house keeps when an order is filled.
    pub fee_percent: f64,
}

impl Default for Config {
    fn default() -> Self {
        Config { enabled: true, max_open_per_player: 5, min_total: 1.0, max_total: 1_000_000.0, max_amount: 2304, expire_hours: 168, claim_minutes: 60, max_claims_per_player: 3, fee_percent: 0.0 }
    }
}

impl Config {
    pub fn sanitize(&mut self) {
        let fin = |v: f64, lo: f64, hi: f64, d: f64| if v.is_finite() { round2(v.clamp(lo, hi)) } else { d };
        self.max_open_per_player = self.max_open_per_player.clamp(1, 100);
        self.min_total = fin(self.min_total, 0.01, 1e9, 1.0);
        self.max_total = fin(self.max_total, self.min_total, 1e12, 1_000_000.0);
        self.max_amount = self.max_amount.clamp(1, 36_864);
        self.expire_hours = self.expire_hours.clamp(1, 24 * 30);
        self.claim_minutes = self.claim_minutes.clamp(5, 24 * 60);
        self.max_claims_per_player = self.max_claims_per_player.clamp(1, 50);
        self.fee_percent = fin(self.fee_percent, 0.0, 50.0, 0.0);
    }
}

pub(crate) async fn config(state: &AppState) -> AppResult<Config> {
    let mut c: Config = crate::store::kv_get(state, "orders").await?;
    c.sanitize();
    Ok(c)
}

// ---- rows ------------------------------------------------------------------------------------------------------------

#[derive(sqlx::FromRow, Clone)]
struct Order {
    id: i64,
    server_id: i64,
    buyer_uuid: String,
    buyer_name: String,
    item_id: String,
    item_name: String,
    amount: i64,
    total: f64,
    status: String,
    claimer_uuid: Option<String>,
    claimer_name: Option<String>,
    claim_until: Option<String>,
    filler_name: Option<String>,
    created_at: String,
    expires_at: String,
}

const COLS: &str = "id, server_id, buyer_uuid, buyer_name, item_id, item_name, amount, total, status, claimer_uuid, claimer_name, claim_until, filler_name, created_at, expires_at";

fn now() -> String {
    crate::db::now()
}

fn at(minutes: i64) -> String {
    (Utc::now() + Duration::minutes(minutes)).to_rfc3339_opts(SecondsFormat::Secs, true)
}

impl Order {
    /// A reservation that has run out no longer counts: the order is open again.
    fn live_claim(&self) -> bool {
        self.status == "claimed" && self.claim_until.as_deref().is_some_and(|t| t > now().as_str())
    }
    fn state(&self) -> &str {
        if self.status == "claimed" && !self.live_claim() { "open" } else { &self.status }
    }
    fn view(&self, me: &str) -> Value {
        let live = self.live_claim();
        json!({
            "id": self.id, "buyer_name": self.buyer_name, "item_id": self.item_id, "item_name": self.item_name, "amount": self.amount,
            "total": self.total, "each": round2(self.total / self.amount.max(1) as f64), "status": self.state(),
            "claimer_name": if live { self.claimer_name.clone() } else { None }, "claim_until": if live { self.claim_until.clone() } else { None },
            "created_at": self.created_at, "expires_at": self.expires_at,
            "mine": self.buyer_uuid == me, "claimed_by_me": live && self.claimer_uuid.as_deref() == Some(me),
            "filler_name": self.filler_name,
        })
    }
}

/// Who is acting, with the server the order lives on (orders are per server; money is per economy).
pub(crate) struct Actor<'a> {
    pub uuid: &'a str,
    pub name: &'a str,
}

async fn server_of(state: &AppState, id: i64) -> AppResult<ServerRow> {
    let mut server = get_server(state, id).await?;
    server.economy_id = economy_scope(&state.db, server.id).await?;
    Ok(server)
}

fn op() -> String {
    format!("order-{}", uuid::Uuid::new_v4())
}

async fn enabled(state: &AppState) -> AppResult<Config> {
    let c = config(state).await?;
    if !c.enabled {
        return Err(AppError::forbidden("Buy orders are switched off."));
    }
    Ok(c)
}

// ---- the board -------------------------------------------------------------------------------------------------------

pub(crate) async fn board(state: &AppState, server: &ServerRow, me: &str) -> AppResult<Value> {
    let cfg = config(state).await?;
    let open: Vec<Order> = sqlx::query_as(&format!("SELECT {COLS} FROM buy_orders WHERE server_id = ? AND status IN ('open', 'claimed') ORDER BY id DESC LIMIT 200"))
        .bind(server.id).fetch_all(&state.db).await?;
    let history: Vec<Order> = sqlx::query_as(&format!(
        "SELECT {COLS} FROM buy_orders WHERE server_id = ?1 AND status IN ('filled', 'cancelled', 'expired') AND (buyer_uuid = ?2 OR filler_uuid = ?2) ORDER BY id DESC LIMIT 25"
    ))
    .bind(server.id).bind(me).fetch_all(&state.db).await?;
    let mine = open.iter().filter(|o| o.buyer_uuid == me).count();
    let claims = open.iter().filter(|o| o.live_claim() && o.claimer_uuid.as_deref() == Some(me)).count();
    Ok(json!({
        "enabled": cfg.enabled,
        "server": { "id": server.id, "name": server.name },
        "balance": balance_of(&state.db, server.economy_id, me).await,
        "orders": open.iter().map(|o| o.view(me)).collect::<Vec<_>>(),
        "history": history.iter().map(|o| { let mut v = o.view(me); v["resolved"] = json!(o.status); v }).collect::<Vec<_>>(),
        "rules": {
            "max_open": cfg.max_open_per_player, "min_total": cfg.min_total, "max_total": cfg.max_total, "max_amount": cfg.max_amount,
            "expire_hours": cfg.expire_hours, "claim_minutes": cfg.claim_minutes, "max_claims": cfg.max_claims_per_player, "fee_percent": cfg.fee_percent,
        },
        "my_open": mine, "my_claims": claims,
    }))
}

pub async fn orders_board(auth: AuthUser, State(state): State<AppState>, Path(sid): Path<i64>) -> AppResult<Json<Value>> {
    let server = server_of(&state, sid).await?;
    Ok(Json(board(&state, &server, &auth.uuid).await?))
}

/// A short list for chat: what is open on the board, newest first.
pub async fn server_list(GameServer(server): GameServer, State(state): State<AppState>, Json(p): Json<WhoBody>) -> AppResult<Json<Value>> {
    let me = p.uuid.unwrap_or_default();
    Ok(Json(board(&state, &server, &me).await?))
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct WhoBody {
    pub uuid: Option<String>,
}

// ---- creating and cancelling -----------------------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct CreateBody {
    pub item_id: String,
    pub amount: i64,
    /// The whole price for all of it, held until someone fills the order.
    pub total: f64,
    #[serde(default)]
    pub item_name: Option<String>,
}

pub(crate) async fn create_core(state: &AppState, server: &ServerRow, who: &Actor<'_>, p: CreateBody, operation: &str) -> AppResult<Json<Value>> {
    let cfg = enabled(state).await?;
    let item_id = normalize_item(&p.item_id)?;
    if p.amount < 1 || p.amount > cfg.max_amount as i64 {
        return Err(AppError::bad_request(format!("Ask for between 1 and {} items.", cfg.max_amount)));
    }
    if !p.total.is_finite() || round2(p.total) < cfg.min_total || round2(p.total) > cfg.max_total {
        return Err(AppError::bad_request(format!("The price must be between ${:.2} and ${:.2}.", cfg.min_total, cfg.max_total)));
    }
    let total = round2(p.total);
    let name = p.item_name.as_deref().map(str::trim).filter(|n| !n.is_empty() && n.len() <= 64).map(str::to_owned).unwrap_or_else(|| pretty(&item_id));
    let (mut tx, previous) = begin_operation(state, server.id, operation).await?;
    if let Some(previous) = previous {
        return Ok(Json(previous));
    }
    let open: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM buy_orders WHERE server_id = ? AND buyer_uuid = ? AND status IN ('open', 'claimed')")
        .bind(server.id).bind(who.uuid).fetch_one(&mut *tx).await?;
    if open >= cfg.max_open_per_player as i64 {
        return Err(AppError::conflict(format!("You can have {} buy orders open at once.", cfg.max_open_per_player)));
    }
    let balance = debit(&mut tx, server.economy_id, server.id, who.uuid, who.name, total, ORDERS, &format!("Buy order escrow: {}x {name}", p.amount)).await?;
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO buy_orders (server_id, buyer_uuid, buyer_name, item_id, item_name, amount, total, created_at, expires_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(server.id).bind(who.uuid).bind(who.name).bind(&item_id).bind(&name).bind(p.amount).bind(total).bind(now()).bind(at(cfg.expire_hours as i64 * 60))
    .fetch_one(&mut *tx).await?;
    finish_operation(tx, server.id, operation, json!({
        "ok": true, "id": id, "balance": balance, "message": format!("Buy order #{id} posted: {}x {name} for ${total:.2}. The money is held until it is filled.", p.amount),
    })).await
}

pub async fn create(auth: AuthUser, State(state): State<AppState>, Path(sid): Path<i64>, Json(p): Json<CreateBody>) -> AppResult<Json<Value>> {
    let server = server_of(&state, sid).await?;
    create_core(&state, &server, &Actor { uuid: &auth.uuid, name: &auth.username }, p, &op()).await
}

#[derive(Deserialize)]
pub struct GameCreate {
    pub operation_id: String,
    pub uuid: String,
    pub name: String,
    #[serde(flatten)]
    pub order: CreateBody,
}

pub async fn server_create(GameServer(server): GameServer, State(state): State<AppState>, Json(p): Json<GameCreate>) -> AppResult<Json<Value>> {
    create_core(&state, &server, &Actor { uuid: &p.uuid, name: &p.name }, p.order, &p.operation_id).await
}

/// Close an order that is still open or reserved and return its escrow. `by` is the buyer cancelling, or `None` for expiry.
async fn close(tx: &mut Tx, server: &ServerRow, o: &Order, status: &str, why: &str) -> AppResult<f64> {
    let changed = sqlx::query("UPDATE buy_orders SET status = ?, resolved_at = ? WHERE id = ? AND status IN ('open', 'claimed')").bind(status).bind(now()).bind(o.id).execute(&mut **tx).await?.rows_affected();
    if changed != 1 {
        return Err(AppError::conflict("That order is already finished."));
    }
    credit(tx, server.economy_id, server.id, &o.buyer_uuid, &o.buyer_name, o.total, ORDERS, &format!("Buy order {why}: {}x {}", o.amount, o.item_name)).await
}

async fn load(tx: &mut Tx, server: &ServerRow, id: i64) -> AppResult<Order> {
    sqlx::query_as(&format!("SELECT {COLS} FROM buy_orders WHERE id = ? AND server_id = ?")).bind(id).bind(server.id).fetch_optional(&mut **tx).await?
        .ok_or_else(|| AppError::not_found("There is no such buy order."))
}

pub(crate) async fn cancel_core(state: &AppState, server: &ServerRow, who: &Actor<'_>, id: i64, operation: &str) -> AppResult<Json<Value>> {
    let (mut tx, previous) = begin_operation(state, server.id, operation).await?;
    if let Some(previous) = previous {
        return Ok(Json(previous));
    }
    let o = load(&mut tx, server, id).await?;
    if o.buyer_uuid != who.uuid {
        return Err(AppError::forbidden("That isn't your order."));
    }
    let balance = close(&mut tx, server, &o, "cancelled", "cancelled").await?;
    let notify = o.live_claim().then(|| o.claimer_uuid.clone()).flatten();
    let response = finish_operation(tx, server.id, operation, json!({ "ok": true, "refunded": o.total, "balance": balance, "message": format!("Order #{id} cancelled. ${:.2} is back in your account.", o.total) })).await?;
    if let Some(c) = notify {
        notifications::push(&state.db, &c, "order", "A buy order was cancelled", &format!("{} cancelled order #{id} ({}x {}). Your reservation is released.", o.buyer_name, o.amount, o.item_name), None).await;
    }
    Ok(response)
}

pub async fn cancel(auth: AuthUser, State(state): State<AppState>, Path((sid, id)): Path<(i64, i64)>) -> AppResult<Json<Value>> {
    let server = server_of(&state, sid).await?;
    cancel_core(&state, &server, &Actor { uuid: &auth.uuid, name: &auth.username }, id, &op()).await
}

#[derive(Deserialize)]
pub struct IdBody {
    pub operation_id: String,
    pub uuid: String,
    pub name: String,
    pub order_id: i64,
}

pub async fn server_cancel(GameServer(server): GameServer, State(state): State<AppState>, Json(p): Json<IdBody>) -> AppResult<Json<Value>> {
    cancel_core(&state, &server, &Actor { uuid: &p.uuid, name: &p.name }, p.order_id, &p.operation_id).await
}

// ---- picking an order up -------------------------------------------------------------------------------------------------

pub(crate) async fn claim_core(state: &AppState, server: &ServerRow, who: &Actor<'_>, id: i64, operation: &str) -> AppResult<Json<Value>> {
    let cfg = enabled(state).await?;
    let (mut tx, previous) = begin_operation(state, server.id, operation).await?;
    if let Some(previous) = previous {
        return Ok(Json(previous));
    }
    let o = load(&mut tx, server, id).await?;
    if o.buyer_uuid == who.uuid {
        return Err(AppError::bad_request("You can't fill your own order."));
    }
    if !matches!(o.state(), "open") && !(o.live_claim() && o.claimer_uuid.as_deref() == Some(who.uuid)) {
        return Err(AppError::conflict(if o.live_claim() { "Someone else has already picked that order up." } else { "That order is already finished." }));
    }
    let mine: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM buy_orders WHERE server_id = ? AND claimer_uuid = ? AND status = 'claimed' AND claim_until > ? AND id <> ?")
        .bind(server.id).bind(who.uuid).bind(now()).bind(id).fetch_one(&mut *tx).await?;
    if mine >= cfg.max_claims_per_player as i64 {
        return Err(AppError::conflict(format!("You can have {} orders picked up at once.", cfg.max_claims_per_player)));
    }
    let until = at(cfg.claim_minutes as i64);
    sqlx::query("UPDATE buy_orders SET status = 'claimed', claimer_uuid = ?, claimer_name = ?, claim_until = ? WHERE id = ? AND status IN ('open', 'claimed')")
        .bind(who.uuid).bind(who.name).bind(&until).bind(id).execute(&mut *tx).await?;
    let response = finish_operation(tx, server.id, operation, json!({
        "ok": true, "claim_until": until,
        "message": format!("Order #{id} is yours for {} minutes: bring {}x {} and use /orders fill {id}.", cfg.claim_minutes, o.amount, o.item_name),
    })).await?;
    notifications::push(&state.db, &o.buyer_uuid, "order", "Your buy order was picked up", &format!("{} is gathering {}x {} for order #{id}.", who.name, o.amount, o.item_name), None).await;
    Ok(response)
}

pub(crate) async fn release_core(state: &AppState, server: &ServerRow, who: &Actor<'_>, id: i64, operation: &str) -> AppResult<Json<Value>> {
    let (mut tx, previous) = begin_operation(state, server.id, operation).await?;
    if let Some(previous) = previous {
        return Ok(Json(previous));
    }
    let o = load(&mut tx, server, id).await?;
    if !(o.live_claim() && o.claimer_uuid.as_deref() == Some(who.uuid)) {
        return Err(AppError::conflict("You haven't picked that order up."));
    }
    sqlx::query("UPDATE buy_orders SET status = 'open', claimer_uuid = NULL, claimer_name = NULL, claim_until = NULL WHERE id = ? AND status = 'claimed'").bind(id).execute(&mut *tx).await?;
    finish_operation(tx, server.id, operation, json!({ "ok": true, "message": format!("Order #{id} is back on the board.") })).await
}

pub async fn claim(auth: AuthUser, State(state): State<AppState>, Path((sid, id)): Path<(i64, i64)>) -> AppResult<Json<Value>> {
    let server = server_of(&state, sid).await?;
    claim_core(&state, &server, &Actor { uuid: &auth.uuid, name: &auth.username }, id, &op()).await
}

pub async fn release(auth: AuthUser, State(state): State<AppState>, Path((sid, id)): Path<(i64, i64)>) -> AppResult<Json<Value>> {
    let server = server_of(&state, sid).await?;
    release_core(&state, &server, &Actor { uuid: &auth.uuid, name: &auth.username }, id, &op()).await
}

pub async fn server_claim(GameServer(server): GameServer, State(state): State<AppState>, Json(p): Json<IdBody>) -> AppResult<Json<Value>> {
    claim_core(&state, &server, &Actor { uuid: &p.uuid, name: &p.name }, p.order_id, &p.operation_id).await
}

pub async fn server_release(GameServer(server): GameServer, State(state): State<AppState>, Json(p): Json<IdBody>) -> AppResult<Json<Value>> {
    release_core(&state, &server, &Actor { uuid: &p.uuid, name: &p.name }, p.order_id, &p.operation_id).await
}

// ---- submitting the items ----------------------------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct FillBody {
    pub operation_id: String,
    pub uuid: String,
    pub name: String,
    pub order_id: i64,
    pub item_id: String,
    /// How many the game server took from the player's inventory.
    pub amount: i64,
}

/// Items always travel in stacks of at most this many (the vault splits anything it can't hold).
const STACK: i64 = 64;

/// The game server has already taken `amount` items from the player (and holds them in escrow). If they are the right ones and
/// enough, the order is filled: the items go to the buyer's vault, the filler is paid, and any surplus is handed back. If not,
/// the request is refused and the game server returns everything.
pub async fn server_fill(GameServer(server): GameServer, State(state): State<AppState>, Json(p): Json<FillBody>) -> AppResult<Json<Value>> {
    let cfg = enabled(&state).await?;
    let (mut tx, previous) = begin_operation(&state, server.id, &p.operation_id).await?;
    if let Some(previous) = previous {
        return Ok(Json(previous));
    }
    let o = load(&mut tx, &server, p.order_id).await?;
    if o.buyer_uuid == p.uuid {
        return Err(AppError::bad_request("You can't fill your own order."));
    }
    if o.state() != "open" && !(o.live_claim() && o.claimer_uuid.as_deref() == Some(p.uuid.as_str())) {
        return Err(AppError::conflict(if o.live_claim() { "Someone else has picked that order up." } else { "That order is already finished." }));
    }
    let item = normalize_item(&p.item_id)?;
    if !item.eq_ignore_ascii_case(&o.item_id) {
        return Err(AppError::bad_request(format!("That order wants {}, not {}.", o.item_name, pretty(&item))));
    }
    if p.amount < o.amount {
        return Err(AppError::bad_request(format!("Order #{} needs {}x {} and you brought {}.", o.id, o.amount, o.item_name, p.amount.max(0))));
    }
    let changed = sqlx::query("UPDATE buy_orders SET status = 'filled', filler_uuid = ?, filler_name = ?, resolved_at = ? WHERE id = ? AND status IN ('open', 'claimed')")
        .bind(&p.uuid).bind(&p.name).bind(now()).bind(o.id).execute(&mut *tx).await?.rows_affected();
    if changed != 1 {
        return Err(AppError::conflict("That order is already finished."));
    }
    // Deliver to the buyer's vault, in stacks.
    let mut left = o.amount;
    while left > 0 {
        let n = left.min(STACK);
        sqlx::query("INSERT INTO market_mailbox (server_id, uuid, item_id, item_name, amount, item_data, note, created_at, to_vault) VALUES (?, ?, ?, ?, ?, NULL, ?, ?, 1)")
            .bind(server.id).bind(&o.buyer_uuid).bind(&o.item_id).bind(&o.item_name).bind(n).bind(format!("Buy order #{}", o.id)).bind(now()).execute(&mut *tx).await?;
        left -= n;
    }
    // Release the escrow: the filler gets the price less the house's cut.
    let pay = round2(o.total * (1.0 - cfg.fee_percent / 100.0));
    let balance = credit(&mut tx, server.economy_id, server.id, &p.uuid, &p.name, pay, ORDERS, &format!("Buy order filled: {}x {} for {}", o.amount, o.item_name, o.buyer_name)).await?;
    let surplus = p.amount - o.amount;
    let items: Vec<Value> = if surplus > 0 { vec![json!({ "item_id": o.item_id, "item_name": o.item_name, "amount": surplus, "item_data": null })] } else { vec![] };
    let response = finish_operation(tx, server.id, &p.operation_id, json!({
        "ok": true, "balance": balance, "paid": pay, "items": items,
        "message": format!("Order #{} filled: you earned ${pay:.2} and {}x {} went to {}'s vault.", o.id, o.amount, o.item_name, o.buyer_name),
    })).await?;
    notifications::push(&state.db, &o.buyer_uuid, "order", "Your buy order was filled", &format!("{} delivered {}x {} for ${:.2}. It is waiting in your vault.", p.name, o.amount, o.item_name, o.total), None).await;
    Ok(response)
}

// ---- housekeeping ----------------------------------------------------------------------------------------------------

/// Called by the scheduler: orders nobody filled in time come down and the escrow goes back to the buyer; lapsed reservations clear.
pub async fn settle_due(state: &AppState) -> AppResult<String> {
    let now_s = now();
    sqlx::query("UPDATE buy_orders SET status = 'open', claimer_uuid = NULL, claimer_name = NULL, claim_until = NULL WHERE status = 'claimed' AND claim_until <= ?").bind(&now_s).execute(&state.db).await?;
    let due: Vec<Order> = sqlx::query_as(&format!("SELECT {COLS} FROM buy_orders WHERE status IN ('open', 'claimed') AND expires_at <= ? LIMIT 100")).bind(&now_s).fetch_all(&state.db).await?;
    let mut expired = 0;
    for o in due {
        let server = server_of(state, o.server_id).await?;
        let operation = format!("order-expire-{}", o.id);
        let (mut tx, previous) = begin_operation(state, server.id, &operation).await?;
        if previous.is_some() {
            continue;
        }
        if close(&mut tx, &server, &o, "expired", "expired").await.is_ok() {
            expired += 1;
            let _ = finish_operation(tx, server.id, &operation, json!({ "ok": true })).await?;
            notifications::push(&state.db, &o.buyer_uuid, "order", "Your buy order expired", &format!("Nobody filled order #{} in time. ${:.2} is back in your account.", o.id, o.total), None).await;
        }
    }
    Ok(format!("{expired} buy orders expired"))
}

// ---- admin ---------------------------------------------------------------------------------------------------------------

pub async fn admin_get(_admin: crate::auth::AdminUser, State(state): State<AppState>) -> AppResult<Json<Value>> {
    let open: Vec<(i64, i64, String, String, i64, f64, String, String)> = sqlx::query_as(
        "SELECT id, server_id, buyer_name, item_name, amount, total, status, created_at FROM buy_orders ORDER BY (status IN ('open','claimed')) DESC, id DESC LIMIT 100",
    )
    .fetch_all(&state.db).await?;
    Ok(Json(json!({
        "config": config(&state).await?, "defaults": Config::default(),
        "orders": open.into_iter().map(|(id, server, buyer, item, amount, total, status, at)| json!({ "id": id, "server_id": server, "buyer": buyer, "item": item, "amount": amount, "total": total, "status": status, "at": at })).collect::<Vec<_>>(),
    })))
}

pub async fn admin_save(_admin: crate::auth::AdminUser, State(state): State<AppState>, Json(mut c): Json<Config>) -> AppResult<Json<Value>> {
    c.sanitize();
    crate::store::kv_set(&state, "orders", &c).await?;
    Ok(Json(json!({ "config": c })))
}

/// Take any open order down and give the escrow back to its buyer.
pub async fn admin_cancel(_admin: crate::auth::AdminUser, State(state): State<AppState>, Path(id): Path<i64>) -> AppResult<Json<Value>> {
    let sid: i64 = sqlx::query_scalar("SELECT server_id FROM buy_orders WHERE id = ?").bind(id).fetch_optional(&state.db).await?.ok_or_else(|| AppError::not_found("There is no such buy order."))?;
    let server = server_of(&state, sid).await?;
    let operation = format!("order-admin-cancel-{id}");
    let (mut tx, previous) = begin_operation(&state, server.id, &operation).await?;
    if let Some(previous) = previous {
        return Ok(Json(previous));
    }
    let o = load(&mut tx, &server, id).await?;
    close(&mut tx, &server, &o, "cancelled", "removed by an admin").await?;
    let response = finish_operation(tx, server.id, &operation, json!({ "ok": true })).await?;
    notifications::push(&state.db, &o.buyer_uuid, "order", "Your buy order was removed", &format!("An admin removed order #{id}. ${:.2} is back in your account.", o.total), None).await;
    Ok(response)
}
