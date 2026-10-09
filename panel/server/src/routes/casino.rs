//! The Casino in the launcher: Slots, Wheel, Plinko, Mines, the daily free spin, bounties on players' heads and bets on what
//! players will do. Everything is paid from and to the Velora economy, so winnings show up in game right away.
//!
//! Every money-moving request runs in one transaction that starts with an idempotency row (like the market does), so a bet is never
//! half taken: either the bet, the result and the payout are all saved, or nothing is.

use crate::state::RequestState as State;
use crate::auth::AuthUser;
use crate::casino::{self, round2, Config};
use crate::error::{AppError, AppResult};
use crate::routes::economy::{begin_operation, ensure_balance, finish_operation};
use crate::routes::notifications;
use crate::routes::servers::{economy_scope, get_server, ServerRow};
use crate::state::AppState;
use axum::extract::{Path, Query};
use axum::Json;
use chrono::{Duration, SecondsFormat, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::{Sqlite, SqliteConnection, Transaction};

mod games;
pub use games::*;

type Tx = Transaction<'static, Sqlite>;

// ---- settings and servers ------------------------------------------------------------------------------------------

pub(crate) async fn config(state: &AppState) -> AppResult<Config> {
    let mut c: Config = crate::store::kv_get(state, "casino").await?;
    c.sanitize();
    Ok(c)
}

async fn config_conn(conn: &mut SqliteConnection) -> Config {
    let raw: Option<String> = sqlx::query_scalar("SELECT value FROM kv WHERE key = 'casino'").fetch_optional(&mut *conn).await.ok().flatten();
    let mut c: Config = raw.and_then(|r| serde_json::from_str(&r).ok()).unwrap_or_default();
    c.sanitize();
    c
}

struct Ctx {
    server: ServerRow,
    cfg: Config,
}

async fn ctx(state: &AppState, id: i64) -> AppResult<Ctx> {
    let mut server = get_server(state, id).await?;
    server.economy_id = economy_scope(&state.db, server.id).await?;
    Ok(Ctx { server, cfg: config(state).await? })
}

async fn ctx_open(state: &AppState, id: i64) -> AppResult<Ctx> {
    let c = ctx(state, id).await?;
    if !c.cfg.enabled {
        return Err(AppError::forbidden("The casino is closed right now."));
    }
    Ok(c)
}

fn at(minutes: i64) -> String {
    (Utc::now() + Duration::minutes(minutes)).to_rfc3339_opts(SecondsFormat::Secs, true)
}
fn today() -> String {
    Utc::now().format("%Y-%m-%d").to_string()
}
fn tomorrow_start() -> String {
    (Utc::now().date_naive() + Duration::days(1)).and_hms_opt(0, 0, 0).map(|t| t.and_utc().to_rfc3339_opts(SecondsFormat::Secs, true)).unwrap_or_default()
}
fn day_start() -> String {
    Utc::now().date_naive().and_hms_opt(0, 0, 0).map(|t| t.and_utc().to_rfc3339_opts(SecondsFormat::Secs, true)).unwrap_or_default()
}

// ---- the wallet ----------------------------------------------------------------------------------------------------

/// Whose money, on which economy.
#[derive(Clone, Copy)]
struct Who<'a> {
    economy: i64,
    server: i64,
    uuid: &'a str,
    name: &'a str,
}

