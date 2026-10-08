//! The admin side of the casino: settings with a live preview of the odds, takings and losses, and clean-up of bounties and bets.

use crate::state::RequestState as State;
use crate::auth::AdminUser;
use crate::casino::{self, round2, Config};
use crate::error::{AppError, AppResult};
use crate::routes::casino::{config, void_by_id};
use crate::state::AppState;
use axum::extract::{Path};
use axum::Json;
use chrono::{Duration, Utc};
use serde_json::{json, Value};

/// Return-to-player figures for a set of settings, so the admin sees the house edge change as they type.
pub fn preview(cfg: &Config) -> Value {
    let (slots_rtp, slots_hit) = casino::slots_stats(&cfg.slots);
    let mut plinko = serde_json::Map::new();
    for risk in casino::RISKS {
        let mut rows = serde_json::Map::new();
        for n in cfg.plinko.min_rows..=cfg.plinko.max_rows {
            let table = casino::plinko_table(n, risk, cfg.plinko.rtp);
            rows.insert(n.to_string(), json!({ "rtp": casino::plinko_rtp(&table), "max": table.iter().cloned().fold(0.0, f64::max), "table": table }));
        }
        plinko.insert(risk.to_string(), Value::Object(rows));
    }
    let cells = cfg.mines.size * cfg.mines.size;
    let mines: Vec<Value> = [1, 3, 5, 10, cells - 1]
        .into_iter()
        .filter(|m| *m >= cfg.mines.min_mines && *m <= cfg.mines.max_mines)
        .map(|m| {
            let steps: Vec<f64> = [1u32, 3, 5].iter().map(|s| casino::mines_multiplier(cells, m, (*s).min(cells - m), cfg.mines.house_edge, cfg.mines.max_multiplier)).collect();
            json!({ "mines": m, "steps": steps })
        })
        .collect();
    let dice_samples: Vec<Value> = [5.0, 25.0, 50.0, 75.0, 95.0]
        .into_iter()
        .filter(|c| *c >= cfg.dice.min_chance && *c <= cfg.dice.max_chance)
        .map(|c| json!({ "chance": c, "pays": casino::dice_multiplier(c, cfg.dice.house_edge) }))
        .collect();
    let max_slot = cfg.slots.symbols.iter().map(|s| s.pay).fold(0.0, f64::max);
    let max_wheel = cfg.wheel.segments.iter().map(|s| s.value).fold(0.0, f64::max);
    let daily_total: f64 = cfg.daily.segments.iter().map(|s| s.weight).sum();
    let wheel_total: f64 = cfg.wheel.segments.iter().map(|s| s.weight).sum();
    let wheel_odds: Vec<f64> = cfg.wheel.segments.iter().map(|s| s.weight / wheel_total).collect();
    let daily_odds: Vec<f64> = cfg.daily.segments.iter().map(|s| if daily_total > 0.0 { s.weight / daily_total } else { 0.0 }).collect();
    json!({
        "slots": { "rtp": slots_rtp, "hit": slots_hit, "max": max_slot },
        "wheel": { "rtp": casino::segments_expected(&cfg.wheel.segments), "max": max_wheel },
        "daily": {
            "expected": round2(casino::segments_expected(&cfg.daily.segments)),
            "per_day": cfg.daily.spins_per_day,
            "odds": daily_odds,
        },
        "wheel_odds": wheel_odds,
        "plinko": plinko,
        "mines": { "rtp": 1.0 - cfg.mines.house_edge, "samples": mines },
        "blackjack": { "rtp": 0.99, "blackjack_pay": cfg.blackjack.blackjack_pay },
        "crash": { "rtp": 1.0 - cfg.crash.house_edge, "max": cfg.crash.max_multiplier },
        "dice": { "rtp": 1.0 - cfg.dice.house_edge, "samples": dice_samples },
        "coinflip": { "rtp": cfg.coinflip.payout / 2.0 },
        "double": { "rtp": cfg.double.win_chance * 2.0 },
        "chaos": { "factor": casino::chaos_factor(&cfg.chaos) },
    })
}

pub async fn get(_admin: AdminUser, State(state): State<AppState>) -> AppResult<Json<Value>> {
    let cfg = config(&state).await?;
    Ok(Json(json!({ "config": cfg, "defaults": Config::default(), "preview": preview(&cfg), "metrics": casino::METRICS })))
}

pub async fn preview_route(_admin: AdminUser, Json(mut cfg): Json<Config>) -> AppResult<Json<Value>> {
    cfg.sanitize();
    Ok(Json(json!({ "config": cfg, "preview": preview(&cfg) })))
}

