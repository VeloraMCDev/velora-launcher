//! The newer casino games: Dice, Coin Flip, Crash and Blackjack, plus Double or Nothing, the gamble offered after any win.
//!
//! Dice and Coin Flip are instant rounds and go through the same `play` path as Slots, so they share the bet, payout, chaos
//! twist and loss-limit rules. Crash and Blackjack are live rounds saved between requests, like Mines: the bet is taken when the
//! round starts and the round is settled in one transaction, so a bet is never taken without its result being recorded.

use crate::state::RequestState as State;
use super::{balance_of_tx, check_bet, credit, debit, ctx, ctx_open, op, play, record, loss_limit_reached, Played, Tx, Who, LOSS_LIMIT_MESSAGE};
use crate::auth::AuthUser;
use crate::casino::{self, round2, Config};
use crate::error::{AppError, AppResult};
use crate::routes::economy::{begin_operation, finish_operation};
use crate::state::AppState;
use axum::extract::{Path};
use axum::Json;
use chrono::{Duration, SecondsFormat, Utc};
use rand::Rng;
use serde::Deserialize;
use serde_json::{json, Value};

fn now_ms() -> i64 {
    Utc::now().timestamp_millis()
}

// ---- dice ----------------------------------------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct DiceBody {
    pub bet: f64,
    /// Win chance as a percentage, chosen by the player.
    pub chance: f64,
    /// `under` wins when the roll lands below the chance, `over` when it lands in the top slice of the same size.
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub chaos: bool,
}

pub async fn dice(auth: AuthUser, State(state): State<AppState>, Path(sid): Path<i64>, Json(p): Json<DiceBody>) -> AppResult<Json<Value>> {
    let over = p.mode == "over";
    let chance = p.chance;
    play(&state, &auth, sid, "dice", "Dice", p.bet, p.chaos, move |cfg, _| {
        let d = &cfg.dice;
        if !chance.is_finite() || chance < d.min_chance || chance > d.max_chance {
            return Err(AppError::bad_request(format!("Pick a win chance between {}% and {}%.", d.min_chance, d.max_chance)));
        }
        let chance = round2(chance);
        let roll = casino::dice_roll(&mut rand::thread_rng());
        let win = casino::dice_wins(roll, chance, over);
        let multiplier = if win { casino::dice_multiplier(chance, d.house_edge) } else { 0.0 };
        Ok(Played { multiplier, detail: json!({ "roll": roll, "chance": chance, "over": over, "win": win, "target": if over { round2(100.0 - chance) } else { chance } }) })
    })
    .await
}

// ---- coin flip -----------------------------------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct CoinBody {
    pub bet: f64,
    pub side: String,
    #[serde(default)]
    pub chaos: bool,
}

pub async fn coinflip(auth: AuthUser, State(state): State<AppState>, Path(sid): Path<i64>, Json(p): Json<CoinBody>) -> AppResult<Json<Value>> {
    let call = p.side.clone();
    play(&state, &auth, sid, "coinflip", "Coin Flip", p.bet, p.chaos, move |cfg, _| {
        if call != "heads" && call != "tails" {
            return Err(AppError::bad_request("Call heads or tails."));
        }
        let landed = if rand::thread_rng().gen_bool(0.5) { "heads" } else { "tails" };
        let win = landed == call;
        Ok(Played { multiplier: if win { cfg.coinflip.payout } else { 0.0 }, detail: json!({ "call": call, "landed": landed, "win": win }) })
    })
    .await
}

// ---- double or nothing ---------------------------------------------------------------------------------------------

