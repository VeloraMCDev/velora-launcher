//! Content Studio API: blocks, chests, decorations, NPCs, vehicles, crops and mobs (stored in `custom_content`) next to the
//! item-like kinds (tools, weapons, armor, food, items — stored as custom items), plus the analyse/import endpoints behind the
//! setup wizard.

use crate::state::RequestState as State;
use super::resource_assets::{valid_reference, Assets};
use crate::{
    auth::AdminUser,
    content_import::{self, Entry, ITEM_KINDS},
    error::{AppError, AppResult},
    pack_import,
    state::AppState,
    store,
};
use axum::{
    extract::{Path, Query},
    Json,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Deserialize;
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet};

pub const CONTENT_KINDS: [&str; 7] = ["block", "chest", "decoration", "npc", "vehicle", "crop", "mob"];
const FIRST_MODEL_DATA: i64 = 100_000;

// ---------------------------------------------------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------------------------------------------------

fn bad<T>(msg: impl Into<String>) -> AppResult<T> {
    Err(AppError::bad_request(msg))
}

fn one_line(v: &Value, max: usize, what: &str) -> AppResult<Option<String>> {
    match v.as_str().map(str::trim) {
        None | Some("") => Ok(None),
        Some(s) if s.chars().count() > max || s.contains('\n') || s.contains('\r') => bad(format!("{what} must be one line of up to {max} characters")),
        Some(s) => Ok(Some(s.to_string())),
    }
}

fn clamp_f(v: &Value, default: f64, lo: f64, hi: f64) -> f64 {
    v.as_f64().filter(|n| n.is_finite()).unwrap_or(default).clamp(lo, hi)
}

fn asset_exists(assets: &Assets, reference: &str, folder: &str, ext: &str) -> bool {
    reference.split_once(':').is_some_and(|(ns, p)| assets.files.contains_key(&format!("assets/{ns}/{folder}/{p}.{ext}")))
}

/// One visual: a base item, a model-data number and a texture or model reference.
fn clean_look(raw: &Value, assets: &Assets, require_look: bool, what: &str) -> AppResult<Value> {
    let item = raw["item"].as_str().unwrap_or("minecraft:paper").trim().to_lowercase();
    if !item.starts_with("minecraft:") || !valid_reference(&item) {
        return bad(format!("{what}: the base item must look like minecraft:paper"));
    }
    let mut out = Map::new();
    out.insert("item".into(), json!(item));
    let mut has_look = false;
    for (key, folder, ext) in [("texture", "textures", "png"), ("model", "models", "json")] {
        if let Some(r) = raw[key].as_str().map(str::trim).filter(|s| !s.is_empty()) {
            if !valid_reference(r) {
                return bad(format!("{what}: {key} must look like namespace:path"));
            }
            if !asset_exists(assets, r, folder, ext) {
                return bad(format!("{what}: the {key} {r} isn't in your server assets — import or upload it first"));
            }
            out.insert(key.into(), json!(r));
            has_look = true;
        }
    }
    if has_look {
        match raw["custom_model_data"].as_i64().filter(|m| (1..=16_777_215).contains(m)) {
            Some(m) => out.insert("custom_model_data".into(), json!(m)),
            None => return bad(format!("{what}: needs a model data number between 1 and 16777215")),
        };
    } else if require_look {
        return bad(format!("{what}: pick a texture or model"));
    }
    out.insert("scale".into(), json!(clamp_f(&raw["scale"], 1.0, 0.05, 32.0)));
    Ok(Value::Object(out))
}

fn clean_drops(raw: &Value) -> AppResult<Value> {
    let mut out = Vec::new();
    for d in raw.as_array().map(Vec::as_slice).unwrap_or(&[]).iter().take(32) {
        let item = d["item"].as_str().unwrap_or("").trim().to_lowercase();
        if item.is_empty() || item.len() > 80 || !item.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "_:-.".contains(c)) {
            return bad("A drop needs an item such as diamond or a custom item id");
        }
        let min = d["min"].as_i64().unwrap_or(1).clamp(0, 64);
        let max = d["max"].as_i64().unwrap_or(min).clamp(min, 64);
        out.push(json!({"item": item, "min": min, "max": max, "chance": clamp_f(&d["chance"], 1.0, 0.0, 1.0)}));
    }
    Ok(Value::Array(out))
}

