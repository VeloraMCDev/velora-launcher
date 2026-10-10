//! Velora SMP faction upgrades. Legacy guild instances have no upgrade market.
use super::guilds::guild_can;
use super::servers::GameServer;
use crate::auth::AuthUser;
use crate::error::{AppError, AppResult};
use crate::state::{AppState, RequestState as State};
use crate::{factions_core, velora_core};
use axum::{extract::Path, Json};
use serde::Deserialize;
use serde_json::{json, Value};

/// The game server and balance scope that pays a faction's bills (the same choice as upkeep).
pub(crate) async fn scope(conn: &mut sqlx::SqliteConnection, guild: &str) -> AppResult<(i64, i64)> {
    let server: Option<i64> = sqlx::query_scalar("SELECT MIN(s.id) FROM game_servers s JOIN guilds g ON g.instance_id=s.instance_id WHERE g.id=?")
        .bind(guild).fetch_one(&mut *conn).await?;
    let server = server.ok_or_else(|| AppError::bad_request("assign a game server to this instance before using faction upgrades"))?;
    let economy: i64 = sqlx::query_scalar("SELECT COALESCE((SELECT MIN(b.id) FROM game_servers b WHERE b.economy_group<>'' AND b.economy_group=a.economy_group COLLATE NOCASE),a.id) FROM game_servers a WHERE a.id=?")
        .bind(server).fetch_one(&mut *conn).await?;
    Ok((server, economy))
}

async fn smp(state: &AppState) -> AppResult<velora_core::Policy> {
    let policy = velora_core::load(state).await?.ok_or_else(|| AppError::not_found("faction upgrades belong to Velora SMP"))?;
    if !policy.enabled("factions") || !policy.enabled("economy") { return Err(AppError::forbidden("factions or economy is disabled")); }
    Ok(policy)
}

async fn view(state: &AppState, policy: &velora_core::Policy, guild: &str) -> AppResult<Value> {
    let mut conn = state.db.acquire().await?;
    let (_, economy) = scope(&mut conn, guild).await?;
    factions_core::upgrades_view(&mut conn, policy, guild, economy).await
}

async fn buy(state: &AppState, policy: &velora_core::Policy, guild: &str, actor: &str, p: &Buy) -> AppResult<Value> {
    if !guild_can(state, guild, actor, "manage").await? { return Err(AppError::forbidden("only faction leaders and officers can buy upgrades")); }
    if let Some(expected) = p.expected_price_cents {
        let (price, _) = factions_core::track_terms(policy, &p.track).ok_or_else(|| AppError::bad_request("unknown upgrade"))?;
        if expected != price { return Err(AppError::conflict("the upgrade price changed; review it again")); }
    }
    let mut tx = state.db.begin().await?;
    sqlx::query("UPDATE guilds SET id=id WHERE id=?").bind(guild).execute(&mut *tx).await?;
    let (server, economy) = scope(&mut tx, guild).await?;
    let tiers = factions_core::buy_upgrade(&mut tx, policy, guild, server, economy, actor, &p.track, p.expected_tiers).await?;
    tx.commit().await?;
    Ok(json!({"ok":true,"track":p.track,"tiers":tiers,"upgrades":view(state, policy, guild).await?}))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Buy { pub track: String, pub expected_tiers: i64, pub expected_price_cents: Option<i64> }

pub async fn get_upgrades(auth: AuthUser, Path(id): Path<String>, State(state): State<AppState>) -> AppResult<Json<Value>> {
    let policy = smp(&state).await?;
    let member: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM guild_members WHERE guild_id=? AND uuid=?)").bind(&id).bind(&auth.uuid).fetch_one(&state.db).await?;
    if !member { return Err(AppError::forbidden("faction membership is required")); }
    Ok(Json(view(&state, &policy, &id).await?))
}

pub async fn buy_upgrade(auth: AuthUser, Path(id): Path<String>, State(state): State<AppState>, Json(p): Json<Buy>) -> AppResult<Json<Value>> {
    let policy = smp(&state).await?;
    Ok(Json(buy(&state, &policy, &id, &auth.uuid, &p).await?))
}

#[derive(Deserialize)]
pub struct GameUpgrade { pub uuid: String, pub track: Option<String>, pub expected_tiers: Option<i64>, pub expected_price_cents: Option<i64> }

/// `/f upgrade [track]`: list without a track, otherwise buy the next tier the player just saw.
pub async fn server_upgrades(GameServer(server): GameServer, State(state): State<AppState>, Json(p): Json<GameUpgrade>) -> AppResult<Json<Value>> {
    let policy = smp(&state).await?;
    let member = super::guild_bank::membership(&mut *state.db.acquire().await?, &server, &p.uuid).await?
        .ok_or_else(|| AppError::bad_request("join a faction first"))?;
    match (p.track, p.expected_tiers) {
        (Some(track), Some(expected_tiers)) => Ok(Json(buy(&state, &policy, &member.guild_id, &p.uuid, &Buy { track, expected_tiers, expected_price_cents: p.expected_price_cents }).await?)),
        (None, _) => Ok(Json(view(&state, &policy, &member.guild_id).await?)),
        (Some(_), None) => Err(AppError::bad_request("expected_tiers is required to buy")),
    }
}