/// After a win, offer to gamble the winnings: win and they double, lose and they are gone. The winnings are already in the
/// player's balance, so an unused offer costs nothing; it simply expires. A fresh offer replaces any older one.
pub(super) async fn offer_double(tx: &mut Tx, cfg: &Config, server: i64, uuid: &str, stake: f64, streak: i64) -> AppResult<Option<Value>> {
    sqlx::query("UPDATE casino_double SET status = 'expired' WHERE uuid = ? AND status = 'open'").bind(uuid).execute(&mut **tx).await?;
    if !cfg.double.enabled || streak >= cfg.double.max_streak as i64 || stake <= 0.0 || stake >= cfg.max_payout {
        return Ok(None);
    }
    let expires = (Utc::now() + Duration::minutes(cfg.double.offer_minutes as i64)).to_rfc3339_opts(SecondsFormat::Secs, true);
    let id: i64 = sqlx::query_scalar("INSERT INTO casino_double (server_id, uuid, stake, streak, expires_at, created_at) VALUES (?, ?, ?, ?, ?, ?) RETURNING id")
        .bind(server).bind(uuid).bind(stake).bind(streak).bind(&expires).bind(crate::db::now()).fetch_one(&mut **tx).await?;
    Ok(Some(double_view(cfg, id, stake, streak, &expires)))
}

fn double_view(cfg: &Config, id: i64, stake: f64, streak: i64, expires: &str) -> Value {
    json!({
        "id": id, "stake": stake, "streak": streak, "win_chance": cfg.double.win_chance,
        "payout": round2((stake * 2.0).min(cfg.max_payout)), "expires_at": expires, "max_streak": cfg.double.max_streak,
    })
}

pub(super) async fn open_double_view(db: &sqlx::SqlitePool, uuid: &str, server: i64, cfg: &Config) -> AppResult<Option<Value>> {
    let row: Option<(i64, f64, i64, String)> = sqlx::query_as("SELECT id, stake, streak, expires_at FROM casino_double WHERE uuid = ? AND server_id = ? AND status = 'open' AND expires_at > ? ORDER BY id DESC LIMIT 1")
        .bind(uuid).bind(server).bind(crate::db::now()).fetch_optional(db).await?;
    Ok(row.filter(|_| cfg.double.enabled).map(|(id, stake, streak, exp)| double_view(cfg, id, stake, streak, &exp)))
}

#[derive(Deserialize)]
pub struct DoubleBody {
    pub id: i64,
}

pub async fn double(auth: AuthUser, State(state): State<AppState>, Path(sid): Path<i64>, Json(p): Json<DoubleBody>) -> AppResult<Json<Value>> {
    let c = ctx_open(&state, sid).await?;
    if !c.cfg.double.enabled {
        return Err(AppError::forbidden("Double or Nothing is switched off."));
    }
    let operation = op();
    let (mut tx, _) = begin_operation(&state, c.server.id, &operation).await?;
    if loss_limit_reached(&mut tx, &c.cfg, &auth.uuid).await? {
        return Err(AppError::forbidden(LOSS_LIMIT_MESSAGE));
    }
    // Taking the offer and marking it used is one statement, so the same offer can never be played twice.
    let taken: Option<(f64, i64)> = sqlx::query_as("UPDATE casino_double SET status = 'taken' WHERE id = ? AND uuid = ? AND server_id = ? AND status = 'open' AND expires_at > ? RETURNING stake, streak")
        .bind(p.id).bind(&auth.uuid).bind(c.server.id).bind(crate::db::now()).fetch_optional(&mut *tx).await?;
    let Some((stake, streak)) = taken else {
        return Err(AppError::conflict("That offer has expired. Win another round to get a new one."));
    };
    let who = Who { economy: c.server.economy_id, server: c.server.id, uuid: &auth.uuid, name: &auth.username };
    let after_stake = debit(&mut tx, who, stake, "Casino: Double or Nothing stake").await?;
    let won = rand::thread_rng().gen::<f64>() < c.cfg.double.win_chance;
    let payout = if won { round2((stake * 2.0).min(c.cfg.max_payout)) } else { 0.0 };
    let balance = if won { credit(&mut tx, who, payout, "Casino: Double or Nothing win").await? } else { after_stake };
    record(&mut tx, c.server.id, &auth.uuid, &auth.username, "double", stake, payout, &json!({ "won": won, "streak": streak + 1 })).await?;
    let next = if won { offer_double(&mut tx, &c.cfg, c.server.id, &auth.uuid, payout, streak + 1).await? } else { None };
    finish_operation(
        tx,
        c.server.id,
        &operation,
        json!({ "won": won, "stake": stake, "payout": payout, "profit": round2(payout - stake), "streak": streak + 1, "balance": balance, "double": next }),
    )
    .await
}

