//! Contracts: every player has a small board of randomly generated tasks, like "Kill 50 Skeletons" or "Submit 256x Cobblestone",
//! that pay money when finished. Kill contracts count by themselves as the game server reports kills. Resource contracts only
//! progress when the items are actually handed in (`/contracts submit`): the items are taken from the player and destroyed, so a
//! contract is a real sink for the resource, never just a counter.

use crate::state::RequestState as State;
use super::economy::{begin_operation, finish_operation};
use super::ledger::{balance_of, credit, normalize_item, pretty, round2, Tx, CONTRACTS};
use super::notifications;
use crate::auth::AuthUser;
use crate::error::{AppError, AppResult};
use crate::routes::servers::{economy_scope, get_server, GameServer, ServerRow};
use crate::state::AppState;
use axum::extract::{Path};
use axum::Json;
use chrono::{Duration, SecondsFormat, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

// ---- settings --------------------------------------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Config {
    pub enabled: bool,
    /// Contracts a player has to choose from at once.
    pub board_size: u32,
    pub expire_hours: u32,
    /// Scales every payout up or down.
    pub reward_multiplier: f64,
    /// Most contracts one player can finish in a UTC day. 0 means no limit.
    pub daily_limit: u32,
    /// How many contracts a player may throw away per day for a fresh one.
    pub rerolls_per_day: u32,
    pub kills: bool,
    pub gather: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config { enabled: true, board_size: 4, expire_hours: 24, reward_multiplier: 1.0, daily_limit: 12, rerolls_per_day: 3, kills: true, gather: true }
    }
}

impl Config {
    pub fn sanitize(&mut self) {
        self.board_size = self.board_size.clamp(1, 12);
        self.expire_hours = self.expire_hours.clamp(1, 24 * 14);
        self.reward_multiplier = if self.reward_multiplier.is_finite() { round2(self.reward_multiplier.clamp(0.1, 20.0)) } else { 1.0 };
        self.daily_limit = self.daily_limit.min(500);
        self.rerolls_per_day = self.rerolls_per_day.min(50);
        if !self.kills && !self.gather {
            self.kills = true;
            self.gather = true;
        }
    }
}

pub(crate) async fn config(state: &AppState) -> AppResult<Config> {
    let mut c: Config = crate::store::kv_get(state, "contracts").await?;
    c.sanitize();
    Ok(c)
}

async fn config_conn(conn: &mut sqlx::SqliteConnection) -> Config {
    let raw: Option<String> = sqlx::query_scalar("SELECT value FROM kv WHERE key = 'contracts'").fetch_optional(&mut *conn).await.ok().flatten();
    let mut c: Config = raw.and_then(|r| serde_json::from_str(&r).ok()).unwrap_or_default();
    c.sanitize();
    c
}

// ---- what can be asked for ---------------------------------------------------------------------------------------------

/// (entity, plural name, pay per kill, fewest, most, step)
const KILLS: &[(&str, &str, f64, i64, i64, i64)] = &[
    ("ZOMBIE", "Zombies", 6.0, 20, 80, 10),
    ("SKELETON", "Skeletons", 7.0, 20, 70, 10),
    ("SPIDER", "Spiders", 7.0, 15, 60, 5),
    ("CREEPER", "Creepers", 10.0, 10, 40, 5),
    ("DROWNED", "Drowned", 9.0, 10, 40, 5),
    ("ENDERMAN", "Endermen", 22.0, 5, 25, 5),
    ("WITCH", "Witches", 28.0, 3, 12, 1),
    ("BLAZE", "Blazes", 26.0, 8, 40, 4),
    ("PIGLIN", "Piglins", 14.0, 8, 30, 2),
    ("GUARDIAN", "Guardians", 18.0, 5, 25, 5),
    ("PHANTOM", "Phantoms", 20.0, 4, 20, 2),
    ("WITHER_SKELETON", "Wither Skeletons", 34.0, 4, 16, 2),
];