fn clean_lines(raw: &Value, max_lines: usize, max_len: usize, what: &str) -> AppResult<Value> {
    let mut out = Vec::new();
    for l in raw.as_array().map(Vec::as_slice).unwrap_or(&[]).iter().take(max_lines) {
        if let Some(s) = one_line(l, max_len, what)? {
            out.push(json!(s));
        }
    }
    Ok(Value::Array(out))
}

fn clean_hitbox(raw: &Value, default_w: f64, default_h: f64) -> Value {
    json!({"width": clamp_f(&raw["width"], default_w, 0.1, 8.0), "height": clamp_f(&raw["height"], default_h, 0.1, 8.0)})
}

/// Normalise the spec of a non-item content entry, rejecting anything the game side could not use.
pub fn clean_content(kind: &str, raw: &Value, assets: &Assets) -> AppResult<Value> {
    if !CONTENT_KINDS.contains(&kind) {
        return bad("Unknown content kind");
    }
    let mut out = Map::new();
    // Mobs can stay vanilla-looking; everything else needs something to show.
    let look = clean_look(&raw["display"], assets, kind != "mob" && kind != "crop", "Appearance")?;
    out.insert("display".into(), look);
    if let Some(n) = one_line(&raw["name"], 100, "The name")? {
        out.insert("name".into(), json!(n));
    }
    out.insert("lore".into(), clean_lines(&raw["lore"], 12, 160, "Lore lines")?);
    match kind {
        "block" => {
            out.insert("hardness".into(), json!(clamp_f(&raw["hardness"], 1.5, 0.0, 600.0)));
            out.insert("light".into(), json!(raw["light"].as_i64().unwrap_or(0).clamp(0, 15)));
            out.insert("drops".into(), clean_drops(&raw["drops"])?);
        }
        "chest" => {
            out.insert("rows".into(), json!(raw["rows"].as_i64().unwrap_or(3).clamp(1, 6)));
            if let Some(t) = one_line(&raw["title"], 40, "The chest title")? {
                out.insert("title".into(), json!(t));
            }
        }
        "decoration" => {
            out.insert("seat".into(), json!(raw["seat"].as_bool().unwrap_or(false)));
            out.insert("solid".into(), json!(raw["solid"].as_bool().unwrap_or(false)));
            out.insert("hitbox".into(), clean_hitbox(&raw["hitbox"], 1.0, 1.0));
        }
        "npc" => {
            out.insert("name_visible".into(), json!(raw["name_visible"].as_bool().unwrap_or(true)));
            out.insert("commands".into(), clean_lines(&raw["commands"], 8, 200, "Commands")?);
            out.insert("messages".into(), clean_lines(&raw["messages"], 8, 200, "Messages")?);
            out.insert("hitbox".into(), clean_hitbox(&raw["hitbox"], 0.8, 1.9));
        }
        "vehicle" => {
            let mount = raw["mount"].as_str().unwrap_or("horse");
            if !["horse", "donkey", "mule"].contains(&mount) {
                return bad("A vehicle rides on a horse, donkey or mule (the ones players can steer with just a saddle)");
            }
            out.insert("mount".into(), json!(mount));
            out.insert("speed".into(), json!(clamp_f(&raw["speed"], 0.25, 0.05, 1.0)));
            out.insert("hitbox".into(), clean_hitbox(&raw["hitbox"], 1.2, 1.2));
        }
        "crop" => {
            let stages = raw["stages"].as_array().map(Vec::as_slice).unwrap_or(&[]);
            if !(2..=8).contains(&stages.len()) {
                return bad("A crop needs between 2 and 8 growth stages");
            }
            let mut list = Vec::new();
            for (i, s) in stages.iter().enumerate() {
                let mut s = s.clone();
                s["item"] = out["display"]["item"].clone();
                list.push(clean_look(&s, assets, true, &format!("Stage {}", i + 1))?);
            }
            out.insert("stages".into(), Value::Array(list));
            out.insert("growth_seconds".into(), json!(raw["growth_seconds"].as_i64().unwrap_or(600).clamp(10, 7 * 86_400)));
            out.insert("replant".into(), json!(raw["replant"].as_bool().unwrap_or(true)));
            out.insert("drops".into(), clean_drops(&raw["drops"])?);
        }
        "mob" => {
            let entity = raw["entity"].as_str().unwrap_or("ZOMBIE").trim().to_uppercase();
            if entity.is_empty() || entity.len() > 40 || !entity.chars().all(|c| c.is_ascii_uppercase() || c == '_') {
                return bad("The mob's entity type looks like ZOMBIE or WITHER_SKELETON");
            }
            out.insert("entity".into(), json!(entity));
            out.insert("health".into(), json!(clamp_f(&raw["health"], 20.0, 1.0, 1024.0)));
            out.insert("damage".into(), json!(clamp_f(&raw["damage"], 2.0, 0.0, 1024.0)));
            out.insert("armor".into(), json!(clamp_f(&raw["armor"], 0.0, 0.0, 30.0)));
            out.insert("speed".into(), json!(clamp_f(&raw["speed"], 0.0, 0.0, 2.0)));
            out.insert("drops".into(), clean_drops(&raw["drops"])?);
            out.insert("hitbox".into(), clean_hitbox(&raw["hitbox"], 0.6, 1.8));
        }
        _ => {}
    }
    Ok(Value::Object(out))
}