pub async fn save(_admin: AdminUser, State(state): State<AppState>, Json(mut cfg): Json<Config>) -> AppResult<Json<Value>> {
    cfg.sanitize();
    crate::store::kv_set(&state, "casino", &cfg).await?;
    Ok(Json(json!({ "config": cfg, "preview": preview(&cfg) })))
}

pub async fn stats(_admin: AdminUser, State(state): State<AppState>) -> AppResult<Json<Value>> {
    let week = (Utc::now() - Duration::days(7)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let games: Vec<(String, i64, f64, f64, i64, f64, f64)> = sqlx::query_as(
        "SELECT game, COUNT(*), COALESCE(SUM(bet),0.0), COALESCE(SUM(payout),0.0),
                SUM(created_at >= ?1), COALESCE(SUM(CASE WHEN created_at >= ?1 THEN bet END),0.0), COALESCE(SUM(CASE WHEN created_at >= ?1 THEN payout END),0.0)
         FROM casino_rounds GROUP BY game",
    )
    .bind(&week).fetch_all(&state.db).await?;
    let days: Vec<(String, f64, f64, i64)> = sqlx::query_as(
        "SELECT substr(created_at, 1, 10), COALESCE(SUM(bet),0.0), COALESCE(SUM(payout),0.0), COUNT(DISTINCT uuid) FROM casino_rounds
         WHERE created_at >= ? GROUP BY 1 ORDER BY 1",
    )
    .bind((Utc::now() - Duration::days(14)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true)).fetch_all(&state.db).await?;
    let big: Vec<(String, String, f64, f64, String)> = sqlx::query_as("SELECT name, game, bet, payout, created_at FROM casino_rounds WHERE payout > bet ORDER BY payout - bet DESC LIMIT 8").fetch_all(&state.db).await?;
    let players: Vec<(String, f64, f64, i64)> = sqlx::query_as(
        "SELECT MAX(name), SUM(bet), SUM(payout), COUNT(*) FROM casino_rounds WHERE bet > 0 GROUP BY uuid ORDER BY SUM(bet) DESC LIMIT 10",
    )
    .fetch_all(&state.db).await?;
    let bounties: (i64, f64, i64, f64) = sqlx::query_as(
        "SELECT COALESCE(SUM(status = 'active'),0), COALESCE(SUM(CASE WHEN status = 'active' THEN reward END),0.0), COALESCE(SUM(status = 'claimed'),0), COALESCE(SUM(CASE WHEN status = 'claimed' THEN reward END),0.0) FROM casino_bounties",
    )
    .fetch_one(&state.db).await?;
    let tax: f64 = sqlx::query_scalar("SELECT COALESCE(SUM(paid - reward),0.0) FROM casino_bounties WHERE status IN ('active','claimed','expired')").fetch_one(&state.db).await?;
    let markets: (i64, i64, f64) = sqlx::query_as(
        "SELECT COALESCE(SUM(m.status = 'open'),0), COUNT(DISTINCT m.id), COALESCE(SUM(b.stake),0.0) FROM casino_markets m LEFT JOIN casino_bets b ON b.market_id = m.id",
    )
    .fetch_one(&state.db).await?;
    let spins_today: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(spins),0) FROM casino_free_spins WHERE day = ?").bind(Utc::now().format("%Y-%m-%d").to_string()).fetch_one(&state.db).await?;
    let cfg = config(&state).await?;
    let rake_taken: f64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(s.pool - s.paid_out),0.0) FROM (SELECT m.id, (SELECT COALESCE(SUM(stake),0.0) FROM casino_bets WHERE market_id = m.id) AS pool, (SELECT COALESCE(SUM(payout),0.0) FROM casino_bets WHERE market_id = m.id) AS paid_out
         FROM casino_markets m WHERE m.status = 'settled') s",
    )
    .fetch_one(&state.db).await?;
    let _ = cfg;
    Ok(Json(json!({
        "games": games.into_iter().map(|(game, rounds, bet, paid, rounds7, bet7, paid7)| json!({ "game": game, "rounds": rounds, "wagered": round2(bet), "paid": round2(paid), "profit": round2(bet - paid), "rounds_7d": rounds7, "profit_7d": round2(bet7 - paid7) })).collect::<Vec<_>>(),
        "days": days.into_iter().map(|(day, bet, paid, players)| json!({ "day": day, "wagered": round2(bet), "paid": round2(paid), "profit": round2(bet - paid), "players": players })).collect::<Vec<_>>(),
        "big_wins": big.into_iter().map(|(name, game, bet, payout, at)| json!({ "name": name, "game": game, "bet": bet, "payout": payout, "at": at })).collect::<Vec<_>>(),
        "top_players": players.into_iter().map(|(name, bet, paid, rounds)| json!({ "name": name, "wagered": round2(bet), "paid": round2(paid), "net": round2(paid - bet), "rounds": rounds })).collect::<Vec<_>>(),
        "bounties": { "active": bounties.0, "active_total": round2(bounties.1), "claimed": bounties.2, "claimed_total": round2(bounties.3), "tax_taken": round2(tax) },
        "betting": { "open": markets.0, "markets": markets.1, "volume": round2(markets.2), "rake_taken": round2(rake_taken) },
        "free_spins_today": spins_today,
    })))
}

