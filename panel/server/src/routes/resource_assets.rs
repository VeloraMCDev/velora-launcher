//! Custom textures, models, sounds and generated item resource packs.
use crate::state::RequestState as State;
use crate::{
    auth::AdminUser,
    error::{AppError, AppResult},
    state::AppState,
    store,
};
use axum::{
    extract::{Path, Query},
    http::header,
    response::IntoResponse,
    Json,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha1::{Digest, Sha1};
use std::{
    collections::BTreeMap,
    io::{Cursor, Write},
};

pub const MAX_FILES: usize = 2048;
/// Legacy `pack_format` written to new packs (Minecraft 26.x). Clients that understand a version range ignore it, so it only
/// matters to very old game versions; the range below is what makes the pack load everywhere.
pub const DEFAULT_PACK_FORMAT: u32 = 84;

/// `pack.mcmeta` that every Minecraft version from 1.20.2 up accepts. Minecraft refuses a server pack whose format it doesn't
/// recognise (the red entry in the pack list), and the number changes with nearly every release, so instead of one value the
/// pack declares a wide range, in every spelling the game has used: `supported_formats` (1.20.2+) and `min_format` /
/// `max_format` (1.21.9+, including 26.x), next to the legacy `pack_format`.
pub fn mcmeta(pack_format: u32) -> Value {
    json!({"pack": {
        "pack_format": pack_format,
        "supported_formats": {"min_inclusive": 15, "max_inclusive": 9999},
        "min_format": 15,
        "max_format": 9999,
        "description": "Velora custom assets",
    }})
}

#[derive(Serialize, Deserialize, Default)]
pub struct Assets {
    pub files: BTreeMap<String, String>,
}
#[derive(Serialize, Deserialize)]
pub struct PackSettings {
    pub enabled: bool,
    pub required: bool,
    pub pack_format: u32,
    /// Keep imported replacements for vanilla blocks, colour maps and shaders. Off by default: they are what corrupts the world's
    /// textures on clients that load more than this one pack.
    #[serde(default)]
    pub allow_vanilla_overrides: bool,
}
impl Default for PackSettings {
    fn default() -> Self {
        Self { enabled: false, required: false, pack_format: DEFAULT_PACK_FORMAT, allow_vanilla_overrides: false }
    }
}

pub fn valid_reference(s: &str) -> bool {
    s.split_once(':').is_some_and(|(ns, path)| {
        !ns.is_empty()
            && !path.is_empty()
            && !path.split('/').any(|p| p.is_empty() || p == "." || p == "..")
            && ns.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "_.-".contains(c))
            && path.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "_.-/".contains(c))
    })
}

pub async fn list(_: AdminUser, State(state): State<AppState>) -> AppResult<Json<Value>> {
    let assets: Assets = store::kv_get(&state, "resource_assets").await?;
    let settings: PackSettings = store::kv_get(&state, "resource_pack").await?;
    Ok(Json(json!({"files": assets.files.keys().collect::<Vec<_>>(), "settings": settings})))
}