// ---------------------------------------------------------------------------------------------------------------------
// Storage helpers
// ---------------------------------------------------------------------------------------------------------------------

pub async fn content_for_sync(state: &AppState) -> AppResult<Vec<Value>> {
    let rows: Vec<(String, String, String, String, String)> =
        sqlx::query_as("SELECT id, kind, title, source, spec FROM custom_content ORDER BY id").fetch_all(&state.db).await?;
    Ok(rows
        .into_iter()
        .map(|(id, kind, title, source, spec)| json!({"id": id, "kind": kind, "title": title, "source": source, "spec": serde_json::from_str::<Value>(&spec).unwrap_or(json!({}))}))
        .collect())
}

/// Every visual the resource pack must carry for content: shaped like custom items so the pack builder treats them the same.
pub async fn pack_displays(state: &AppState) -> AppResult<Vec<Value>> {
    let mut out = Vec::new();
    for c in content_for_sync(state).await? {
        let id = c["id"].as_str().unwrap_or("");
        let mut looks = vec![c["spec"]["display"].clone()];
        looks.extend(c["spec"]["stages"].as_array().cloned().unwrap_or_default());
        for (i, look) in looks.into_iter().enumerate() {
            if look["custom_model_data"].is_null() || (look["texture"].is_null() && look["model"].is_null()) {
                continue;
            }
            out.push(json!({"id": format!("c_{id}_{i}"), "spec": {
                "item": look["item"], "custom_model_data": look["custom_model_data"], "texture": look["texture"], "model": look["model"],
            }}));
        }
    }
    // Modelled cosmetics (Cosmetics Studio) ride along in the same pack.
    let cosmetics: Vec<(i64, String)> = sqlx::query_as("SELECT id, metadata FROM cosmetic_templates WHERE type = 'cosmetic'").fetch_all(&state.db).await?;
    for (id, metadata) in cosmetics {
        let look = serde_json::from_str::<Value>(&metadata).unwrap_or(json!({}))["look"].clone();
        if look["custom_model_data"].is_null() || (look["texture"].is_null() && look["model"].is_null()) {
            continue;
        }
        out.push(json!({"id": format!("cosmetic_{id}"), "spec": {
            "item": look["item"], "custom_model_data": look["custom_model_data"], "texture": look["texture"], "model": look["model"],
        }}));
    }
    Ok(out)
}

/// (material, model data) pairs already used by items or content, plus numbers claimed inside a pack being imported.
pub(crate) async fn taken_model_data(state: &AppState) -> AppResult<BTreeSet<(String, i64)>> {
    let mut taken = BTreeSet::new();
    for it in super::utilities::custom_items_for_sync(state).await? {
        if let (Some(m), Some(n)) = (it["spec"]["item"].as_str(), it["spec"]["custom_model_data"].as_i64()) {
            taken.insert((m.to_string(), n));
        }
    }
    for d in pack_displays(state).await? {
        if let (Some(m), Some(n)) = (d["spec"]["item"].as_str(), d["spec"]["custom_model_data"].as_i64()) {
            taken.insert((m.to_string(), n));
        }
    }
    Ok(taken)
}

pub(crate) fn claim(taken: &mut BTreeSet<(String, i64)>, material: &str, wanted: Option<i64>) -> i64 {
    let mut n = wanted.filter(|n| (1..=16_777_215).contains(n)).unwrap_or(FIRST_MODEL_DATA);
    while taken.contains(&(material.to_string(), n)) {
        n += 1;
    }
    taken.insert((material.to_string(), n));
    n
}

async fn id_in_use(state: &AppState, id: &str) -> AppResult<bool> {
    let a: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM custom_items WHERE id = ?").bind(id).fetch_one(&state.db).await?;
    let b: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM custom_content WHERE id = ?").bind(id).fetch_one(&state.db).await?;
    Ok(a + b > 0)
}