pub async fn list_bounties(_admin: AdminUser, State(state): State<AppState>) -> AppResult<Json<Value>> {
    let rows: Vec<(i64, i64, String, String, f64, f64, bool, String, Option<String>, String, Option<String>)> = sqlx::query_as(
        "SELECT id, server_id, target_name, placer_name, paid, reward, anonymous, status, claimer_name, created_at, expires_at FROM casino_bounties ORDER BY (status = 'active') DESC, id DESC LIMIT 100",
    )
    .fetch_all(&state.db).await?;
    Ok(Json(json!({ "bounties": rows.into_iter().map(|(id, server, target, placer, paid, reward, anon, status, claimer, at, exp)| json!({
        "id": id, "server_id": server, "target": target, "placer": placer, "paid": paid, "reward": reward, "anonymous": anon, "status": status, "claimer": claimer, "at": at, "expires_at": exp })).collect::<Vec<_>>() })))
}

/// Take a bounty down and give the placer their reward back (the tax is not returned).
pub async fn cancel_bounty(_admin: AdminUser, State(state): State<AppState>, Path(id): Path<i64>) -> AppResult<Json<Value>> {
    let operation = format!("casino-admin-cancel-{id}");
    let row: Option<(i64, String, String, f64, String)> = sqlx::query_as("SELECT server_id, placer_uuid, placer_name, reward, target_name FROM casino_bounties WHERE id = ? AND status = 'active'")
        .bind(id).fetch_optional(&state.db).await?;
    let Some((economy, uuid, name, reward, target)) = row else { return Err(AppError::not_found("That bounty is already finished.")) };
    let (mut tx, _) = crate::routes::economy::begin_operation(&state, economy, &operation).await?;
    let changed = sqlx::query("UPDATE casino_bounties SET status = 'cancelled', resolved_at = ? WHERE id = ? AND status = 'active'").bind(crate::db::now()).bind(id).execute(&mut *tx).await?.rows_affected();
    if changed == 1 {
        crate::routes::economy::ensure_balance(&mut tx, economy, &uuid, &name).await?;
        sqlx::query("UPDATE server_economy SET balance = balance + ?, updated_at = ? WHERE server_id = ? AND uuid = ?").bind(reward).bind(Utc::now().to_rfc3339()).bind(economy).bind(&uuid).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO economy_transactions (server_id, from_uuid, from_name, to_uuid, to_name, amount, description, created_at) VALUES (?, 'casino', 'Casino', ?, ?, ?, ?, ?)")
            .bind(economy).bind(&uuid).bind(&name).bind(reward).bind(format!("Casino: bounty on {target} removed by an admin")).bind(Utc::now().to_rfc3339()).execute(&mut *tx).await?;
    }
    crate::routes::economy::finish_operation(tx, economy, &operation, json!({ "ok": true })).await
}

pub async fn list_markets(_admin: AdminUser, State(state): State<AppState>) -> AppResult<Json<Value>> {
    let rows: Vec<(i64, i64, String, String, String, i64, String, String, Option<String>, f64, i64)> = sqlx::query_as(
        "SELECT m.id, m.server_id, m.creator_name, m.subject_name, m.metric, m.threshold, m.ends_at, m.status, m.outcome,
                COALESCE((SELECT SUM(stake) FROM casino_bets WHERE market_id = m.id),0.0), (SELECT COUNT(*) FROM casino_bets WHERE market_id = m.id)
         FROM casino_markets m ORDER BY (m.status = 'open') DESC, m.id DESC LIMIT 100",
    )
    .fetch_all(&state.db).await?;
    Ok(Json(json!({ "markets": rows.into_iter().map(|(id, server, creator, subject, metric, threshold, ends, status, outcome, volume, bets)| json!({
        "id": id, "server_id": server, "creator": creator, "subject": subject, "metric": metric, "threshold": threshold, "ends_at": ends, "status": status, "outcome": outcome, "volume": round2(volume), "bets": bets })).collect::<Vec<_>>() })))
}

pub async fn void_market(_admin: AdminUser, State(state): State<AppState>, Path(id): Path<i64>) -> AppResult<Json<Value>> {
    void_by_id(&state, id, "cancelled by an admin").await?;
    Ok(Json(json!({ "ok": true })))
}
