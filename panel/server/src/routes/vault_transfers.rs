//! Durable custody protocol: prepare holds a vault until the originating game server reads its player-data checkpoint.
//! Prepared transfers never expire: elapsed time cannot tell whether Minecraft persisted the inventory change.
use super::{
    servers::GameServer,
    vaults::{clean, Stack},
};
use crate::{
    error::{AppError, AppResult},
    state::{AppState, RequestState as State},
};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::Digest;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Prepare {
    pub id: String,
    pub uuid: String,
    pub key: String,
    pub number: i64,
    pub revision: i64,
    pub lease: String,
    pub contents: Vec<Stack>,
    #[serde(default)]
    pub permitted: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Finish {
    pub id: String,
    pub uuid: String,
    pub commit: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Player {
    pub uuid: String,
}

pub async fn prepare(GameServer(server): GameServer, State(state): State<AppState>, Json(p): Json<Prepare>) -> AppResult<Json<Value>> {
    uuid::Uuid::parse_str(&p.id).map_err(|_| AppError::bad_request("invalid transfer identity"))?;
    uuid::Uuid::parse_str(&p.uuid).map_err(|_| AppError::bad_request("invalid player identity"))?;
    let raw = clean(&p.contents)?;
    let owner = if p.key == format!("player:{}", p.uuid) {
        "player"
    } else if p.key.starts_with("faction:") {
        "faction"
    } else if p.key.starts_with("shop:") {
        p.key.as_str()
    } else {
        return Err(AppError::forbidden("invalid transfer owner"));
    };
    if super::vaults::authorize(&state, &server.instance_id, &p.uuid, owner, p.number, p.permitted).await? != p.key {
        return Err(AppError::forbidden("vault belongs to another player or faction"));
    }
    let fingerprint = format!("{:x}", sha2::Sha256::digest(serde_json::to_vec(&p)?));
    let mut tx = state.db.begin().await?;
    sqlx::query("UPDATE game_servers SET id=id WHERE id=?").bind(server.id).execute(&mut *tx).await?;
    let previous: Option<(i64, String, String, String)> =
        sqlx::query_as("SELECT server_id,uuid,fingerprint,status FROM vault_transfers WHERE id=?")
            .bind(&p.id)
            .fetch_optional(&mut *tx)
            .await?;
    if let Some(row) = previous {
        if row.0 != server.id || row.1 != p.uuid || row.2 != fingerprint {
            return Err(AppError::conflict("transfer identity was reused"));
        }
        return Ok(Json(json!({"id":p.id,"status":row.3})));
    }
    let before:Option<String>=sqlx::query_scalar("UPDATE cloud_vaults SET lease_until='9999-12-31T23:59:59Z' WHERE owner=? AND number=? AND revision=? AND lease_server=? AND lease_token=? AND lease_until>? RETURNING contents")
        .bind(&p.key).bind(p.number).bind(p.revision).bind(server.id).bind(&p.lease).bind(crate::db::now()).fetch_optional(&mut *tx).await?;
    let before = before.ok_or_else(|| AppError::conflict("vault lease or revision changed"))?;
    let pending: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM vault_transfers WHERE status='prepared' AND ((owner=? AND number=?) OR (server_id=? AND uuid=?)))",
    )
    .bind(&p.key)
    .bind(p.number)
    .bind(server.id)
    .bind(&p.uuid)
    .fetch_one(&mut *tx)
    .await?;
    if pending {
        return Err(AppError::conflict("resolve the pending inventory transfer first"));
    }
    sqlx::query("INSERT INTO vault_transfers(id,server_id,uuid,owner,number,revision,lease,before_contents,after_contents,fingerprint,created_at) VALUES(?,?,?,?,?,?,?,?,?,?,?)")
        .bind(&p.id).bind(server.id).bind(&p.uuid).bind(&p.key).bind(p.number).bind(p.revision).bind(&p.lease).bind(before).bind(raw).bind(fingerprint).bind(crate::db::now()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(Json(json!({"id":p.id,"status":"prepared"})))
}
pub async fn finish(GameServer(server): GameServer, State(state): State<AppState>, Json(p): Json<Finish>) -> AppResult<Json<Value>> {
    let mut tx = state.db.begin().await?;
    sqlx::query("UPDATE game_servers SET id=id WHERE id=?").bind(server.id).execute(&mut *tx).await?;
    let row:Option<(String,i64,i64,String,String,String,String)>=sqlx::query_as("SELECT owner,number,revision,lease,before_contents,after_contents,status FROM vault_transfers WHERE id=? AND server_id=? AND uuid=?")
        .bind(&p.id).bind(server.id).bind(&p.uuid).fetch_optional(&mut *tx).await?;
    let (owner, number, revision, lease, before, after, status) =
        row.ok_or_else(|| AppError::not_found("transfer does not belong to this server and player"))?;
    let desired = if p.commit { "committed" } else { "cancelled" };
    if status != "prepared" && status != desired {
        return Err(AppError::conflict("transfer was already resolved with a different outcome"));
    }
    if status == "prepared" {
        let changed=sqlx::query("UPDATE cloud_vaults SET contents=?,revision=revision+?,lease_until=?,updated_at=? WHERE owner=? AND number=? AND revision=? AND lease_server=? AND lease_token=?")
            .bind(if p.commit{&after}else{&before}).bind(i64::from(p.commit))
            .bind((chrono::Utc::now()+chrono::Duration::seconds(120)).to_rfc3339_opts(chrono::SecondsFormat::Secs,true)).bind(crate::db::now())
            .bind(&owner).bind(number).bind(revision).bind(server.id).bind(lease).execute(&mut *tx).await?.rows_affected();
        if changed != 1 {
            return Err(AppError::conflict("custody checkpoint no longer matches the vault; operator recovery required"));
        }
        sqlx::query("UPDATE vault_transfers SET status=?,resolved_at=? WHERE id=?")
            .bind(desired)
            .bind(crate::db::now())
            .bind(&p.id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(Json(json!({"id":p.id,"status":desired,"revision":revision+i64::from(p.commit)})))
}
pub async fn pending(GameServer(server): GameServer, State(state): State<AppState>, Json(p): Json<Player>) -> AppResult<Json<Value>> {
    let ids: Vec<String> =
        sqlx::query_scalar("SELECT id FROM vault_transfers WHERE server_id=? AND uuid=? AND status='prepared' ORDER BY created_at")
            .bind(server.id)
            .bind(p.uuid)
            .fetch_all(&state.db)
            .await?;
    Ok(Json(json!({"pending":ids})))
}