fn kind_of_item(spec: &Value) -> String {
    spec["category"].as_str().map(String::from).unwrap_or_else(|| {
        let m = spec["item"].as_str().unwrap_or("");
        let probe = content_import::ITEM_KINDS.iter().find(|_| false);
        let _ = probe;
        if m.ends_with("_helmet") || m.ends_with("_chestplate") || m.ends_with("_leggings") || m.ends_with("_boots") { "armor" }
        else if m.ends_with("_sword") || m.ends_with("bow") || m.ends_with("trident") { "weapon" }
        else if m.ends_with("_axe") || m.ends_with("_pickaxe") || m.ends_with("_shovel") || m.ends_with("_hoe") { "tool" }
        else { "item" }
        .to_string()
    })
}

fn preview_of(assets: &Assets, look: &Value) -> Option<String> {
    let get = |p: &str| assets.files.get(p).and_then(|b| STANDARD.decode(b).ok());
    let bytes = pack_import::preview_png(get, look["texture"].as_str(), look["model"].as_str())?;
    (bytes.len() <= 48 * 1024).then(|| format!("data:image/png;base64,{}", STANDARD.encode(bytes)))
}

// ---------------------------------------------------------------------------------------------------------------------
// Library
// ---------------------------------------------------------------------------------------------------------------------

/// Everything in the library: items and content together, each with a small picture.
pub async fn list(_: AdminUser, State(state): State<AppState>) -> AppResult<Json<Value>> {
    let assets: Assets = store::kv_get(&state, "resource_assets").await?;
    let mut out = Vec::new();
    for it in super::utilities::custom_items_for_sync(&state).await? {
        let spec = &it["spec"];
        out.push(json!({
            "id": it["id"], "kind": kind_of_item(spec), "title": it["title"], "source": spec["source"].as_str().unwrap_or("manual"), "store": "item",
            "name": spec["name"], "item": spec["item"], "preview": preview_of(&assets, spec), "has_look": spec["texture"].is_string() || spec["model"].is_string(),
        }));
    }
    for c in content_for_sync(&state).await? {
        out.push(json!({
            "id": c["id"], "kind": c["kind"], "title": c["title"], "source": c["source"], "store": "content", "name": c["spec"]["name"],
            "item": c["spec"]["display"]["item"], "preview": preview_of(&assets, &c["spec"]["display"]), "has_look": true,
        }));
    }
    Ok(Json(json!({"items": out})))
}

pub async fn get_one(_: AdminUser, State(state): State<AppState>, Path(id): Path<String>) -> AppResult<Json<Value>> {
    let row: Option<(String, String, String, String)> =
        sqlx::query_as("SELECT kind, title, source, spec FROM custom_content WHERE id = ?").bind(&id).fetch_optional(&state.db).await?;
    let (kind, title, source, spec) = row.ok_or_else(|| AppError::not_found("No such content"))?;
    Ok(Json(json!({"id": id, "kind": kind, "title": title, "source": source, "spec": serde_json::from_str::<Value>(&spec).unwrap_or(json!({}))})))
}

#[derive(Deserialize)]
pub struct SaveBody {
    kind: String,
    title: String,
    spec: Value,
    source: Option<String>,
}