// ---- crash ---------------------------------------------------------------------------------------------------------

#[derive(sqlx::FromRow, Clone)]
struct CrashRow {
    id: i64,
    server_id: i64,
    bet: f64,
    crash_point: f64,
    auto_cashout: Option<f64>,
    started_ms: i64,
    status: String,
    /// When the page last asked about this round. A player who has gone away is cashed out at the multiplier they last saw.
    last_poll_ms: Option<i64>,
}

const CRASH_COLS: &str = "id, server_id, bet, crash_point, auto_cashout, started_ms, status, last_poll_ms";

/// How long a Crash round can go unwatched before it is cashed out for the player (the page polls several times a second).
const ABANDON_MS: i64 = 20_000;

/// What the round has come to by `now`: still climbing, crashed, or reached the player's auto cash-out first.
enum CrashState {
    Running(f64),
    Busted,
    Auto(f64),
    /// The player left: cashed out at the multiplier at their last visit rather than losing the bet to a crash they never saw.
    Away(f64),
}

fn crash_state(g: &CrashRow, cfg: &Config, now: i64) -> CrashState {
    let m = casino::crash_curve(now - g.started_ms, cfg.crash.max_multiplier);
    if let Some(seen) = g.last_poll_ms.filter(|seen| now - seen > ABANDON_MS) {
        let at_last_visit = casino::crash_curve(seen - g.started_ms, cfg.crash.max_multiplier);
        // The page would have settled a crash or an auto cash-out on that visit, so what is left is a round still climbing.
        if at_last_visit < g.crash_point && g.auto_cashout.map_or(true, |a| a > at_last_visit) {
            return CrashState::Away(at_last_visit);
        }
    }
    match g.auto_cashout {
        Some(a) if a <= m && a < g.crash_point => CrashState::Auto(a),
        _ if m >= g.crash_point => CrashState::Busted,
        _ => CrashState::Running(m),
    }
}

/// The player's view of a round. While it runs, the crash point stays secret.
fn crash_view(g: &CrashRow, cfg: &Config, now: i64, payout: Option<f64>) -> Value {
    let elapsed = (now - g.started_ms).max(0);
    let mut v = json!({
        "id": g.id, "bet": g.bet, "status": g.status, "auto": g.auto_cashout, "elapsed_ms": elapsed,
        "multiplier": casino::crash_curve(elapsed, cfg.crash.max_multiplier), "rate": casino::CRASH_RATE,
    });
    if g.status != "active" {
        v["crash_point"] = json!(g.crash_point);
        v["payout"] = json!(payout.unwrap_or(0.0));
    }
    v
}

pub(super) async fn active_crash_view(db: &sqlx::SqlitePool, uuid: &str, server: i64, cfg: &Config) -> AppResult<Option<Value>> {
    let row: Option<CrashRow> = sqlx::query_as(&format!("SELECT {CRASH_COLS} FROM casino_crash WHERE uuid = ? AND status = 'active' ORDER BY id DESC LIMIT 1")).bind(uuid).fetch_optional(db).await?;
    Ok(row.filter(|g| g.server_id == server).map(|g| crash_view(&g, cfg, now_ms(), None)))
}