#[derive(Deserialize)]
pub struct AssetBody {
    path: String,
    data: String,
}
pub async fn upload(_: AdminUser, State(state): State<AppState>, Json(body): Json<AssetBody>) -> AppResult<Json<Value>> {
    let parts: Vec<_> = body.path.split('/').collect();
    if parts.len() < 3
        || parts[0] != "assets"
        || !valid_reference(&format!("{}:{}", parts[1], parts[2..].join("/")))
        || !(parts.len() == 3 && parts[2] == "sounds.json"
            || parts.len() >= 4 && ["textures", "models", "sounds", "font", "items", "lang", "atlases"].contains(&parts[2]))
        || !["png", "json", "ogg"].iter().any(|ext| body.path.ends_with(&format!(".{ext}")))
    {
        return Err(AppError::bad_request("Use a resource-pack path such as assets/scopenet/textures/item/blade.png"));
    }
    let bytes = STANDARD.decode(&body.data).map_err(|_| AppError::bad_request("Invalid base64 asset"))?;
    if bytes.is_empty() || bytes.len() > 2 * 1024 * 1024 {
        return Err(AppError::bad_request("Assets must be at most 2 MiB"));
    }
    if body.path.ends_with(".json") {
        if !serde_json::from_slice::<Value>(&bytes)?.is_object() {
            return Err(AppError::bad_request("Resource JSON must be an object"));
        }
    }
    if body.path.ends_with(".png") {
        let decoder = image::codecs::png::PngDecoder::new(Cursor::new(&bytes)).map_err(|_| AppError::bad_request("Invalid PNG"))?;
        use image::ImageDecoder;
        let (w, h) = decoder.dimensions();
        if w > 4096 || h > 4096 {
            return Err(AppError::bad_request("Textures must be at most 4096 pixels on each side"));
        }
    }
    if body.path.ends_with(".ogg") && !bytes.starts_with(b"OggS") {
        return Err(AppError::bad_request("Invalid Ogg audio"));
    }
    let mut assets: Assets = store::kv_get(&state, "resource_assets").await?;
    assets.files.insert(body.path, body.data);
    if assets.files.len() > MAX_FILES || assets.files.values().map(String::len).sum::<usize>() > 64 * 1024 * 1024 {
        return Err(AppError::bad_request("Resource pack is full (2048 files / 48 MiB)"));
    }
    store::kv_set(&state, "resource_assets", &assets).await?;
    Ok(Json(json!({"ok": true})))
}

#[derive(Deserialize)]
pub struct ImportBody {
    /// Base64 zip of an Oraxen or ItemsAdder pack (or any resource pack).
    data: String,
    #[serde(default)]
    dry_run: bool,
    #[serde(default = "yes")]
    create_items: bool,
    /// Only create these item ids (all when absent).
    only: Option<Vec<String>>,
    /// Per-item changes made in the import preview.
    #[serde(default)]
    edits: std::collections::BTreeMap<String, ItemEdit>,
}
#[derive(Deserialize, Default)]
pub struct ItemEdit {
    title: Option<String>,
    name: Option<String>,
    item: Option<String>,
}
fn yes() -> bool {
    true
}

