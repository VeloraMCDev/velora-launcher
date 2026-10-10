//! Cloud vaults: the panel stores every vault, so the launcher and web can show and rearrange them and a delivery is
//! acknowledged in the same transaction that writes it into a vault. A game server leases a vault while a player has it
//! open; nothing else may write it until the lease is released or expires.
use super::economy::{begin_operation, finish_operation};
use super::guilds::guild_can;
use super::servers::{economy_scope, get_server, GameServer, ServerRow};
use crate::auth::AuthUser;
use crate::error::{AppError, AppResult};
use crate::state::{AppState, RequestState as State};
use axum::{extract::Path, Json};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// How long a game server holds a vault without renewing it (it renews while the vault is open).
const LEASE_SECONDS: i64 = 120;
pub const MAX_SLOTS: u32 = 63;
const MAX_STACK_DATA: usize = 1024 * 1024;
const MAX_VAULT_BYTES: usize = 3 * 1024 * 1024;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Stack {
    pub slot: u32,
    /// Registry id, e.g. `minecraft:diamond`.
    pub item: String,
    pub count: u32,
    #[serde(default)]
    pub name: String,
    /// The game's exact serialization (SNBT). Never sent to players.
    #[serde(default)]
    pub data: String,
    /// Registry id and exact tag hash, supplied by the authoritative game server (ignores stack count).
    #[serde(default)]
    pub fingerprint: String,
}

pub fn clean(contents: &[Stack]) -> AppResult<String> {
    let mut slots = std::collections::HashSet::new();
    for s in contents {
        if s.slot >= MAX_SLOTS || !slots.insert(s.slot) || !(1..=127).contains(&s.count) || s.item.len() > 128 || !s.item.contains(':')
            || !s.item.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, ':' | '_' | '-' | '.' | '/'))
            || s.name.chars().count() > 128 || s.data.len() > MAX_STACK_DATA
            || (!s.fingerprint.is_empty() && (s.fingerprint.len()!=64 || !s.fingerprint.bytes().all(|b|b.is_ascii_hexdigit()))) {
            return Err(AppError::bad_request("invalid vault contents"));
        }
    }
    let raw = serde_json::to_string(contents)?;
    if raw.len() > MAX_VAULT_BYTES { return Err(AppError::bad_request("vault contents are too large")); }
    Ok(raw)
}

fn parse(raw: &str) -> Vec<Stack> { serde_json::from_str(raw).unwrap_or_default() }

fn lease_until() -> String { (Utc::now() + Duration::seconds(LEASE_SECONDS)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true) }

/// Vault settings for this instance: (enabled, count, rows, free_count).
async fn settings(state: &AppState) -> AppResult<super::utilities::Vault> { Ok(super::utilities::load(state).await?.vault) }

/// Faction vault pages bought by this guild (Velora SMP); legacy guilds have none.
async fn faction_pages(state: &AppState, guild: &str) -> AppResult<i64> {
    if !crate::velora_core::load(state).await?.is_some_and(|policy| policy.enabled("factions")) { return Ok(0); }
    crate::factions_core::tiers(&mut *state.db.acquire().await?, guild, "vault").await
}

/// The faction this player may open vaults for on this instance, if any.
async fn faction_of(state: &AppState, instance: &str, uuid: &str) -> AppResult<Option<String>> {
    let guild: Option<String> = sqlx::query_scalar("SELECT gm.guild_id FROM guild_members gm JOIN guilds g ON g.id=gm.guild_id WHERE gm.uuid=? AND g.instance_id=? ORDER BY gm.joined_at LIMIT 1")
        .bind(uuid).bind(instance).fetch_optional(&state.db).await?;
    match guild {
        Some(guild) if guild_can(state, &guild, uuid, "vault").await? => Ok(Some(guild)),
        _ => Ok(None),
    }
}

async fn entitled(state: &AppState, uuid: &str, number: i64) -> AppResult<bool> {
    Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM vault_entitlements WHERE uuid=? AND number=?)").bind(uuid).bind(number).fetch_one(&state.db).await?)
}

