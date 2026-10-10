//! Admin-owned Darknet catalog. Every purchase is charged and queued in one receipt transaction.
use crate::{auth::{AdminUser, AuthUser}, error::{AppError, AppResult}, state::{AppState, RequestState as State}, store};
use super::{economy::{begin_operation, finish_operation}, ledger::debit, servers::{economy_scope, get_server, GameServer, ServerRow}};
use axum::{extract::Path, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Product {
    pub id: String,
    pub item_id: String,
    pub item_name: String,
    pub amount: i32,
    pub price_cents: i64,
    pub enabled: bool,
}

fn validate(products: &[Product]) -> AppResult<()> {
    let mut ids = std::collections::HashSet::new();
    if products.len() > 200 { return Err(AppError::bad_request("at most 200 Darknet products")); }
    for p in products {
        if p.id.is_empty() || p.id.starts_with("velora-vault-") || p.id.len() > 64 || !p.id.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-')) || !ids.insert(&p.id)
            || p.item_id.len() > 128 || !p.item_id.contains(':') || !p.item_id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, ':' | '_' | '-' | '.' | '/'))
            || p.item_name.trim().is_empty() || p.item_name.len() > 100 || !(1..=64).contains(&p.amount) || !(1..=100_000_000_000).contains(&p.price_cents) {
            return Err(AppError::bad_request("invalid or duplicate Darknet product; use a registered item ID, 1-64 items and a positive price in cents"));
        }
    }
    Ok(())
}