/// Bring in an Oraxen / ItemsAdder pack: its textures, models and sounds become server assets, and its item definitions become
/// custom items (existing items with the same id are left alone). With `dry_run` nothing is stored.
pub async fn import_pack(_: AdminUser, State(state): State<AppState>, Json(body): Json<ImportBody>) -> AppResult<Json<Value>> {
    let zip = STANDARD.decode(body.data.trim()).map_err(|_| AppError::bad_request("Invalid base64 pack"))?;
    let imported = tokio::task::spawn_blocking(move || crate::pack_import::parse(&zip))
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?
        .map_err(AppError::bad_request)?;

    let mut assets: Assets = store::kv_get(&state, "resource_assets").await?;
    let mut added = 0;
    let mut replaced = 0;
    for (path, bytes) in &imported.files {
        if assets.files.contains_key(path) {
            replaced += 1;
        } else {
            added += 1;
        }
        if !body.dry_run {
            assets.files.insert(path.clone(), STANDARD.encode(bytes));
        }
    }
    if body.dry_run {
        let total = assets.files.len() + added;
        if total > MAX_FILES {
            return Err(AppError::bad_request(format!("The pack would hold {total} files; the limit is {MAX_FILES}")));
        }
    } else if assets.files.len() > MAX_FILES || assets.files.values().map(String::len).sum::<usize>() > 64 * 1024 * 1024 {
        return Err(AppError::bad_request("Resource pack is full (2048 files / 48 MiB). Remove some assets first"));
    }

    // Items: keep model-data numbers unique per base item, across what is already stored too.
    let existing = super::utilities::custom_items_for_sync(&state).await?;
    let existing_keys: std::collections::BTreeSet<String> = existing.iter().filter_map(|e| e["id"].as_str().map(String::from)).collect();
    let mut taken: std::collections::BTreeSet<(String, i64)> = existing
        .iter()
        .filter(|e| e["spec"].get("texture").is_some() || e["spec"].get("model").is_some())
        .filter_map(|e| Some((e["spec"]["item"].as_str()?.to_string(), e["spec"]["custom_model_data"].as_i64()?)))
        .collect();
    let mut report = Vec::new();
    let preview_of = |item: &crate::pack_import::ImportedItem| -> Option<String> {
        let bytes = crate::pack_import::preview_png(|p| imported.files.get(p).cloned(), item.texture.as_deref(), item.model.as_deref())?;
        (bytes.len() <= 64 * 1024).then(|| format!("data:image/png;base64,{}", STANDARD.encode(bytes)))
    };
    for mut item in imported.items.clone() {
        let preview = preview_of(&item);
        if let Some(edit) = body.edits.get(&item.key) {
            if let Some(t) = edit.title.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
                item.title = t.chars().take(60).collect();
            }
            if let Some(n) = edit.name.as_deref() {
                item.name = (!n.trim().is_empty()).then(|| n.trim().to_string());
            }
            if let Some(m) = edit.item.as_deref().map(str::trim).filter(|m| !m.is_empty()) {
                item.material = if m.contains(':') { m.to_lowercase() } else { format!("minecraft:{}", m.to_lowercase()) };
            }
        }
        if !body.create_items || body.only.as_ref().is_some_and(|only| !only.contains(&item.key)) {
            // Still listed, so the wizard can show everything the pack holds.
            report.push(json!({
                "id": item.key, "title": item.title, "item": item.material, "name": item.name, "lore": item.lore, "custom_model_data": item.custom_model_data,
                "texture": item.texture, "model": item.model, "preview": preview, "status": "not_selected", "note": "", "missing": item.missing,
            }));
            continue;
        }
        let status = if existing_keys.contains(&item.key) {
            ("exists", "An item with this id already exists".to_string())
        } else {
            while taken.contains(&(item.material.clone(), item.custom_model_data)) {
                item.custom_model_data += 1;
            }
            taken.insert((item.material.clone(), item.custom_model_data));
            match super::utilities::clean_item(&item.spec()) {
                Err(e) => ("skipped", format!("{e:?}")),
                Ok(spec) if body.dry_run => ("would_create", spec.to_string()),
                Ok(spec) => {
                    sqlx::query("INSERT INTO custom_items (id, title, spec, created_at, updated_at) VALUES (?, ?, ?, ?, ?)")
                        .bind(&item.key)
                        .bind(&item.title)
                        .bind(spec.to_string())
                        .bind(crate::db::now())
                        .bind(crate::db::now())
                        .execute(&state.db)
                        .await?;
                    ("created", String::new())
                }
            }
        };
        report.push(json!({
            "id": item.key, "title": item.title, "item": item.material, "name": item.name, "lore": item.lore, "custom_model_data": item.custom_model_data,
            "texture": item.texture, "model": item.model, "preview": preview, "missing": item.missing, "status": status.0,
            "note": if status.0 == "would_create" || status.0 == "created" { String::new() } else { status.1 },
        }));
    }
    if !body.dry_run {
        store::kv_set(&state, "resource_assets", &assets).await?;
    }
    Ok(Json(json!({
        "source": imported.source, "dry_run": body.dry_run, "files_added": added, "files_replaced": replaced,
        "skipped": imported.skipped, "notes": imported.notes, "items": report,
    })))
}

#[derive(Deserialize)]
pub struct BrowseQuery {
    kind: Option<String>,
    q: Option<String>,
    #[serde(default)]
    offset: usize,
    limit: Option<usize>,
}

fn data_uri(b64: &str) -> Option<String> {
    (b64.len() <= 88 * 1024).then(|| format!("data:image/png;base64,{b64}"))
}