/// (item, name, value each, fewest, most, step)
const GATHER: &[(&str, &str, f64, i64, i64, i64)] = &[
    ("COBBLESTONE", "Cobblestone", 0.6, 128, 768, 64),
    ("OAK_LOG", "Oak Logs", 1.6, 64, 384, 32),
    ("SAND", "Sand", 0.8, 128, 640, 64),
    ("COAL", "Coal", 2.2, 64, 256, 32),
    ("IRON_INGOT", "Iron Ingots", 6.0, 32, 192, 16),
    ("COPPER_INGOT", "Copper Ingots", 2.4, 64, 320, 32),
    ("GOLD_INGOT", "Gold Ingots", 9.0, 16, 96, 8),
    ("REDSTONE", "Redstone Dust", 2.5, 64, 320, 32),
    ("LAPIS_LAZULI", "Lapis Lazuli", 3.0, 32, 192, 16),
    ("WHEAT", "Wheat", 1.4, 64, 384, 32),
    ("CARROT", "Carrots", 1.4, 64, 256, 32),
    ("POTATO", "Potatoes", 1.4, 64, 256, 32),
    ("SUGAR_CANE", "Sugar Cane", 1.3, 64, 320, 32),
    ("BONE", "Bones", 2.0, 32, 192, 16),
    ("STRING", "String", 2.2, 32, 192, 16),
    ("GUNPOWDER", "Gunpowder", 4.0, 16, 128, 16),
    ("QUARTZ", "Nether Quartz", 3.0, 32, 192, 16),
    ("GLOWSTONE_DUST", "Glowstone Dust", 3.2, 32, 192, 16),
    ("DIAMOND", "Diamonds", 40.0, 4, 24, 2),
    ("EMERALD", "Emeralds", 22.0, 8, 40, 4),
];

struct Fresh {
    kind: &'static str,
    target: String,
    title: String,
    required: i64,
    reward: f64,
    bonus: bool,
}

/// How hard the work is, 1 (easy) to 5 (brutal), from what each unit is worth: rarer things take more effort per piece.
fn tier(each: f64) -> u8 {
    match each {
        e if e < 3.0 => 1,
        e if e < 8.0 => 2,
        e if e < 15.0 => 3,
        e if e < 30.0 => 4,
        _ => 5,
    }
}

/// What a contract pays: the work's base value, a premium that grows with its difficulty tier, a little more for bigger
/// asks, and a bonus for "hot" contracts. Always a flat number ending in 5 or 0, never less than 5.
fn payout(each: f64, required: i64, lo: i64, hi: i64, bonus: bool, multiplier: f64) -> f64 {
    let premium = 1.1 + 0.15 * f64::from(tier(each));
    let size = 1.0 + 0.2 * (required - lo) as f64 / (hi - lo).max(1) as f64;
    crate::rewards::round5(each * required as f64 * premium * size * if bonus { 1.5 } else { 1.0 } * multiplier)
}

/// The difficulty of a contract's target, for showing on the board.
fn difficulty_of(kind: &str, target: &str) -> u8 {
    let each = if kind == "kill" { KILLS.iter().find(|k| k.0 == target).map(|k| k.2) } else { GATHER.iter().find(|k| k.0 == target).map(|k| k.2) };
    each.map(tier).unwrap_or(1)
}

/// Roll one new contract, avoiding anything already on the board. Pure, so it can be tested with a seeded generator.
fn roll<R: Rng>(cfg: &Config, taken: &[String], rng: &mut R) -> Option<Fresh> {
    let want_kill = if cfg.kills && cfg.gather { rng.gen_bool(0.5) } else { cfg.kills };
    let bonus = rng.gen_bool(0.1);
    if want_kill {
        let options: Vec<usize> = (0..KILLS.len()).filter(|i| !taken.contains(&KILLS[*i].0.to_string())).collect();
        let (entity, plural, each, lo, hi, step) = KILLS[*options.get(rng.gen_range(0..options.len().max(1)))?];
        let required = lo + rng.gen_range(0..=((hi - lo) / step)) * step;
        return Some(Fresh { kind: "kill", target: entity.into(), title: format!("Kill {required} {plural}"), required, reward: payout(each, required, lo, hi, bonus, cfg.reward_multiplier), bonus });
    }
    let options: Vec<usize> = (0..GATHER.len()).filter(|i| !taken.contains(&GATHER[*i].0.to_string())).collect();
    let (item, name, each, lo, hi, step) = GATHER[*options.get(rng.gen_range(0..options.len().max(1)))?];
    let required = lo + rng.gen_range(0..=((hi - lo) / step)) * step;
    Some(Fresh { kind: "gather", target: item.into(), title: format!("Submit {required}x {name}"), required, reward: payout(each, required, lo, hi, bonus, cfg.reward_multiplier), bonus })
}

