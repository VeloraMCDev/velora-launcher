//! Scheduled tasks: everything the panel does on its own, in one place.
//!
//! Two kinds exist. *Driven* tasks are run by the scheduler loop on their interval (and on demand from the admin page).
//! *Built-in* tasks have their own loops elsewhere (reward queue, Discord announcements); they report each run here so the
//! page can show when they last ran and whether it worked.

use crate::auth::AdminUser;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::state::RequestState as State;
use axum::extract::Path;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::Instant;

pub struct TaskDef {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub category: &'static str,
    pub default_interval_secs: i64,
    /// `true` when the scheduler loop runs it; `false` for loops that live elsewhere and only report in.
    pub driven: bool,
    /// Fast tasks only update their counters; the rest also keep a run history.
    pub history: bool,
    pub default_settings: &'static str,
}

pub const TASKS: &[TaskDef] = &[
    TaskDef {
        id: "purge_deleted_players",
        name: "Purge deleted players",
        description: "Removes leaderboard rows, levels, stats, quests, economy and guild data for players whose account no longer exists. \
            Turn on dry run to preview what would be deleted.",
        category: "Maintenance",
        default_interval_secs: 86_400,
        driven: true,
        history: true,
        default_settings: r#"{"dry_run":false,"include_unregistered":false,"grace_days":60}"#,
    },
    TaskDef {
        id: "refresh_titles",
        name: "Refresh rank titles",
        description: "Makes every player's saved title match the current level rewards.",
        category: "Progression",
        default_interval_secs: 3_600,
        driven: true,
        history: true,
        default_settings: "{}",
    },
    TaskDef {
        id: "prune_notifications",
        name: "Prune old notifications",
        description: "Deletes read bell notifications older than 30 days and anything older than 90 days.",
        category: "Maintenance",
        default_interval_secs: 86_400,
        driven: true,
        history: true,
        default_settings: "{}",
    },
    TaskDef {
        id: "default_money",
        name: "Default money rewards",
        description: "Gives every quest, achievement and rank milestone without a money reward the default one for its difficulty, and keeps the automatic amounts in step with the rates in Progression.",
        category: "Progression",
        default_interval_secs: 3_600,
        driven: true,
        history: true,
        default_settings: "{}",
    },
    TaskDef {
        id: "economy_accounts",
        name: "Create missing bank accounts",
        description: "Opens an account (at the starting balance) for every player who has none, and re-links an unclaimed account that carries the player's name. \
            Also available as a button in the Economy tab.",
        category: "Economy",
        default_interval_secs: 86_400,
        driven: true,
        history: true,
        default_settings: "{}",
    },
    TaskDef {
        id: "settle_auctions",
        name: "Settle auctions",
        description: "Closes auctions that have ended: pays the seller and leaves the item in the winner's mailbox, or returns it to the seller when nobody bid.",
        category: "Economy",
        default_interval_secs: 30,
        driven: true,
        history: false,
        default_settings: "{}",
    },
    TaskDef {
        id: "settle_orders",
        name: "Settle buy orders",
        description: "Takes down buy orders nobody filled in time and returns the escrowed money to the buyer, and releases lapsed reservations.",
        category: "Economy",
        default_interval_secs: 60,
        driven: true,
        history: false,
        default_settings: "{}",
    },
    TaskDef {
        id: "settle_casino",
        name: "Settle casino bets and bounties",
        description: "Pays out player bets whose time is up, returns bounties nobody collected, and closes Mines games left open for a day.",
        category: "Economy",
        default_interval_secs: 30,
        driven: true,
        history: false,
        default_settings: "{}",
    },
    TaskDef {
        id: "reward_queue",
        name: "Apply earned rewards",
        description: "Grants rewards from levels, quests and achievements within seconds of being earned.",
        category: "Progression",
        default_interval_secs: 5,
        driven: false,
        history: false,
        default_settings: "{}",
    },
    TaskDef {
        id: "discord_announce",
        name: "Discord announcements & live embeds",
        description: "Posts achievements, new guilds and server events to Discord, and refreshes live status embeds.",
        category: "Discord",
        default_interval_secs: 30,
        driven: false,
        history: false,
        default_settings: "{}",
    },
    TaskDef {
        id: "discord_role_sync",
        name: "Discord role sync",
        description: "Keeps Discord roles and panel groups in step according to the role sync mode.",
        category: "Discord",
        default_interval_secs: 900,
        driven: false,
        history: true,
        default_settings: "{}",
    },
];

pub fn def(id: &str) -> Option<&'static TaskDef> {
    TASKS.iter().find(|t| t.id == id)
}

fn epoch() -> i64 {
    chrono::Utc::now().timestamp()
}

