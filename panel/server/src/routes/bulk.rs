//! Bulk editor: every quest, achievement or level reward as one table row, and one request that saves many edits.
//! The page does the "set / multiply / add / round" maths on the rows you picked and sends only what changed.

use crate::state::RequestState as State;
use crate::auth::AdminUser;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use axum::extract::{Path};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::SqliteConnection;

type Money = Option<(f64, bool)>;

fn money_of(actions: &[Value]) -> Money {
    actions.iter().find(|a| a["type"] == "money" && a["server_id"].is_null()).map(|a| (a["amount"].as_f64().unwrap_or(0.0), a["auto"] == true))
}

async fn stored(conn: &mut SqliteConnection, kind: &str, id: &str) -> Vec<Value> {
    let raw: Option<String> = sqlx::query_scalar("SELECT actions FROM reward_bundles WHERE source_type = ? AND source_id = ?")
        .bind(kind).bind(id).fetch_optional(&mut *conn).await.ok().flatten();
    raw.and_then(|r| serde_json::from_str(&r).ok()).unwrap_or_default()
}

fn kind_ok(kind: &str) -> AppResult<&'static str> {
    match kind {
        "quest" => Ok("quest"),
        "achievement" => Ok("achievement"),
        "level_reward" => Ok("level_reward"),
        _ => Err(AppError::bad_request("Unknown kind")),
    }
}

/// Every row of one kind, with its XP, money and whether the money is the automatic amount.
pub async fn list(_: AdminUser, State(state): State<AppState>, Path(kind): Path<String>) -> AppResult<Json<Value>> {
    let kind = kind_ok(&kind)?;
    let mut conn = state.db.acquire().await?;
    let rows: Vec<(String, String, String, Option<i64>, Option<i64>, Option<bool>)> = match kind {
        "quest" => sqlx::query_as("SELECT id, title, period, xp_reward, target_count, enabled FROM quests ORDER BY period, xp_reward, title").fetch_all(&mut *conn).await?,
        "achievement" => sqlx::query_as("SELECT id, title, category, xp_reward, requirement_value, NULL FROM achievements ORDER BY category, xp_reward, title").fetch_all(&mut *conn).await?,
        _ => sqlx::query_as("SELECT CAST(id AS TEXT), reward_name, level_type, NULL, level_req, NULL FROM level_rewards ORDER BY level_type, level_req, id").fetch_all(&mut *conn).await?,
    };
    let mut out = Vec::with_capacity(rows.len());
    for (id, title, group, xp, count, enabled) in rows {
        let actions = stored(&mut conn, kind, &id).await;
        let money = money_of(&actions);
        out.push(json!({
            "id": id, "title": title, "group": group, "xp": xp, "count": count, "enabled": enabled,
            "money": money.map(|m| m.0), "auto": money.map(|m| m.1).unwrap_or(true),
            "other_rewards": actions.iter().filter(|a| a["type"] != "money").count(),
        }));
    }
    Ok(Json(json!({ "kind": kind, "rows": out })))
}

#[derive(Deserialize)]
pub struct Patch {
    id: String,
    xp: Option<i64>,
    enabled: Option<bool>,
    level_req: Option<i64>,
    /// A number sets the amount yourself; `reset_money` goes back to the automatic one.
    money: Option<f64>,
    #[serde(default)]
    reset_money: bool,
}

#[derive(Deserialize)]
pub struct Body {
    patches: Vec<Patch>,
}

pub async fn save(_: AdminUser, State(state): State<AppState>, Path(kind): Path<String>, Json(body): Json<Body>) -> AppResult<Json<Value>> {
    let kind = kind_ok(&kind)?;
    if body.patches.len() > 5000 {
        return Err(AppError::bad_request("Too many rows at once"));
    }
    for p in &body.patches {
        if p.xp.is_some_and(|x| !(0..=10_000_000).contains(&x)) {
            return Err(AppError::bad_request("XP must be between 0 and 10,000,000"));
        }
        if p.money.is_some_and(|m| !m.is_finite() || !(0.0..=1e9).contains(&m)) {
            return Err(AppError::bad_request("Money must be between 0 and 1,000,000,000"));
        }
        if p.level_req.is_some_and(|l| !(1..=100_000).contains(&l)) {
            return Err(AppError::bad_request("Level must be at least 1"));
        }
    }
    let mut tx = state.db.begin().await?;
    let mut changed = 0;
    for p in &body.patches {
        let res = match kind {
            "quest" => sqlx::query("UPDATE quests SET xp_reward = COALESCE(?, xp_reward), enabled = COALESCE(?, enabled) WHERE id = ?").bind(p.xp).bind(p.enabled).bind(&p.id).execute(&mut *tx).await?,
            "achievement" => sqlx::query("UPDATE achievements SET xp_reward = COALESCE(?, xp_reward) WHERE id = ?").bind(p.xp).bind(&p.id).execute(&mut *tx).await?,
            _ => sqlx::query("UPDATE level_rewards SET level_req = COALESCE(?, level_req) WHERE id = ?").bind(p.level_req).bind(&p.id).execute(&mut *tx).await?,
        };
        if res.rows_affected() == 0 {
            continue;
        }
        changed += 1;
        if p.money.is_some() || p.reset_money {
            let mut actions = stored(&mut tx, kind, &p.id).await;
            actions.retain(|a| !(a["type"] == "money" && a["server_id"].is_null()));
            if let Some(amount) = p.money.filter(|_| !p.reset_money) {
                actions.insert(0, json!({ "type": "money", "amount": (amount * 100.0).round() / 100.0 }));
            }
            if actions.is_empty() {
                sqlx::query("DELETE FROM reward_bundles WHERE source_type = ? AND source_id = ?").bind(kind).bind(&p.id).execute(&mut *tx).await?;
            } else {
                sqlx::query("INSERT INTO reward_bundles (source_type, source_id, actions) VALUES (?, ?, ?) ON CONFLICT(source_type, source_id) DO UPDATE SET actions = excluded.actions")
                    .bind(kind).bind(&p.id).bind(serde_json::to_string(&actions)?).execute(&mut *tx).await?;
            }
        }
    }
    tx.commit().await?;
    // Changed XP moves the automatic money with it, and a reset needs its default back.
    crate::rewards::apply_defaults(&state).await?;
    Ok(Json(json!({ "ok": true, "changed": changed })))
}