/// Textures or models in the server assets as `namespace:path` references, with a small picture each, for the item creator's picker.
pub async fn browse(_: AdminUser, State(state): State<AppState>, Query(q): Query<BrowseQuery>) -> AppResult<Json<Value>> {
    let assets: Assets = store::kv_get(&state, "resource_assets").await?;
    let models = q.kind.as_deref() == Some("model");
    let (folder, ext) = if models { ("models", "json") } else { ("textures", "png") };
    let needle = q.q.unwrap_or_default().to_lowercase();
    let mut all = Vec::new();
    for (path, _) in &assets.files {
        let Some(rest) = path.strip_prefix("assets/") else { continue };
        let Some((ns, rest)) = rest.split_once('/') else { continue };
        let Some(rest) = rest.strip_prefix(&format!("{folder}/")).and_then(|r| r.strip_suffix(&format!(".{ext}"))) else { continue };
        if !models && (rest.starts_with("font/") || rest.starts_with("gui/")) {
            continue;
        }
        let reference = format!("{ns}:{rest}");
        if needle.is_empty() || reference.contains(&needle) {
            all.push(reference);
        }
    }
    let total = all.len();
    let get = |p: &str| assets.files.get(p).and_then(|b| STANDARD.decode(b).ok());
    let items: Vec<Value> = all
        .into_iter()
        .skip(q.offset)
        .take(q.limit.unwrap_or(96).clamp(1, 200))
        .map(|reference| {
            let preview = if models {
                crate::pack_import::preview_png(get, None, Some(&reference)).and_then(|b| data_uri(&STANDARD.encode(b)))
            } else {
                let (ns, rest) = reference.split_once(':').unwrap();
                assets.files.get(&format!("assets/{ns}/textures/{rest}.png")).and_then(|b| data_uri(b))
            };
            json!({"ref": reference, "preview": preview})
        })
        .collect();
    Ok(Json(json!({"total": total, "items": items})))
}

#[derive(Deserialize)]
pub struct PreviewQuery {
    texture: Option<String>,
    model: Option<String>,
}

/// A picture of what a custom item looks like with this texture / model.
pub async fn asset_preview(_: AdminUser, State(state): State<AppState>, Query(q): Query<PreviewQuery>) -> AppResult<Json<Value>> {
    let assets: Assets = store::kv_get(&state, "resource_assets").await?;
    let get = |p: &str| assets.files.get(p).and_then(|b| STANDARD.decode(b).ok());
    let preview =
        crate::pack_import::preview_png(get, q.texture.as_deref(), q.model.as_deref()).and_then(|b| data_uri(&STANDARD.encode(b)));
    Ok(Json(json!({"preview": preview})))
}

pub async fn remove(_: AdminUser, State(state): State<AppState>, Path(path): Path<String>) -> AppResult<Json<Value>> {
    for item in super::utilities::custom_items_for_sync(&state).await? {
        for (field, folder, extension) in [("texture", "textures", "png"), ("model", "models", "json")] {
            if let Some(reference) = item["spec"][field].as_str() {
                if let Some((ns, name)) = reference.split_once(':') {
                    if path == format!("assets/{ns}/{folder}/{name}.{extension}") {
                        return Err(AppError::bad_request("A custom item still uses this asset"));
                    }
                }
            }
        }
    }
    let mut assets: Assets = store::kv_get(&state, "resource_assets").await?;
    assets.files.remove(&path);
    store::kv_set(&state, "resource_assets", &assets).await?;
    Ok(Json(json!({"ok": true})))
}
pub async fn configure(_: AdminUser, State(state): State<AppState>, Json(p): Json<PackSettings>) -> AppResult<Json<Value>> {
    if !(1..=1000).contains(&p.pack_format) {
        return Err(AppError::bad_request("Invalid resource pack format"));
    }
    store::kv_set(&state, "resource_pack", &p).await?;
    Ok(Json(json!({"ok": true})))
}

