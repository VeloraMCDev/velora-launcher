//! Purchased, single-use outpost flags. Claims share the faction's normal capacity and upkeep.
use super::{guilds::guild_can, servers::GameServer};
use crate::{error::{AppError, AppResult}, state::{AppState, RequestState as State}, velora_core};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

pub const MAX_OUTPOSTS: i64 = 3;
pub const MAX_CHUNKS: i64 = 12;
pub const RADIUS: i64 = 3;

pub async fn issue(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, guild: &str, server: i64, actor: &str) -> AppResult<String> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM faction_outposts WHERE guild_id=?").bind(guild).fetch_one(&mut **tx).await?;
    if count >= MAX_OUTPOSTS { return Err(AppError::bad_request("all three faction outpost flags have been purchased")); }
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO faction_outposts(id,guild_id,purchased_at) VALUES(?,?,?)").bind(&id).bind(guild).bind(crate::db::now()).execute(&mut **tx).await?;
    // Exact server-owned serialization is delivered through the existing cloud mailbox.
    let data = format!("{{id:\"minecraft:white_banner\",Count:1b,tag:{{VeloraOutpostId:\"{id}\",display:{{Name:'{{\"text\":\"Outpost Flag\",\"italic\":false}}'}}}}}}");
    sqlx::query("INSERT INTO market_mailbox(server_id,uuid,item_id,item_name,amount,item_data,note,created_at,to_vault) VALUES(?,?,'minecraft:white_banner','Outpost Flag',1,?,'faction outpost',?,1)")
        .bind(server).bind(actor).bind(data).bind(crate::db::now()).execute(&mut **tx).await?;
    Ok(id)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Place { pub uuid: String, pub id: String, pub dimension: String, pub x: i32, pub y: i32, pub z: i32 }

pub async fn place(GameServer(server): GameServer, State(state): State<AppState>, Json(p): Json<Place>) -> AppResult<Json<Value>> {
    let policy = velora_core::load(&state).await?.ok_or_else(|| AppError::not_found("outposts belong to Velora SMP"))?;
    if !policy.enabled("factions") || !policy.enabled("economy") { return Err(AppError::forbidden("factions or economy is disabled")); }
    uuid::Uuid::parse_str(&p.id).map_err(|_| AppError::bad_request("invalid flag identity"))?;
    if p.dimension.is_empty() || p.dimension.len()>128 || !p.dimension.contains(':') || p.x.unsigned_abs()>30_000_000 || p.z.unsigned_abs()>30_000_000 || !(-2048..=2048).contains(&p.y) {
        return Err(AppError::bad_request("invalid flag location"));
    }
    let guild: Option<String> = sqlx::query_scalar("SELECT guild_id FROM guild_members m JOIN guilds g ON g.id=m.guild_id WHERE m.uuid=? AND g.instance_id=?")
        .bind(&p.uuid).bind(&server.instance_id).fetch_optional(&state.db).await?;
    let guild = guild.ok_or_else(|| AppError::forbidden("join the flag's faction first"))?;
    if !guild_can(&state,&guild,&p.uuid,"manage").await? { return Err(AppError::forbidden("your faction role cannot place outposts")); }
    let mut tx = state.db.begin().await?;
    // Serialize competing placement/claim requests before examining the anchor.
    sqlx::query("UPDATE guilds SET id=id WHERE id=?").bind(&guild).execute(&mut *tx).await?;
    let row: Option<(String,Option<i64>,Option<String>,Option<i32>,Option<i32>,Option<i32>)> = sqlx::query_as("SELECT status,server_id,dimension,x,y,z FROM faction_outposts WHERE id=? AND guild_id=?")
        .bind(&p.id).bind(&guild).fetch_optional(&mut *tx).await?;
    let row = row.ok_or_else(|| AppError::forbidden("this flag does not belong to your faction"))?;
    if row.0 == "placed" {
        if row.1!=Some(server.id) || row.2.as_deref()!=Some(&p.dimension) || row.3!=Some(p.x) || row.4!=Some(p.y) || row.5!=Some(p.z) {
            return Err(AppError::conflict("this flag has already been placed elsewhere"));
        }
        return Ok(Json(json!({"ok":true,"id":p.id,"replayed":true})));
    }
    crate::factions_core::upkeep_allowed(&mut tx,&policy,&guild).await?;
    let occupied: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM guild_claims WHERE server_id=? AND dimension=? AND chunk_x=? AND chunk_z=?) OR EXISTS(SELECT 1 FROM admin_claim_chunks WHERE server_id=? AND dimension=? AND chunk_x=? AND chunk_z=?)")
        .bind(server.id).bind(&p.dimension).bind(p.x.div_euclid(16)).bind(p.z.div_euclid(16))
        .bind(server.id).bind(&p.dimension).bind(p.x.div_euclid(16)).bind(p.z.div_euclid(16)).fetch_one(&mut *tx).await?;
    if occupied { return Err(AppError::bad_request("place the outpost flag in unclaimed land")); }
    let nearby: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM faction_outposts WHERE status='placed' AND server_id=? AND dimension=? AND ABS((CASE WHEN x<0 THEN (x-15)/16 ELSE x/16 END)-?)<=6 AND ABS((CASE WHEN z<0 THEN (z-15)/16 ELSE z/16 END)-?)<=6)")
        .bind(server.id).bind(&p.dimension).bind(p.x.div_euclid(16)).bind(p.z.div_euclid(16)).fetch_one(&mut *tx).await?;
    if nearby { return Err(AppError::bad_request("another outpost area overlaps this location")); }
    sqlx::query("UPDATE faction_outposts SET status='placed',server_id=?,dimension=?,x=?,y=?,z=?,placed_at=? WHERE id=?")
        .bind(server.id).bind(&p.dimension).bind(p.x).bind(p.y).bind(p.z).bind(crate::db::now()).bind(&p.id).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(Json(json!({"ok":true,"id":p.id,"max_chunks":MAX_CHUNKS,"radius_chunks":RADIUS})))
}