async fn log_tx(conn: &mut SqliteConnection, server: i64, from: (&str, &str), to: (&str, &str), amount: f64, description: &str) -> AppResult<()> {
    sqlx::query("INSERT INTO economy_transactions (server_id, from_uuid, from_name, to_uuid, to_name, amount, description, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(server).bind(from.0).bind(from.1).bind(to.0).bind(to.1).bind(amount).bind(description).bind(Utc::now().to_rfc3339())
        .execute(&mut *conn).await?;
    Ok(())
}

const HOUSE: (&str, &str) = ("casino", "Casino");

/// Take money from a player. Fails (and the caller's transaction is dropped) when they cannot afford it.
async fn debit(tx: &mut Tx, w: Who<'_>, amount: f64, description: &str) -> AppResult<f64> {
    ensure_balance(tx, w.economy, w.uuid, w.name).await?;
    let balance: Option<f64> = sqlx::query_scalar("UPDATE server_economy SET balance = balance - ?1, updated_at = ?2 WHERE server_id = ?3 AND uuid = ?4 AND balance - ?1 >= 0 RETURNING balance")
        .bind(amount).bind(Utc::now().to_rfc3339()).bind(w.economy).bind(w.uuid).fetch_optional(&mut **tx).await?;
    let Some(balance) = balance else { return Err(AppError::bad_request("You can't afford that.")) };
    log_tx(&mut **tx, w.server, (w.uuid, w.name), HOUSE, amount, description).await?;
    Ok(balance)
}

async fn credit(tx: &mut Tx, w: Who<'_>, amount: f64, description: &str) -> AppResult<f64> {
    ensure_balance(tx, w.economy, w.uuid, w.name).await?;
    let balance: f64 = sqlx::query_scalar("UPDATE server_economy SET balance = balance + ?, updated_at = ? WHERE server_id = ? AND uuid = ? RETURNING balance")
        .bind(amount).bind(Utc::now().to_rfc3339()).bind(w.economy).bind(w.uuid).fetch_one(&mut **tx).await?;
    log_tx(&mut **tx, w.server, HOUSE, (w.uuid, w.name), amount, description).await?;
    Ok(balance)
}

async fn balance_of(db: &sqlx::SqlitePool, economy: i64, uuid: &str) -> Option<f64> {
    sqlx::query_scalar("SELECT balance FROM server_economy WHERE server_id = ? AND uuid = ?").bind(economy).bind(uuid).fetch_optional(db).await.ok().flatten()
}

async fn record(tx: &mut Tx, server: i64, uuid: &str, name: &str, game: &str, bet: f64, payout: f64, detail: &Value) -> AppResult<()> {
    sqlx::query("INSERT INTO casino_rounds (server_id, uuid, name, game, bet, payout, detail, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(server).bind(uuid).bind(name).bind(game).bind(bet).bind(payout).bind(detail.to_string()).bind(crate::db::now())
        .execute(&mut **tx).await?;
    Ok(())
}

fn op() -> String {
    format!("casino-{}", uuid::Uuid::new_v4())
}

fn check_bet(bet: f64, min: f64, max: f64) -> AppResult<f64> {
    if !bet.is_finite() {
        return Err(AppError::bad_request("Enter a bet."));
    }
    let bet = round2(bet);
    if bet < min {
        return Err(AppError::bad_request(format!("The smallest bet is ${min:.2}.")));
    }
    if bet > max {
        return Err(AppError::bad_request(format!("The biggest bet is ${max:.2}.")));
    }
    Ok(bet)
}

async fn loss_limit_reached(tx: &mut Tx, cfg: &Config, uuid: &str) -> AppResult<bool> {
    if cfg.daily_loss_limit <= 0.0 {
        return Ok(false);
    }
    let lost: f64 = sqlx::query_scalar("SELECT COALESCE(SUM(bet - payout), 0.0) FROM casino_rounds WHERE uuid = ? AND created_at >= ? AND bet > 0")
        .bind(uuid).bind(day_start()).fetch_one(&mut **tx).await?;
    Ok(lost >= cfg.daily_loss_limit)
}

const LOSS_LIMIT_MESSAGE: &str = "You've reached today's loss limit. Take a break, the casino opens again tomorrow.";

// ---- one round of an instant game ----------------------------------------------------------------------------------

struct Played {
    multiplier: f64,
    detail: Value,
}

/// Debit the bet, run `game`, pay out, and save the round. `limits` says whether the game is on and its bet range.
async fn play(
    state: &AppState,
    auth: &AuthUser,
    server_id: i64,
    game: &'static str,
    title: &str,
    bet: f64,
    chaos: bool,
    game_fn: impl FnOnce(&Config, f64) -> AppResult<Played>,
) -> AppResult<Json<Value>> {
    let c = ctx_open(state, server_id).await?;
    let (enabled, min, max) = match game {
        "slots" => (c.cfg.slots.enabled, c.cfg.slots.min_bet, c.cfg.slots.max_bet),
        "wheel" => (c.cfg.wheel.enabled, c.cfg.wheel.min_bet, c.cfg.wheel.max_bet),
        "dice" => (c.cfg.dice.enabled, c.cfg.dice.min_bet, c.cfg.dice.max_bet),
        "coinflip" => (c.cfg.coinflip.enabled, c.cfg.coinflip.min_bet, c.cfg.coinflip.max_bet),
        _ => (c.cfg.plinko.enabled, c.cfg.plinko.min_bet, c.cfg.plinko.max_bet),
    };
    if !enabled {
        return Err(AppError::forbidden(format!("{title} is switched off.")));
    }
    let bet = check_bet(bet, min, max)?;
    let operation = op();
    let (mut tx, _) = begin_operation(state, c.server.id, &operation).await?;
    if loss_limit_reached(&mut tx, &c.cfg, &auth.uuid).await? {
        return Err(AppError::forbidden(LOSS_LIMIT_MESSAGE));
    }
    let who = Who { economy: c.server.economy_id, server: c.server.id, uuid: &auth.uuid, name: &auth.username };
    let after_bet = debit(&mut tx, who, bet, &format!("Casino: {title} bet")).await?;
    let mut played = game_fn(&c.cfg, bet)?;
    // Chaos mode (opt-in per round): a winning round can be boosted by a lucky surge or halved by a curse.
    let twist = if chaos && played.multiplier > 0.0 { casino::chaos_roll(&c.cfg.chaos, &mut rand::thread_rng()) } else { casino::Twist::None };
    let table_multiplier = played.multiplier;
    match twist {
        casino::Twist::Surge(x) => {
            played.multiplier = round2(played.multiplier * x);
            played.detail["twist"] = json!({ "kind": "surge", "x": x, "table": table_multiplier });
        }
        casino::Twist::Curse => {
            played.multiplier = round2(played.multiplier * 0.5);
            played.detail["twist"] = json!({ "kind": "curse", "x": 0.5, "table": table_multiplier });
        }
        casino::Twist::None => {}
    }
    let payout = round2((bet * played.multiplier).min(c.cfg.max_payout));
    let balance = if payout > 0.0 { credit(&mut tx, who, payout, &format!("Casino: {title} win")).await? } else { after_bet };
    record(&mut tx, c.server.id, &auth.uuid, &auth.username, game, bet, payout, &played.detail).await?;
    let double = if payout > bet { offer_double(&mut tx, &c.cfg, c.server.id, &auth.uuid, payout, 0).await? } else { None };
    finish_operation(
        tx,
        c.server.id,
        &operation,
        json!({ "game": game, "bet": bet, "multiplier": played.multiplier, "payout": payout, "profit": round2(payout - bet), "balance": balance, "result": played.detail, "double": double }),
    )
    .await
}

#[derive(Deserialize)]
pub struct BetBody {
    pub bet: f64,
    /// Chaos mode: the player switched it on for this round (and the admin allows it), so a win can be surged or cursed.
    #[serde(default)]
    pub chaos: bool,
}

pub async fn slots(auth: AuthUser, State(state): State<AppState>, Path(sid): Path<i64>, Json(p): Json<BetBody>) -> AppResult<Json<Value>> {
    play(&state, &auth, sid, "slots", "Slots", p.bet, p.chaos, |cfg, _| {
        let spin = casino::slots_spin(&cfg.slots, &mut rand::thread_rng());
        let pair = spin.multiplier > 0.0 && !(spin.reels[0] == spin.reels[1] && spin.reels[1] == spin.reels[2]);
        Ok(Played { multiplier: spin.multiplier, detail: json!({ "reels": spin.reels, "pair": pair }) })
    })
    .await
}

pub async fn wheel(auth: AuthUser, State(state): State<AppState>, Path(sid): Path<i64>, Json(p): Json<BetBody>) -> AppResult<Json<Value>> {
    play(&state, &auth, sid, "wheel", "Wheel", p.bet, p.chaos, |cfg, _| {
        let weights: Vec<f64> = cfg.wheel.segments.iter().map(|s| s.weight).collect();
        let index = casino::pick_weighted(&weights, &mut rand::thread_rng());
        Ok(Played { multiplier: cfg.wheel.segments[index].value, detail: json!({ "segment": index }) })
    })
    .await
}

#[derive(Deserialize)]
pub struct PlinkoBody {
    pub bet: f64,
    pub rows: u32,
    pub risk: String,
    #[serde(default)]
    pub chaos: bool,
}

pub async fn plinko(auth: AuthUser, State(state): State<AppState>, Path(sid): Path<i64>, Json(p): Json<PlinkoBody>) -> AppResult<Json<Value>> {
    let (rows, risk) = (p.rows, p.risk.clone());
    play(&state, &auth, sid, "plinko", "Plinko", p.bet, p.chaos, move |cfg, _| {
        if rows < cfg.plinko.min_rows || rows > cfg.plinko.max_rows {
            return Err(AppError::bad_request(format!("Pick between {} and {} rows.", cfg.plinko.min_rows, cfg.plinko.max_rows)));
        }
        if !cfg.plinko.risks.contains(&risk) {
            return Err(AppError::bad_request("That risk level isn't available."));
        }
        let table = casino::plinko_table(rows, &risk, cfg.plinko.rtp);
        let (path, slot) = casino::plinko_drop(rows, &mut rand::thread_rng());
        Ok(Played { multiplier: table[slot], detail: json!({ "rows": rows, "risk": risk, "path": path, "slot": slot }) })
    })
    .await
}

// ---- mines ---------------------------------------------------------------------------------------------------------

#[derive(sqlx::FromRow)]
struct MinesRow {
    id: i64,
    server_id: i64,
    bet: f64,
    size: i64,
    mines: i64,
    layout: String,
    revealed: String,
    status: String,
}

fn parse_list(raw: &str) -> Vec<u32> {
    serde_json::from_str(raw).unwrap_or_default()
}

fn mines_view(g: &MinesRow, cfg: &Config) -> Value {
    let revealed = parse_list(&g.revealed);
    let cells = (g.size * g.size) as u32;
    let safe = revealed.len() as u32;
    let now = casino::mines_multiplier(cells, g.mines as u32, safe, cfg.mines.house_edge, cfg.mines.max_multiplier);
    let next = casino::mines_multiplier(cells, g.mines as u32, safe + 1, cfg.mines.house_edge, cfg.mines.max_multiplier);
    let mut v = json!({
        "id": g.id, "bet": g.bet, "size": g.size, "mines": g.mines, "revealed": revealed, "status": g.status,
        "multiplier": now, "next_multiplier": next, "cashout": round2((g.bet * now).min(cfg.max_payout)),
    });
    if g.status != "active" {
        v["layout"] = json!(parse_list(&g.layout));
    }
    v
}

async fn active_mines(db: &sqlx::SqlitePool, uuid: &str) -> AppResult<Option<MinesRow>> {
    Ok(sqlx::query_as("SELECT id, server_id, bet, size, mines, layout, revealed, status FROM casino_mines WHERE uuid = ? AND status = 'active' ORDER BY id DESC LIMIT 1")
        .bind(uuid).fetch_optional(db).await?)
}

#[derive(Deserialize)]
pub struct MinesStart {
    pub bet: f64,
    pub mines: u32,
}

pub async fn mines_start(auth: AuthUser, State(state): State<AppState>, Path(sid): Path<i64>, Json(p): Json<MinesStart>) -> AppResult<Json<Value>> {
    let c = ctx_open(&state, sid).await?;
    let m = &c.cfg.mines;
    if !m.enabled {
        return Err(AppError::forbidden("Mines is switched off."));
    }
    if active_mines(&state.db, &auth.uuid).await?.is_some() {
        return Err(AppError::conflict("Finish your current Mines game first."));
    }
    let bet = check_bet(p.bet, m.min_bet, m.max_bet)?;
    if p.mines < m.min_mines || p.mines > m.max_mines {
        return Err(AppError::bad_request(format!("Choose between {} and {} mines.", m.min_mines, m.max_mines)));
    }
    let cells = m.size * m.size;
    let layout = casino::mines_layout(cells, p.mines, &mut rand::thread_rng());
    let operation = op();
    let (mut tx, _) = begin_operation(&state, c.server.id, &operation).await?;
    if loss_limit_reached(&mut tx, &c.cfg, &auth.uuid).await? {
        return Err(AppError::forbidden(LOSS_LIMIT_MESSAGE));
    }
    let who = Who { economy: c.server.economy_id, server: c.server.id, uuid: &auth.uuid, name: &auth.username };
    let balance = debit(&mut tx, who, bet, "Casino: Mines bet").await?;
    let id: i64 = sqlx::query_scalar("INSERT INTO casino_mines (server_id, uuid, name, bet, size, mines, layout, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?) RETURNING id")
        .bind(c.server.id).bind(&auth.uuid).bind(&auth.username).bind(bet).bind(m.size as i64).bind(p.mines as i64)
        .bind(serde_json::to_string(&layout)?).bind(crate::db::now()).fetch_one(&mut *tx).await?;
    let game = MinesRow { id, server_id: c.server.id, bet, size: m.size as i64, mines: p.mines as i64, layout: String::new(), revealed: "[]".into(), status: "active".into() };
    finish_operation(tx, c.server.id, &operation, json!({ "game": mines_view(&game, &c.cfg), "balance": balance })).await
}

#[derive(Deserialize)]
pub struct MinesReveal {
    pub tile: u32,
}

pub async fn mines_reveal(auth: AuthUser, State(state): State<AppState>, Path(sid): Path<i64>, Json(p): Json<MinesReveal>) -> AppResult<Json<Value>> {
    let c = ctx(&state, sid).await?;
    let operation = op();
    let (mut tx, _) = begin_operation(&state, c.server.id, &operation).await?;
    let mut g: MinesRow = sqlx::query_as("SELECT id, server_id, bet, size, mines, layout, revealed, status FROM casino_mines WHERE uuid = ? AND status = 'active' ORDER BY id DESC LIMIT 1")
        .bind(&auth.uuid).fetch_optional(&mut *tx).await?.ok_or_else(|| AppError::not_found("You have no Mines game running."))?;
    if g.server_id != c.server.id {
        return Err(AppError::conflict("Your Mines game is on another server."));
    }
    let cells = (g.size * g.size) as u32;
    let mut revealed = parse_list(&g.revealed);
    if p.tile >= cells || revealed.contains(&p.tile) {
        return Err(AppError::bad_request("That tile is already turned over."));
    }
    let layout = parse_list(&g.layout);
    let who = Who { economy: c.server.economy_id, server: c.server.id, uuid: &auth.uuid, name: &auth.username };
    if layout.contains(&p.tile) {
        sqlx::query("UPDATE casino_mines SET status = 'lost' WHERE id = ?").bind(g.id).execute(&mut *tx).await?;
        g.status = "lost".into();
        record(&mut tx, c.server.id, &auth.uuid, &auth.username, "mines", g.bet, 0.0, &json!({ "mines": g.mines, "revealed": revealed.len(), "hit": p.tile })).await?;
        let balance = balance_of_tx(&mut tx, who.economy, who.uuid).await;
        let mut view = mines_view(&g, &c.cfg);
        view["hit"] = json!(p.tile);
        return finish_operation(tx, c.server.id, &operation, json!({ "game": view, "balance": balance, "payout": 0.0 })).await;
    }
    revealed.push(p.tile);
    g.revealed = serde_json::to_string(&revealed)?;
    let cleared = revealed.len() as u32 == cells - g.mines as u32;
    if cleared {
        let (view, payout, balance) = cash_out(&mut tx, &c.cfg, who, &mut g, &revealed).await?;
        let double = if payout > g.bet { offer_double(&mut tx, &c.cfg, c.server.id, &auth.uuid, payout, 0).await? } else { None };
        return finish_operation(tx, c.server.id, &operation, json!({ "game": view, "balance": balance, "payout": payout, "double": double })).await;
    }
    sqlx::query("UPDATE casino_mines SET revealed = ? WHERE id = ?").bind(&g.revealed).bind(g.id).execute(&mut *tx).await?;
    let balance = balance_of_tx(&mut tx, who.economy, who.uuid).await;
    finish_operation(tx, c.server.id, &operation, json!({ "game": mines_view(&g, &c.cfg), "balance": balance })).await
}

async fn balance_of_tx(tx: &mut Tx, economy: i64, uuid: &str) -> Option<f64> {
    sqlx::query_scalar("SELECT balance FROM server_economy WHERE server_id = ? AND uuid = ?").bind(economy).bind(uuid).fetch_optional(&mut **tx).await.ok().flatten()
}

async fn cash_out(tx: &mut Tx, cfg: &Config, who: Who<'_>, g: &mut MinesRow, revealed: &[u32]) -> AppResult<(Value, f64, f64)> {
    let cells = (g.size * g.size) as u32;
    let mult = casino::mines_multiplier(cells, g.mines as u32, revealed.len() as u32, cfg.mines.house_edge, cfg.mines.max_multiplier);
    let payout = round2((g.bet * mult).min(cfg.max_payout));
    let balance = credit(tx, who, payout, "Casino: Mines cash out").await?;
    sqlx::query("UPDATE casino_mines SET status = 'cashed', revealed = ? WHERE id = ?").bind(serde_json::to_string(revealed)?).bind(g.id).execute(&mut **tx).await?;
    g.status = "cashed".into();
    g.revealed = serde_json::to_string(revealed)?;
    record(tx, who.server, who.uuid, who.name, "mines", g.bet, payout, &json!({ "mines": g.mines, "revealed": revealed.len(), "multiplier": mult })).await?;
    Ok((mines_view(g, cfg), payout, balance))
}

pub async fn mines_cashout(auth: AuthUser, State(state): State<AppState>, Path(sid): Path<i64>) -> AppResult<Json<Value>> {
    let c = ctx(&state, sid).await?;
    let operation = op();
    let (mut tx, _) = begin_operation(&state, c.server.id, &operation).await?;
    let mut g: MinesRow = sqlx::query_as("SELECT id, server_id, bet, size, mines, layout, revealed, status FROM casino_mines WHERE uuid = ? AND status = 'active' ORDER BY id DESC LIMIT 1")
        .bind(&auth.uuid).fetch_optional(&mut *tx).await?.ok_or_else(|| AppError::not_found("You have no Mines game running."))?;
    if g.server_id != c.server.id {
        return Err(AppError::conflict("Your Mines game is on another server."));
    }
    let revealed = parse_list(&g.revealed);
    if revealed.is_empty() {
        return Err(AppError::bad_request("Turn over at least one tile before cashing out."));
    }
    let who = Who { economy: c.server.economy_id, server: c.server.id, uuid: &auth.uuid, name: &auth.username };
    let (view, payout, balance) = cash_out(&mut tx, &c.cfg, who, &mut g, &revealed).await?;
    let double = if payout > g.bet { offer_double(&mut tx, &c.cfg, c.server.id, &auth.uuid, payout, 0).await? } else { None };
    finish_operation(tx, c.server.id, &operation, json!({ "game": view, "balance": balance, "payout": payout, "double": double })).await
}

// ---- the daily free spin -------------------------------------------------------------------------------------------

async fn spins_used(db: &sqlx::SqlitePool, uuid: &str) -> i64 {
    sqlx::query_scalar("SELECT spins FROM casino_free_spins WHERE uuid = ? AND day = ?").bind(uuid).bind(today()).fetch_optional(db).await.ok().flatten().unwrap_or(0)
}

pub async fn daily(auth: AuthUser, State(state): State<AppState>, Path(sid): Path<i64>) -> AppResult<Json<Value>> {
    let c = ctx_open(&state, sid).await?;
    if !c.cfg.daily.enabled || c.cfg.daily.spins_per_day == 0 {
        return Err(AppError::forbidden("The daily spin is switched off."));
    }
    let operation = op();
    let (mut tx, _) = begin_operation(&state, c.server.id, &operation).await?;
    let used: Option<i64> = sqlx::query_scalar(
        "INSERT INTO casino_free_spins (uuid, day, spins) VALUES (?1, ?2, 1) ON CONFLICT(uuid, day) DO UPDATE SET spins = spins + 1 WHERE spins < ?3 RETURNING spins",
    )
    .bind(&auth.uuid).bind(today()).bind(c.cfg.daily.spins_per_day as i64).fetch_optional(&mut *tx).await?;
    let Some(used) = used else {
        return Err(AppError::conflict("You've used today's free spins. They reset at midnight UTC."));
    };
    let weights: Vec<f64> = c.cfg.daily.segments.iter().map(|s| s.weight).collect();
    let index = casino::pick_weighted(&weights, &mut rand::thread_rng());
    let amount = round2(c.cfg.daily.segments[index].value);
    let who = Who { economy: c.server.economy_id, server: c.server.id, uuid: &auth.uuid, name: &auth.username };
    let balance = credit(&mut tx, who, amount, "Casino: daily free spin").await?;
    record(&mut tx, c.server.id, &auth.uuid, &auth.username, "daily", 0.0, amount, &json!({ "segment": index })).await?;
    finish_operation(
        tx,
        c.server.id,
        &operation,
        json!({ "segment": index, "payout": amount, "balance": balance, "left": (c.cfg.daily.spins_per_day as i64 - used).max(0), "resets_at": tomorrow_start() }),
    )
    .await
}

// ---- the lobby -----------------------------------------------------------------------------------------------------

/// Everything the Casino page needs in one call: settings, plinko tables, balance, spins left, a running Mines game and the ticker.
pub async fn lobby(auth: AuthUser, State(app): State<AppState>, Path(sid): Path<i64>) -> AppResult<Json<Value>> {
    let c = ctx(&app, sid).await?;
    let cfg = &c.cfg;
    let balance = balance_of(&app.db, c.server.economy_id, &auth.uuid).await;
    let used = spins_used(&app.db, &auth.uuid).await;
    let mines = match active_mines(&app.db, &auth.uuid).await? {
        Some(g) if g.server_id == c.server.id => Some(mines_view(&g, cfg)),
        _ => None,
    };
    let crash = games::active_crash_view(&app.db, &auth.uuid, c.server.id, cfg).await?;
    let blackjack = games::active_blackjack_view(&app.db, &auth.uuid, c.server.id, cfg).await?;
    let double = games::open_double_view(&app.db, &auth.uuid, c.server.id, cfg).await?;
    let mut tables = serde_json::Map::new();
    for risk in &cfg.plinko.risks {
        let mut by_rows = serde_json::Map::new();
        for rows in cfg.plinko.min_rows..=cfg.plinko.max_rows {
            by_rows.insert(rows.to_string(), json!(casino::plinko_table(rows, risk, cfg.plinko.rtp)));
        }
        tables.insert(risk.clone(), Value::Object(by_rows));
    }
    let feed: Vec<(String, String, f64, f64, String)> = sqlx::query_as(
        "SELECT name, game, bet, payout, created_at FROM casino_rounds WHERE payout > bet AND payout - bet >= ? ORDER BY id DESC LIMIT 12",
    )
    .bind(cfg.feed_min_win).fetch_all(&app.db).await?;
    let lost_today: f64 = sqlx::query_scalar("SELECT COALESCE(SUM(bet - payout), 0.0) FROM casino_rounds WHERE uuid = ? AND created_at >= ? AND bet > 0")
        .bind(&auth.uuid).bind(day_start()).fetch_one(&app.db).await?;
    let my_bounty: f64 = sqlx::query_scalar("SELECT COALESCE(SUM(reward), 0.0) FROM casino_bounties WHERE server_id = ? AND target_uuid = ? AND status = 'active'")
        .bind(c.server.economy_id).bind(&auth.uuid).fetch_one(&app.db).await?;
    let (slots_rtp, slots_hit) = casino::slots_stats(&cfg.slots);
    Ok(Json(json!({
        "enabled": cfg.enabled,
        "server": { "id": c.server.id, "name": c.server.name },
        "me": { "uuid": auth.uuid, "name": auth.username, "admin": auth.is_admin() },
        "balance": balance,
        "config": cfg,
        "plinko_tables": tables,
        "rtp": { "slots": slots_rtp, "slots_hit": slots_hit, "wheel": casino::segments_expected(&cfg.wheel.segments), "plinko": cfg.plinko.rtp, "mines": 1.0 - cfg.mines.house_edge,
            "blackjack": 0.99, "crash": 1.0 - cfg.crash.house_edge, "dice": 1.0 - cfg.dice.house_edge, "coinflip": cfg.coinflip.payout / 2.0, "double": cfg.double.win_chance * 2.0, "chaos": casino::chaos_factor(&cfg.chaos) },
        "free": { "per_day": cfg.daily.spins_per_day, "used": used, "left": (cfg.daily.spins_per_day as i64 - used).max(0), "resets_at": tomorrow_start() },
        "mines": mines,
        "crash": crash,
        "blackjack": blackjack,
        "double": double,
        "lost_today": round2(lost_today.max(0.0)),
        "bounty_on_me": round2(my_bounty),
        "feed": feed.into_iter().map(|(name, game, bet, payout, at)| json!({ "name": name, "game": game, "bet": bet, "payout": payout, "at": at })).collect::<Vec<_>>(),
    })))
}

pub async fn history(auth: AuthUser, State(app): State<AppState>, Path(_sid): Path<i64>) -> AppResult<Json<Value>> {
    let rows: Vec<(i64, String, f64, f64, String, String)> = sqlx::query_as("SELECT id, game, bet, payout, detail, created_at FROM casino_rounds WHERE uuid = ? ORDER BY id DESC LIMIT 40")
        .bind(&auth.uuid).fetch_all(&app.db).await?;
    let totals: (f64, f64, i64) = sqlx::query_as("SELECT COALESCE(SUM(bet), 0.0), COALESCE(SUM(payout), 0.0), COUNT(*) FROM casino_rounds WHERE uuid = ?").bind(&auth.uuid).fetch_one(&app.db).await?;
    Ok(Json(json!({
        "rounds": rows.into_iter().map(|(id, game, bet, payout, detail, at)| json!({ "id": id, "game": game, "bet": bet, "payout": payout, "detail": serde_json::from_str::<Value>(&detail).unwrap_or(Value::Null), "at": at })).collect::<Vec<_>>(),
        "wagered": round2(totals.0), "won": round2(totals.1), "rounds_played": totals.2,
    })))
}

#[derive(Deserialize)]
pub struct PlayerQuery {
    pub q: Option<String>,
}

/// Names for the target and subject pickers: players this server has seen, those online first.
pub async fn players(_auth: AuthUser, State(app): State<AppState>, Path(sid): Path<i64>, Query(q): Query<PlayerQuery>) -> AppResult<Json<Value>> {
    let needle = format!("%{}%", q.q.unwrap_or_default().trim().replace(['%', '_'], ""));
    let rows: Vec<(String, String, bool)> = sqlx::query_as(
        "SELECT s.uuid, s.name, EXISTS(SELECT 1 FROM server_online o WHERE o.server_id = s.server_id AND o.uuid = s.uuid) AS online
         FROM player_stats s WHERE s.server_id = ? AND s.name LIKE ? ORDER BY online DESC, s.last_seen DESC LIMIT 12",
    )
    .bind(sid).bind(needle).fetch_all(&app.db).await?;
    Ok(Json(json!({ "players": rows.into_iter().map(|(uuid, name, online)| json!({ "uuid": uuid, "name": name, "online": online })).collect::<Vec<_>>() })))
}

// ---- bounties ------------------------------------------------------------------------------------------------------

pub async fn bounties(auth: AuthUser, State(app): State<AppState>, Path(sid): Path<i64>) -> AppResult<Json<Value>> {
    let c = ctx(&app, sid).await?;
    let eco = c.server.economy_id;
    let board: Vec<(String, String, f64, i64, Option<String>)> = sqlx::query_as(
        "SELECT target_uuid, MAX(target_name), SUM(reward), COUNT(*),
                GROUP_CONCAT(CASE WHEN anonymous = 0 THEN placer_name END, ', ')
         FROM casino_bounties WHERE server_id = ? AND status = 'active' GROUP BY target_uuid ORDER BY SUM(reward) DESC LIMIT 60",
    )
    .bind(eco).fetch_all(&app.db).await?;
    let mine: Vec<(i64, String, f64, f64, String, Option<String>)> = sqlx::query_as(
        "SELECT id, target_name, paid, reward, created_at, expires_at FROM casino_bounties WHERE server_id = ? AND placer_uuid = ? AND status = 'active' ORDER BY id DESC",
    )
    .bind(eco).bind(&auth.uuid).fetch_all(&app.db).await?;
    let recent: Vec<(String, String, f64, Option<String>)> = sqlx::query_as(
        "SELECT target_name, claimer_name, SUM(reward), MAX(resolved_at) FROM casino_bounties WHERE server_id = ? AND status = 'claimed'
         GROUP BY target_uuid, claimer_uuid, resolved_at ORDER BY resolved_at DESC LIMIT 12",
    )
    .bind(eco).fetch_all(&app.db).await?;
    let b = &c.cfg.bounties;
    Ok(Json(json!({
        "enabled": c.cfg.enabled && b.enabled,
        "board": board.into_iter().map(|(uuid, name, total, count, by)| json!({ "uuid": uuid, "name": name, "total": round2(total), "count": count, "by": by, "me": uuid == auth.uuid })).collect::<Vec<_>>(),
        "mine": mine.into_iter().map(|(id, target, paid, reward, at, exp)| json!({ "id": id, "target": target, "paid": paid, "reward": reward, "at": at, "expires_at": exp })).collect::<Vec<_>>(),
        "recent": recent.into_iter().map(|(target, killer, total, at)| json!({ "target": target, "killer": killer, "total": round2(total), "at": at })).collect::<Vec<_>>(),
        "rules": { "min": b.min_amount, "max": b.max_amount, "tax_percent": b.tax_percent, "expire_days": b.expire_days, "max_active": b.max_active_per_player, "allow_anonymous": b.allow_anonymous, "allow_cancel": b.allow_cancel },
    })))
}

#[derive(Deserialize)]
pub struct PlaceBounty {
    pub target: String,
    pub amount: f64,
    #[serde(default)]
    pub anonymous: bool,
}

async fn find_player(db: &sqlx::SqlitePool, server_id: i64, name: &str) -> AppResult<(String, String)> {
    sqlx::query_as("SELECT uuid, name FROM player_stats WHERE server_id = ? AND name = ? COLLATE NOCASE LIMIT 1")
        .bind(server_id).bind(name.trim()).fetch_optional(db).await?
        .ok_or_else(|| AppError::not_found(format!("{} hasn't played on this server.", name.trim())))
}

pub async fn place_bounty(auth: AuthUser, State(app): State<AppState>, Path(sid): Path<i64>, Json(p): Json<PlaceBounty>) -> AppResult<Json<Value>> {
    let c = ctx_open(&app, sid).await?;
    let b = &c.cfg.bounties;
    if !b.enabled {
        return Err(AppError::forbidden("Bounties are switched off."));
    }
    let amount = check_bet(p.amount, b.min_amount, b.max_amount).map_err(|_| AppError::bad_request(format!("A bounty must be between ${:.2} and ${:.2}.", b.min_amount, b.max_amount)))?;
    let (target_uuid, target_name) = find_player(&app.db, c.server.id, &p.target).await?;
    if target_uuid == auth.uuid {
        return Err(AppError::bad_request("You can't put a bounty on yourself."));
    }
    let operation = op();
    let (mut tx, _) = begin_operation(&app, c.server.id, &operation).await?;
    let active: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM casino_bounties WHERE server_id = ? AND placer_uuid = ? AND status = 'active'")
        .bind(c.server.economy_id).bind(&auth.uuid).fetch_one(&mut *tx).await?;
    if active >= b.max_active_per_player as i64 {
        return Err(AppError::conflict(format!("You can have {} bounties out at once.", b.max_active_per_player)));
    }
    let who = Who { economy: c.server.economy_id, server: c.server.id, uuid: &auth.uuid, name: &auth.username };
    let balance = debit(&mut tx, who, amount, &format!("Casino: bounty on {target_name}")).await?;
    let reward = round2(amount * (1.0 - b.tax_percent / 100.0));
    let expires = if b.expire_days > 0 { Some(at(b.expire_days as i64 * 1440)) } else { None };
    let anonymous = p.anonymous && b.allow_anonymous;
    sqlx::query("INSERT INTO casino_bounties (server_id, target_uuid, target_name, placer_uuid, placer_name, paid, reward, anonymous, created_at, expires_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(c.server.economy_id).bind(&target_uuid).bind(&target_name).bind(&auth.uuid).bind(&auth.username).bind(amount).bind(reward).bind(anonymous).bind(crate::db::now()).bind(expires)
        .execute(&mut *tx).await?;
    let total: f64 = sqlx::query_scalar("SELECT COALESCE(SUM(reward), 0.0) FROM casino_bounties WHERE server_id = ? AND target_uuid = ? AND status = 'active'")
        .bind(c.server.economy_id).bind(&target_uuid).fetch_one(&mut *tx).await?;
    let response = finish_operation(tx, c.server.id, &operation, json!({ "ok": true, "balance": balance, "reward": reward, "total_on_target": round2(total) })).await?;
    let who_text = if anonymous { "Someone".to_string() } else { auth.username.clone() };
    notifications::push(&app.db, &target_uuid, "bounty", "A bounty is on your head", &format!("{who_text} put ${reward:.2} on you. Total: ${total:.2}. Watch your back."), None).await;
    Ok(response)
}

pub async fn cancel_bounty(auth: AuthUser, State(app): State<AppState>, Path((sid, id)): Path<(i64, i64)>) -> AppResult<Json<Value>> {
    let c = ctx(&app, sid).await?;
    if !c.cfg.bounties.allow_cancel {
        return Err(AppError::forbidden("Bounties can't be taken back on this server."));
    }
    let operation = op();
    let (mut tx, _) = begin_operation(&app, c.server.id, &operation).await?;
    let row: Option<(f64, String)> = sqlx::query_as("SELECT reward, target_name FROM casino_bounties WHERE id = ? AND placer_uuid = ? AND server_id = ? AND status = 'active'")
        .bind(id).bind(&auth.uuid).bind(c.server.economy_id).fetch_optional(&mut *tx).await?;
    let Some((reward, target)) = row else { return Err(AppError::not_found("That bounty is already finished.")) };
    sqlx::query("UPDATE casino_bounties SET status = 'cancelled', resolved_at = ? WHERE id = ?").bind(crate::db::now()).bind(id).execute(&mut *tx).await?;
    let who = Who { economy: c.server.economy_id, server: c.server.id, uuid: &auth.uuid, name: &auth.username };
    let balance = credit(&mut tx, who, reward, &format!("Casino: bounty on {target} withdrawn")).await?;
    finish_operation(tx, c.server.id, &operation, json!({ "ok": true, "refunded": reward, "balance": balance })).await
}

/// A pending notification, sent once the transaction that caused it has committed.
pub struct Note {
    pub uuid: String,
    pub title: String,
    pub body: String,
}

/// Called while the game server's sync is applied when `killer` killed another player: pays out every bounty on `victim`.
pub async fn claim_on_kill(tx: &mut Tx, server: &ServerRow, killer_uuid: &str, killer_name: &str, victim_uuid: &str) -> AppResult<Vec<Note>> {
    let mut notes = Vec::new();
    if killer_uuid == victim_uuid {
        return Ok(notes);
    }
    let cfg = config_conn(&mut **tx).await;
    if !cfg.enabled || !cfg.bounties.enabled {
        return Ok(notes);
    }
    let now = crate::db::now();
    if cfg.bounties.claim_cooldown_minutes > 0 {
        let since = (Utc::now() - Duration::minutes(cfg.bounties.claim_cooldown_minutes as i64)).to_rfc3339_opts(SecondsFormat::Secs, true);
        let recent: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM casino_bounties WHERE claimer_uuid = ? AND target_uuid = ? AND status = 'claimed' AND resolved_at >= ?")
            .bind(killer_uuid).bind(victim_uuid).bind(since).fetch_one(&mut **tx).await?;
        if recent > 0 {
            return Ok(notes);
        }
    }
    let rows: Vec<(i64, String, f64, String)> = sqlx::query_as(
        "SELECT id, placer_uuid, reward, target_name FROM casino_bounties WHERE server_id = ? AND target_uuid = ? AND status = 'active' AND placer_uuid <> ? AND (expires_at IS NULL OR expires_at > ?)",
    )
    .bind(server.economy_id).bind(victim_uuid).bind(killer_uuid).bind(&now).fetch_all(&mut **tx).await?;
    if rows.is_empty() {
        return Ok(notes);
    }
    let total = round2(rows.iter().map(|r| r.2).sum());
    let target_name = rows[0].3.clone();
    for (id, ..) in &rows {
        sqlx::query("UPDATE casino_bounties SET status = 'claimed', claimer_uuid = ?, claimer_name = ?, resolved_at = ? WHERE id = ?")
            .bind(killer_uuid).bind(killer_name).bind(&now).bind(id).execute(&mut **tx).await?;
    }
    let who = Who { economy: server.economy_id, server: server.id, uuid: killer_uuid, name: killer_name };
    credit(tx, who, total, &format!("Casino: bounty on {target_name} collected")).await?;
    notes.push(Note { uuid: killer_uuid.into(), title: "Bounty collected".into(), body: format!("You collected ${total:.2} for taking down {target_name}.") });
    notes.push(Note { uuid: victim_uuid.into(), title: "Your bounty was claimed".into(), body: format!("{killer_name} collected the ${total:.2} bounty on you.") });
    let mut told = std::collections::HashSet::new();
    for (_, placer, ..) in &rows {
        if told.insert(placer.clone()) {
            notes.push(Note { uuid: placer.clone(), title: "Your bounty was claimed".into(), body: format!("{killer_name} took down {target_name}.") });
        }
    }
    Ok(notes)
}

pub async fn send_notes(db: &sqlx::SqlitePool, notes: Vec<Note>) {
    for n in notes {
        notifications::push(db, &n.uuid, "bounty", &n.title, &n.body, None).await;
    }
}

// ---- betting on players --------------------------------------------------------------------------------------------

fn metric_label(metric: &str) -> &'static str {
    casino::METRICS.iter().find(|(k, _)| *k == metric).map(|(_, l)| *l).unwrap_or("points")
}

/// The player's running total for a metric on one server. The column name comes from the fixed list, never from a request.
async fn stat_value(conn: &mut SqliteConnection, server_id: i64, uuid: &str, metric: &str) -> i64 {
    let Some((column, _)) = casino::METRICS.iter().find(|(k, _)| *k == metric) else { return 0 };
    sqlx::query_scalar(&format!("SELECT {column} FROM player_stats WHERE server_id = ? AND uuid = ?"))
        .bind(server_id).bind(uuid).fetch_optional(&mut *conn).await.ok().flatten().unwrap_or(0)
}

#[derive(sqlx::FromRow)]
struct MarketRow {
    id: i64,
    server_id: i64,
    creator_uuid: String,
    creator_name: String,
    subject_uuid: String,
    subject_name: String,
    metric: String,
    threshold: i64,
    baseline: i64,
    final_value: Option<i64>,
    created_at: String,
    locks_at: String,
    ends_at: String,
    status: String,
    outcome: Option<String>,
}

const MARKET_COLS: &str = "id, server_id, creator_uuid, creator_name, subject_uuid, subject_name, metric, threshold, baseline, final_value, created_at, locks_at, ends_at, status, outcome";

async fn market_json(conn: &mut SqliteConnection, m: &MarketRow, me: &str, rake: f64) -> AppResult<Value> {
    let bets: Vec<(String, f64, Option<f64>, String)> = sqlx::query_as("SELECT side, stake, payout, uuid FROM casino_bets WHERE market_id = ?").bind(m.id).fetch_all(&mut *conn).await?;
    let yes: f64 = bets.iter().filter(|b| b.0 == "yes").map(|b| b.1).sum();
    let no: f64 = bets.iter().filter(|b| b.0 == "no").map(|b| b.1).sum();
    let total = yes + no;
    let mine: Vec<Value> = bets.iter().filter(|b| b.3 == me).map(|b| json!({ "side": b.0, "stake": b.1, "payout": b.2 })).collect();
    let progress = if m.status == "open" { stat_value(&mut *conn, m.server_id, &m.subject_uuid, &m.metric).await - m.baseline } else { m.final_value.unwrap_or(m.baseline) - m.baseline };
    Ok(json!({
        "id": m.id, "subject_uuid": m.subject_uuid, "subject": m.subject_name, "creator": m.creator_name, "metric": m.metric, "metric_label": metric_label(&m.metric),
        "threshold": m.threshold, "progress": progress.max(0), "created_at": m.created_at, "locks_at": m.locks_at, "ends_at": m.ends_at, "status": m.status, "outcome": m.outcome,
        "yes_pool": round2(yes), "no_pool": round2(no), "bettors": bets.len(), "odds_yes": casino::pool_odds(yes, total, rake), "odds_no": casino::pool_odds(no, total, rake),
        "mine": mine, "mine_creator": m.creator_uuid == me,
    }))
}

pub async fn markets(auth: AuthUser, State(app): State<AppState>, Path(sid): Path<i64>) -> AppResult<Json<Value>> {
    let c = ctx(&app, sid).await?;
    let b = &c.cfg.betting;
    let mut conn = app.db.acquire().await?;
    let open: Vec<MarketRow> = sqlx::query_as(&format!("SELECT {MARKET_COLS} FROM casino_markets WHERE server_id = ? AND status = 'open' ORDER BY ends_at LIMIT 60")).bind(c.server.id).fetch_all(&mut *conn).await?;
    let done: Vec<MarketRow> = sqlx::query_as(&format!("SELECT {MARKET_COLS} FROM casino_markets WHERE server_id = ? AND status <> 'open' ORDER BY id DESC LIMIT 15")).bind(c.server.id).fetch_all(&mut *conn).await?;
    let mut open_json = Vec::new();
    for m in &open {
        open_json.push(market_json(&mut conn, m, &auth.uuid, b.rake_percent).await?);
    }
    let mut done_json = Vec::new();
    for m in &done {
        done_json.push(market_json(&mut conn, m, &auth.uuid, b.rake_percent).await?);
    }
    let my_open: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM casino_markets WHERE server_id = ? AND creator_uuid = ? AND status = 'open'").bind(c.server.id).bind(&auth.uuid).fetch_one(&mut *conn).await?;
    Ok(Json(json!({
        "enabled": c.cfg.enabled && b.enabled, "open": open_json, "done": done_json, "can_create": b.creators == "players" || auth.is_admin(), "my_open": my_open,
        "rules": {
            "metrics": b.metrics.iter().map(|m| json!({ "id": m, "label": metric_label(m) })).collect::<Vec<_>>(),
            "windows": b.windows_minutes, "lock_minutes": b.lock_minutes, "min_stake": b.min_stake, "max_stake": b.max_stake,
            "rake_percent": b.rake_percent, "max_open": b.max_open_per_player, "max_threshold": b.max_threshold, "creators": b.creators,
        },
    })))
}

#[derive(Deserialize)]
pub struct CreateMarket {
    pub subject: String,
    pub metric: String,
    pub threshold: i64,
    pub window_minutes: u32,
}

pub async fn create_market(auth: AuthUser, State(app): State<AppState>, Path(sid): Path<i64>, Json(p): Json<CreateMarket>) -> AppResult<Json<Value>> {
    let c = ctx_open(&app, sid).await?;
    let b = &c.cfg.betting;
    if !b.enabled {
        return Err(AppError::forbidden("Betting is switched off."));
    }
    if b.creators == "admins" && !auth.is_admin() {
        return Err(AppError::forbidden("Only admins open new bets on this server."));
    }
    if !b.metrics.contains(&p.metric) {
        return Err(AppError::bad_request("That can't be bet on."));
    }
    if p.threshold < 1 || p.threshold > b.max_threshold as i64 {
        return Err(AppError::bad_request(format!("Pick a target between 1 and {}.", b.max_threshold)));
    }
    if !b.windows_minutes.contains(&p.window_minutes) {
        return Err(AppError::bad_request("Pick one of the offered time limits."));
    }
    let (subject_uuid, subject_name) = find_player(&app.db, c.server.id, &p.subject).await?;
    let mut conn = app.db.acquire().await?;
    let open: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM casino_markets WHERE server_id = ? AND creator_uuid = ? AND status = 'open'").bind(c.server.id).bind(&auth.uuid).fetch_one(&mut *conn).await?;
    if open >= b.max_open_per_player as i64 && !auth.is_admin() {
        return Err(AppError::conflict(format!("You can have {} open bets at once.", b.max_open_per_player)));
    }
    let baseline = stat_value(&mut conn, c.server.id, &subject_uuid, &p.metric).await;
    let locks = at(p.window_minutes as i64 - b.lock_minutes as i64);
    let id: i64 = sqlx::query_scalar("INSERT INTO casino_markets (server_id, creator_uuid, creator_name, subject_uuid, subject_name, metric, threshold, baseline, created_at, locks_at, ends_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id")
        .bind(c.server.id).bind(&auth.uuid).bind(&auth.username).bind(&subject_uuid).bind(&subject_name).bind(&p.metric).bind(p.threshold).bind(baseline)
        .bind(crate::db::now()).bind(locks).bind(at(p.window_minutes as i64)).fetch_one(&mut *conn).await?;
    Ok(Json(json!({ "ok": true, "id": id })))
}

#[derive(Deserialize)]
pub struct PlaceBet {
    pub side: String,
    pub stake: f64,
}

pub async fn place_bet(auth: AuthUser, State(app): State<AppState>, Path((sid, id)): Path<(i64, i64)>, Json(p): Json<PlaceBet>) -> AppResult<Json<Value>> {
    let c = ctx_open(&app, sid).await?;
    let b = &c.cfg.betting;
    if !b.enabled {
        return Err(AppError::forbidden("Betting is switched off."));
    }
    if p.side != "yes" && p.side != "no" {
        return Err(AppError::bad_request("Pick yes or no."));
    }
    let stake = check_bet(p.stake, b.min_stake, b.max_stake).map_err(|_| AppError::bad_request(format!("A stake must be between ${:.2} and ${:.2}.", b.min_stake, b.max_stake)))?;
    let operation = op();
    let (mut tx, _) = begin_operation(&app, c.server.id, &operation).await?;
    let m: MarketRow = sqlx::query_as(&format!("SELECT {MARKET_COLS} FROM casino_markets WHERE id = ? AND server_id = ?")).bind(id).bind(c.server.id).fetch_optional(&mut *tx).await?
        .ok_or_else(|| AppError::not_found("That bet doesn't exist."))?;
    if m.status != "open" || crate::db::now() >= m.locks_at {
        return Err(AppError::conflict("Betting on this has closed."));
    }
    if m.subject_uuid == auth.uuid {
        return Err(AppError::forbidden("You can't bet on yourself."));
    }
    let other: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM casino_bets WHERE market_id = ? AND uuid = ? AND side <> ?").bind(id).bind(&auth.uuid).bind(&p.side).fetch_one(&mut *tx).await?;
    if other > 0 {
        return Err(AppError::conflict("You've already bet the other way on this."));
    }
    let who = Who { economy: c.server.economy_id, server: c.server.id, uuid: &auth.uuid, name: &auth.username };
    let balance = debit(&mut tx, who, stake, &format!("Casino: bet on {}", m.subject_name)).await?;
    sqlx::query("INSERT INTO casino_bets (market_id, uuid, name, side, stake, created_at) VALUES (?, ?, ?, ?, ?, ?)")
        .bind(id).bind(&auth.uuid).bind(&auth.username).bind(&p.side).bind(stake).bind(crate::db::now()).execute(&mut *tx).await?;
    finish_operation(tx, c.server.id, &operation, json!({ "ok": true, "balance": balance })).await
}

pub async fn cancel_market(auth: AuthUser, State(app): State<AppState>, Path((sid, id)): Path<(i64, i64)>) -> AppResult<Json<Value>> {
    let c = ctx(&app, sid).await?;
    let m: MarketRow = sqlx::query_as(&format!("SELECT {MARKET_COLS} FROM casino_markets WHERE id = ? AND server_id = ?")).bind(id).bind(c.server.id).fetch_optional(&app.db).await?
        .ok_or_else(|| AppError::not_found("That bet doesn't exist."))?;
    if m.creator_uuid != auth.uuid && !auth.is_admin() {
        return Err(AppError::forbidden("Only the person who opened it can cancel."));
    }
    void_market(&app, &m, c.server.economy_id, "cancelled").await?;
    Ok(Json(json!({ "ok": true })))
}

/// Cancel an open bet by id and give every stake back (admins and the person who opened it).
pub(crate) async fn void_by_id(app: &AppState, id: i64, reason: &str) -> AppResult<()> {
    let m: MarketRow = sqlx::query_as(&format!("SELECT {MARKET_COLS} FROM casino_markets WHERE id = ?")).bind(id).fetch_optional(&app.db).await?
        .ok_or_else(|| AppError::not_found("That bet doesn't exist."))?;
    let economy = economy_scope(&app.db, m.server_id).await?;
    void_market(app, &m, economy, reason).await
}

/// Close a bet and give every stake back.
async fn void_market(app: &AppState, m: &MarketRow, economy: i64, reason: &str) -> AppResult<()> {
    let operation = format!("casino-void-{}", m.id);
    let (mut tx, previous) = begin_operation(app, m.server_id, &operation).await?;
    if previous.is_some() {
        return Ok(());
    }
    let still_open: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM casino_markets WHERE id = ? AND status = 'open'").bind(m.id).fetch_one(&mut *tx).await?;
    if still_open == 0 {
        return finish_operation(tx, m.server_id, &operation, json!({ "ok": true })).await.map(|_| ());
    }
    let bets: Vec<(i64, String, String, f64)> = sqlx::query_as("SELECT id, uuid, name, stake FROM casino_bets WHERE market_id = ?").bind(m.id).fetch_all(&mut *tx).await?;
    let mut notes = Vec::new();
    for (bet_id, uuid, name, stake) in &bets {
        let who = Who { economy, server: m.server_id, uuid, name };
        credit(&mut tx, who, *stake, &format!("Casino: bet on {} refunded", m.subject_name)).await?;
        sqlx::query("UPDATE casino_bets SET payout = stake WHERE id = ?").bind(bet_id).execute(&mut *tx).await?;
        notes.push(Note { uuid: uuid.clone(), title: "Bet refunded".into(), body: format!("The bet on {} was {reason}. Your ${stake:.2} is back.", m.subject_name) });
    }
    sqlx::query("UPDATE casino_markets SET status = 'void', outcome = ?, resolved_at = ? WHERE id = ?").bind(reason).bind(crate::db::now()).bind(m.id).execute(&mut *tx).await?;
    let _ = finish_operation(tx, m.server_id, &operation, json!({ "ok": true })).await?;
    send_notes(&app.db, notes).await;
    Ok(())
}

// ---- settlement (run every 30 seconds by the scheduler) -----------------------------------------------------------

const SETTLE_GRACE_SECS: i64 = 60;

pub async fn settle_due(app: &AppState) -> AppResult<String> {
    let cfg = config(app).await?;
    let now = crate::db::now();
    let mut counts = (0, 0, 0, 0);

    // Bounties nobody collected come back to whoever put them up.
    let expired: Vec<(i64, i64, String, String, f64, String)> = sqlx::query_as(
        "SELECT id, server_id, placer_uuid, placer_name, reward, target_name FROM casino_bounties WHERE status = 'active' AND expires_at IS NOT NULL AND expires_at <= ? LIMIT 100",
    )
    .bind(&now).fetch_all(&app.db).await?;
    for (id, economy, uuid, name, reward, target) in expired {
        let operation = format!("casino-expire-{id}");
        let (mut tx, previous) = begin_operation(app, economy, &operation).await?;
        if previous.is_some() {
            continue;
        }
        let claimed = sqlx::query("UPDATE casino_bounties SET status = 'expired', resolved_at = ? WHERE id = ? AND status = 'active'").bind(&now).bind(id).execute(&mut *tx).await?.rows_affected();
        if claimed == 1 {
            let who = Who { economy, server: economy, uuid: &uuid, name: &name };
            credit(&mut tx, who, reward, &format!("Casino: bounty on {target} expired")).await?;
            counts.0 += 1;
        }
        let _ = finish_operation(tx, economy, &operation, json!({ "ok": true })).await?;
        notifications::push(&app.db, &uuid, "bounty", "Bounty expired", &format!("Nobody took down {target}. ${reward:.2} is back in your account."), None).await;
    }

    // Bets whose time is up.
    let cutoff = (Utc::now() - Duration::seconds(SETTLE_GRACE_SECS)).to_rfc3339_opts(SecondsFormat::Secs, true);
    let due: Vec<MarketRow> = sqlx::query_as(&format!("SELECT {MARKET_COLS} FROM casino_markets WHERE status = 'open' AND ends_at <= ? LIMIT 50")).bind(cutoff).fetch_all(&app.db).await?;
    for m in due {
        let economy = economy_scope(&app.db, m.server_id).await?;
        if settle_market(app, &m, economy, &cfg).await? {
            counts.1 += 1;
        }
    }

    // A game left open for a day is settled in the player's favour rather than lost: Mines pays what the tiles already turned over
    // were worth (or gives the bet back if none were), and a Blackjack hand is a push.
    let stale = (Utc::now() - Duration::hours(24)).to_rfc3339_opts(SecondsFormat::Secs, true);
    let games: Vec<(i64, i64, String, String, f64, i64, i64, String)> = sqlx::query_as(
        "SELECT id, server_id, uuid, name, bet, size, mines, revealed FROM casino_mines WHERE status = 'active' AND created_at <= ? LIMIT 100",
    )
    .bind(&stale).fetch_all(&app.db).await?;
    for (id, server_id, uuid, name, bet, size, mines, revealed) in games {
        let economy = economy_scope(&app.db, server_id).await?;
        let safe = parse_list(&revealed).len() as u32;
        let mult = if safe == 0 { 1.0 } else { casino::mines_multiplier((size * size) as u32, mines as u32, safe, cfg.mines.house_edge, cfg.mines.max_multiplier) };
        let payout = round2((bet * mult).min(cfg.max_payout));
        let mut tx = app.db.begin().await?;
        let changed = sqlx::query("UPDATE casino_mines SET status = 'cashed' WHERE id = ? AND status = 'active'").bind(id).execute(&mut *tx).await?.rows_affected();
        if changed == 1 {
            let who = Who { economy, server: server_id, uuid: &uuid, name: &name };
            credit(&mut tx, who, payout, "Casino: Mines game left open, settled").await?;
            record(&mut tx, server_id, &uuid, &name, "mines", bet, payout, &json!({ "abandoned": true, "multiplier": mult })).await?;
            counts.2 += 1;
        }
        tx.commit().await?;
    }
    let hands: Vec<(i64, i64, String, String, f64)> = sqlx::query_as("SELECT id, server_id, uuid, name, bet FROM casino_blackjack WHERE status = 'active' AND created_at <= ? LIMIT 100")
        .bind(&stale).fetch_all(&app.db).await?;
    for (id, server_id, uuid, name, bet) in hands {
        let economy = economy_scope(&app.db, server_id).await?;
        let mut tx = app.db.begin().await?;
        let changed = sqlx::query("UPDATE casino_blackjack SET status = 'done' WHERE id = ? AND status = 'active'").bind(id).execute(&mut *tx).await?.rows_affected();
        if changed == 1 {
            let who = Who { economy, server: server_id, uuid: &uuid, name: &name };
            credit(&mut tx, who, bet, "Casino: Blackjack hand left open, bet returned").await?;
            record(&mut tx, server_id, &uuid, &name, "blackjack", bet, bet, &json!({ "abandoned": true })).await?;
            counts.2 += 1;
        }
        tx.commit().await?;
    }
    // Old Double or Nothing offers are just cleared out.
    sqlx::query("DELETE FROM casino_double WHERE created_at <= ?").bind((Utc::now() - Duration::days(2)).to_rfc3339_opts(SecondsFormat::Secs, true)).execute(&app.db).await?;
    counts.3 = counts.0 + counts.1 + counts.2;
    Ok(format!("{} bounties expired, {} bets settled, {} abandoned games closed", counts.0, counts.1, counts.2))
}

async fn settle_market(app: &AppState, m: &MarketRow, economy: i64, cfg: &Config) -> AppResult<bool> {
    let operation = format!("casino-settle-{}", m.id);
    let (mut tx, previous) = begin_operation(app, m.server_id, &operation).await?;
    if previous.is_some() {
        return Ok(false);
    }
    let open: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM casino_markets WHERE id = ? AND status = 'open'").bind(m.id).fetch_one(&mut *tx).await?;
    if open == 0 {
        let _ = finish_operation(tx, m.server_id, &operation, json!({ "ok": true })).await?;
        return Ok(false);
    }
    let bets: Vec<(i64, String, String, String, f64)> = sqlx::query_as("SELECT id, uuid, name, side, stake FROM casino_bets WHERE market_id = ?").bind(m.id).fetch_all(&mut *tx).await?;
    let final_value = stat_value(&mut tx, m.server_id, &m.subject_uuid, &m.metric).await;
    let hit = final_value - m.baseline >= m.threshold;
    let winner = if hit { "yes" } else { "no" };
    let pool = |side: &str| -> f64 { bets.iter().filter(|b| b.3 == side).map(|b| b.4).sum() };
    let (win_pool, lose_pool) = (pool(winner), pool(if hit { "no" } else { "yes" }));
    let total = win_pool + lose_pool;
    let mut notes = Vec::new();
    if win_pool <= 0.0 || lose_pool <= 0.0 {
        // Nobody to pay: give every stake back.
        for (bet_id, uuid, name, _, stake) in &bets {
            credit(&mut tx, Who { economy, server: m.server_id, uuid, name }, *stake, &format!("Casino: bet on {} refunded", m.subject_name)).await?;
            sqlx::query("UPDATE casino_bets SET payout = stake WHERE id = ?").bind(bet_id).execute(&mut *tx).await?;
            notes.push(Note { uuid: uuid.clone(), title: "Bet refunded".into(), body: format!("Nobody took the other side of the bet on {}, so your ${stake:.2} is back.", m.subject_name) });
        }
        sqlx::query("UPDATE casino_markets SET status = 'void', outcome = 'no opposing bets', final_value = ?, resolved_at = ? WHERE id = ?").bind(final_value).bind(crate::db::now()).bind(m.id).execute(&mut *tx).await?;
    } else {
        for (bet_id, uuid, name, side, stake) in &bets {
            if side == winner {
                let payout = casino::pool_payout(*stake, win_pool, total, cfg.betting.rake_percent);
                credit(&mut tx, Who { economy, server: m.server_id, uuid, name }, payout, &format!("Casino: bet on {} won", m.subject_name)).await?;
                sqlx::query("UPDATE casino_bets SET payout = ? WHERE id = ?").bind(payout).bind(bet_id).execute(&mut *tx).await?;
                notes.push(Note { uuid: uuid.clone(), title: "You won a bet".into(), body: format!("{} {} {} {}: you won ${payout:.2}.", m.subject_name, if hit { "reached" } else { "didn't reach" }, m.threshold, metric_label(&m.metric)) });
            } else {
                sqlx::query("UPDATE casino_bets SET payout = 0 WHERE id = ?").bind(bet_id).execute(&mut *tx).await?;
                notes.push(Note { uuid: uuid.clone(), title: "You lost a bet".into(), body: format!("{} {} {} {}. Your ${stake:.2} went to the winners.", m.subject_name, if hit { "reached" } else { "didn't reach" }, m.threshold, metric_label(&m.metric)) });
            }
        }
        sqlx::query("UPDATE casino_markets SET status = 'settled', outcome = ?, final_value = ?, resolved_at = ? WHERE id = ?").bind(winner).bind(final_value).bind(crate::db::now()).bind(m.id).execute(&mut *tx).await?;
    }
    let _ = finish_operation(tx, m.server_id, &operation, json!({ "ok": true })).await?;
    send_notes(&app.db, notes).await;
    Ok(true)
}