/// Imported inventories remain accessible even if their old permission grant is no longer available.
async fn personal_unlocked(state: &AppState, uuid: &str, number: i64, free: i64) -> AppResult<bool> {
    if number <= free || entitled(state, uuid, number).await? { return Ok(true); }
    Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM cloud_vaults WHERE owner=? AND number=? AND revision>0)")
        .bind(format!("player:{uuid}")).bind(number).fetch_one(&state.db).await?)
}

/// Resolve and authorize one vault for a player. `permitted` is the game server's own permission answer (scopenet.vault.N).
pub(crate) async fn authorize(state: &AppState, instance: &str, uuid: &str, owner: &str, number: i64, permitted: bool) -> AppResult<String> {
    let v = settings(state).await?;
    if !v.enabled { return Err(AppError::forbidden("vaults are switched off")); }
    match owner {
        "player" => {
            if number < 1 || number > v.count { return Err(AppError::bad_request(format!("pick a vault from 1 to {}", v.count))); }
            if !permitted && !personal_unlocked(state, uuid, number, v.free_count).await? {
                return Err(AppError::forbidden(format!("vault {number} is locked; buy it or ask staff")));
            }
            Ok(format!("player:{uuid}"))
        }
        "faction" => {
            let guild = faction_of(state, instance, uuid).await?.ok_or_else(|| AppError::forbidden("your faction role cannot use faction vaults"))?;
            let pages = faction_pages(state, &guild).await?;
            if number < 1 || number > pages { return Err(AppError::forbidden("buy a faction vault page with /guild upgrade vault first")); }
            Ok(format!("faction:{guild}"))
        }
        shop if shop.starts_with("shop:") => {
            let allowed:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM physical_shops s JOIN game_servers g ON g.id=s.server_id WHERE s.id=? AND s.owner_uuid=? AND g.instance_id=?)")
                .bind(&shop[5..]).bind(uuid).bind(instance).fetch_one(&state.db).await?;
            if !allowed || number!=1 {return Err(AppError::forbidden("shop stock belongs to its seller"));}
            Ok(shop.to_owned())
        }
        _ => Err(AppError::bad_request("owner must be player or faction")),
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Game server

#[derive(Deserialize)]
pub struct Open { pub uuid: String, pub owner: String, pub number: i64, #[serde(default)] pub permitted: bool }

/// Lease a vault for this server and return its contents. A server that already holds the lease keeps its token,
/// so a second viewer on the same server shares the live copy.
pub async fn game_open(GameServer(server): GameServer, State(state): State<AppState>, Json(p): Json<Open>) -> AppResult<Json<Value>> {
    let key = authorize(&state, &server.instance_id, &p.uuid, &p.owner, p.number, p.permitted).await?;
    let now = crate::db::now();
    let mut tx = state.db.begin().await?;
    sqlx::query("INSERT INTO cloud_vaults(owner,number,contents,revision,updated_at) VALUES(?,?,'[]',0,?) ON CONFLICT(owner,number) DO NOTHING")
        .bind(&key).bind(p.number).bind(&now).execute(&mut *tx).await?;
    let row: Option<(String, i64, String)> = sqlx::query_as(
        "UPDATE cloud_vaults SET lease_token=CASE WHEN lease_server=? AND lease_until>? THEN lease_token ELSE ? END, lease_server=?, lease_until=?
         WHERE owner=? AND number=? AND (lease_server IS NULL OR lease_until<=? OR lease_server=?)
         AND NOT EXISTS(SELECT 1 FROM vault_transfers t WHERE t.owner=cloud_vaults.owner AND t.number=cloud_vaults.number AND t.status='prepared') RETURNING contents,revision,lease_token")
        .bind(server.id).bind(&now).bind(uuid::Uuid::new_v4().to_string()).bind(server.id).bind(lease_until())
        .bind(&key).bind(p.number).bind(&now).bind(server.id).fetch_optional(&mut *tx).await?;
    let (contents, revision, lease) = row.ok_or_else(|| AppError::conflict("this vault is open on another server or in the launcher; try again shortly"))?;
    tx.commit().await?;
    let rows = if key.starts_with("shop:"){3}else{settings(&state).await?.rows};
    Ok(Json(json!({"key":key,"number":p.number,"revision":revision,"lease":lease,"rows":rows,"contents":parse(&contents)})))
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Save {
    #[serde(default)]
    pub operation_id: Option<String>,
    pub key: String,
    pub number: i64,
    pub lease: String,
    pub revision: i64,
    pub contents: Vec<Stack>,
    #[serde(default)]
    pub release: bool,
    /// Mailbox deliveries this write contains: they are removed in the same transaction, so none can land twice.
    #[serde(default)]
    pub delivery_ids: Vec<i64>,
}

/// Write a leased vault. The revision must match, so a lost response cannot overwrite newer contents.
pub async fn game_save(GameServer(server): GameServer, State(state): State<AppState>, Json(p): Json<Save>) -> AppResult<Json<Value>> {
    let raw = clean(&p.contents)?;
    let now = crate::db::now();
    let operation = p.operation_id.as_ref().map(|id| {
        uuid::Uuid::parse_str(id).map(|id| format!("vault-save:{id}"))
            .map_err(|_| AppError::bad_request("operation_id must be a UUID"))
    }).transpose()?;
    use sha2::Digest;
    let hash = format!("{:x}", sha2::Sha256::digest(serde_json::to_vec(&p)?));
    let (mut tx, previous) = match &operation {
        Some(operation) => begin_operation(&state, server.id, operation).await?,
        None => (state.db.begin().await?, None),
    };
    if let Some(previous) = previous {
        if previous["request_hash"] != hash { return Err(AppError::conflict("save identity was reused for different contents")); }
        return Ok(Json(previous));
    }
    if p.delivery_ids.len() > 100 { return Err(AppError::bad_request("too many deliveries in one save")); }
    for delivery in &p.delivery_ids {
        let owner: Option<String> = sqlx::query_scalar("DELETE FROM market_mailbox WHERE id=? AND server_id=? AND to_vault=1 RETURNING uuid")
            .bind(delivery).bind(server.id).fetch_optional(&mut *tx).await?;
        if owner.is_none_or(|uuid| p.key != format!("player:{uuid}")) {
            return Err(AppError::conflict("this delivery was already stored or belongs to another vault"));
        }
    }
    let (lease_server, lease_until) = if p.release { (None, None) } else { (Some(server.id), Some(lease_until())) };
    let revision: Option<i64> = sqlx::query_scalar(
        "UPDATE cloud_vaults SET contents=?,revision=revision+1,updated_at=?,lease_server=?,lease_until=?,lease_token=CASE WHEN ? THEN NULL ELSE lease_token END
         WHERE owner=? AND number=? AND revision=? AND lease_server=? AND lease_token=? AND lease_until>?
         AND NOT EXISTS(SELECT 1 FROM vault_transfers t WHERE t.owner=cloud_vaults.owner AND t.number=cloud_vaults.number AND t.status='prepared') RETURNING revision")
        .bind(&raw).bind(&now).bind(lease_server).bind(lease_until).bind(p.release)
        .bind(&p.key).bind(p.number).bind(p.revision).bind(server.id).bind(&p.lease).bind(&now).fetch_optional(&mut *tx).await?;
    let revision = revision.ok_or_else(|| AppError::conflict("the vault lease expired or the vault changed; reopen it"))?;
    let result = json!({"revision":revision,"released":p.release,"request_hash":hash});
    if let Some(operation) = operation { finish_operation(tx,server.id,&operation,result).await }
    else { tx.commit().await?; Ok(Json(result)) }
}

#[derive(Deserialize)]
pub struct Renew { pub key: String, pub number: i64, pub lease: String }

pub async fn game_renew(GameServer(server): GameServer, State(state): State<AppState>, Json(p): Json<Renew>) -> AppResult<Json<Value>> {
    let now = crate::db::now();
    let renewed = sqlx::query("UPDATE cloud_vaults SET lease_until=CASE WHEN EXISTS(SELECT 1 FROM vault_transfers t WHERE t.owner=cloud_vaults.owner AND t.number=cloud_vaults.number AND t.status='prepared') THEN '9999-12-31T00:00:00Z' ELSE ? END WHERE owner=? AND number=? AND lease_server=? AND lease_token=? AND lease_until>?")
        .bind(lease_until()).bind(&p.key).bind(p.number).bind(server.id).bind(&p.lease).bind(&now).execute(&state.db).await?.rows_affected();
    if renewed != 1 { return Err(AppError::conflict("the vault lease expired; reopen it")); }
    Ok(Json(json!({"ok":true})))
}

#[derive(Deserialize)]
pub struct ImportVault { pub number: i64, pub contents: Vec<Stack> }
#[derive(Deserialize)]
pub struct Import { pub uuid: String, pub vaults: Vec<ImportVault> }

/// One-time move of a player's local vault file into the panel. Refused once the player has cloud vaults,
/// so a second server's stale file can never overwrite them.
pub async fn game_import(GameServer(server): GameServer, State(state): State<AppState>, Json(p): Json<Import>) -> AppResult<Json<Value>> {
    uuid::Uuid::parse_str(&p.uuid).map_err(|_| AppError::bad_request("invalid player"))?;
    let key = format!("player:{}", p.uuid);
    let mut tx = state.db.begin().await?;
    let marked: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM cloud_vault_imports WHERE owner=?)").bind(&key).fetch_one(&mut *tx).await?;
    if marked { return Ok(Json(json!({"imported":false,"migrated":true}))); }
    let used: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM cloud_vaults WHERE owner=? AND (revision>0 OR contents<>'[]' OR lease_until>? OR EXISTS(SELECT 1 FROM vault_transfers t WHERE t.owner=cloud_vaults.owner AND t.number=cloud_vaults.number AND t.status='prepared')))")
        .bind(&key).bind(crate::db::now()).fetch_one(&mut *tx).await?;
    if used { return Ok(Json(json!({"imported":false,"migrated":false,"reason":"this player already has cloud vaults; keep the local file for staff review"}))); }
    let now = crate::db::now();
    let mut numbers = std::collections::HashSet::new();
    for v in &p.vaults {
        if !(1..=54).contains(&v.number) { return Err(AppError::bad_request("vault numbers are 1-54")); }
        if !numbers.insert(v.number) { return Err(AppError::bad_request("duplicate vault number")); }
        sqlx::query("INSERT INTO cloud_vaults(owner,number,contents,revision,updated_at) VALUES(?,?,?,1,?) ON CONFLICT(owner,number) DO UPDATE SET contents=excluded.contents,revision=cloud_vaults.revision+1,updated_at=excluded.updated_at")
            .bind(&key).bind(v.number).bind(clean(&v.contents)?).bind(&now).execute(&mut *tx).await?;
    }
    sqlx::query("INSERT INTO cloud_vault_imports(owner,server_id,vaults,imported_at) VALUES(?,?,?,?)").bind(&key).bind(server.id).bind(p.vaults.len() as i64).bind(&now).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(Json(json!({"imported":true,"migrated":true,"vaults":p.vaults.len()})))
}

// ---------------------------------------------------------------------------------------------------------------
// Players (launcher and web)

fn stack_view(s: &Stack) -> Value {
    // Enchantment or custom-name hints only; the exact serialization stays on the server.
    json!({"slot":s.slot,"item":s.item,"count":s.count,"name":s.name,"enchanted":s.data.contains("Enchantments"),"custom":s.data.contains("\"Name\"") || s.data.contains("Name:")})
}

async fn server_for(state: &AppState, sid: i64) -> AppResult<ServerRow> {
    let mut server = get_server(state, sid).await?;
    server.economy_id = economy_scope(&state.db, sid).await?;
    Ok(server)
}

/// Every vault this player can see: personal vaults (open or buyable) and their faction's pages, plus deliveries in transit.
pub async fn player_list(auth: AuthUser, State(state): State<AppState>, Path(sid): Path<i64>) -> AppResult<Json<Value>> {
    let server = server_for(&state, sid).await?;
    let v = settings(&state).await?;
    if !v.enabled { return Err(AppError::forbidden("vaults are switched off")); }
    let policy = crate::velora_core::load(&state).await?;
    let now = crate::db::now();
    let own = format!("player:{}", auth.uuid);
    let rows: Vec<(String, i64, String, i64, Option<String>)> = sqlx::query_as("SELECT owner,number,contents,revision,lease_until FROM cloud_vaults WHERE owner=? OR owner=?")
        .bind(&own).bind(match faction_of(&state, &server.instance_id, &auth.uuid).await? { Some(g) => format!("faction:{g}"), None => String::new() })
        .fetch_all(&state.db).await?;
    let found = |owner: &str, number: i64| rows.iter().find(|r| r.0 == owner && r.1 == number);
    let owned: Vec<i64> = sqlx::query_scalar("SELECT number FROM vault_entitlements WHERE uuid=?")
        .bind(&auth.uuid).fetch_all(&state.db).await?;
    let mut vaults = Vec::new();
    let mut offered_locked = false;
    for number in 1..=v.count {
        let row = found(&own, number);
        let unlocked = number <= v.free_count || owned.contains(&number) || row.is_some_and(|r| r.3 > 0);
        if !unlocked {
            if offered_locked { continue; }
            offered_locked = true;
        }
        let price = policy.as_ref().filter(|p| p.enabled("economy")).map(|p| p.vault_price_cents);
        vaults.push(json!({"key":own,"owner":"player","number":number,"label":format!("Vault {number}"),"rows":v.rows,"unlocked":unlocked,
            "price_cents":if unlocked {None} else {price},"revision":row.map_or(0, |r| r.3),
            "in_game":row.is_some_and(|r| r.4.as_deref().is_some_and(|until| until > now.as_str())),
            "contents":row.map(|r| parse(&r.2).iter().map(stack_view).collect::<Vec<_>>()).unwrap_or_default()}));
    }
    if let Some(guild) = faction_of(&state, &server.instance_id, &auth.uuid).await? {
        let key = format!("faction:{guild}");
        for number in 1..=faction_pages(&state, &guild).await? {
            let row = found(&key, number);
            vaults.push(json!({"key":key,"owner":"faction","number":number,"label":format!("Faction vault {number}"),"rows":v.rows,"unlocked":true,
                "price_cents":null,"revision":row.map_or(0, |r| r.3),"in_game":row.is_some_and(|r| r.4.as_deref().is_some_and(|until| until > now.as_str())),
                "contents":row.map(|r| parse(&r.2).iter().map(stack_view).collect::<Vec<_>>()).unwrap_or_default()}));
        }
    }
    let incoming: Vec<(i64, String, String, i64, String)> = sqlx::query_as("SELECT id,item_id,item_name,amount,created_at FROM market_mailbox WHERE server_id=? AND uuid=? AND to_vault=1 ORDER BY id")
        .bind(server.id).bind(&auth.uuid).fetch_all(&state.db).await?;
    let balance: Option<f64> = sqlx::query_scalar("SELECT balance FROM server_economy WHERE server_id=? AND uuid=?").bind(server.economy_id).bind(&auth.uuid).fetch_optional(&state.db).await?;
    Ok(Json(json!({"vaults":vaults,"rows":v.rows,"balance":balance,
        "incoming":incoming.into_iter().map(|(id,item,name,amount,at)| json!({"id":id,"item_id":item,"item_name":name,"amount":amount,"created_at":at})).collect::<Vec<_>>()})))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Place { pub key: String, pub number: i64, pub slot: u32, pub revision: i64 }
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Move { pub from: Place, pub to: Place }

/// May this player rearrange `key`? Personal vaults belong to their owner; faction vaults need the vault role permission.
async fn may_edit(state: &AppState, server: &ServerRow, uuid: &str, key: &str, number: i64) -> AppResult<()> {
    let v = settings(state).await?;
    let ok = if key == format!("player:{uuid}") {
        (1..=v.count).contains(&number) && personal_unlocked(state, uuid, number, v.free_count).await?
    } else if let Some(guild) = key.strip_prefix("faction:") {
        faction_of(state, &server.instance_id, uuid).await?.as_deref() == Some(guild) && (1..=faction_pages(state, guild).await?).contains(&number)
    } else { false };
    if !ok { return Err(AppError::forbidden("you cannot edit that vault")); }
    Ok(())
}

/// Move or swap one stack, within a vault or between two of the player's vaults. Refused while either is open in game.
pub async fn player_move(auth: AuthUser, State(state): State<AppState>, Path(sid): Path<i64>, Json(p): Json<Move>) -> AppResult<Json<Value>> {
    let server = server_for(&state, sid).await?;
    if !settings(&state).await?.enabled { return Err(AppError::forbidden("vaults are switched off")); }
    let rows = settings(&state).await?.rows as u32;
    for place in [&p.from, &p.to] {
        may_edit(&state, &server, &auth.uuid, &place.key, place.number).await?;
        if place.slot >= (rows * 9).min(MAX_SLOTS) { return Err(AppError::bad_request("slot is outside the vault")); }
    }
    let same = p.from.key == p.to.key && p.from.number == p.to.number;
    if same && p.from.revision != p.to.revision { return Err(AppError::bad_request("one vault has one revision")); }
    let now = crate::db::now();
    let mut tx = state.db.begin().await?;
    let mut from = load_for_edit(&mut tx, &p.from, &now).await?;
    let mut to = if same { Vec::new() } else { load_for_edit(&mut tx, &p.to, &now).await? };
    let moving = from.iter().position(|s| s.slot == p.from.slot).ok_or_else(|| AppError::bad_request("that slot is empty"))?;
    let mut stack = from.remove(moving);
    if same {
        if let Some(other) = from.iter_mut().find(|s| s.slot == p.to.slot) { other.slot = p.from.slot; }
        stack.slot = p.to.slot;
        from.push(stack);
    } else {
        if let Some(i) = to.iter().position(|s| s.slot == p.to.slot) {
            let mut swapped = to.remove(i);
            swapped.slot = p.from.slot;
            from.push(swapped);
        }
        stack.slot = p.to.slot;
        to.push(stack);
    }
    let from_revision = store_edit(&mut tx, &p.from, &mut from, &now).await?;
    let to_revision = if same { from_revision } else { store_edit(&mut tx, &p.to, &mut to, &now).await? };
    tx.commit().await?;
    Ok(Json(json!({"ok":true,"from_revision":from_revision,"to_revision":to_revision})))
}

async fn load_for_edit(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, place: &Place, now: &str) -> AppResult<Vec<Stack>> {
    let pending: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM vault_transfers WHERE owner=? AND number=? AND status='prepared')")
        .bind(&place.key).bind(place.number).fetch_one(&mut **tx).await?;
    if pending { return Err(AppError::conflict("this vault has an unresolved inventory transfer; reconnect in game to recover it")); }
    let row: Option<(String, i64, Option<String>)> = sqlx::query_as("SELECT contents,revision,lease_until FROM cloud_vaults WHERE owner=? AND number=?")
        .bind(&place.key).bind(place.number).fetch_optional(&mut **tx).await?;
    let (contents, revision, lease) = row.unwrap_or(("[]".into(), 0, None));
    if lease.as_deref().is_some_and(|until| until > now) { return Err(AppError::conflict("close this vault in game before editing it here")); }
    if revision != place.revision { return Err(AppError::conflict("this vault changed; reload it")); }
    Ok(parse(&contents))
}

async fn store_edit(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, place: &Place, contents: &mut [Stack], now: &str) -> AppResult<i64> {
    contents.sort_by_key(|s| s.slot);
    let raw = clean(contents)?;
    sqlx::query("INSERT INTO cloud_vaults(owner,number,contents,revision,updated_at) VALUES(?,?,'[]',0,?) ON CONFLICT(owner,number) DO NOTHING")
        .bind(&place.key).bind(place.number).bind(now).execute(&mut **tx).await?;
    Ok(sqlx::query_scalar("UPDATE cloud_vaults SET contents=?,revision=revision+1,updated_at=? WHERE owner=? AND number=? RETURNING revision")
        .bind(raw).bind(now).bind(&place.key).bind(place.number).fetch_one(&mut **tx).await?)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuyVault { pub number: i64, pub expected_price_cents: i64, pub operation_id: String }

/// Unlock a personal vault for in-game dollars (Velora SMP). One receipt per operation; buying an unlocked vault is refused.
pub async fn player_buy(auth: AuthUser, State(state): State<AppState>, Path(sid): Path<i64>, Json(p): Json<BuyVault>) -> AppResult<Json<Value>> {
    let server = server_for(&state, sid).await?;
    buy(&state, &server, &auth.uuid, &auth.username, p).await
}

#[derive(Deserialize)]
pub struct GameBuy { pub uuid: String, pub username: String, pub number: i64, pub expected_price_cents: i64, pub operation_id: String }

pub async fn game_buy(GameServer(server): GameServer, State(state): State<AppState>, Json(p): Json<GameBuy>) -> AppResult<Json<Value>> {
    buy(&state, &server, &p.uuid, &p.username, BuyVault { number: p.number, expected_price_cents: p.expected_price_cents, operation_id: p.operation_id }).await
}

pub(crate) async fn buy(state: &AppState, server: &ServerRow, uuid: &str, name: &str, p: BuyVault) -> AppResult<Json<Value>> {
    let policy = crate::velora_core::load(state).await?.ok_or_else(|| AppError::not_found("vault purchases belong to Velora SMP"))?;
    if !policy.enabled("vaults") || !policy.enabled("economy") { return Err(AppError::forbidden("vault purchases need the vaults and economy modules")); }
    let v = settings(state).await?;
    if !v.enabled || p.number <= v.free_count || p.number > v.count { return Err(AppError::bad_request("that vault cannot be bought")); }
    uuid::Uuid::parse_str(&p.operation_id).map_err(|_| AppError::bad_request("operation_id must be a UUID"))?;
    let operation = format!("vault-buy:{uuid}:{}", p.operation_id);
    let (mut tx, previous) = begin_operation(state, server.id, &operation).await?;
    if let Some(previous) = previous {
        if previous["number"] != p.number { return Err(AppError::conflict("purchase identity was reused for another vault")); }
        return Ok(Json(previous));
    }
    if p.expected_price_cents != policy.vault_price_cents { return Err(AppError::conflict("the vault price changed; review it again")); }
    let owned: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM vault_entitlements WHERE uuid=? AND number=?) OR EXISTS(SELECT 1 FROM cloud_vaults WHERE owner=? AND number=? AND revision>0)")
        .bind(uuid).bind(p.number).bind(format!("player:{uuid}")).bind(p.number).fetch_one(&mut *tx).await?;
    if owned { return Err(AppError::bad_request("you already own that vault")); }
    let balance = super::ledger::debit(&mut tx, server.economy_id, server.id, uuid, name, policy.vault_price_cents as f64 / 100.0,
        super::ledger::Account("server", "Vaults"), &format!("Vault {} unlock", p.number)).await?;
    sqlx::query("INSERT INTO vault_entitlements(uuid,number,price_cents,purchased_at) VALUES(?,?,?,?)")
        .bind(uuid).bind(p.number).bind(policy.vault_price_cents).bind(crate::db::now()).execute(&mut *tx).await?;
    finish_operation(tx, server.id, &operation, json!({"ok":true,"number":p.number,"price_cents":policy.vault_price_cents,"balance":balance})).await
}

/// Owned vault numbers, so the game can unlock bought vaults without a per-open round trip.
pub async fn game_entitlements(GameServer(_): GameServer, State(state): State<AppState>, Json(p): Json<super::darknet::Who>) -> AppResult<Json<Value>> {
    let owned: Vec<i64> = sqlx::query_scalar("SELECT number FROM vault_entitlements WHERE uuid=? ORDER BY number").bind(&p.uuid).fetch_all(&state.db).await?;
    let price = crate::velora_core::load(&state).await?.filter(|p| p.enabled("economy")).map(|p| p.vault_price_cents);
    Ok(Json(json!({"owned":owned,"price_cents":price})))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn stack(slot: u32) -> Stack { Stack { slot, item: "minecraft:diamond".into(), count: 3, name: "Diamond".into(), data: String::new(), fingerprint:String::new() } }
    #[test] fn contents_are_bounded() {
        assert!(clean(&[stack(0), stack(62)]).is_ok());
        assert!(clean(&[stack(0), stack(0)]).is_err(), "duplicate slots");
        assert!(clean(&[stack(63)]).is_err());
        assert!(clean(&[Stack { item: "DIAMOND".into(), ..stack(1) }]).is_err(), "registry ids only");
        assert!(clean(&[Stack { count: 0, ..stack(1) }]).is_err());
    }
}