/// Settle a round that has crashed or reached its auto cash-out. Returns the payout and the new balance.
async fn crash_settle(tx: &mut Tx, cfg: &Config, who: Who<'_>, g: &mut CrashRow, outcome: &CrashState, now: i64) -> AppResult<(f64, f64)> {
    match outcome {
        CrashState::Busted => {
            sqlx::query("UPDATE casino_crash SET status = 'busted' WHERE id = ?").bind(g.id).execute(&mut **tx).await?;
            g.status = "busted".into();
            record(tx, who.server, who.uuid, who.name, "crash", g.bet, 0.0, &json!({ "crash_point": g.crash_point, "auto": g.auto_cashout })).await?;
            Ok((0.0, balance_of_tx(tx, who.economy, who.uuid).await.unwrap_or(0.0)))
        }
        CrashState::Auto(m) | CrashState::Running(m) | CrashState::Away(m) => {
            let m = round2(*m).max(1.0);
            let payout = round2((g.bet * m).min(cfg.max_payout));
            let balance = credit(tx, who, payout, "Casino: Crash cash out").await?;
            sqlx::query("UPDATE casino_crash SET status = 'cashed' WHERE id = ?").bind(g.id).execute(&mut **tx).await?;
            g.status = "cashed".into();
            record(tx, who.server, who.uuid, who.name, "crash", g.bet, payout, &json!({ "multiplier": m, "crash_point": g.crash_point, "auto": g.auto_cashout, "elapsed_ms": now - g.started_ms })).await?;
            Ok((payout, balance))
        }
    }
}

#[derive(Deserialize)]
pub struct CrashStart {
    pub bet: f64,
    /// Optional: cash out automatically at this multiplier.
    #[serde(default)]
    pub auto: Option<f64>,
}

pub async fn crash_start(auth: AuthUser, State(state): State<AppState>, Path(sid): Path<i64>, Json(p): Json<CrashStart>) -> AppResult<Json<Value>> {
    let c = ctx_open(&state, sid).await?;
    let k = &c.cfg.crash;
    if !k.enabled {
        return Err(AppError::forbidden("Crash is switched off."));
    }
    let bet = check_bet(p.bet, k.min_bet, k.max_bet)?;
    let auto = match p.auto {
        Some(a) if a.is_finite() => {
            if a < 1.01 || a > k.max_multiplier {
                return Err(AppError::bad_request(format!("Auto cash-out must be between 1.01× and {}×.", k.max_multiplier)));
            }
            Some(round2(a))
        }
        _ => None,
    };
    let operation = op();
    let (mut tx, _) = begin_operation(&state, c.server.id, &operation).await?;
    if loss_limit_reached(&mut tx, &c.cfg, &auth.uuid).await? {
        return Err(AppError::forbidden(LOSS_LIMIT_MESSAGE));
    }
    let who = Who { economy: c.server.economy_id, server: c.server.id, uuid: &auth.uuid, name: &auth.username };
    // A round the player walked away from may already be over: settle it, then carry on.
    let open: Option<CrashRow> = sqlx::query_as(&format!("SELECT {CRASH_COLS} FROM casino_crash WHERE uuid = ? AND status = 'active' ORDER BY id DESC LIMIT 1")).bind(&auth.uuid).fetch_optional(&mut *tx).await?;
    if let Some(mut old) = open {
        let now = now_ms();
        match crash_state(&old, &c.cfg, now) {
            CrashState::Running(_) => return Err(AppError::conflict("Finish your current Crash round first.")),
            done => {
                let old_who = Who { economy: c.server.economy_id, server: old.server_id, uuid: &auth.uuid, name: &auth.username };
                crash_settle(&mut tx, &c.cfg, old_who, &mut old, &done, now).await?;
            }
        }
    }
    let balance = debit(&mut tx, who, bet, "Casino: Crash bet").await?;
    let point = casino::crash_point(k.house_edge, k.max_multiplier, &mut rand::thread_rng());
    let started = now_ms();
    let id: i64 = sqlx::query_scalar("INSERT INTO casino_crash (server_id, uuid, name, bet, crash_point, auto_cashout, started_ms, created_at, last_poll_ms) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id")
        .bind(c.server.id).bind(&auth.uuid).bind(&auth.username).bind(bet).bind(point).bind(auto).bind(started).bind(crate::db::now()).bind(started).fetch_one(&mut *tx).await?;
    let g = CrashRow { id, server_id: c.server.id, bet, crash_point: point, auto_cashout: auto, started_ms: started, status: "active".into(), last_poll_ms: Some(started) };
    finish_operation(tx, c.server.id, &operation, json!({ "game": crash_view(&g, &c.cfg, started, None), "balance": balance })).await
}