/// Minecraft only stitches `textures/item` and `textures/block` into its atlases by default. A model that uses a texture anywhere
/// else (`textures/default/…`, `textures/medieval/…`, or straight under `textures/`) renders as the black-and-magenta missing
/// texture unless an `atlases/*.json` file registers it. Registers every such texture a model names, merged with atlas files
/// the pack already ships.
pub fn add_atlas_sources(files: &mut BTreeMap<String, Vec<u8>>) {
    let mut wanted = std::collections::BTreeSet::new();
    for path in files.keys().filter(|k| k.starts_with("assets/") && k.contains("/models/") && k.ends_with(".json")) {
        let Some(json) = files.get(path).and_then(|b| serde_json::from_slice::<Value>(b).ok()) else { continue };
        for value in json.get("textures").and_then(Value::as_object).into_iter().flat_map(|t| t.values()).filter_map(Value::as_str) {
            if value.starts_with('#') {
                continue;
            }
            let value = value.to_lowercase();
            let (ns, rel) = value.split_once(':').unwrap_or(("minecraft", value.as_str()));
            if rel.starts_with("item/") || rel.starts_with("block/") || !files.contains_key(&format!("assets/{ns}/textures/{rel}.png")) {
                continue;
            }
            wanted.insert(format!("{ns}:{rel}"));
        }
    }
    if wanted.is_empty() {
        return;
    }
    for atlas in ["blocks", "items"] {
        let path = format!("assets/minecraft/atlases/{atlas}.json");
        let mut doc: Value = files.get(&path).and_then(|b| serde_json::from_slice(b).ok()).unwrap_or_else(|| json!({}));
        if !doc["sources"].is_array() {
            doc["sources"] = json!([]);
        }
        let sources = doc["sources"].as_array_mut().unwrap();
        for resource in &wanted {
            let present = sources
                .iter()
                .any(|s| s["type"].as_str().is_some_and(|t| t.ends_with("single")) && s["resource"].as_str() == Some(resource));
            if !present {
                sources.push(json!({"type": "minecraft:single", "resource": resource}));
            }
        }
        if let Ok(bytes) = serde_json::to_vec(&doc) {
            files.insert(path, bytes);
        }
    }
}