fn iso(secs: Option<i64>) -> Value {
    secs.and_then(|s| chrono::DateTime::from_timestamp(s, 0)).map(|d| json!(d.to_rfc3339())).unwrap_or(Value::Null)
}

/// Create the rows for tasks that have none yet (first start, or a task added by an update).
pub async fn ensure_rows(state: &AppState) -> AppResult<()> {
    for t in TASKS {
        sqlx::query("INSERT OR IGNORE INTO scheduled_tasks (id, interval_secs, settings, next_run_at) VALUES (?, ?, ?, ?)")
            .bind(t.id)
            .bind(t.default_interval_secs)
            .bind(t.default_settings)
            .bind(epoch() + 20)
            .execute(&state.db)
            .await?;
    }
    Ok(())
}

/// Record the outcome of one run. Used by the scheduler and by the built-in loops.
pub async fn record(state: &AppState, id: &str, started: Instant, result: &Result<String, String>) {
    let Some(t) = def(id) else { return };
    let (ok, msg) = match result {
        Ok(m) => (true, m.as_str()),
        Err(m) => (false, m.as_str()),
    };
    let ms = started.elapsed().as_millis() as i64;
    let now = epoch();
    let upd = sqlx::query(
        "UPDATE scheduled_tasks SET last_run_at = ?, last_ok = ?, last_message = ?, last_duration_ms = ?, runs = runs + 1,
         failures = failures + ?, next_run_at = ? + interval_secs WHERE id = ?",
    )
    .bind(now)
    .bind(ok as i64)
    .bind(msg)
    .bind(ms)
    .bind(!ok as i64)
    .bind(now)
    .bind(id)
    .execute(&state.db)
    .await;
    if let Err(e) = upd {
        tracing::warn!("scheduler: could not record {id}: {e}");
        return;
    }
    if t.history {
        let _ = sqlx::query("INSERT INTO scheduled_task_runs (task_id, started_at, duration_ms, ok, message) VALUES (?, ?, ?, ?, ?)")
            .bind(id)
            .bind(now)
            .bind(ms)
            .bind(ok as i64)
            .bind(msg)
            .execute(&state.db)
            .await;
        let _ = sqlx::query(
            "DELETE FROM scheduled_task_runs WHERE task_id = ? AND id NOT IN (SELECT id FROM scheduled_task_runs WHERE task_id = ? ORDER BY id DESC LIMIT 50)",
        )
        .bind(id)
        .bind(id)
        .execute(&state.db)
        .await;
    }
}

/// Convenience for the built-in loops: time `fut`, then record how it went.
pub async fn track<T, E: std::fmt::Debug>(
    state: &AppState,
    id: &str,
    fut: impl std::future::Future<Output = Result<T, E>>,
    describe: impl FnOnce(&T) -> String,
) -> Option<T> {
    let started = Instant::now();
    match fut.await {
        Ok(v) => {
            record(state, id, started, &Ok(describe(&v))).await;
            Some(v)
        }
        Err(e) => {
            record(state, id, started, &Err(format!("{e:?}"))).await;
            None
        }
    }
}

async fn settings_of(state: &AppState, id: &str) -> Value {
    let raw: Option<String> =
        sqlx::query_scalar("SELECT settings FROM scheduled_tasks WHERE id = ?").bind(id).fetch_optional(&state.db).await.ok().flatten();
    raw.and_then(|r| serde_json::from_str(&r).ok()).unwrap_or(json!({}))
}

async fn available(state: &AppState, id: &str) -> bool {
    let Some(instance) = &state.instance_id else {
        return true;
    };
    let feature = match id {
        "refresh_titles" | "default_money" | "reward_queue" => "progression",
        "economy_accounts" | "settle_auctions" | "settle_orders" => "economy",
        "settle_casino" => "casino",
        _ => return true,
    };
    let Ok(row) = crate::store::get_instance(state, instance).await else {
        return false;
    };
    serde_json::from_str::<scopenet_shared::Experience>(&row.experience).is_ok_and(|e| e.enabled(feature))
}