async fn load_crash(tx: &mut Tx, uuid: &str) -> AppResult<CrashRow> {
    sqlx::query_as(&format!("SELECT {CRASH_COLS} FROM casino_crash WHERE uuid = ? AND status = 'active' ORDER BY id DESC LIMIT 1"))
        .bind(uuid).fetch_optional(&mut **tx).await?.ok_or_else(|| AppError::not_found("You have no Crash round running."))
}

/// Polled by the page while a round climbs. Cheap while nothing has happened; settles the round the moment it crashes
/// or hits the auto cash-out.
pub async fn crash_status(auth: AuthUser, State(state): State<AppState>, Path(sid): Path<i64>) -> AppResult<Json<Value>> {
    let c = ctx(&state, sid).await?;
    let now = now_ms();
    let row: Option<CrashRow> = sqlx::query_as(&format!("SELECT {CRASH_COLS} FROM casino_crash WHERE uuid = ? AND status = 'active' ORDER BY id DESC LIMIT 1")).bind(&auth.uuid).fetch_optional(&state.db).await?;
    let Some(g) = row.filter(|g| g.server_id == c.server.id) else { return Ok(Json(json!({ "game": null }))) };
    if matches!(crash_state(&g, &c.cfg, now), CrashState::Running(_)) {
        // Still climbing: note that someone is watching.
        sqlx::query("UPDATE casino_crash SET last_poll_ms = ? WHERE id = ? AND status = 'active'").bind(now).bind(g.id).execute(&state.db).await?;
        return Ok(Json(json!({ "game": crash_view(&g, &c.cfg, now, None) })));
    }
    let operation = op();
    let (mut tx, _) = begin_operation(&state, c.server.id, &operation).await?;
    let mut g = load_crash(&mut tx, &auth.uuid).await?;
    let who = Who { economy: c.server.economy_id, server: c.server.id, uuid: &auth.uuid, name: &auth.username };
    let outcome = crash_state(&g, &c.cfg, now);
    if let CrashState::Running(_) = outcome {
        return Ok(Json(json!({ "game": crash_view(&g, &c.cfg, now, None) })));
    }
    let (payout, balance) = crash_settle(&mut tx, &c.cfg, who, &mut g, &outcome, now).await?;
    let double = if payout > g.bet { offer_double(&mut tx, &c.cfg, c.server.id, &auth.uuid, payout, 0).await? } else { None };
    finish_operation(tx, c.server.id, &operation, json!({ "game": crash_view(&g, &c.cfg, now, Some(payout)), "balance": balance, "payout": payout, "double": double })).await
}