fn generate(cfg: &Config, taken: &[String], n: usize) -> Vec<Fresh> {
    let mut rng = rand::thread_rng();
    let mut taken = taken.to_vec();
    let mut out = Vec::new();
    for _ in 0..n {
        if let Some(c) = roll(cfg, &taken, &mut rng) {
            taken.push(c.target.clone());
            out.push(c);
        }
    }
    out
}

// ---- rows ------------------------------------------------------------------------------------------------------------

#[derive(sqlx::FromRow, Clone)]
struct Contract {
    id: i64,
    kind: String,
    target: String,
    title: String,
    required: i64,
    progress: i64,
    reward: f64,
    bonus: bool,
    status: String,
    expires_at: String,
}

const COLS: &str = "id, kind, target, title, required, progress, reward, bonus, status, expires_at";

fn now() -> String {
    crate::db::now()
}

fn day_start() -> String {
    Utc::now().date_naive().and_hms_opt(0, 0, 0).map(|t| t.and_utc().to_rfc3339_opts(SecondsFormat::Secs, true)).unwrap_or_default()
}

fn view(c: &Contract) -> Value {
    json!({
        "id": c.id, "kind": c.kind, "target": c.target, "title": c.title, "required": c.required, "progress": c.progress,
        "reward": c.reward, "bonus": c.bonus, "status": c.status, "expires_at": c.expires_at,
        "difficulty": difficulty_of(&c.kind, &c.target),
        "item_name": if c.kind == "gather" { Some(pretty(&c.target)) } else { None },
    })
}

async fn server_of(state: &AppState, id: i64) -> AppResult<ServerRow> {
    let mut server = get_server(state, id).await?;
    server.economy_id = economy_scope(&state.db, server.id).await?;
    Ok(server)
}

fn op() -> String {
    format!("contract-{}", uuid::Uuid::new_v4())
}