pub async fn save(_: AdminUser, State(state): State<AppState>, Path(id): Path<String>, Json(b): Json<SaveBody>) -> AppResult<Json<Value>> {
    let key = super::utilities::clean_item_key(&id)?;
    let title = b.title.trim();
    if title.is_empty() || title.chars().count() > 60 {
        return bad("Give it a title (up to 60 characters)");
    }
    let assets: Assets = store::kv_get(&state, "resource_assets").await?;
    let existing_kind: Option<String> = sqlx::query_scalar("SELECT kind FROM custom_content WHERE id = ?").bind(&key).fetch_optional(&state.db).await?;
    let in_items: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM custom_items WHERE id = ?").bind(&key).fetch_one(&state.db).await?;
    if in_items > 0 {
        return bad("An item with this id already exists");
    }
    // Model data numbers must be unique per base item across everything (this entry's own old numbers are free again).
    let mut taken = taken_model_data(&state).await?;
    if existing_kind.is_some() {
        let mine = content_for_sync(&state).await?.into_iter().find(|c| c["id"] == key);
        for look in mine.iter().flat_map(|c| std::iter::once(c["spec"]["display"].clone()).chain(c["spec"]["stages"].as_array().cloned().unwrap_or_default())) {
            if let (Some(i), Some(n)) = (look["item"].as_str(), look["custom_model_data"].as_i64()) {
                taken.remove(&(i.to_string(), n));
            }
        }
    }
    // Pick numbers for looks that have a texture/model but no number yet, then validate the whole thing.
    let mut raw = b.spec.clone();
    let mut explicit: Vec<(String, i64)> = Vec::new();
    for look in std::iter::once(&raw["display"]).chain(raw["stages"].as_array().into_iter().flatten()) {
        if let (Some(i), Some(n)) = (look["item"].as_str(), look["custom_model_data"].as_i64()) {
            explicit.push((i.to_string(), n));
        }
    }
    for (i, n) in &explicit {
        if !taken.insert((i.clone(), *n)) {
            return bad(format!("Model data {n} is already used on {i} by another item; pick another number"));
        }
    }
    let assign = |look: &mut Value, taken: &mut BTreeSet<(String, i64)>| {
        let has_look = look["texture"].as_str().is_some_and(|t| !t.is_empty()) || look["model"].as_str().is_some_and(|t| !t.is_empty());
        if has_look && look["custom_model_data"].as_i64().is_none() {
            let item = look["item"].as_str().unwrap_or("minecraft:paper").to_lowercase();
            look["item"] = json!(item);
            look["custom_model_data"] = json!(claim(taken, &item, None));
        }
    };
    if raw.get("display").is_some() {
        assign(&mut raw["display"], &mut taken);
    }
    if let Some(stages) = raw.get_mut("stages").and_then(Value::as_array_mut) {
        for st in stages {
            assign(st, &mut taken);
        }
    }
    let spec = clean_content(&b.kind, &raw, &assets)?;
    sqlx::query(
        "INSERT INTO custom_content (id, kind, title, source, spec, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET kind = excluded.kind, title = excluded.title, spec = excluded.spec, updated_at = excluded.updated_at",
    )
    .bind(&key)
    .bind(&b.kind)
    .bind(title)
    .bind(b.source.as_deref().unwrap_or("manual"))
    .bind(spec.to_string())
    .bind(crate::db::now())
    .bind(crate::db::now())
    .execute(&state.db)
    .await?;
    Ok(Json(json!({"ok": true, "id": key, "spec": spec})))
}

pub async fn delete(_: AdminUser, State(state): State<AppState>, Path(id): Path<String>) -> AppResult<Json<Value>> {
    let n = sqlx::query("DELETE FROM custom_content WHERE id = ?").bind(&id).execute(&state.db).await?.rows_affected();
    if n == 0 {
        return Err(AppError::not_found("No such content"));
    }
    Ok(Json(json!({"ok": true})))
}

#[derive(Deserialize)]
pub struct ViewQuery {
    texture: Option<String>,
    model: Option<String>,
}

/// A 3D-viewer bundle for something already in the server assets.
pub async fn view(_: AdminUser, State(state): State<AppState>, Query(q): Query<ViewQuery>) -> AppResult<Json<Value>> {
    let assets: Assets = store::kv_get(&state, "resource_assets").await?;
    let get = |p: &str| assets.files.get(p).and_then(|b| STANDARD.decode(b).ok());
    Ok(Json(json!({"view": crate::model_view::view(&get, q.texture.as_deref().filter(|s| !s.is_empty()), q.model.as_deref().filter(|s| !s.is_empty()))})))
}

// ---------------------------------------------------------------------------------------------------------------------
// Wizard: analyse and import
// ---------------------------------------------------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct AnalyzeBody {
    data: String,
    /// Entry keys that need a 3D bundle even when the response is large.
    #[serde(default)]
    view_for: Vec<String>,
}

async fn run_analysis(data: &str) -> AppResult<content_import::Analysis> {
    let zip = STANDARD.decode(data.trim()).map_err(|_| AppError::bad_request("Invalid base64 zip"))?;
    tokio::task::spawn_blocking(move || content_import::analyze(&zip))
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?
        .map_err(AppError::bad_request)
}