pub async fn crash_cashout(auth: AuthUser, State(state): State<AppState>, Path(sid): Path<i64>) -> AppResult<Json<Value>> {
    let c = ctx(&state, sid).await?;
    let operation = op();
    let (mut tx, _) = begin_operation(&state, c.server.id, &operation).await?;
    let mut g = load_crash(&mut tx, &auth.uuid).await?;
    if g.server_id != c.server.id {
        return Err(AppError::conflict("Your Crash round is on another server."));
    }
    let who = Who { economy: c.server.economy_id, server: c.server.id, uuid: &auth.uuid, name: &auth.username };
    let now = now_ms();
    // If the round already crashed (or reached the auto cash-out) before this request arrived, that is what happens.
    let outcome = crash_state(&g, &c.cfg, now);
    let (payout, balance) = crash_settle(&mut tx, &c.cfg, who, &mut g, &outcome, now).await?;
    let double = if payout > g.bet { offer_double(&mut tx, &c.cfg, c.server.id, &auth.uuid, payout, 0).await? } else { None };
    finish_operation(tx, c.server.id, &operation, json!({ "game": crash_view(&g, &c.cfg, now, Some(payout)), "balance": balance, "payout": payout, "double": double })).await
}

// ---- blackjack -----------------------------------------------------------------------------------------------------

#[derive(sqlx::FromRow)]
struct BjRow {
    id: i64,
    server_id: i64,
    bet: f64,
    player: String,
    dealer: String,
    doubled: bool,
    status: String,
}

const BJ_COLS: &str = "id, server_id, bet, player, dealer, doubled, status";

fn cards(raw: &str) -> Vec<u8> {
    serde_json::from_str(raw).unwrap_or_default()
}

/// What the player sees: the dealer's second card stays face down until the hand is over.
fn bj_view(g: &BjRow, cfg: &Config, payout: Option<f64>) -> Value {
    let (player, dealer) = (cards(&g.player), cards(&g.dealer));
    let done = g.status != "active";
    let (total, soft) = casino::bj_total(&player);
    let shown: Vec<u8> = if done { dealer.clone() } else { dealer.iter().take(1).cloned().collect() };
    let mut v = json!({
        "id": g.id, "bet": g.bet, "status": g.status, "player": player, "player_total": total, "soft": soft,
        "dealer": shown, "dealer_total": casino::bj_total(&shown).0, "hidden": if done { 0 } else { 1 },
        "doubled": g.doubled, "can_double": !done && player.len() == 2 && !g.doubled,
    });
    if done {
        let mult = casino::bj_payout(&player, &dealer, cfg.blackjack.blackjack_pay, g.doubled);
        v["payout"] = json!(payout.unwrap_or(0.0));
        v["outcome"] = json!(if total > 21 { "bust" } else if casino::bj_natural(&player) && !g.doubled && mult > 1.0 { "blackjack" } else if mult > 1.0 { "win" } else if mult == 1.0 { "push" } else { "lose" });
    }
    v
}

pub(super) async fn active_blackjack_view(db: &sqlx::SqlitePool, uuid: &str, server: i64, cfg: &Config) -> AppResult<Option<Value>> {
    let row: Option<BjRow> = sqlx::query_as(&format!("SELECT {BJ_COLS} FROM casino_blackjack WHERE uuid = ? AND status = 'active' ORDER BY id DESC LIMIT 1")).bind(uuid).fetch_optional(db).await?;
    Ok(row.filter(|g| g.server_id == server).map(|g| bj_view(&g, cfg, None)))
}

async fn load_bj(tx: &mut Tx, uuid: &str, server: i64) -> AppResult<BjRow> {
    let g: BjRow = sqlx::query_as(&format!("SELECT {BJ_COLS} FROM casino_blackjack WHERE uuid = ? AND status = 'active' ORDER BY id DESC LIMIT 1"))
        .bind(uuid).fetch_optional(&mut **tx).await?.ok_or_else(|| AppError::not_found("You have no Blackjack hand running."))?;
    if g.server_id != server {
        return Err(AppError::conflict("Your Blackjack hand is on another server."));
    }
    Ok(g)
}