/// Run one driven task now and record the result.
pub async fn run_now(state: &AppState, id: &str) -> Result<String, String> {
    if !available(state, id).await {
        return Err("task feature is disabled for this instance".into());
    }
    let started = Instant::now();
    let settings = settings_of(state, id).await;
    let result = match id {
        "purge_deleted_players" => purge_deleted_players(state, &settings).await.map_err(|e| e.message),
        "refresh_titles" => crate::routes::leveling::refresh_global_titles(state)
            .await
            .map(|(players, changed)| format!("checked {players} players, updated {changed}"))
            .map_err(|e| e.message),
        "prune_notifications" => prune_notifications(state).await.map_err(|e| e.message),
        "default_money" => crate::rewards::apply_defaults(state).await.map(|n| format!("{n} rewards written")).map_err(|e| e.message),
        "economy_accounts" => crate::routes::economy_admin::repair_all(state).await.map_err(|e| e.message),
        "settle_auctions" => crate::routes::auctions::settle_due(state).await.map_err(|e| e.message),
        "settle_orders" => crate::routes::orders::settle_due(state).await.map_err(|e| e.message),
        "settle_casino" => crate::routes::casino::settle_due(state).await.map_err(|e| e.message),
        "reward_queue" => crate::rewards::process_queue(state).await.map(|n| format!("applied {n} rewards")).map_err(|e| e.message),
        "discord_role_sync" => crate::routes::discord::sync_all(state)
            .await
            .map(|r| format!("{} roles added, {} removed", r.roles_added, r.roles_removed))
            .map_err(|e| e.message),
        _ => Err("unknown task".into()),
    };
    record(state, id, started, &result).await;
    result
}

async fn prune_notifications(state: &AppState) -> AppResult<String> {
    let now = chrono::Utc::now();
    let read_cut = (now - chrono::Duration::days(30)).to_rfc3339();
    let any_cut = (now - chrono::Duration::days(90)).to_rfc3339();
    let a = sqlx::query("DELETE FROM user_notifications WHERE (read_at IS NOT NULL AND created_at < ?) OR created_at < ?")
        .bind(read_cut)
        .bind(any_cut)
        .execute(&state.db)
        .await?
        .rows_affected();
    Ok(format!("removed {a} old notifications"))
}

/// Players whose account was removed (before data removal existed, or while the game kept reporting them) still sit in
/// leaderboards. Their UUIDs are remembered in `reserved_usernames`, which is how they are found without touching players
/// who simply never made an account.
async fn purge_deleted_players(state: &AppState, settings: &Value) -> AppResult<String> {
    let dry = settings["dry_run"].as_bool().unwrap_or(false);
    let include_unregistered = settings["include_unregistered"].as_bool().unwrap_or(false);
    let grace = settings["grace_days"].as_i64().unwrap_or(60).clamp(1, 3650);

    let mut candidates: Vec<(String, String)> = sqlx::query_as(
        "SELECT r.uuid, MIN(r.name) FROM reserved_usernames r
         WHERE r.uuid <> '' AND r.uuid <> ? AND NOT EXISTS (SELECT 1 FROM users u WHERE u.uuid = r.uuid) GROUP BY r.uuid",
    )
    .bind(crate::purge::DELETED_UUID)
    .fetch_all(&state.db)
    .await?;
    if include_unregistered {
        let cut = (chrono::Utc::now() - chrono::Duration::days(grace)).to_rfc3339();
        let extra: Vec<(String, String)> = sqlx::query_as(
            "SELECT p.uuid, MAX(p.name) FROM player_stats p
             WHERE p.uuid <> ? AND NOT EXISTS (SELECT 1 FROM users u WHERE u.uuid = p.uuid)
               AND NOT EXISTS (SELECT 1 FROM server_online o WHERE o.uuid = p.uuid)
             GROUP BY p.uuid HAVING MAX(p.last_seen) < ?",
        )
        .bind(crate::purge::DELETED_UUID)
        .bind(cut)
        .fetch_all(&state.db)
        .await?;
        for e in extra {
            if !candidates.iter().any(|c| c.0 == e.0) {
                candidates.push(e);
            }
        }
    }

    let (mut players, mut rows) = (0u64, 0u64);
    for (uuid, name) in candidates {
        let mut tx = state.db.begin().await?;
        let mut report = crate::purge::PurgeReport::default();
        crate::purge::purge_player_data(&mut tx, &uuid, &name, &mut report).await?;
        let touched = report.total() + report.guilds_dissolved + report.guilds_transferred;
        if touched == 0 || dry {
            tx.rollback().await?;
        } else {
            tx.commit().await?;
            state.worldmap.forget_player(&uuid);
        }
        if touched > 0 {
            players += 1;
            rows += report.total();
        }
    }
    Ok(if dry {
        format!("dry run: would clean {players} deleted players ({rows} rows)")
    } else {
        format!("cleaned {players} deleted players ({rows} rows)")
    })
}

/// The scheduler loop: wakes every 15 seconds and runs whatever is due.
pub fn spawn(state: AppState) {
    tokio::spawn(async move {
        if let Err(e) = ensure_rows(&state).await {
            tracing::warn!("scheduler: {}", e.message);
        }
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(15)).await;
            if state.db.is_closed() {
                return;
            }
            let due: Vec<String> = sqlx::query_scalar("SELECT id FROM scheduled_tasks WHERE enabled = 1 AND next_run_at <= ?")
                .bind(epoch())
                .fetch_all(&state.db)
                .await
                .unwrap_or_default();
            for id in due {
                if def(&id).is_some_and(|t| t.driven) && available(&state, &id).await {
                    if let Err(e) = run_now(&state, &id).await {
                        tracing::warn!("scheduler: {id} failed: {e}");
                    }
                }
            }
        }
    });
}