/// Retire expired contracts and top the board up to its size, within the day's limit. Returns the player's active contracts.
async fn ensure_board(state: &AppState, cfg: &Config, server_id: i64, uuid: &str) -> AppResult<Vec<Contract>> {
    let mut tx = state.db.begin().await?;
    let t = now();
    sqlx::query("UPDATE contracts SET status = 'expired', resolved_at = ? WHERE server_id = ? AND uuid = ? AND status = 'active' AND expires_at <= ?").bind(&t).bind(server_id).bind(uuid).bind(&t).execute(&mut *tx).await?;
    let active: Vec<Contract> = sqlx::query_as(&format!("SELECT {COLS} FROM contracts WHERE server_id = ? AND uuid = ? AND status = 'active' ORDER BY id")).bind(server_id).bind(uuid).fetch_all(&mut *tx).await?;
    let done: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM contracts WHERE server_id = ? AND uuid = ? AND status = 'completed' AND resolved_at >= ?").bind(server_id).bind(uuid).bind(day_start()).fetch_one(&mut *tx).await?;
    let room = (cfg.board_size as usize).saturating_sub(active.len());
    let allowed = if cfg.daily_limit == 0 { room } else { room.min((cfg.daily_limit as i64 - done).max(0) as usize) };
    if allowed == 0 {
        tx.commit().await?;
        return Ok(active);
    }
    let taken: Vec<String> = active.iter().map(|c| c.target.clone()).collect();
    for c in generate(cfg, &taken, allowed) {
        sqlx::query("INSERT INTO contracts (server_id, uuid, kind, target, title, required, reward, bonus, created_at, expires_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(server_id).bind(uuid).bind(c.kind).bind(&c.target).bind(&c.title).bind(c.required).bind(c.reward).bind(c.bonus).bind(&t)
            .bind((Utc::now() + Duration::hours(cfg.expire_hours as i64)).to_rfc3339_opts(SecondsFormat::Secs, true)).execute(&mut *tx).await?;
    }
    let active: Vec<Contract> = sqlx::query_as(&format!("SELECT {COLS} FROM contracts WHERE server_id = ? AND uuid = ? AND status = 'active' ORDER BY id")).bind(server_id).bind(uuid).fetch_all(&mut *tx).await?;
    tx.commit().await?;
    Ok(active)
}

pub(crate) async fn board(state: &AppState, server: &ServerRow, uuid: &str) -> AppResult<Value> {
    let cfg = config(state).await?;
    if !cfg.enabled {
        return Ok(json!({ "enabled": false, "contracts": [] }));
    }
    let active = ensure_board(state, &cfg, server.id, uuid).await?;
    let done_today: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM contracts WHERE server_id = ? AND uuid = ? AND status = 'completed' AND resolved_at >= ?").bind(server.id).bind(uuid).bind(day_start()).fetch_one(&state.db).await?;
    let rerolls_used: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM contracts WHERE server_id = ? AND uuid = ? AND status = 'abandoned' AND resolved_at >= ?").bind(server.id).bind(uuid).bind(day_start()).fetch_one(&state.db).await?;
    let totals: (i64, f64) = sqlx::query_as("SELECT COUNT(*), COALESCE(SUM(reward), 0.0) FROM contracts WHERE server_id = ? AND uuid = ? AND status = 'completed'").bind(server.id).bind(uuid).fetch_one(&state.db).await?;
    let recent: Vec<Contract> = sqlx::query_as(&format!("SELECT {COLS} FROM contracts WHERE server_id = ? AND uuid = ? AND status = 'completed' ORDER BY resolved_at DESC LIMIT 5")).bind(server.id).bind(uuid).fetch_all(&state.db).await?;
    Ok(json!({
        "enabled": true,
        "server": { "id": server.id, "name": server.name },
        "balance": balance_of(&state.db, server.economy_id, uuid).await,
        "contracts": active.iter().map(view).collect::<Vec<_>>(),
        "recent": recent.iter().map(view).collect::<Vec<_>>(),
        "stats": {
            "done_today": done_today, "daily_limit": cfg.daily_limit, "limit_reached": cfg.daily_limit > 0 && done_today >= cfg.daily_limit as i64,
            "rerolls_left": (cfg.rerolls_per_day as i64 - rerolls_used).max(0), "completed": totals.0, "earned": round2(totals.1),
            "board_size": cfg.board_size, "expire_hours": cfg.expire_hours,
        },
    }))
}

pub async fn contracts_board(auth: AuthUser, State(state): State<AppState>, Path(sid): Path<i64>) -> AppResult<Json<Value>> {
    let server = server_of(&state, sid).await?;
    Ok(Json(board(&state, &server, &auth.uuid).await?))
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct WhoBody {
    pub uuid: String,
}

pub async fn server_list(GameServer(server): GameServer, State(state): State<AppState>, Json(p): Json<WhoBody>) -> AppResult<Json<Value>> {
    if p.uuid.is_empty() {
        return Err(AppError::bad_request("Missing player"));
    }
    Ok(Json(board(&state, &server, &p.uuid).await?))
}

// ---- finishing -----------------------------------------------------------------------------------------------------------

/// Mark a contract done and pay it. Returns the new balance.
async fn complete(tx: &mut Tx, server: &ServerRow, uuid: &str, name: &str, c: &Contract) -> AppResult<f64> {
    sqlx::query("UPDATE contracts SET status = 'completed', progress = required, resolved_at = ? WHERE id = ? AND status = 'active'").bind(now()).bind(c.id).execute(&mut **tx).await?;
    credit(tx, server.economy_id, server.id, uuid, name, c.reward, CONTRACTS, &format!("Contract complete: {}", c.title)).await
}

pub struct Note {
    pub uuid: String,
    pub title: String,
    pub body: String,
}

pub async fn send_notes(db: &sqlx::SqlitePool, notes: Vec<Note>) {
    for n in notes {
        notifications::push(db, &n.uuid, "contract", &n.title, &n.body, None).await;
    }
}

/// Called while a game server's sync is applied: `action` looks like `mob_kills:SKELETON`. Counts toward every matching kill
/// contract the player has and pays any that this finishes.
pub async fn advance_kills(tx: &mut Tx, server: &ServerRow, uuid: &str, name: &str, action: &str, count: i64) -> AppResult<Vec<Note>> {
    let mut notes = Vec::new();
    let Some(entity) = action.strip_prefix("mob_kills:") else { return Ok(notes) };
    let entity = entity.split('@').next().unwrap_or(entity).trim().to_ascii_uppercase();
    if count <= 0 || entity.is_empty() {
        return Ok(notes);
    }
    let cfg = config_conn(&mut **tx).await;
    if !cfg.enabled || !cfg.kills {
        return Ok(notes);
    }
    let rows: Vec<Contract> = sqlx::query_as(&format!("SELECT {COLS} FROM contracts WHERE server_id = ? AND uuid = ? AND status = 'active' AND kind = 'kill' AND target = ? AND expires_at > ?"))
        .bind(server.id).bind(uuid).bind(&entity).bind(now()).fetch_all(&mut **tx).await?;
    for mut c in rows {
        c.progress = (c.progress + count).min(c.required);
        if c.progress >= c.required {
            let balance = complete(tx, server, uuid, name, &c).await?;
            notes.push(Note { uuid: uuid.into(), title: "Contract complete".into(), body: format!("{} is done: ${:.2} paid. Balance ${balance:.2}.", c.title, c.reward) });
        } else {
            sqlx::query("UPDATE contracts SET progress = ? WHERE id = ?").bind(c.progress).bind(c.id).execute(&mut **tx).await?;
        }
    }
    Ok(notes)
}

#[derive(Deserialize)]
pub struct SubmitBody {
    pub operation_id: String,
    pub uuid: String,
    pub name: String,
    /// Leave out (or 0) to put the items toward whichever contract wants them.
    #[serde(default)]
    pub contract_id: i64,
    pub item_id: String,
    /// How many the game server took from the player's inventory.
    pub amount: i64,
}

/// The game server has taken `amount` items from the player and holds them in escrow. Whatever the contract still needs is
/// consumed; the rest is returned in `items`. A request that matches nothing is refused so the game server gives it all back.
pub async fn server_submit(GameServer(server): GameServer, State(state): State<AppState>, Json(p): Json<SubmitBody>) -> AppResult<Json<Value>> {
    let cfg = config(&state).await?;
    if !cfg.enabled || !cfg.gather {
        return Err(AppError::forbidden("Resource contracts are switched off."));
    }
    let item = normalize_item(&p.item_id)?;
    if p.amount < 1 {
        return Err(AppError::bad_request("Nothing to submit."));
    }
    ensure_board(&state, &cfg, server.id, &p.uuid).await?;
    let (mut tx, previous) = begin_operation(&state, server.id, &p.operation_id).await?;
    if let Some(previous) = previous {
        return Ok(Json(previous));
    }
    let mut candidates: Vec<Contract> = sqlx::query_as(&format!("SELECT {COLS} FROM contracts WHERE server_id = ? AND uuid = ? AND status = 'active' AND kind = 'gather' AND target = ? AND expires_at > ? ORDER BY id"))
        .bind(server.id).bind(&p.uuid).bind(&item).bind(now()).fetch_all(&mut *tx).await?;
    if p.contract_id > 0 {
        candidates.retain(|c| c.id == p.contract_id);
    }
    let mut left = p.amount;
    let mut messages = Vec::new();
    let mut balance = None;
    for mut c in candidates {
        if left <= 0 {
            break;
        }
        let take = left.min(c.required - c.progress);
        c.progress += take;
        left -= take;
        if c.progress >= c.required {
            balance = Some(complete(&mut tx, &server, &p.uuid, &p.name, &c).await?);
            messages.push(format!("{} complete: +${:.2}", c.title, c.reward));
        } else {
            sqlx::query("UPDATE contracts SET progress = ? WHERE id = ?").bind(c.progress).bind(c.id).execute(&mut *tx).await?;
            messages.push(format!("{}: {}/{}", c.title, c.progress, c.required));
        }
    }
    if left == p.amount {
        return Err(AppError::bad_request(format!("None of your contracts needs {}.", pretty(&item))));
    }
    let items: Vec<Value> = if left > 0 { vec![json!({ "item_id": item, "item_name": pretty(&item), "amount": left, "item_data": null })] } else { vec![] };
    let bal = match balance {
        Some(b) => Some(b),
        None => balance_of(&state.db, server.economy_id, &p.uuid).await,
    };
    let finished = balance.is_some();
    let response = finish_operation(tx, server.id, &p.operation_id, json!({ "ok": true, "balance": bal, "items": items, "message": messages.join(" | "), "completed": finished })).await?;
    if finished {
        notifications::push(&state.db, &p.uuid, "contract", "Contract complete", &messages.join(" | "), None).await;
    }
    Ok(response)
}

// ---- throwing one away -------------------------------------------------------------------------------------------------

pub(crate) async fn abandon_core(state: &AppState, server: &ServerRow, uuid: &str, id: i64) -> AppResult<Json<Value>> {
    let cfg = config(state).await?;
    let mut tx = state.db.begin().await?;
    let used: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM contracts WHERE server_id = ? AND uuid = ? AND status = 'abandoned' AND resolved_at >= ?").bind(server.id).bind(uuid).bind(day_start()).fetch_one(&mut *tx).await?;
    if used >= cfg.rerolls_per_day as i64 {
        return Err(AppError::conflict("You've swapped as many contracts as you can today."));
    }
    let changed = sqlx::query("UPDATE contracts SET status = 'abandoned', resolved_at = ? WHERE id = ? AND server_id = ? AND uuid = ? AND status = 'active'").bind(now()).bind(id).bind(server.id).bind(uuid).execute(&mut *tx).await?.rows_affected();
    if changed != 1 {
        return Err(AppError::not_found("That contract isn't on your board."));
    }
    tx.commit().await?;
    Ok(Json(board(state, server, uuid).await?))
}

pub async fn abandon(auth: AuthUser, State(state): State<AppState>, Path((sid, id)): Path<(i64, i64)>) -> AppResult<Json<Value>> {
    let server = server_of(&state, sid).await?;
    abandon_core(&state, &server, &auth.uuid, id).await
}

#[derive(Deserialize)]
pub struct AbandonBody {
    pub uuid: String,
    pub contract_id: i64,
}

pub async fn server_abandon(GameServer(server): GameServer, State(state): State<AppState>, Json(p): Json<AbandonBody>) -> AppResult<Json<Value>> {
    abandon_core(&state, &server, &p.uuid, p.contract_id).await
}

// ---- admin ---------------------------------------------------------------------------------------------------------------

pub async fn admin_get(_admin: crate::auth::AdminUser, State(state): State<AppState>) -> AppResult<Json<Value>> {
    let totals: (i64, f64, i64) = sqlx::query_as("SELECT COALESCE(SUM(status = 'completed'), 0), COALESCE(SUM(CASE WHEN status = 'completed' THEN reward END), 0.0), COALESCE(SUM(status = 'active'), 0) FROM contracts").fetch_one(&state.db).await?;
    Ok(Json(json!({
        "config": config(&state).await?, "defaults": Config::default(),
        "stats": { "completed": totals.0, "paid": round2(totals.1), "active": totals.2 },
        "kinds": { "kills": KILLS.iter().map(|k| json!({ "target": k.0, "name": k.1 })).collect::<Vec<_>>(), "gather": GATHER.iter().map(|k| json!({ "target": k.0, "name": k.1 })).collect::<Vec<_>>() },
    })))
}

pub async fn admin_save(_admin: crate::auth::AdminUser, State(state): State<AppState>, Json(mut c): Json<Config>) -> AppResult<Json<Value>> {
    c.sanitize();
    crate::store::kv_set(&state, "contracts", &c).await?;
    Ok(Json(json!({ "config": c })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{rngs::StdRng, SeedableRng};

    #[test]
    fn rolled_contracts_are_valid_unique_and_fairly_paid() {
        let cfg = Config::default();
        let mut rng = StdRng::seed_from_u64(4);
        let mut kinds = std::collections::HashSet::new();
        for _ in 0..500 {
            let taken: Vec<String> = Vec::new();
            let c = roll(&cfg, &taken, &mut rng).expect("a contract");
            kinds.insert(c.kind);
            assert!(c.required > 0 && c.reward > 0.0 && c.title.contains(&c.required.to_string()));
            let (each, lo, hi) = match c.kind {
                "kill" => KILLS.iter().find(|k| k.0 == c.target).map(|k| (k.2, k.3, k.4)).unwrap(),
                _ => GATHER.iter().find(|k| k.0 == c.target).map(|k| (k.2, k.3, k.4)).unwrap(),
            };
            assert!((lo..=hi).contains(&c.required));
            let base = each * c.required as f64;
            assert!(c.reward % 5.0 == 0.0 && c.reward >= 5.0, "{} pays {}, not a flat 5 or 0", c.title, c.reward);
            assert!(c.reward >= base * 1.25 - 5.0 && c.reward <= base * 1.85 * 1.2 * 1.5 + 5.0, "{} pays {} for base {base}", c.title, c.reward);
        }
        assert_eq!(kinds.len(), 2, "both kill and resource contracts come up");
        // A board never repeats a target.
        let board = generate(&cfg, &[], 8);
        let mut seen = std::collections::HashSet::new();
        assert!(board.iter().all(|c| seen.insert(c.target.clone())));
        // Switching a kind off removes it.
        let only_gather = Config { kills: false, ..Config::default() };
        assert!(generate(&only_gather, &[], 6).iter().all(|c| c.kind == "gather"));
    }

    #[test]
    fn harder_work_pays_more_per_piece_and_always_in_fives() {
        // Same size of job, rarer thing, better rate.
        let rate = |each: f64| payout(each, 100, 100, 100, false, 1.0) / (each * 100.0);
        assert!(rate(1.0) < rate(5.0) && rate(5.0) < rate(10.0) && rate(10.0) < rate(20.0) && rate(20.0) < rate(40.0));
        // Bigger asks pay a better rate, hot ones pay more, and the multiplier scales it.
        assert!(payout(6.0, 80, 20, 80, false, 1.0) / 480.0 > payout(6.0, 20, 20, 80, false, 1.0) / 120.0);
        assert!(payout(6.0, 40, 20, 80, true, 1.0) > payout(6.0, 40, 20, 80, false, 1.0));
        assert!(payout(6.0, 40, 20, 80, false, 2.0) > payout(6.0, 40, 20, 80, false, 1.0));
        for (e, r) in [(0.6, 128), (1.4, 64), (6.0, 20), (22.0, 5), (40.0, 4)] {
            for m in [0.1, 0.55, 1.0, 3.3] {
                let p = payout(e, r, 1, 1000, m > 1.0, m);
                assert!(p >= 5.0 && p % 5.0 == 0.0, "{p}");
            }
        }
        assert_eq!(difficulty_of("kill", "ZOMBIE"), 2);
        assert_eq!(difficulty_of("gather", "DIAMOND"), 5);
    }

    #[test]
    fn settings_are_clamped() {
        let mut c = Config { board_size: 0, reward_multiplier: f64::NAN, kills: false, gather: false, expire_hours: 0, ..Config::default() };
        c.sanitize();
        assert!(c.board_size >= 1 && c.reward_multiplier == 1.0 && c.kills && c.gather && c.expire_hours >= 1);
    }
}
