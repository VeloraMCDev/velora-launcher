//! Claim-owned pedestal shops. The stock chest is a portal to durable shop custody, never a second item store.
use super::{
    economy::{begin_operation, finish_operation},
    ledger::{credit, debit, Account},
    servers::GameServer,
    vaults::Stack,
};
use crate::{
    error::{AppError, AppResult},
    state::{AppState, RequestState as State},
};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Serialize, sqlx::FromRow)]
pub struct Shop {
    pub id: String,
    pub server_id: i64,
    pub owner_uuid: String,
    pub dimension: String,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub chest_x: i32,
    pub chest_y: i32,
    pub chest_z: i32,
    pub item_id: String,
    pub item_name: String,
    pub fingerprint: String,
    #[serde(skip_serializing)]
    pub display_data: String,
    pub quantity: i64,
    pub price_cents: i64,
    pub enabled: bool,
    pub promoted_until: Option<String>,
    pub warp_enabled: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Create {
    pub id: String,
    pub uuid: String,
    pub dimension: String,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub chest_x: i32,
    pub chest_y: i32,
    pub chest_z: i32,
    pub item_id: String,
    pub item_name: String,
    pub fingerprint: String,
    pub display_data: String,
    pub quantity: i64,
    pub price_cents: i64,
}
async fn policy(state: &AppState) -> AppResult<crate::velora_core::Policy> {
    let policy = crate::velora_core::load(state).await?.ok_or_else(|| AppError::forbidden("physical shops require Velora SMP"))?;
    if !policy.enabled("economy") || !policy.enabled("vaults") || (policy.shop_requires_claim && !policy.enabled("factions")) {
        return Err(AppError::forbidden("physical shop modules are disabled"));
    }
    Ok(policy)
}
async fn owned_claim(tx: &mut super::ledger::Tx, server: i64, uuid: &str, dimension: &str, x: i32, z: i32) -> AppResult<bool> {
    Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM guild_claims c JOIN guild_members m ON m.guild_id=c.guild_id WHERE c.server_id=? AND c.dimension=? AND c.chunk_x=? AND c.chunk_z=? AND m.uuid=?)")
        .bind(server).bind(dimension).bind(x.div_euclid(16)).bind(z.div_euclid(16)).bind(uuid).fetch_one(&mut **tx).await?)
}
pub async fn create(GameServer(server): GameServer, State(state): State<AppState>, Json(p): Json<Create>) -> AppResult<Json<Value>> {
    let policy = policy(&state).await?;
    uuid::Uuid::parse_str(&p.id).map_err(|_| AppError::bad_request("invalid shop identity"))?;
    uuid::Uuid::parse_str(&p.uuid).map_err(|_| AppError::bad_request("invalid seller identity"))?;
    if !(1..=64).contains(&p.quantity)
        || !(1..=100_000_000_000).contains(&p.price_cents)
        || p.dimension.len() > 128
        || p.fingerprint.len() != 64
        || !p.fingerprint.bytes().all(|b| b.is_ascii_hexdigit())
        || p.display_data.len() > 1024 * 1024
        || p.item_name.chars().count() > 128
        || !p.item_id.contains(':')
        || p.item_id.len() > 128
        || (i64::from(p.x) - i64::from(p.chest_x)).abs()
            + (i64::from(p.y) - i64::from(p.chest_y)).abs()
            + (i64::from(p.z) - i64::from(p.chest_z)).abs()
            > 2
    {
        return Err(AppError::bad_request("invalid pedestal, chest, item or price"));
    }
    let mut tx = state.db.begin().await?;
    sqlx::query("UPDATE game_servers SET id=id WHERE id=?").bind(server.id).execute(&mut *tx).await?;
    if policy.shop_requires_claim
        && (!owned_claim(&mut tx, server.id, &p.uuid, &p.dimension, p.x, p.z).await?
            || !owned_claim(&mut tx, server.id, &p.uuid, &p.dimension, p.chest_x, p.chest_z).await?)
    {
        return Err(AppError::forbidden("the pedestal and stock chest must be inside your faction's claims"));
    }
    let exists:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM physical_shops WHERE id=? OR (server_id=? AND dimension=? AND ((x=? AND y=? AND z=?) OR (chest_x=? AND chest_y=? AND chest_z=?))))")
        .bind(&p.id).bind(server.id).bind(&p.dimension).bind(p.x).bind(p.y).bind(p.z).bind(p.chest_x).bind(p.chest_y).bind(p.chest_z).fetch_one(&mut *tx).await?;
    if exists {
        return Err(AppError::conflict("this shop identity, pedestal or stock chest is already registered"));
    }
    sqlx::query("INSERT INTO physical_shops(id,server_id,owner_uuid,dimension,x,y,z,chest_x,chest_y,chest_z,item_id,item_name,fingerprint,display_data,quantity,price_cents,created_at) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)")
        .bind(&p.id).bind(server.id).bind(&p.uuid).bind(&p.dimension).bind(p.x).bind(p.y).bind(p.z).bind(p.chest_x).bind(p.chest_y).bind(p.chest_z)
        .bind(&p.item_id).bind(&p.item_name).bind(&p.fingerprint).bind(&p.display_data).bind(p.quantity).bind(p.price_cents).bind(crate::db::now()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(Json(json!({"ok":true,"id":p.id})))
}
pub async fn index(GameServer(server): GameServer, State(state): State<AppState>) -> AppResult<Json<Value>> {
    let policy = policy(&state).await?;
    let shops: Vec<Shop> = sqlx::query_as("SELECT * FROM physical_shops WHERE server_id=? ORDER BY created_at LIMIT 1000")
        .bind(server.id)
        .fetch_all(&state.db)
        .await?;
    // Exact NBT is only exposed to the authenticated game server, for its display entities.
    let values: Vec<Value> = shops
        .into_iter()
        .map(|s| {
            let data = s.display_data.clone();
            let mut v = serde_json::to_value(s).expect("shop serializes");
            v["display_data"] = json!(data);
            v
        })
        .collect();
    Ok(Json(
        json!({"shops":values,"promotion_cents_per_day":policy.shop_promotion_cents_per_day,"requires_claim":policy.shop_requires_claim}),
    ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Purchase {
    pub uuid: String,
    pub username: String,
    pub id: String,
    pub operation_id: String,
    pub expected_price_cents: i64,
}
pub async fn buy(GameServer(server): GameServer, State(state): State<AppState>, Json(p): Json<Purchase>) -> AppResult<Json<Value>> {
    let policy = policy(&state).await?;
    uuid::Uuid::parse_str(&p.operation_id).map_err(|_| AppError::bad_request("invalid purchase identity"))?;
    let operation = format!("physical:{}:{}", p.uuid, p.operation_id);
    let (mut tx, receipt) = begin_operation(&state, server.id, &operation).await?;
    if let Some(receipt) = receipt {
        if receipt["shop_id"] != p.id || receipt["price_cents"] != p.expected_price_cents {
            return Err(AppError::conflict("purchase identity was reused"));
        }
        return Ok(Json(receipt));
    }
    let shop: Shop = sqlx::query_as("SELECT * FROM physical_shops WHERE id=? AND server_id=? AND enabled=1")
        .bind(&p.id)
        .bind(server.id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::not_found("shop unavailable"))?;
    if shop.owner_uuid == p.uuid {
        return Err(AppError::bad_request("open your stock chest to withdraw stock"));
    }
    if shop.price_cents != p.expected_price_cents {
        return Err(AppError::conflict("price changed; review this shop again"));
    }
    if policy.shop_requires_claim
        && (!owned_claim(&mut tx, server.id, &shop.owner_uuid, &shop.dimension, shop.x, shop.z).await?
            || !owned_claim(&mut tx, server.id, &shop.owner_uuid, &shop.dimension, shop.chest_x, shop.chest_z).await?)
    {
        return Err(AppError::forbidden("seller no longer owns this claim"));
    }
    let key = format!("shop:{}", shop.id);
    let stock:Option<String>=sqlx::query_scalar("UPDATE cloud_vaults SET updated_at=updated_at WHERE owner=? AND number=1 AND (lease_until IS NULL OR lease_until<=?) AND NOT EXISTS(SELECT 1 FROM vault_transfers WHERE owner=? AND status='prepared') RETURNING contents")
        .bind(&key).bind(crate::db::now()).bind(&key).fetch_optional(&mut *tx).await?;
    let stock = stock.ok_or_else(|| AppError::conflict("stock chest is empty or being edited; close it and try again"))?;
    let mut stock: Vec<Stack> = serde_json::from_str(&stock)?;
    let available: u32 = stock.iter().filter(|s| s.item == shop.item_id && s.fingerprint == shop.fingerprint).map(|s| s.count).sum();
    if available < shop.quantity as u32 {
        return Err(AppError::conflict("this shop is out of stock"));
    }
    let mut remaining = shop.quantity as u32;
    for stack in &mut stock {
        if stack.item != shop.item_id || stack.fingerprint != shop.fingerprint || remaining == 0 {
            continue;
        }
        let take = remaining.min(stack.count);
        stack.count -= take;
        remaining -= take;
    }
    stock.retain(|s| s.count > 0);
    let amount = shop.price_cents as f64 / 100.0;
    let balance = debit(
        &mut tx,
        server.economy_id,
        server.id,
        &p.uuid,
        &p.username,
        amount,
        Account("market", "Physical shop custody"),
        "Physical shop purchase",
    )
    .await?;
    credit(
        &mut tx,
        server.economy_id,
        server.id,
        &shop.owner_uuid,
        "Shop seller",
        amount,
        Account("market", "Physical shop custody"),
        "Physical shop sale (no fee)",
    )
    .await?;
    sqlx::query("UPDATE cloud_vaults SET contents=?,revision=revision+1,updated_at=? WHERE owner=? AND number=1")
        .bind(super::vaults::clean(&stock)?)
        .bind(crate::db::now())
        .bind(&key)
        .execute(&mut *tx)
        .await?;
    let delivery:i64=sqlx::query_scalar("INSERT INTO market_mailbox(server_id,uuid,item_id,item_name,amount,item_data,note,created_at,to_vault) VALUES(?,?,?,?,?,?,'physical_shop',?,1) RETURNING id")
        .bind(server.id).bind(&p.uuid).bind(&shop.item_id).bind(&shop.item_name).bind(shop.quantity).bind(&shop.display_data).bind(crate::db::now()).fetch_one(&mut *tx).await?;
    finish_operation(
        tx,
        server.id,
        &operation,
        json!({"ok":true,"shop_id":p.id,"price_cents":shop.price_cents,"balance":balance,"delivery_id":delivery,"to_vault":true}),
    )
    .await
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Promote {
    pub uuid: String,
    pub username: String,
    pub id: String,
    pub days: i64,
    pub warp: bool,
    pub expected_price_cents: i64,
    pub operation_id: String,
}
pub async fn promote(GameServer(server): GameServer, State(state): State<AppState>, Json(p): Json<Promote>) -> AppResult<Json<Value>> {
    let policy = policy(&state).await?;
    if !(1..=30).contains(&p.days) || uuid::Uuid::parse_str(&p.operation_id).is_err() {
        return Err(AppError::bad_request("promotion requires 1-30 days and a purchase identity"));
    }
    let operation = format!("shop-promotion:{}:{}", p.uuid, p.operation_id);
    let (mut tx, receipt) = begin_operation(&state, server.id, &operation).await?;
    if let Some(receipt) = receipt {
        if receipt["shop_id"] != p.id
            || receipt["days"] != p.days
            || receipt["warp"] != p.warp
            || receipt["price_cents"] != p.expected_price_cents
        {
            return Err(AppError::conflict("promotion identity was reused"));
        }
        return Ok(Json(receipt));
    }
    let shop: Shop = sqlx::query_as("SELECT * FROM physical_shops WHERE id=? AND server_id=? AND owner_uuid=?")
        .bind(&p.id)
        .bind(server.id)
        .bind(&p.uuid)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::forbidden("only the seller can promote a shop"))?;
    let price = policy.shop_promotion_cents_per_day.checked_mul(p.days).ok_or_else(|| AppError::bad_request("promotion price overflow"))?;
    if price != p.expected_price_cents {
        return Err(AppError::conflict("promotion price changed; review it again"));
    }
    let now = chrono::Utc::now();
    let start = shop
        .promoted_until
        .as_deref()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.with_timezone(&chrono::Utc))
        .filter(|d| *d > now)
        .unwrap_or(now);
    let until = (start + chrono::Duration::days(p.days)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let balance = if price > 0 {
        debit(
            &mut tx,
            server.economy_id,
            server.id,
            &p.uuid,
            &p.username,
            price as f64 / 100.0,
            Account("server", "Shop directory"),
            "Physical shop promotion",
        )
        .await?
    } else {
        super::economy::ensure_balance(&mut tx, server.economy_id, &p.uuid, &p.username).await?;
        sqlx::query_scalar::<_, f64>("SELECT balance FROM server_economy WHERE server_id=? AND uuid=?")
            .bind(server.economy_id)
            .bind(&p.uuid)
            .fetch_one(&mut *tx)
            .await?
    };
    sqlx::query("UPDATE physical_shops SET promoted_until=?,warp_enabled=? WHERE id=?")
        .bind(&until)
        .bind(p.warp)
        .bind(&p.id)
        .execute(&mut *tx)
        .await?;
    finish_operation(
        tx,
        server.id,
        &operation,
        json!({"ok":true,"shop_id":p.id,"days":p.days,"warp":p.warp,"price_cents":price,"promoted_until":until,"balance":balance}),
    )
    .await
}
/// Map listing respects module/layer filtering at its caller. No item serialization reaches a player.
pub async fn pins(state: &AppState, server: i64) -> AppResult<Vec<Value>> {
    let rows: Vec<(String, String, String, i32, i32, i32, bool)> = sqlx::query_as(
        "SELECT id,item_name,dimension,x,y,z,warp_enabled FROM physical_shops WHERE server_id=? AND enabled=1 AND promoted_until>?",
    )
    .bind(server)
    .bind(crate::db::now())
    .fetch_all(&state.db)
    .await?;
    Ok(rows.into_iter().map(|(id,name,dimension,x,y,z,warp)|json!({"id":format!("physical:{id}"),"kind":"shop","label":name,"dimension":dimension,"x":x,"y":y,"z":z,"shop_id":id,"warp":warp})).collect())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Update {
    pub uuid: String,
    pub id: String,
    pub price_cents: i64,
    pub enabled: bool,
}
pub async fn update(GameServer(server): GameServer, State(state): State<AppState>, Json(p): Json<Update>) -> AppResult<Json<Value>> {
    policy(&state).await?;
    if !(1..=100_000_000_000).contains(&p.price_cents) {
        return Err(AppError::bad_request("invalid shop price"));
    }
    let changed = sqlx::query("UPDATE physical_shops SET price_cents=?,enabled=? WHERE id=? AND server_id=? AND owner_uuid=?")
        .bind(p.price_cents)
        .bind(p.enabled)
        .bind(&p.id)
        .bind(server.id)
        .bind(p.uuid)
        .execute(&state.db)
        .await?
        .rows_affected();
    if changed != 1 {
        return Err(AppError::forbidden("only the seller can update this shop"));
    }
    Ok(Json(json!({"ok":true})))
}