fn entry_json(e: &Entry, files: &BTreeMap<String, Vec<u8>>, exists: bool, want_view: bool, budget: &mut usize) -> Value {
    let get = |p: &str| files.get(p).cloned();
    let preview = pack_import::preview_png(get, e.texture.as_deref(), e.model.as_deref())
        .filter(|b| b.len() <= 48 * 1024)
        .map(|b| format!("data:image/png;base64,{}", STANDARD.encode(b)));
    let mut j = serde_json::to_value(e).unwrap_or(json!({}));
    j["preview"] = json!(preview);
    j["exists"] = json!(exists);
    j["store"] = json!(if ITEM_KINDS.contains(&e.kind.as_str()) { "item" } else { "content" });
    let first_stage = e.extras.get("stages").and_then(|s| s.get(0));
    let (tex, model) = (e.texture.as_deref().or_else(|| first_stage.and_then(|s| s["texture"].as_str())), e.model.as_deref().or_else(|| first_stage.and_then(|s| s["model"].as_str())));
    if want_view || *budget > 0 {
        if let Some(v) = crate::model_view::view(&get, tex, model) {
            let size = v.to_string().len();
            if want_view || size <= *budget {
                *budget = budget.saturating_sub(size);
                j["view"] = v;
            }
        }
    }
    j
}

pub async fn analyze(_: AdminUser, State(state): State<AppState>, Json(b): Json<AnalyzeBody>) -> AppResult<Json<Value>> {
    let a = run_analysis(&b.data).await?;
    let stored: Assets = store::kv_get(&state, "resource_assets").await?;
    let mut budget: usize = 10 * 1024 * 1024;
    let mut entries = Vec::new();
    for e in &a.entries {
        let exists = id_in_use(&state, &e.id).await?;
        entries.push(entry_json(e, &a.files, exists, b.view_for.contains(&e.key), &mut budget));
    }
    let replaced = a.files.keys().filter(|k| stored.files.contains_key(*k)).count();
    Ok(Json(json!({
        "detected": a.detected, "entries": entries, "notes": a.notes, "skipped": a.skipped,
        "files": a.files.len(), "files_replaced": replaced,
    })))
}

#[derive(Deserialize, Default)]
pub struct Pick {
    key: String,
    kind: Option<String>,
    title: Option<String>,
    name: Option<String>,
    material: Option<String>,
    scale: Option<f64>,
    /// Overrides for the kind-specific numbers (hardness, rows, health…).
    extras: Option<Map<String, Value>>,
}

#[derive(Deserialize)]
pub struct ImportBody {
    data: String,
    entries: Vec<Pick>,
    /// Also keep every texture/model in the zip, not only what the chosen entries use.
    #[serde(default)]
    all_assets: bool,
}

/// Files an entry needs: its models (with their parents), textures and animation metadata.
fn needed_files(entry_looks: &[(Option<String>, Option<String>)], files: &BTreeMap<String, Vec<u8>>) -> BTreeSet<String> {
    let mut need = BTreeSet::new();
    let mut queue: Vec<String> = Vec::new();
    for (tex, model) in entry_looks {
        if let Some((ns, p)) = tex.as_deref().and_then(|t| t.split_once(':')) {
            need.insert(format!("assets/{ns}/textures/{p}.png"));
        }
        if let Some((ns, p)) = model.as_deref().and_then(|t| t.split_once(':')) {
            queue.push(format!("assets/{ns}/models/{p}.json"));
        }
    }
    let mut guard = 0;
    while let Some(path) = queue.pop() {
        guard += 1;
        if guard > 4000 || !need.insert(path.clone()) {
            continue;
        }
        let Some(json) = files.get(&path).and_then(|b| serde_json::from_slice::<Value>(b).ok()) else { continue };
        let own = path.strip_prefix("assets/").and_then(|r| r.split_once('/')).map_or("minecraft", |x| x.0).to_string();
        if let Some(p) = json["parent"].as_str() {
            let (ns, rel) = p.split_once(':').map_or(("minecraft".to_string(), p.to_string()), |(a, b)| (a.to_string(), b.to_string()));
            queue.push(format!("assets/{ns}/models/{rel}.json"));
        }
        for t in json["textures"].as_object().into_iter().flat_map(|m| m.values()).filter_map(Value::as_str).filter(|t| !t.starts_with('#')) {
            let (ns, rel) = t.split_once(':').map_or((own.clone(), t.to_string()), |(a, b)| (a.to_string(), b.to_string()));
            let p = format!("assets/{ns}/textures/{rel}.png");
            let p = if files.contains_key(&p) { p } else { format!("assets/minecraft/textures/{rel}.png") };
            need.insert(p);
        }
    }
    let metas: Vec<String> = need.iter().map(|p| format!("{p}.mcmeta")).filter(|p| files.contains_key(p)).collect();
    need.extend(metas);
    need.retain(|p| files.contains_key(p));
    need
}