/// Resolve a remote claim to an owned flag. All paths use the same transaction as inserting the claim.
pub async fn claim_anchor(conn: &mut sqlx::SqliteConnection, guild: &str, server: i64, dimension: &str, x: i32, z: i32) -> AppResult<Option<String>> {
    if x.unsigned_abs()>1_875_000 || z.unsigned_abs()>1_875_000 { return Err(AppError::bad_request("chunk is outside the world border")); }
    let anchors: Vec<(String,i32,i32)> = sqlx::query_as("SELECT id,x,z FROM faction_outposts WHERE guild_id=? AND status='placed' AND server_id=? AND dimension=? ORDER BY placed_at,id")
        .bind(guild).bind(server).bind(dimension).fetch_all(&mut *conn).await?;
    for (id,bx,bz) in anchors {
        let (cx,cz)=(bx.div_euclid(16),bz.div_euclid(16));
        if (i64::from(cx)-i64::from(x)).abs()>RADIUS || (i64::from(cz)-i64::from(z)).abs()>RADIUS { continue; }
        let count: i64=sqlx::query_scalar("SELECT COUNT(*) FROM guild_claims WHERE outpost_id=?").bind(&id).fetch_one(&mut *conn).await?;
        if count>=MAX_CHUNKS { return Err(AppError::bad_request("this outpost already has 12 claimed chunks")); }
        let connected:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM guild_claims WHERE outpost_id=? AND ABS(chunk_x-?)+ABS(chunk_z-?)=1)")
            .bind(&id).bind(x).bind(z).fetch_one(&mut *conn).await?;
        if !connected && (x!=cx || z!=cz) { return Err(AppError::bad_request("claim the flag's chunk first, then adjacent chunks")); }
        return Ok(Some(id));
    }
    // One home territory. Retained legacy disconnected claims remain protected but do not unlock new remote territory.
    let rows:Vec<(i64,String,i64,i32,i32)>=sqlx::query_as("SELECT server_id,dimension,id,chunk_x,chunk_z FROM guild_claims WHERE guild_id=? AND outpost_id IS NULL ORDER BY id")
        .bind(guild).fetch_all(&mut *conn).await?;
    if rows.is_empty() { return Ok(None); }
    let first=&rows[0];
    if first.0==server && first.1==dimension {
        let mut connected=std::collections::HashSet::from([(first.3,first.4)]);
        loop {
            let before=connected.len();
            for row in &rows {
                if row.0==server && row.1==dimension && [(row.3.saturating_sub(1),row.4),(row.3.saturating_add(1),row.4),(row.3,row.4.saturating_sub(1)),(row.3,row.4.saturating_add(1))].iter().any(|p| connected.contains(p)) { connected.insert((row.3,row.4)); }
            }
            if before==connected.len() { break; }
        }
        if [(x-1,z),(x+1,z),(x,z-1),(x,z+1)].iter().any(|p|connected.contains(p)) { return Ok(None); }
    }
    Err(AppError::bad_request("remote land needs a placed outpost flag from the faction market"))
}
