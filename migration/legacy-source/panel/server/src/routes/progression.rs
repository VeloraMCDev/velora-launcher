//! Admin control of progression: quest limits, XP rates, the level curve and
//! manual per-player XP/level changes.

use crate::auth::AdminUser;
use crate::error::{AppError, AppResult};
use crate::progression::{self, Mode, Progression};
use crate::state::AppState;
use crate::state::RequestState as State;
use axum::extract::{Path, Query};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

async fn pool_counts(state: &AppState) -> AppResult<Value> {
    let rows: Vec<(String, i64, i64)> =
        sqlx::query_as("SELECT period, COUNT(*), COALESCE(SUM(pinned), 0) FROM quests WHERE enabled = 1 GROUP BY period")
            .fetch_all(&state.db)
            .await?;
    let mut out = json!({ "daily": { "enabled": 0, "pinned": 0 }, "weekly": { "enabled": 0, "pinned": 0 } });
    for (period, n, pinned) in rows {
        if period == "daily" || period == "weekly" {
            out[period] = json!({ "enabled": n, "pinned": pinned });
        }
    }
    Ok(out)
}

fn curve_preview(p: &Progression) -> Vec<Value> {
    // The preview is computed from the settings being shown, not the live curve.
    let cap = if p.max_level == 0 { 50 } else { p.max_level.min(50) };
    (2..=cap).map(|l| json!({ "level": l, "xp": (p.level_base * (l as f64).powf(p.level_exponent)).round() as i64 })).collect()
}

/// Sample payouts so an admin sees what the rates mean before saving them.
fn money_preview(p: &Progression) -> Value {
    use crate::rewards::{achievement_money, level_money, milestone_money, quest_money};
    let a = &p.rules.auto_money;
    let levels: Vec<Value> = [2i64, 5, 10, 25, 50, 100]
        .iter()
        .map(|&l| json!({ "level": l, "money": level_money(a, l), "milestone": milestone_money(a, l, false) }))
        .collect();
    let daily: Vec<Value> = [80i64, 150, 250, 450].iter().map(|&xp| json!({ "xp": xp, "money": quest_money(a, "daily", xp) })).collect();
    let weekly: Vec<Value> =
        [600i64, 1000, 1800, 2500].iter().map(|&xp| json!({ "xp": xp, "money": quest_money(a, "weekly", xp) })).collect();
    let achievements: Vec<Value> =
        [100i64, 500, 1500, 5000].iter().map(|&xp| json!({ "xp": xp, "money": achievement_money(a, xp) })).collect();
    json!({ "levels": levels, "daily": daily, "weekly": weekly, "achievements": achievements })
}

async fn view(state: &AppState, p: &Progression) -> AppResult<Value> {
    Ok(json!({ "settings": p, "quest_pool": pool_counts(state).await?, "curve": curve_preview(p), "money_preview": money_preview(p) }))
}

/// `POST /progression/auto-money/apply`: write the default money into every quest, achievement and rank milestone that has none.
pub async fn apply_auto_money(admin: AdminUser, State(state): State<AppState>) -> AppResult<Json<Value>> {
    let written = crate::rewards::apply_defaults(&state).await?;
    crate::routes::activity::record(&state, &admin, "panel", "auto_money_applied", Some(&format!("{written} rewards"))).await?;
    Ok(Json(json!({ "ok": true, "written": written })))
}

pub async fn get_settings(_: AdminUser, State(state): State<AppState>) -> AppResult<Json<Value>> {
    let p = progression::load_pool(&state.db).await?;
    Ok(Json(view(&state, &p).await?))
}

pub async fn put_settings(admin: AdminUser, State(state): State<AppState>, Json(next): Json<Progression>) -> AppResult<Json<Value>> {
    next.validate()?;
    let before = progression::load_pool(&state.db).await?;
    progression::save(&state.db, &next).await?;

    // New rates (or switching default money on) reach every quest, achievement and milestone straight away.
    crate::rewards::apply_defaults(&state).await?;
    let curve_changed =
        before.level_base != next.level_base || before.level_exponent != next.level_exponent || before.max_level != next.max_level;
    let recalculated = if curve_changed { progression::recalculate_levels(&state.db).await? } else { 0 };
    crate::routes::activity::record(&state, &admin, "panel", "progression_settings", Some(&format!("recalculated {recalculated} levels")))
        .await?;
    let mut v = view(&state, &next).await?;
    v["recalculated"] = json!(recalculated);
    Ok(Json(v))
}

#[derive(Deserialize)]
pub struct PlayerQuery {
    #[serde(default)]
    q: String,
    #[serde(default = "default_limit")]
    limit: i64,
    #[serde(default)]
    offset: i64,
}
fn default_limit() -> i64 {
    40
}