pub async fn admin_get(_: AdminUser, State(state): State<AppState>) -> AppResult<Json<Vec<Product>>> {
    Ok(Json(store::kv_get(&state, "velora_core_darknet").await?))
}
pub async fn admin_put(_: AdminUser, State(state): State<AppState>, Json(products): Json<Vec<Product>>) -> AppResult<Json<Vec<Product>>> {
    validate(&products)?; store::kv_set(&state, "velora_core_darknet", &products).await?; Ok(Json(products))
}
async fn catalog(state: &AppState, server: &ServerRow, uuid: &str) -> AppResult<Json<Value>> {
    let products: Vec<Product> = store::kv_get(state, "velora_core_darknet").await?;
    let balance: Option<f64> = sqlx::query_scalar("SELECT balance FROM server_economy WHERE server_id=? AND uuid=?")
        .bind(server.economy_id).bind(uuid).fetch_optional(&state.db).await?;
    let mut products:Vec<Value>=products.into_iter().filter(|p|p.enabled).map(|p|serde_json::to_value(p).expect("product serializes")).collect();
    if let Some(policy)=crate::velora_core::load(state).await?.filter(|p|p.enabled("economy") && p.enabled("vaults")) {
        let settings=super::utilities::load(state).await?.vault;
        let owned:Vec<i64>=sqlx::query_scalar("SELECT number FROM vault_entitlements WHERE uuid=? UNION SELECT number FROM cloud_vaults WHERE owner=? AND revision>0")
            .bind(uuid).bind(format!("player:{uuid}")).fetch_all(&state.db).await?;
        if settings.enabled {
            if let Some(number)=((settings.free_count+1)..=settings.count).find(|n|!owned.contains(n)) {
                products.push(json!({"id":format!("velora-vault-{number}"),"item_id":"minecraft:chest","item_name":format!("Vault {number} unlock"),"amount":1,"price_cents":policy.vault_price_cents,"vault_number":number}));
            }
        }
    }
    Ok(Json(json!({"products":products,"balance":balance,"delivery":"vault"})))
}
pub async fn player_catalog(auth: AuthUser, State(state): State<AppState>, Path(sid): Path<i64>) -> AppResult<Json<Value>> {
    let mut server = get_server(&state, sid).await?; server.economy_id = economy_scope(&state.db, sid).await?;
    catalog(&state, &server, &auth.uuid).await
}
#[derive(Deserialize)]
pub struct Who { pub uuid: String }
pub async fn game_catalog(GameServer(server): GameServer, State(state): State<AppState>, Json(who): Json<Who>) -> AppResult<Json<Value>> {
    catalog(&state, &server, &who.uuid).await
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Purchase { pub operation_id: String, pub product_id: String, pub expected_price_cents: Option<i64> }
#[derive(Deserialize)]
pub struct GamePurchase { pub uuid: String, pub username: String, pub operation_id: String, pub product_id: String, pub expected_price_cents: Option<i64> }

async fn buy(state: &AppState, server: &ServerRow, uuid: &str, name: &str, purchase: Purchase) -> AppResult<Json<Value>> {
    if let Some(policy) = crate::velora_core::load(state).await? {
        if !policy.enabled("economy") || !policy.enabled("vaults") { return Err(AppError::forbidden("Darknet purchases require economy and vaults")); }
    }
    // Caller-specific IDs prevent a player from replaying another player's receipt.
    if purchase.operation_id.len() > 64 || uuid::Uuid::parse_str(&purchase.operation_id).is_err() {
        return Err(AppError::bad_request("operation_id must be a UUID"));
    }
    if let Some(number)=purchase.product_id.strip_prefix("velora-vault-") {
        let number=number.parse::<i64>().map_err(|_|AppError::bad_request("invalid vault product"))?;
        if purchase.product_id!=format!("velora-vault-{number}") { return Err(AppError::bad_request("invalid vault product")); }
        let price=purchase.expected_price_cents.ok_or_else(||AppError::bad_request("review the vault price before buying"))?;
        return super::vaults::buy(state,server,uuid,name,super::vaults::BuyVault{number,expected_price_cents:price,operation_id:purchase.operation_id}).await;
    }
    let operation = format!("darknet:{uuid}:{}", purchase.operation_id);
    let (mut tx, previous) = begin_operation(state, server.id, &operation).await?;
    if let Some(previous) = previous {
        if previous["product_id"] != purchase.product_id { return Err(AppError::conflict("purchase identity was reused for another product")); }
        return Ok(Json(previous));
    }
    let raw: Option<String> = sqlx::query_scalar("SELECT value FROM kv WHERE key='velora_core_darknet'").fetch_optional(&mut *tx).await?;
    let products: Vec<Product> = raw.map(|raw| serde_json::from_str(&raw)).transpose()?.unwrap_or_default();
    let product = products.into_iter().find(|p| p.id == purchase.product_id && p.enabled).ok_or_else(|| AppError::not_found("Darknet product is unavailable"))?;
    if purchase.expected_price_cents.is_some_and(|price| price != product.price_cents) { return Err(AppError::conflict("Darknet price changed; reload the catalog and review the purchase")); }
    let balance = debit(&mut tx, server.economy_id, server.id, uuid, name, product.price_cents as f64 / 100.0,
        super::ledger::Account("server", "Darknet"), &format!("Darknet purchase: {}", product.item_name)).await?;
    let delivery: i64 = sqlx::query_scalar("INSERT INTO market_mailbox(server_id,uuid,item_id,item_name,amount,note,created_at,to_vault) VALUES(?,?,?,?,?,'darknet',?,1) RETURNING id")
        .bind(server.id).bind(uuid).bind(&product.item_id).bind(&product.item_name).bind(product.amount).bind(crate::db::now()).fetch_one(&mut *tx).await?;
    finish_operation(tx, server.id, &operation, json!({"ok":true,"product_id":product.id,"price_cents":product.price_cents,"balance":balance,"delivery_id":delivery,"to_vault":true})).await
}
pub async fn player_buy(auth: AuthUser, State(state): State<AppState>, Path(sid): Path<i64>, Json(purchase): Json<Purchase>) -> AppResult<Json<Value>> {
    let mut server = get_server(&state, sid).await?; server.economy_id = economy_scope(&state.db, sid).await?;
    buy(&state, &server, &auth.uuid, &auth.username, purchase).await
}
pub async fn game_buy(GameServer(server): GameServer, State(state): State<AppState>, Json(p): Json<GamePurchase>) -> AppResult<Json<Value>> {
    buy(&state, &server, &p.uuid, &p.username, Purchase { operation_id:p.operation_id, product_id:p.product_id, expected_price_cents:p.expected_price_cents }).await
}