/// Dealer plays (unless nothing depends on it), the hand is paid, saved and closed.
async fn bj_finish(tx: &mut Tx, cfg: &Config, who: Who<'_>, g: &mut BjRow, mut dealer: Vec<u8>, player: &[u8]) -> AppResult<(f64, f64)> {
    let busted = casino::bj_total(player).0 > 21;
    let decided = casino::bj_natural(player) || casino::bj_natural(&dealer);
    if !busted && !decided {
        casino::bj_dealer_plays(&mut dealer, cfg.blackjack.dealer_hits_soft_17, &mut rand::thread_rng());
    }
    let mult = casino::bj_payout(player, &dealer, cfg.blackjack.blackjack_pay, g.doubled);
    let payout = round2((g.bet * mult).min(cfg.max_payout));
    let balance = if payout > 0.0 { credit(tx, who, payout, "Casino: Blackjack win").await? } else { balance_of_tx(tx, who.economy, who.uuid).await.unwrap_or(0.0) };
    g.dealer = serde_json::to_string(&dealer)?;
    g.player = serde_json::to_string(player)?;
    g.status = "done".into();
    sqlx::query("UPDATE casino_blackjack SET status = 'done', player = ?, dealer = ?, bet = ?, doubled = ? WHERE id = ?")
        .bind(&g.player).bind(&g.dealer).bind(g.bet).bind(g.doubled).bind(g.id).execute(&mut **tx).await?;
    record(tx, who.server, who.uuid, who.name, "blackjack", g.bet, payout, &json!({ "player": player, "dealer": dealer, "doubled": g.doubled, "multiplier": mult })).await?;
    Ok((payout, balance))
}

async fn bj_response(tx: Tx, cfg: &Config, server: i64, operation: &str, uuid: &str, g: &BjRow, settled: Option<(f64, f64)>, balance_now: f64) -> AppResult<Json<Value>> {
    let mut tx = tx;
    let (payout, balance) = settled.unwrap_or((0.0, balance_now));
    let double = if settled.is_some() && payout > g.bet { offer_double(&mut tx, cfg, server, uuid, payout, 0).await? } else { None };
    finish_operation(tx, server, operation, json!({ "game": bj_view(g, cfg, settled.map(|s| s.0)), "balance": balance, "payout": payout, "double": double })).await
}

#[derive(Deserialize)]
pub struct BjStart {
    pub bet: f64,
}

pub async fn blackjack_start(auth: AuthUser, State(state): State<AppState>, Path(sid): Path<i64>, Json(p): Json<BjStart>) -> AppResult<Json<Value>> {
    let c = ctx_open(&state, sid).await?;
    let j = &c.cfg.blackjack;
    if !j.enabled {
        return Err(AppError::forbidden("Blackjack is switched off."));
    }
    let bet = check_bet(p.bet, j.min_bet, j.max_bet)?;
    let operation = op();
    let (mut tx, _) = begin_operation(&state, c.server.id, &operation).await?;
    if loss_limit_reached(&mut tx, &c.cfg, &auth.uuid).await? {
        return Err(AppError::forbidden(LOSS_LIMIT_MESSAGE));
    }
    let open: Option<i64> = sqlx::query_scalar("SELECT id FROM casino_blackjack WHERE uuid = ? AND status = 'active' LIMIT 1").bind(&auth.uuid).fetch_optional(&mut *tx).await?;
    if open.is_some() {
        return Err(AppError::conflict("Finish your current Blackjack hand first."));
    }
    let who = Who { economy: c.server.economy_id, server: c.server.id, uuid: &auth.uuid, name: &auth.username };
    let balance = debit(&mut tx, who, bet, "Casino: Blackjack bet").await?;
    let (player, dealer) = {
        let mut rng = rand::thread_rng();
        (vec![casino::bj_draw(&mut rng), casino::bj_draw(&mut rng)], vec![casino::bj_draw(&mut rng), casino::bj_draw(&mut rng)])
    };
    let id: i64 = sqlx::query_scalar("INSERT INTO casino_blackjack (server_id, uuid, name, bet, player, dealer, created_at) VALUES (?, ?, ?, ?, ?, ?, ?) RETURNING id")
        .bind(c.server.id).bind(&auth.uuid).bind(&auth.username).bind(bet)
        .bind(serde_json::to_string(&player)?).bind(serde_json::to_string(&dealer)?).bind(crate::db::now()).fetch_one(&mut *tx).await?;
    let mut g = BjRow { id, server_id: c.server.id, bet, player: serde_json::to_string(&player)?, dealer: serde_json::to_string(&dealer)?, doubled: false, status: "active".into() };
    // A natural 21 on either side ends the hand straight away (the dealer peeks).
    let settled = if casino::bj_natural(&player) || casino::bj_natural(&dealer) {
        Some(bj_finish(&mut tx, &c.cfg, who, &mut g, dealer, &player).await?)
    } else {
        None
    };
    bj_response(tx, &c.cfg, c.server.id, &operation, &auth.uuid, &g, settled, balance).await
}

