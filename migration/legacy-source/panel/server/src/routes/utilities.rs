//! Player utility commands (/heal, /feed, /fly, /vault, /echest, /kit) and admin-made custom items.
//!
//! Admins design all of it in the panel; every game server receives the current settings and item definitions with
//! its sync, so a change is live within seconds and never needs a config file edit or a restart.

use crate::state::RequestState as State;
use crate::auth::AdminUser;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::store;
use axum::extract::{Path};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

// ---------------------------------------------------------------------------
// Item definitions (shared by kits and custom items)
// ---------------------------------------------------------------------------

/// Checks and tidies an item definition. Unknown keys are dropped so game servers only ever see what they understand.
///
/// `{ "item": "minecraft:diamond_sword", "amount": 1, "name": "&bBlade", "lore": ["&7…"], "enchants": {"sharpness": 10},
///    "unbreakable": true, "glow": false, "hide_flags": true, "custom_model_data": 7,
///    "attributes": [{"attribute": "generic.attack_damage", "amount": 40, "operation": "add", "slot": "mainhand"}] }`
pub fn clean_item(raw: &Value) -> AppResult<Value> {
    let o = raw.as_object().ok_or_else(|| AppError::bad_request("An item must be an object"))?;
    let item = o.get("item").and_then(Value::as_str).unwrap_or("").trim().to_lowercase();
    let valid_id = !item.is_empty()
        && item.len() <= 80
        && item.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '_' | '.' | '-' | ':' | '/'));
    if !valid_id {
        return Err(AppError::bad_request("Item needs a valid id such as minecraft:diamond_sword"));
    }
    let amount = o.get("amount").and_then(Value::as_i64).unwrap_or(1);
    if !(1..=64).contains(&amount) {
        return Err(AppError::bad_request("Amount must be between 1 and 64"));
    }
    let mut out = Map::new();
    if let Some(data) = o.get("data") {
        let format = data["format"].as_str().unwrap_or("");
        let value = data["value"].as_str().unwrap_or("");
        if !["bukkit", "nbt"].contains(&format) || value.is_empty() || value.len() > 131072 {
            return Err(AppError::bad_request("Invalid inventory item metadata"));
        }
        out.insert("data".into(), json!({"format": format, "value": value}));
    }
    out.insert("item".into(), json!(item));
    out.insert("amount".into(), json!(amount));
    for key in ["texture", "model"] {
        if let Some(reference) = o.get(key).and_then(Value::as_str).filter(|s| !s.is_empty()) {
            if !super::resource_assets::valid_reference(reference)
                || !item.starts_with("minecraft:")
                || !super::resource_assets::valid_reference(&item)
                || !o.get("custom_model_data").and_then(Value::as_i64).is_some_and(|m| (1..=16_777_215).contains(&m))
            {
                return Err(AppError::bad_request(
                    "Textured items need a minecraft: base item, a model data number and a namespace:path asset reference",
                ));
            }
            out.insert(key.into(), json!(reference));
        }
    }
    if let Some(name) = o.get("name").and_then(Value::as_str).filter(|n| !n.trim().is_empty()) {
        if name.chars().count() > 100 || name.contains('\n') {
            return Err(AppError::bad_request("Item names are one line of up to 100 characters"));
        }
        out.insert("name".into(), json!(name));
    }
    if let Some(lore) = o.get("lore").and_then(Value::as_array) {
        if lore.len() > 24 {
            return Err(AppError::bad_request("Lore can have up to 24 lines"));
        }
        let mut lines = Vec::new();
        for l in lore {
            let l = l.as_str().ok_or_else(|| AppError::bad_request("Lore lines must be text"))?;
            if l.chars().count() > 160 || l.contains('\n') {
                return Err(AppError::bad_request("Each lore line can be up to 160 characters"));
            }
            lines.push(json!(l));
        }
        if !lines.is_empty() {
            out.insert("lore".into(), Value::Array(lines));
        }
    }
    if let Some(ench) = o.get("enchants").and_then(Value::as_object) {
        if ench.len() > 40 {
            return Err(AppError::bad_request("Too many enchantments"));
        }
        let mut e = Map::new();
        for (k, v) in ench {
            let key = k.trim().to_lowercase();
            let level = v.as_i64().unwrap_or(0);
            if key.is_empty()
                || key.len() > 60
                || !key.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '_' | ':' | '.' | '-'))
            {
                return Err(AppError::bad_request(format!("\"{k}\" is not an enchantment id")));
            }
            if !(1..=255).contains(&level) {
                return Err(AppError::bad_request("Enchantment levels run from 1 to 255"));
            }
            e.insert(key, json!(level));
        }
        if !e.is_empty() {
            out.insert("enchants".into(), Value::Object(e));
        }
    }
    for flag in ["unbreakable", "glow", "hide_flags"] {
        if o.get(flag).and_then(Value::as_bool) == Some(true) {
            out.insert(flag.into(), json!(true));
        }
    }
    if let Some(m) = o.get("custom_model_data").and_then(Value::as_i64) {
        if !(0..=2_000_000_000).contains(&m) {
            return Err(AppError::bad_request("custom_model_data is out of range"));
        }
        out.insert("custom_model_data".into(), json!(m));
    }
    if let Some(attrs) = o.get("attributes").and_then(Value::as_array) {
        if attrs.len() > 12 {
            return Err(AppError::bad_request("Up to 12 attribute bonuses per item"));
        }
        let mut list = Vec::new();
        for a in attrs {
            let attribute = a.get("attribute").and_then(Value::as_str).unwrap_or("").trim().to_lowercase();
            let known = [
                "generic.max_health",
                "generic.attack_damage",
                "generic.attack_speed",
                "generic.movement_speed",
                "generic.armor",
                "generic.armor_toughness",
                "generic.knockback_resistance",
                "generic.luck",
                "generic.attack_knockback",
            ];
            if !known.contains(&attribute.as_str()) {
                return Err(AppError::bad_request(format!("Unknown attribute \"{attribute}\"")));
            }
            let amount = a.get("amount").and_then(Value::as_f64).unwrap_or(0.0);
            if !amount.is_finite() || amount.abs() > 10_000.0 {
                return Err(AppError::bad_request("Attribute amounts must be between -10000 and 10000"));
            }
            let operation = a.get("operation").and_then(Value::as_str).unwrap_or("add");
            if !["add", "multiply_base", "multiply_total"].contains(&operation) {
                return Err(AppError::bad_request("Operation must be add, multiply_base or multiply_total"));
            }
            let slot = a.get("slot").and_then(Value::as_str).unwrap_or("mainhand");
            if !["mainhand", "offhand", "head", "chest", "legs", "feet", "any"].contains(&slot) {
                return Err(AppError::bad_request("Slot must be mainhand, offhand, head, chest, legs, feet or any"));
            }
            list.push(json!({ "attribute": attribute, "amount": amount, "operation": operation, "slot": slot }));
        }
        if !list.is_empty() {
            out.insert("attributes".into(), Value::Array(list));
        }
    }
    // Where the item came from and what it is, for the content library; and how much hunger edible ones restore.
    if let Some(cat) = o.get("category").and_then(Value::as_str) {
        if ["tool", "weapon", "armor", "food", "item"].contains(&cat) {
            out.insert("category".into(), json!(cat));
        }
    }
    if let Some(src) = o.get("source").and_then(Value::as_str).filter(|s| s.len() <= 24 && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')) {
        out.insert("source".into(), json!(src));
    }
    if let Some(food) = o.get("food").and_then(Value::as_object) {
        let nutrition = food.get("nutrition").and_then(Value::as_i64).unwrap_or(4).clamp(0, 20);
        let saturation = food.get("saturation").and_then(Value::as_f64).unwrap_or(2.0).clamp(0.0, 20.0);
        out.insert("food".into(), json!({"nutrition": nutrition, "saturation": saturation}));
    }
    Ok(Value::Object(out))
}

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Heal {
    pub enabled: bool,
    /// Health points restored (2 = one heart). 0 restores everything.
    pub amount: f64,
    pub cooldown_secs: i64,
}
impl Default for Heal {
    fn default() -> Self {
        Self { enabled: true, amount: 0.0, cooldown_secs: 300 }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Feed {
    pub enabled: bool,
    /// Hunger points restored (20 fills the bar).
    pub amount: i64,
    pub cooldown_secs: i64,
}
impl Default for Feed {
    fn default() -> Self {
        Self { enabled: true, amount: 20, cooldown_secs: 120 }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Fly {
    pub enabled: bool,
}
impl Default for Fly {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Vault {
    pub enabled: bool,
    /// How many vaults exist (/vault 1 … /vault N).
    pub count: i64,
    /// Rows in each vault (9 slots per row).
    pub rows: i64,
    /// Vaults everyone gets; the rest need the permission scopenet.vault.<number>.
    pub free_count: i64,
}
impl Default for Vault {
    fn default() -> Self {
        Self { enabled: true, count: 3, rows: 6, free_count: 1 }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Echest {
    pub enabled: bool,
}
impl Default for Echest {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Kit {
    /// Lower-case name typed in game: /kit starter.
    pub id: String,
    pub name: String,
    pub description: String,
    pub cooldown_secs: i64,
    /// Can be claimed once per player, ever.
    pub one_time: bool,
    /// LuckPerms groups that may use the kit. Empty means everyone.
    pub groups: Vec<String>,
    pub items: Vec<Value>,
    /// Console commands run when the kit is claimed; {player} is replaced.
    pub commands: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Utilities {
    pub heal: Heal,
    pub feed: Feed,
    pub fly: Fly,
    pub vault: Vault,
    pub echest: Echest,
    pub kits: Vec<Kit>,
}

fn check_utilities(u: &mut Utilities) -> AppResult<()> {
    if !(0.0..=1000.0).contains(&u.heal.amount) {
        return Err(AppError::bad_request("Heal amount must be between 0 and 1000"));
    }
    if !(0..=20).contains(&u.feed.amount) {
        return Err(AppError::bad_request("Feed amount must be between 0 and 20"));
    }
    for (label, secs) in [("Heal", u.heal.cooldown_secs), ("Feed", u.feed.cooldown_secs)] {
        if !(0..=30 * 86_400).contains(&secs) {
            return Err(AppError::bad_request(format!("{label} cooldown must be between 0 seconds and 30 days")));
        }
    }
    if !(1..=54).contains(&u.vault.count) {
        return Err(AppError::bad_request("Vault count must be between 1 and 54"));
    }
    if !(1..=6).contains(&u.vault.rows) {
        return Err(AppError::bad_request("Vault rows must be between 1 and 6"));
    }
    u.vault.free_count = u.vault.free_count.clamp(0, u.vault.count);
    if u.kits.len() > 60 {
        return Err(AppError::bad_request("Up to 60 kits"));
    }
    let mut seen = std::collections::HashSet::new();
    for k in &mut u.kits {
        k.id = k.id.trim().to_lowercase();
        if k.id.is_empty()
            || k.id == "create"
            || k.id.len() > 24
            || !k.id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '_' | '-'))
        {
            return Err(AppError::bad_request("Kit ids use letters, numbers, - and _ (up to 24 characters)"));
        }
        if !seen.insert(k.id.clone()) {
            return Err(AppError::bad_request(format!("Two kits are called \"{}\"", k.id)));
        }
        if k.name.trim().is_empty() {
            k.name = k.id.clone();
        }
        if k.name.chars().count() > 40 || k.description.chars().count() > 200 {
            return Err(AppError::bad_request("Kit name or description is too long"));
        }
        if !(0..=365 * 86_400).contains(&k.cooldown_secs) {
            return Err(AppError::bad_request("Kit cooldown must be between 0 seconds and a year"));
        }
        k.groups = k.groups.iter().map(|g| g.trim().to_lowercase()).filter(|g| !g.is_empty()).collect();
        if k.groups.iter().any(|g| g.len() > 64 || g.contains(char::is_whitespace)) {
            return Err(AppError::bad_request("LuckPerms group names have no spaces"));
        }
        if k.items.len() > 36 {
            return Err(AppError::bad_request("A kit holds up to 36 item stacks"));
        }
        k.items = k.items.iter().map(clean_item_or_custom).collect::<AppResult<_>>()?;
        k.commands = k.commands.iter().map(|c| c.trim().trim_start_matches('/').to_string()).filter(|c| !c.is_empty()).collect();
        if k.commands.len() > 10 || k.commands.iter().any(|c| c.len() > 200) {
            return Err(AppError::bad_request("A kit runs up to 10 commands of 200 characters"));
        }
    }
    Ok(())
}

/// A kit entry is either a full item definition or a reference to a custom item: `{ "custom": "thunder_axe", "amount": 1 }`.
fn clean_item_or_custom(v: &Value) -> AppResult<Value> {
    if let Some(id) = v.get("custom").and_then(Value::as_str) {
        let amount = v.get("amount").and_then(Value::as_i64).unwrap_or(1).clamp(1, 64);
        return Ok(json!({ "custom": id.trim().to_lowercase(), "amount": amount }));
    }
    clean_item(v)
}

pub async fn load(state: &AppState) -> AppResult<Utilities> {
    store::kv_get(state, "utilities").await
}

#[derive(Deserialize)]
pub struct InventoryKit {
    id: String,
    items: Vec<Value>,
}

/// The authenticated game integration checks scopenet.admin.kits before taking the snapshot.
pub async fn game_create_kit(
    crate::routes::servers::GameServer(_): crate::routes::servers::GameServer,
    State(state): State<AppState>,
    Json(p): Json<InventoryKit>,
) -> AppResult<Json<Value>> {
    if p.items.is_empty() {
        return Err(AppError::bad_request("The inventory is empty"));
    }
    // Serialize competing creates with SQLite's write transaction before reading the settings.
    let mut tx = state.db.begin().await?;
    sqlx::query("INSERT OR IGNORE INTO kv (key, value) VALUES ('utilities', '{}')").execute(&mut *tx).await?;
    let raw: String = sqlx::query_scalar("SELECT value FROM kv WHERE key = 'utilities'").fetch_one(&mut *tx).await?;
    let mut u: Utilities = serde_json::from_str(&raw)?;
    let id = p.id.trim().to_lowercase();
    if id == "create" || u.kits.iter().any(|k| k.id == id) {
        return Err(AppError::bad_request("That kit name is already in use or reserved"));
    }
    u.kits.push(Kit { id: id.clone(), name: id.clone(), items: p.items, ..Default::default() });
    check_utilities(&mut u)?;
    sqlx::query("UPDATE kv SET value = ? WHERE key = 'utilities'").bind(serde_json::to_string(&u)?).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(Json(json!({"ok": true, "id": id})))
}

pub async fn get(_: AdminUser, State(state): State<AppState>) -> AppResult<Json<Value>> {
    Ok(Json(json!({ "settings": load(&state).await?, "defaults": Utilities::default() })))
}

pub async fn put(_: AdminUser, State(state): State<AppState>, Json(mut u): Json<Utilities>) -> AppResult<Json<Value>> {
    check_utilities(&mut u)?;
    let known: Vec<String> = sqlx::query_scalar("SELECT id FROM custom_items").fetch_all(&state.db).await?;
    for k in &u.kits {
        for it in &k.items {
            if let Some(c) = it.get("custom").and_then(Value::as_str) {
                if !known.iter().any(|x| x == c) {
                    return Err(AppError::bad_request(format!("Kit \"{}\" uses the custom item \"{c}\", which doesn't exist", k.id)));
                }
            }
        }
    }
    store::kv_set(&state, "utilities", &u).await?;
    Ok(Json(json!({ "ok": true, "settings": u })))
}

// ---------------------------------------------------------------------------
// Custom items
// ---------------------------------------------------------------------------

pub(crate) fn clean_item_key(raw: &str) -> AppResult<String> {
    let k = raw.trim().to_lowercase();
    if k.is_empty() || k.len() > 32 || !k.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '_' | '-')) {
        return Err(AppError::bad_request("Item ids use letters, numbers, - and _ (up to 32 characters)"));
    }
    Ok(k)
}

/// Everything game servers need to hand these items out.
pub async fn custom_items_for_sync(state: &AppState) -> AppResult<Vec<Value>> {
    let rows: Vec<(String, String, String)> =
        sqlx::query_as("SELECT id, title, spec FROM custom_items ORDER BY id").fetch_all(&state.db).await?;
    Ok(rows
        .into_iter()
        .map(|(id, title, spec)| json!({ "id": id, "title": title, "spec": serde_json::from_str::<Value>(&spec).unwrap_or(json!({})) }))
        .collect())
}

pub async fn list_items(_: AdminUser, State(state): State<AppState>) -> AppResult<Json<Value>> {
    Ok(Json(json!({ "items": custom_items_for_sync(&state).await? })))
}

#[derive(Deserialize)]
pub struct ItemPayload {
    id: Option<String>,
    title: String,
    spec: Value,
}

pub async fn save_item(
    _: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(p): Json<ItemPayload>,
) -> AppResult<Json<Value>> {
    let key = clean_item_key(p.id.as_deref().unwrap_or(&id))?;
    let title = p.title.trim();
    if title.is_empty() || title.chars().count() > 60 {
        return Err(AppError::bad_request("Give the item a title (up to 60 characters) so you can find it in the list"));
    }
    let spec = clean_item(&p.spec)?;
    if spec.get("texture").is_some() || spec.get("model").is_some() {
        let assets: super::resource_assets::Assets = store::kv_get(&state, "resource_assets").await?;
        for (field, folder, extension) in [("texture", "textures", "png"), ("model", "models", "json")] {
            if let Some(reference) = spec[field].as_str() {
                let (namespace, path) = reference.split_once(':').unwrap();
                if !assets.files.contains_key(&format!("assets/{namespace}/{folder}/{path}.{extension}")) {
                    return Err(AppError::bad_request(format!("Upload the {field} asset before saving this item")));
                }
            }
        }
    }
    if spec.get("texture").is_some() || spec.get("model").is_some() {
        let existing = custom_items_for_sync(&state).await?;
        if existing.iter().any(|e| {
            e["id"] != key
                && e["spec"]["item"] == spec["item"]
                && e["spec"]["custom_model_data"] == spec["custom_model_data"]
                && (e["spec"].get("texture").is_some() || e["spec"].get("model").is_some())
        }) {
            return Err(AppError::bad_request("Another custom item uses this base item and model data number"));
        }
    }
    sqlx::query(
        "INSERT INTO custom_items (id, title, spec, created_at, updated_at) VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET title = excluded.title, spec = excluded.spec, updated_at = excluded.updated_at",
    )
    .bind(&key)
    .bind(title)
    .bind(spec.to_string())
    .bind(crate::db::now())
    .bind(crate::db::now())
    .execute(&state.db)
    .await?;
    Ok(Json(json!({ "ok": true, "id": key, "spec": spec })))
}

pub async fn delete_item(_: AdminUser, State(state): State<AppState>, Path(id): Path<String>) -> AppResult<Json<Value>> {
    let bundles: Vec<String> = sqlx::query_scalar("SELECT actions FROM reward_bundles").fetch_all(&state.db).await?;
    for raw in bundles {
        let actions: Vec<Value> = serde_json::from_str(&raw)?;
        if actions.iter().any(|a| a["type"] == "custom_item" && a["custom"] == id) {
            return Err(AppError::bad_request("A reward bundle still uses this item"));
        }
    }
    // Kits that hand it out would otherwise fail in game.
    let u = load(&state).await?;
    if let Some(k) = u.kits.iter().find(|k| k.items.iter().any(|i| i.get("custom").and_then(Value::as_str) == Some(id.as_str()))) {
        return Err(AppError::bad_request(format!("Kit \"{}\" still gives this item; remove it from the kit first", k.id)));
    }
    sqlx::query("DELETE FROM custom_items WHERE id = ?").bind(&id).execute(&state.db).await?;
    Ok(Json(json!({ "ok": true })))
}