pub async fn import(_: AdminUser, State(state): State<AppState>, Json(b): Json<ImportBody>) -> AppResult<Json<Value>> {
    let a = run_analysis(&b.data).await?;
    let by_key: BTreeMap<&str, &Entry> = a.entries.iter().map(|e| (e.key.as_str(), e)).collect();
    let mut assets: Assets = store::kv_get(&state, "resource_assets").await?;
    let mut taken = taken_model_data(&state).await?;
    // Numbers the imported pack's own vanilla-item models already use must not be reused for the same base item.
    for (path, bytes) in &a.files {
        if let Some(mat) = path.strip_prefix("assets/minecraft/models/item/").and_then(|p| p.strip_suffix(".json")) {
            if let Some(overrides) = serde_json::from_slice::<Value>(bytes).ok().and_then(|j| j["overrides"].as_array().cloned()) {
                for o in overrides {
                    if let Some(n) = o["predicate"]["custom_model_data"].as_i64() {
                        taken.insert((format!("minecraft:{mat}"), n));
                    }
                }
            }
        }
    }

    let mut report = Vec::new();
    let mut looks: Vec<(Option<String>, Option<String>)> = Vec::new();
    let mut item_rows: Vec<(String, String, Value)> = Vec::new();
    let mut content_rows: Vec<(String, String, String, Value, String)> = Vec::new();
    for pick in &b.entries {
        let Some(e) = by_key.get(pick.key.as_str()) else {
            report.push(json!({"key": pick.key, "status": "missing", "note": "Not found in the zip"}));
            continue;
        };
        if id_in_use(&state, &e.id).await? || item_rows.iter().any(|r| r.0 == e.id) || content_rows.iter().any(|r| r.0 == e.id) {
            report.push(json!({"key": e.key, "id": e.id, "status": "exists", "note": "An item or content with this id already exists"}));
            continue;
        }
        let kind = pick.kind.clone().unwrap_or_else(|| e.kind.clone());
        if !ITEM_KINDS.contains(&kind.as_str()) && !CONTENT_KINDS.contains(&kind.as_str()) {
            report.push(json!({"key": e.key, "id": e.id, "status": "skipped", "note": "Unknown kind"}));
            continue;
        }
        let title: String = pick.title.as_deref().map(str::trim).filter(|t| !t.is_empty()).unwrap_or(&e.title).chars().take(60).collect();
        let name = match &pick.name {
            Some(n) => Some(n.trim().to_string()).filter(|n| !n.is_empty()),
            None => e.name.clone(),
        };
        let mut material = pick.material.as_deref().map(|m| if m.contains(':') { m.to_lowercase() } else { format!("minecraft:{}", m.to_lowercase()) }).unwrap_or_else(|| e.material.clone());
        if !ITEM_KINDS.contains(&kind.as_str()) && pick.material.is_none() && e.framework != "modelengine" && e.framework != "mythicmobs" {
            // Placeable things are handed out as their original base item.
            material = e.material.clone();
        }
        let mut extras = e.extras.clone();
        for (k, v) in pick.extras.iter().flatten() {
            extras.insert(k.clone(), v.clone());
        }
        let scale = pick.scale.unwrap_or(e.scale);
        looks.push((e.texture.clone(), e.model.clone()));
        if let Some(stages) = extras.get("stages").and_then(Value::as_array) {
            looks.extend(stages.iter().map(|s| (s["texture"].as_str().map(String::from), s["model"].as_str().map(String::from))));
        }

        let result = if ITEM_KINDS.contains(&kind.as_str()) {
            let has_look = e.texture.is_some() || e.model.is_some();
            let mut spec = json!({"item": material, "amount": 1, "category": kind, "source": e.framework});
            if has_look {
                spec["custom_model_data"] = json!(claim(&mut taken, &material, e.cmd));
                if let Some(m) = &e.model { spec["model"] = json!(m); } else if let Some(t) = &e.texture { spec["texture"] = json!(t); }
            }
            if let Some(n) = &name { spec["name"] = json!(n); }
            if !e.lore.is_empty() { spec["lore"] = json!(e.lore); }
            if kind == "food" {
                spec["food"] = json!({"nutrition": extras.get("nutrition").cloned().unwrap_or(json!(4)), "saturation": extras.get("saturation").cloned().unwrap_or(json!(2))});
            }
            match super::utilities::clean_item(&spec) {
                Ok(spec) => { item_rows.push((e.id.clone(), title.clone(), spec)); Ok(()) }
                Err(err) => Err(err.message),
            }
        } else {
            let mut raw = Map::new();
            let mut display = json!({"item": material, "scale": scale});
            if e.texture.is_some() || e.model.is_some() {
                display["custom_model_data"] = json!(claim(&mut taken, &material, e.cmd));
                if let Some(m) = &e.model { display["model"] = json!(m); } else if let Some(t) = &e.texture { display["texture"] = json!(t); }
            }
            raw.insert("display".into(), display);
            if let Some(n) = &name { raw.insert("name".into(), json!(n)); }
            raw.insert("lore".into(), json!(e.lore));
            for (k, v) in &extras {
                if k != "stages" {
                    raw.insert(k.clone(), v.clone());
                }
            }
            if let Some(stages) = extras.get("stages").and_then(Value::as_array) {
                let list: Vec<Value> = stages.iter().map(|s| {
                    let mut st = json!({"item": material, "custom_model_data": claim(&mut taken, &material, None)});
                    if let Some(m) = s["model"].as_str() { st["model"] = json!(m); } else if let Some(t) = s["texture"].as_str() { st["texture"] = json!(t); }
                    st
                }).collect();
                if let Some(first) = list.first() {
                    let mut d = raw["display"].clone();
                    d["custom_model_data"] = first["custom_model_data"].clone();
                    for k in ["texture", "model"] { d[k] = first[k].clone(); }
                    raw.insert("display".into(), d);
                }
                raw.insert("stages".into(), Value::Array(list));
            }
            // Hitboxes from model sizes (ModelEngine).
            if let (Some(h), Some(w)) = (extras.get("height"), extras.get("width")) {
                raw.insert("hitbox".into(), json!({"width": w, "height": h}));
            }
            // Validate against the assets as they will be after this import.
            let mut preview_assets = Assets { files: assets.files.clone() };
            for p in needed_files(&looks, &a.files) {
                preview_assets.files.entry(p).or_default();
            }
            match clean_content(&kind, &Value::Object(raw), &preview_assets) {
                Ok(spec) => { content_rows.push((e.id.clone(), kind.clone(), title.clone(), spec, e.framework.to_string())); Ok(()) }
                Err(err) => Err(err.message),
            }
        };
        match result {
            Ok(()) => report.push(json!({"key": e.key, "id": e.id, "kind": kind, "title": title, "status": "created", "store": if ITEM_KINDS.contains(&kind.as_str()) { "item" } else { "content" }})),
            Err(note) => report.push(json!({"key": e.key, "id": e.id, "status": "skipped", "note": note})),
        }
    }

    // Assets: what the chosen entries use (or everything when asked), merged into the server assets.
    let wanted: BTreeSet<String> = if b.all_assets { a.files.keys().cloned().collect() } else { needed_files(&looks, &a.files) };
    let (mut added, mut replaced) = (0, 0);
    for path in &wanted {
        if let Some(bytes) = a.files.get(path) {
            if assets.files.insert(path.clone(), STANDARD.encode(bytes)).is_some() { replaced += 1 } else { added += 1 }
        }
    }
    // Atlas files shipped by the pack are small and decide whether textures outside item/ and block/ load.
    for (path, bytes) in a.files.iter().filter(|(p, _)| p.starts_with("assets/minecraft/atlases/")) {
        assets.files.entry(path.clone()).or_insert_with(|| STANDARD.encode(bytes));
    }
    if assets.files.len() > super::resource_assets::MAX_FILES || assets.files.values().map(String::len).sum::<usize>() > 64 * 1024 * 1024 {
        return bad("The server assets would be full (limit 2048 files / 48 MiB). Import fewer entries or remove assets first");
    }

    let mut tx = state.db.begin().await?;
    let now = crate::db::now();
    for (id, title, spec) in &item_rows {
        sqlx::query("INSERT INTO custom_items (id, title, spec, created_at, updated_at) VALUES (?, ?, ?, ?, ?)")
            .bind(id).bind(title).bind(spec.to_string()).bind(&now).bind(&now).execute(&mut *tx).await?;
    }
    for (id, kind, title, spec, source) in &content_rows {
        sqlx::query("INSERT INTO custom_content (id, kind, title, source, spec, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)")
            .bind(id).bind(kind).bind(title).bind(source).bind(spec.to_string()).bind(&now).bind(&now).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    store::kv_set(&state, "resource_assets", &assets).await?;
    let pack_on = store::kv_get::<super::resource_assets::PackSettings>(&state, "resource_pack").await?.enabled;
    Ok(Json(json!({
        "created": report.iter().filter(|r| r["status"] == "created").count(),
        "entries": report, "files_added": added, "files_replaced": replaced, "pack_enabled": pack_on,
        "notes": a.notes, "skipped": a.skipped,
    })))
}