// ---------------------------------------------------------------------------
// Admin API
// ---------------------------------------------------------------------------

pub async fn list(_: AdminUser, State(state): State<AppState>) -> AppResult<Json<Value>> {
    ensure_rows(&state).await?;
    let rows: Vec<(String, i64, i64, String, Option<i64>, Option<i64>, Option<String>, Option<i64>, Option<i64>, i64, i64)> = sqlx::query_as(
        "SELECT id, enabled, interval_secs, settings, last_run_at, last_ok, last_message, last_duration_ms, next_run_at, runs, failures FROM scheduled_tasks",
    )
    .fetch_all(&state.db)
    .await?;
    let mut out = Vec::new();
    for t in TASKS {
        if !available(&state, t.id).await {
            continue;
        }
        let Some(r) = rows.iter().find(|r| r.0 == t.id) else { continue };
        let history: Vec<(i64, i64, i64, String)> = sqlx::query_as(
            "SELECT started_at, duration_ms, ok, message FROM scheduled_task_runs WHERE task_id = ? ORDER BY id DESC LIMIT 10",
        )
        .bind(t.id)
        .fetch_all(&state.db)
        .await?;
        out.push(json!({
            "id": t.id, "name": t.name, "description": t.description, "category": t.category,
            "driven": t.driven, "default_interval_secs": t.default_interval_secs,
            "enabled": r.1 != 0, "interval_secs": r.2,
            "settings": serde_json::from_str::<Value>(&r.3).unwrap_or(json!({})),
            "last_run_at": iso(r.4), "last_ok": r.5.map(|v| v != 0), "last_message": r.6, "last_duration_ms": r.7,
            "next_run_at": if t.driven && r.1 != 0 { iso(r.8) } else { Value::Null },
            "runs": r.9, "failures": r.10,
            "history": history.into_iter().map(|(at, ms, ok, m)| json!({"at": iso(Some(at)), "duration_ms": ms, "ok": ok != 0, "message": m})).collect::<Vec<_>>(),
        }));
    }
    Ok(Json(json!({ "tasks": out })))
}

#[derive(Deserialize)]
pub struct UpdatePayload {
    enabled: Option<bool>,
    interval_secs: Option<i64>,
    settings: Option<Value>,
}

pub async fn update(
    _: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(p): Json<UpdatePayload>,
) -> AppResult<Json<Value>> {
    let t = def(&id).ok_or_else(|| AppError::not_found("Unknown task"))?;
    if !available(&state, &id).await {
        return Err(AppError::forbidden("task feature is disabled for this instance"));
    }
    ensure_rows(&state).await?;
    if let Some(en) = p.enabled {
        if !t.driven && !en {
            return Err(AppError::bad_request("Built-in tasks always run; turn the feature off in its own settings instead"));
        }
        sqlx::query("UPDATE scheduled_tasks SET enabled = ? WHERE id = ?").bind(en as i64).bind(&id).execute(&state.db).await?;
    }
    if let Some(iv) = p.interval_secs {
        if !t.driven {
            return Err(AppError::bad_request("Built-in tasks have a fixed interval"));
        }
        if !(10..=30 * 86_400).contains(&iv) {
            return Err(AppError::bad_request("Interval must be between 10 seconds and 30 days"));
        }
        sqlx::query("UPDATE scheduled_tasks SET interval_secs = ?, next_run_at = ? + ? WHERE id = ?")
            .bind(iv)
            .bind(epoch())
            .bind(iv)
            .bind(&id)
            .execute(&state.db)
            .await?;
    }
    if let Some(s) = p.settings {
        if !s.is_object() {
            return Err(AppError::bad_request("Settings must be an object"));
        }
        sqlx::query("UPDATE scheduled_tasks SET settings = ? WHERE id = ?").bind(s.to_string()).bind(&id).execute(&state.db).await?;
    }
    Ok(Json(json!({ "ok": true })))
}

pub async fn run(_: AdminUser, State(state): State<AppState>, Path(id): Path<String>) -> AppResult<Json<Value>> {
    def(&id).ok_or_else(|| AppError::not_found("Unknown task"))?;
    ensure_rows(&state).await?;
    let res = run_now(&state, &id).await;
    Ok(Json(match res {
        Ok(m) => json!({ "ok": true, "message": m }),
        Err(m) => json!({ "ok": false, "message": m }),
    }))
}