/// Problems with the stored assets: models that name textures or parents the pack lacks.
pub async fn check(_: AdminUser, State(state): State<AppState>) -> AppResult<Json<Value>> {
    let assets: Assets = store::kv_get(&state, "resource_assets").await?;
    let mut models = BTreeMap::new();
    for (path, data) in &assets.files {
        if path.contains("/models/") && path.ends_with(".json") {
            if let Ok(bytes) = STANDARD.decode(data) {
                models.insert(path.clone(), bytes);
            }
        }
    }
    let found = crate::pack_import::problems(&models, &|p| assets.files.contains_key(p));
    let settings: PackSettings = store::kv_get(&state, "resource_pack").await?;
    let mut decoded: BTreeMap<String, Vec<u8>> =
        assets.files.iter().filter_map(|(k, v)| STANDARD.decode(v).ok().map(|b| (k.clone(), b))).collect();
    let removed = crate::pack_safety::sanitize(&mut decoded, settings.allow_vanilla_overrides);
    Ok(Json(
        json!({"models": models.len(), "textures": assets.files.keys().filter(|k| k.ends_with(".png")).count(), "problems": found, "left_out": crate::pack_safety::summary(&removed)}),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn textures_outside_item_and_block_get_atlas_sources() {
        let mut files: BTreeMap<String, Vec<u8>> = BTreeMap::new();
        files.insert("assets/med/textures/weapons/head.png".into(), vec![1]);
        files.insert("assets/minecraft/textures/hammer.png".into(), vec![1]);
        files.insert("assets/med/textures/item/ok.png".into(), vec![1]);
        files.insert(
            "assets/med/models/item/hammer.json".into(),
            br##"{"textures":{"0":"med:weapons/head","1":"hammer","2":"med:item/ok","particle":"#0","3":"minecraft:item/iron_sword"}}"##
                .to_vec(),
        );
        files.insert(
            "assets/minecraft/atlases/blocks.json".into(),
            br#"{"sources":[{"type":"single","resource":"med:weapons/head"}]}"#.to_vec(),
        );
        add_atlas_sources(&mut files);
        let names = |p: &str| -> Vec<String> {
            let v: Value = serde_json::from_slice(&files[p]).unwrap();
            v["sources"].as_array().unwrap().iter().map(|s| s["resource"].as_str().unwrap().to_string()).collect()
        };
        assert_eq!(
            names("assets/minecraft/atlases/blocks.json"),
            ["med:weapons/head", "minecraft:hammer"],
            "kept, not duplicated; item/ is a default"
        );
        assert_eq!(names("assets/minecraft/atlases/items.json"), ["med:weapons/head", "minecraft:hammer"]);
    }

    #[test]
    fn the_pack_declares_a_range_covering_current_and_future_clients() {
        let m = mcmeta(DEFAULT_PACK_FORMAT);
        assert_eq!(m["pack"]["pack_format"], 84);
        assert_eq!(
            (m["pack"]["supported_formats"]["min_inclusive"].as_u64(), m["pack"]["supported_formats"]["max_inclusive"].as_u64()),
            (Some(15), Some(9999))
        );
        assert_eq!((m["pack"]["min_format"].as_u64(), m["pack"]["max_format"].as_u64()), (Some(15), Some(9999)));
    }
}

pub async fn build(state: &AppState) -> AppResult<Vec<u8>> {
    let assets: Assets = store::kv_get(state, "resource_assets").await?;
    let settings: PackSettings = store::kv_get(state, "resource_pack").await?;
    let mut files: BTreeMap<String, Vec<u8>> = assets
        .files
        .into_iter()
        .map(|(path, data)| STANDARD.decode(data).map(|b| (path, b)).map_err(|_| AppError::bad_request("Invalid stored asset")))
        .collect::<AppResult<_>>()?;
    files.insert("pack.mcmeta".into(), serde_json::to_vec(&mcmeta(settings.pack_format))?);
    crate::rank_glyphs::add_to_pack(&mut files, crate::rank_glyphs::images(state).await?)?;
    let mut overrides: BTreeMap<String, Vec<(i64, String)>> = BTreeMap::new();
    let mut displays = super::utilities::custom_items_for_sync(state).await?;
    displays.extend(super::content::pack_displays(state).await?);
    for entry in displays {
        let spec = &entry["spec"];
        let material = spec["item"].as_str().unwrap_or("").trim_start_matches("minecraft:");
        let Some(data) = spec["custom_model_data"].as_i64() else { continue };
        let model = if let Some(model) = spec["model"].as_str() {
            model.to_string()
        } else if let Some(texture) = spec["texture"].as_str() {
            let model = format!("scopenet:item/{}", entry["id"].as_str().unwrap_or(""));
            files.insert(
                format!("assets/scopenet/models/item/{}.json", entry["id"].as_str().unwrap_or("")),
                serde_json::to_vec(&json!({"parent": "minecraft:item/generated", "textures": {"layer0": texture}}))?,
            );
            model
        } else {
            continue;
        };
        overrides.entry(material.to_string()).or_default().push((data, model));
    }
    for (material, mut entries) in overrides {
        entries.sort_by_key(|e| e.0);
        let path = format!("assets/minecraft/models/item/{material}.json");
        let mut base: Value = if let Some(bytes) = files.get(&path) {
            serde_json::from_slice(bytes)?
        } else {
            json!({"parent": if material.ends_with("_sword") || material.ends_with("_axe") || material.ends_with("_pickaxe") || material.ends_with("_shovel") || material.ends_with("_hoe") { "minecraft:item/handheld" } else { "minecraft:item/generated" }, "textures": {"layer0": format!("minecraft:item/{material}")}})
        };
        let existing = base["overrides"].as_array().cloned().unwrap_or_default();
        let mut all = existing;
        all.extend(entries.iter().map(|(data, model)| json!({"predicate": {"custom_model_data": data}, "model": model})));
        all.sort_by_key(|v| v["predicate"]["custom_model_data"].as_i64().unwrap_or(0));
        base["overrides"] = json!(all);
        files.insert(path, serde_json::to_vec(&base)?);
        // 1.21.4+ item definitions use a range dispatch instead of model predicates.
        let fallback = json!({"type": "minecraft:model", "model": format!("minecraft:item/{material}")});
        let mut ranges = Vec::new();
        for (data, model) in &entries {
            ranges.push(json!({"threshold": data, "model": {"type": "minecraft:model", "model": model}}));
            if !entries.iter().any(|e| e.0 == data + 1) {
                ranges.push(json!({"threshold": data + 1, "model": fallback}));
            }
        }
        ranges.sort_by_key(|v| v["threshold"].as_i64().unwrap_or(0));
        files.entry(format!("assets/minecraft/items/{material}.json")).or_insert(serde_json::to_vec(&json!({"model": {"type": "minecraft:range_dispatch", "property": "minecraft:custom_model_data", "index": 0, "fallback": fallback, "entries": ranges}}))?);
    }
    crate::pack_import::repair_models(&mut files);
    let removed = crate::pack_safety::sanitize(&mut files, settings.allow_vanilla_overrides);
    if !removed.is_empty() {
        tracing::warn!(
            "resource pack: left out {} file(s) that could corrupt world textures (see Resource pack check in the admin panel)",
            removed.len()
        );
    }
    add_atlas_sources(&mut files);
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for (path, bytes) in files {
        zip.start_file(path, options).map_err(|e| AppError::bad_request(e.to_string()))?;
        zip.write_all(&bytes)?;
    }
    Ok(zip.finish().map_err(|e| AppError::bad_request(e.to_string()))?.into_inner())
}

#[derive(Deserialize)]
pub struct PackQuery {
    revision: Option<String>,
}
pub async fn download(State(state): State<AppState>, Query(q): Query<PackQuery>) -> AppResult<impl IntoResponse> {
    let bytes = if let Some(hash) = q.revision {
        if hash.len() != 40 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(AppError::bad_request("Invalid pack revision"));
        }
        let blob: String = store::kv_get(&state, &format!("resource_pack_blob:{hash}")).await?;
        if blob.is_empty() {
            return Err(AppError::not_found("Pack revision expired"));
        }
        STANDARD.decode(blob).map_err(|_| AppError::bad_request("Invalid pack snapshot"))?
    } else {
        build(&state).await?
    };
    Ok((
        [
            (header::CONTENT_TYPE, "application/zip"),
            (header::CACHE_CONTROL, "no-cache"),
            (header::CONTENT_DISPOSITION, "attachment; filename=scopenet-assets.zip"),
        ],
        bytes,
    ))
}
pub async fn game_config(
    super::servers::GameServer(_): super::servers::GameServer,
    State(state): State<AppState>,
) -> AppResult<Json<Value>> {
    let settings: PackSettings = store::kv_get(&state, "resource_pack").await?;
    let sha1 = if settings.enabled {
        let bytes = build(&state).await?;
        let hash = hex::encode(Sha1::digest(&bytes));
        store::kv_set(&state, &format!("resource_pack_blob:{hash}"), &STANDARD.encode(bytes)).await?;
        let mut history: Vec<String> = store::kv_get(&state, "resource_pack_history").await?;
        history.retain(|h| h != &hash);
        history.push(hash.clone());
        while history.len() > 3 {
            let expired = history.remove(0);
            sqlx::query("DELETE FROM kv WHERE key = ?").bind(format!("resource_pack_blob:{expired}")).execute(&state.db).await?;
        }
        store::kv_set(&state, "resource_pack_history", &history).await?;
        if let Some(instance) = &state.instance_id {
            sqlx::query("INSERT INTO kv(key,value) VALUES(?,?) ON CONFLICT(key) DO UPDATE SET value=excluded.value")
                .bind(format!("pack_location:{hash}"))
                .bind(serde_json::to_string(instance)?)
                .execute(&state.platform_db)
                .await?;
        }
        hash
    } else {
        String::new()
    };
    Ok(Json(json!({"enabled": settings.enabled, "required": settings.required, "sha1": sha1})))
}