#[derive(Clone, Copy, PartialEq)]
enum BjMove {
    Hit,
    Stand,
    Double,
}

async fn blackjack_move(auth: AuthUser, state: AppState, sid: i64, mv: BjMove) -> AppResult<Json<Value>> {
    let c = ctx(&state, sid).await?;
    let operation = op();
    let (mut tx, _) = begin_operation(&state, c.server.id, &operation).await?;
    let mut g = load_bj(&mut tx, &auth.uuid, c.server.id).await?;
    let who = Who { economy: c.server.economy_id, server: c.server.id, uuid: &auth.uuid, name: &auth.username };
    let mut player = cards(&g.player);
    let dealer = cards(&g.dealer);
    let mut balance = balance_of_tx(&mut tx, who.economy, who.uuid).await.unwrap_or(0.0);
    let settled = match mv {
        BjMove::Hit => {
            player.push(casino::bj_draw(&mut rand::thread_rng()));
            if casino::bj_total(&player).0 >= 21 { Some(bj_finish(&mut tx, &c.cfg, who, &mut g, dealer, &player).await?) } else { None }
        }
        BjMove::Stand => Some(bj_finish(&mut tx, &c.cfg, who, &mut g, dealer, &player).await?),
        BjMove::Double => {
            if player.len() != 2 || g.doubled {
                return Err(AppError::bad_request("You can only double on your first two cards."));
            }
            if g.bet * 2.0 > c.cfg.blackjack.max_bet {
                return Err(AppError::bad_request(format!("Doubling would go over the biggest bet of ${:.2}.", c.cfg.blackjack.max_bet)));
            }
            balance = debit(&mut tx, who, g.bet, "Casino: Blackjack double down").await?;
            g.bet = round2(g.bet * 2.0);
            g.doubled = true;
            player.push(casino::bj_draw(&mut rand::thread_rng()));
            Some(bj_finish(&mut tx, &c.cfg, who, &mut g, dealer, &player).await?)
        }
    };
    if settled.is_none() {
        g.player = serde_json::to_string(&player)?;
        sqlx::query("UPDATE casino_blackjack SET player = ? WHERE id = ?").bind(&g.player).bind(g.id).execute(&mut *tx).await?;
    }
    bj_response(tx, &c.cfg, c.server.id, &operation, &auth.uuid, &g, settled, balance).await
}

pub async fn blackjack_hit(auth: AuthUser, State(state): State<AppState>, Path(sid): Path<i64>) -> AppResult<Json<Value>> {
    blackjack_move(auth, state, sid, BjMove::Hit).await
}
pub async fn blackjack_stand(auth: AuthUser, State(state): State<AppState>, Path(sid): Path<i64>) -> AppResult<Json<Value>> {
    blackjack_move(auth, state, sid, BjMove::Stand).await
}
pub async fn blackjack_double(auth: AuthUser, State(state): State<AppState>, Path(sid): Path<i64>) -> AppResult<Json<Value>> {
    blackjack_move(auth, state, sid, BjMove::Double).await
}