/// Players with their global level, searchable by name.
pub async fn list_players(_: AdminUser, State(state): State<AppState>, Query(q): Query<PlayerQuery>) -> AppResult<Json<Value>> {
    let like = format!("%{}%", q.q.trim().replace('%', "").replace('_', ""));
    let rows: Vec<(String, String, i64, i64, Option<String>)> = sqlx::query_as(
        "SELECT u.uuid, u.username, COALESCE(ul.global_xp, 0), COALESCE(ul.global_level, 1), ul.title
         FROM users u LEFT JOIN user_levels ul ON ul.uuid = u.uuid
         WHERE u.username LIKE ? ORDER BY COALESCE(ul.global_xp, 0) DESC, u.username COLLATE NOCASE LIMIT ? OFFSET ?",
    )
    .bind(&like)
    .bind(q.limit.clamp(1, 100))
    .bind(q.offset.max(0))
    .fetch_all(&state.db)
    .await?;
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE username LIKE ?").bind(&like).fetch_one(&state.db).await?;
    let curve = progression::load_pool(&state.db).await?;
    let players: Vec<Value> = rows
        .into_iter()
        .map(|(uuid, name, xp, _, title)| {
            let (level, into, span, pct) = curve.level_from_xp(xp);
            json!({ "uuid": uuid, "name": name, "global_xp": xp, "global_level": level, "level_xp": into, "level_span": span, "progress_pct": pct, "title": title })
        })
        .collect();
    Ok(Json(json!({ "players": players, "total": total })))
}

async fn player_detail(state: &AppState, uuid: &str) -> AppResult<Value> {
    let curve = progression::load_pool(&state.db).await?;
    let user: Option<(String,)> = sqlx::query_as("SELECT username FROM users WHERE uuid = ?").bind(uuid).fetch_optional(&state.db).await?;
    let (name,) = user.ok_or_else(|| AppError::not_found("player not found"))?;
    let (xp, title): (i64, Option<String>) = sqlx::query_as("SELECT global_xp, title FROM user_levels WHERE uuid = ?")
        .bind(uuid)
        .fetch_optional(&state.db)
        .await?
        .unwrap_or((0, None));
    let (level, into, span, pct) = curve.level_from_xp(xp);
    let servers: Vec<(i64, String, i64, Option<String>)> = sqlx::query_as(
        "SELECT s.id, s.name, COALESCE(sl.server_xp, 0), sl.rank_name
         FROM game_servers s LEFT JOIN server_levels sl ON sl.server_id = s.id AND sl.uuid = ?
         ORDER BY s.name COLLATE NOCASE",
    )
    .bind(uuid)
    .fetch_all(&state.db)
    .await?;
    let servers: Vec<Value> = servers
        .into_iter()
        .map(|(id, sname, sxp, rank)| {
            let (sl, sinto, sspan, spct) = curve.level_from_xp(sxp);
            json!({ "server_id": id, "server_name": sname, "xp": sxp, "level": sl, "level_xp": sinto, "level_span": sspan, "progress_pct": spct, "rank_name": rank })
        })
        .collect();
    Ok(json!({
        "uuid": uuid, "name": name, "title": title,
        "global": { "xp": xp, "level": level, "level_xp": into, "level_span": span, "progress_pct": pct },
        "servers": servers,
        "max_level": curve.max_level(),
    }))
}

pub async fn get_player(_: AdminUser, State(state): State<AppState>, Path(uuid): Path<String>) -> AppResult<Json<Value>> {
    Ok(Json(player_detail(&state, &uuid).await?))
}

#[derive(Deserialize)]
pub struct Adjust {
    /// `global` or `server`.
    scope: String,
    server_id: Option<i64>,
    mode: Mode,
    /// XP for add/remove/set_xp, the level for set_level. Ignored for reset.
    #[serde(default)]
    amount: i64,
    #[serde(default)]
    reason: String,
}

pub async fn adjust_player(
    admin: AdminUser,
    State(state): State<AppState>,
    Path(uuid): Path<String>,
    Json(a): Json<Adjust>,
) -> AppResult<Json<Value>> {
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE uuid = ?)").bind(&uuid).fetch_one(&state.db).await?;
    if !exists {
        return Err(AppError::not_found("player not found"));
    }
    let now = chrono::Utc::now().to_rfc3339();
    let mut tx = state.db.begin().await?;
    let (label, change) = match a.scope.as_str() {
        "global" => ("global".to_string(), progression::adjust_global(&mut tx, &uuid, a.mode, a.amount, &now).await?),
        "server" => {
            let id = a.server_id.ok_or_else(|| AppError::bad_request("pick a server"))?;
            let name: Option<String> =
                sqlx::query_scalar("SELECT name FROM game_servers WHERE id = ?").bind(id).fetch_optional(&mut *tx).await?;
            let name = name.ok_or_else(|| AppError::not_found("server not found"))?;
            (format!("server {name}"), progression::adjust_server(&mut tx, id, &uuid, a.mode, a.amount, &now).await?)
        }
        _ => return Err(AppError::bad_request("scope must be global or server")),
    };
    // Level rewards (titles, badges) are earned, so they follow upward changes.
    crate::routes::leveling::grant_rewards(&mut tx, &uuid, &now).await?;
    tx.commit().await?;

    let reason: String = a.reason.chars().filter(|c| !c.is_control()).take(120).collect();
    let detail = format!(
        "{label} XP {} → {} (level {} → {}) for {uuid}{}",
        change.old_xp,
        change.new_xp,
        change.old_level,
        change.new_level,
        if reason.is_empty() { String::new() } else { format!(": {reason}") }
    );
    crate::routes::activity::record(&state, &admin, "panel", "xp_adjust", Some(&detail)).await?;
    Ok(Json(player_detail(&state, &uuid).await?))
}
