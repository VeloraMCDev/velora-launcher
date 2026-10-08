//! Import custom content from the frameworks server owners already use: ItemsAdder, Nexo, Oraxen, CraftEngine (items, blocks,
//! furniture), ModelEngine (Blockbench blueprints) and MythicMobs (mobs).
//!
//! One zip in, a list of [`Entry`] values out. Each entry says what the thing is (tool, armor, food, block, chest, decoration,
//! NPC, vehicle, crop or mob), what it looks like (texture or model in the merged resource pack) and the few numbers the game
//! side needs (hardness, rows, health…). Features that only make sense inside the original plugin (scripts, skills, animations,
//! recipes) are never silently dropped: they come back as `warnings`.

use crate::pack_import::{self, collect, item_key, lore_lines, material, num, resolve, strip_formatting, text};
use serde::Serialize;
use serde_json::{json, Map, Value};
use serde_yaml::Value as Yaml;
use std::collections::{BTreeMap, BTreeSet};

pub const KINDS: [&str; 12] = ["tool", "weapon", "armor", "food", "item", "block", "chest", "decoration", "npc", "vehicle", "crop", "mob"];
/// Kinds stored as ordinary custom items (they work in kits, rewards and `/customitem`).
pub const ITEM_KINDS: [&str; 5] = ["tool", "weapon", "armor", "food", "item"];

#[derive(Debug, Clone, Serialize)]
pub struct Entry {
    /// `framework:id`, unique inside one analysis.
    pub key: String,
    pub framework: &'static str,
    pub id: String,
    pub kind: String,
    pub title: String,
    pub name: Option<String>,
    pub lore: Vec<String>,
    /// `minecraft:` base item the thing is shown as / handed out as.
    pub material: String,
    pub cmd: Option<i64>,
    pub texture: Option<String>,
    pub model: Option<String>,
    /// Render size multiplier (models shrunk to fit vanilla's limits are scaled back up).
    pub scale: f64,
    /// Kind-specific numbers: hardness, rows, health, nutrition, seat, stages…
    pub extras: Map<String, Value>,
    /// Features that exist in the original plugin but are not imported.
    pub warnings: Vec<String>,
    /// Texture/parent references the pack lacks.
    pub missing: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Detected {
    pub framework: &'static str,
    pub label: &'static str,
    pub entries: usize,
}

#[derive(Default, Debug)]
pub struct Analysis {
    pub files: BTreeMap<String, Vec<u8>>,
    pub entries: Vec<Entry>,
    pub detected: Vec<Detected>,
    pub skipped: Vec<String>,
    pub notes: Vec<String>,
}

pub fn label(framework: &str) -> &'static str {
    match framework {
        "itemsadder" => "ItemsAdder",
        "nexo" => "Nexo",
        "oraxen" => "Oraxen",
        "craftengine" => "CraftEngine",
        "modelengine" => "ModelEngine",
        "mythicmobs" => "MythicMobs",
        _ => "Resource pack",
    }
}

fn framework_static(f: &str) -> &'static str {
    match f {
        "itemsadder" => "itemsadder",
        "nexo" => "nexo",
        "oraxen" => "oraxen",
        "craftengine" => "craftengine",
        "modelengine" => "modelengine",
        "mythicmobs" => "mythicmobs",
        _ => "resourcepack",
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// YAML helpers (keys are matched case-insensitively because every one of these plugins is inconsistent about it)
// ---------------------------------------------------------------------------------------------------------------------

fn get<'a>(y: &'a Yaml, key: &str) -> Option<&'a Yaml> {
    y.as_mapping()?.iter().find(|(k, _)| k.as_str().is_some_and(|s| s.eq_ignore_ascii_case(key))).map(|(_, v)| v)
}

fn first<'a>(y: &'a Yaml, keys: &[&str]) -> Option<&'a Yaml> {
    keys.iter().find_map(|k| get(y, k))
}

fn keys_of(y: &Yaml) -> Vec<String> {
    y.as_mapping().map(|m| m.keys().filter_map(|k| k.as_str().map(|s| s.to_lowercase())).collect()).unwrap_or_default()
}

/// True when `key` appears anywhere inside `y`.
fn has_key(y: &Yaml, key: &str) -> bool {
    match y {
        Yaml::Mapping(m) => m.iter().any(|(k, v)| k.as_str().is_some_and(|s| s.eq_ignore_ascii_case(key)) || has_key(v, key)),
        Yaml::Sequence(s) => s.iter().any(|v| has_key(v, key)),
        _ => false,
    }
}

fn find_num(y: &Yaml, keys: &[&str]) -> Option<f64> {
    match y {
        Yaml::Mapping(m) => {
            for (k, v) in m {
                if k.as_str().is_some_and(|s| keys.iter().any(|w| s.eq_ignore_ascii_case(w))) {
                    if let Some(n) = v.as_f64().or_else(|| v.as_str().and_then(|s| s.trim().parse().ok())) {
                        return Some(n);
                    }
                }
            }
            m.values().find_map(|v| find_num(v, keys))
        }
        Yaml::Sequence(s) => s.iter().find_map(|v| find_num(v, keys)),
        _ => None,
    }
}

/// First `model: { path: … }` / `model: ns:path` inside `y`.
fn find_model_path(y: &Yaml) -> Option<String> {
    match y {
        Yaml::Mapping(m) => {
            for (k, v) in m {
                if k.as_str().is_some_and(|s| s.eq_ignore_ascii_case("model") || s.eq_ignore_ascii_case("models")) {
                    if let Some(s) = v.as_str() {
                        return Some(s.to_string());
                    }
                    if let Some(p) = text(get(v, "path")).or_else(|| text(get(v, "model"))) {
                        return Some(p);
                    }
                }
            }
            m.values().find_map(find_model_path)
        }
        Yaml::Sequence(s) => s.iter().find_map(find_model_path),
        _ => None,
    }
}

const EDIBLE: [&str; 12] = ["apple", "bread", "cooked_beef", "cooked_chicken", "cooked_porkchop", "carrot", "baked_potato", "golden_apple", "cookie", "melon_slice", "pumpkin_pie", "mushroom_stew"];

fn infer_item_kind(material: &str) -> &'static str {
    let m = material.trim_start_matches("minecraft:");
    if m.ends_with("_helmet") || m.ends_with("_chestplate") || m.ends_with("_leggings") || m.ends_with("_boots") || m == "elytra" || m == "turtle_helmet" {
        "armor"
    } else if m.ends_with("_sword") || m == "bow" || m == "crossbow" || m == "trident" || m == "mace" {
        "weapon"
    } else if m.ends_with("_axe") || m.ends_with("_pickaxe") || m.ends_with("_shovel") || m.ends_with("_hoe") || m == "shears" || m == "fishing_rod" {
        "tool"
    } else if EDIBLE.contains(&m) {
        "food"
    } else {
        "item"
    }
}

fn ensure_edible(material: String) -> String {
    if EDIBLE.contains(&material.trim_start_matches("minecraft:")) { material } else { "minecraft:bread".into() }
}

fn new_entry(framework: &'static str, raw_id: &str) -> Option<Entry> {
    let short = raw_id.rsplit(':').next().unwrap_or(raw_id);
    let id = item_key(short)?;
    Some(Entry {
        key: format!("{framework}:{raw_id}"),
        framework,
        id,
        kind: "item".into(),
        title: short.to_string(),
        name: None,
        lore: vec![],
        material: "minecraft:paper".into(),
        cmd: None,
        texture: None,
        model: None,
        scale: 1.0,
        extras: Map::new(),
        warnings: vec![],
        missing: vec![],
    })
}

fn finish_title(e: &mut Entry) {
    e.title = strip_formatting(e.name.as_deref().unwrap_or(&e.title)).chars().take(60).collect();
    if e.title.is_empty() {
        e.title = e.id.clone();
    }
}

fn clean_name(raw: Option<String>) -> Option<String> {
    let s = pack_import::display(raw?);
    (!s.is_empty()).then_some(s)
}

// ---------------------------------------------------------------------------------------------------------------------
// Oraxen / Nexo
// ---------------------------------------------------------------------------------------------------------------------

const ORAXEN_MECHANICS_HANDLED: [&str; 14] = [
    "furniture", "noteblock", "stringblock", "custom_block", "block", "storage", "food", "armor", "durability", "custom_armor",
    "repair", "misc", "limited_placing", "trim",
];

fn id_map_entries(y: &Yaml, files: &BTreeMap<String, Vec<u8>>, framework: &'static str) -> Vec<Entry> {
    let Some(map) = y.as_mapping() else { return vec![] };
    let mut out = Vec::new();
    for (k, def) in map {
        let (Some(id), Some(_)) = (k.as_str(), def.as_mapping()) else { continue };
        let pack = get(def, "pack");
        if pack.is_none() && get(def, "material").is_none() {
            continue;
        }
        let Some(mut e) = new_entry(framework, id) else { continue };
        let mat = material(text(get(def, "material")));
        e.material = mat.clone();
        e.name = clean_name(text(first(def, &["displayname", "itemname", "display_name", "item_name"])));
        e.lore = lore_lines(pack_import::list(get(def, "lore")));
        if let Some(p) = pack {
            e.cmd = num(get(p, "custom_model_data"));
            e.model = text(get(p, "model")).and_then(|m| resolve(files, "models", "json", &m, &[]));
            let mut tex = pack_import::list(get(p, "textures"));
            tex.extend(pack_import::list(get(p, "texture")));
            e.texture = tex.first().and_then(|t| resolve(files, "textures", "png", t, &[]));
        }
        let mech = first(def, &["mechanics"]);
        let comps = first(def, &["components"]);
        let mkeys = mech.map(keys_of).unwrap_or_default();
        let has = |k: &str| mkeys.iter().any(|m| m == k);
        let furniture = mech.and_then(|m| get(m, "furniture"));
        let custom_block = mech.and_then(|m| first(m, &["custom_block", "noteblock", "stringblock", "block"]));
        let storage = mech.and_then(|m| get(m, "storage")).or_else(|| furniture.and_then(|f| get(f, "storage")));
        let food = mech.and_then(|m| get(m, "food")).or_else(|| comps.and_then(|c| get(c, "food"))).or_else(|| get(def, "food"));

        e.kind = if storage.is_some() {
            "chest"
        } else if furniture.is_some() {
            "decoration"
        } else if custom_block.is_some() || has("noteblock") || has("stringblock") {
            "block"
        } else if has("armor") || has("custom_armor") || has("trim") {
            "armor"
        } else if food.is_some() {
            "food"
        } else {
            infer_item_kind(&mat)
        }
        .into();

        match e.kind.as_str() {
            "block" => {
                if let Some(cb) = custom_block {
                    if let Some(h) = find_num(cb, &["hardness", "mining_speed", "break_time"]) {
                        e.extras.insert("hardness".into(), json!(h));
                    }
                    if let Some(l) = find_num(cb, &["light", "light_level", "luminance"]) {
                        e.extras.insert("light".into(), json!(l as i64));
                    }
                }
            }
            "chest" => {
                let rows = storage.and_then(|s| find_num(s, &["rows"])).unwrap_or(3.0).clamp(1.0, 6.0) as i64;
                e.extras.insert("rows".into(), json!(rows));
            }
            "decoration" => {
                let seat = furniture.is_some_and(|f| has_key(f, "seat") || has_key(f, "seats") || has_key(f, "sit"));
                e.extras.insert("seat".into(), json!(seat));
                if let Some(s) = furniture.and_then(|f| find_num(f, &["scale"])) {
                    e.scale = s.clamp(0.1, 16.0);
                }
            }
            "food" => {
                let n = food.and_then(|f| find_num(f, &["nutrition", "hunger", "food_level", "food"])).unwrap_or(4.0);
                let s = food.and_then(|f| find_num(f, &["saturation"])).unwrap_or(2.0);
                e.extras.insert("nutrition".into(), json!(n.clamp(0.0, 20.0) as i64));
                e.extras.insert("saturation".into(), json!(s.clamp(0.0, 20.0)));
                e.material = ensure_edible(e.material);
            }
            _ => {}
        }
        for m in &mkeys {
            if !ORAXEN_MECHANICS_HANDLED.contains(&m.as_str()) {
                e.warnings.push(format!("Mechanic “{m}” only works inside the original plugin and is not imported"));
            }
        }
        out.push(e);
    }
    out
}

fn looks_like_id_map(y: &Yaml) -> bool {
    y.as_mapping().is_some_and(|m| {
        m.values().any(|d| d.as_mapping().is_some() && (get(d, "pack").is_some() || (get(d, "material").is_some() && (get(d, "mechanics").is_some() || get(d, "itemname").is_some() || get(d, "displayname").is_some()))))
    })
}

// ---------------------------------------------------------------------------------------------------------------------
// ItemsAdder
// ---------------------------------------------------------------------------------------------------------------------

fn itemsadder_entries(y: &Yaml, files: &BTreeMap<String, Vec<u8>>) -> Option<Vec<Entry>> {
    let items = get(y, "items")?.as_mapping()?;
    let info = get(y, "info")?;
    let namespace = text(get(info, "namespace")).unwrap_or_default().to_lowercase();
    let prefer: Vec<&str> = if namespace.is_empty() { vec![] } else { vec![namespace.as_str()] };
    let mut out = Vec::new();
    for (k, def) in items {
        let (Some(id), Some(_)) = (k.as_str(), def.as_mapping()) else { continue };
        let Some(res) = get(def, "resource") else { continue };
        let Some(mut e) = new_entry("itemsadder", id) else { continue };
        let mat = material(text(get(res, "material")));
        e.material = mat.clone();
        e.cmd = num(get(res, "model_id")).or_else(|| num(get(res, "custom_model_data")));
        e.model = text(get(res, "model_path")).and_then(|m| resolve(files, "models", "json", &m, &prefer));
        let mut tex = pack_import::list(get(res, "textures"));
        tex.extend(pack_import::list(get(res, "texture")));
        e.texture = tex.first().and_then(|t| resolve(files, "textures", "png", t, &prefer));
        e.name = clean_name(text(get(def, "display_name")));
        e.lore = lore_lines(pack_import::list(get(def, "lore")));
        let behaviours = get(def, "behaviours");
        let props = get(def, "specific_properties");
        let furniture = behaviours.and_then(|b| get(b, "furniture"));
        let block = props.and_then(|p| get(p, "block")).or_else(|| behaviours.and_then(|b| get(b, "block")));

        e.kind = if furniture.is_some() {
            if has_key(furniture.unwrap(), "container") || has_key(furniture.unwrap(), "storage") { "chest" } else { "decoration" }
        } else if block.is_some() {
            "block"
        } else if has_key(def, "vehicle") {
            "vehicle"
        } else if has_key(def, "crop") || has_key(def, "plant") {
            "crop"
        } else if props.is_some_and(|p| get(p, "armor").is_some()) {
            "armor"
        } else if has_key(def, "consumable") || has_key(def, "food") || has_key(def, "edible") {
            "food"
        } else {
            infer_item_kind(&mat)
        }
        .into();

        match e.kind.as_str() {
            "block" => {
                if let Some(h) = block.and_then(|b| find_num(b, &["hardness", "break_time"])) {
                    e.extras.insert("hardness".into(), json!(h));
                }
            }
            "chest" => {
                e.extras.insert("rows".into(), json!(furniture.and_then(|f| find_num(f, &["rows"])).unwrap_or(3.0).clamp(1.0, 6.0) as i64));
            }
            "decoration" => {
                e.extras.insert("seat".into(), json!(furniture.is_some_and(|f| has_key(f, "sit") || has_key(f, "seat"))));
            }
            "food" => {
                let n = find_num(def, &["nutrition", "hunger", "food"]).unwrap_or(4.0);
                let s = find_num(def, &["saturation"]).unwrap_or(2.0);
                e.extras.insert("nutrition".into(), json!(n.clamp(0.0, 20.0) as i64));
                e.extras.insert("saturation".into(), json!(s.clamp(0.0, 20.0)));
                e.material = ensure_edible(e.material);
            }
            _ => {}
        }
        if has_key(def, "events") {
            e.warnings.push("Item events (custom actions) only work inside ItemsAdder and are not imported".into());
        }
        if has_key(def, "permission") {
            e.warnings.push("Permission requirements are not imported".into());
        }
        out.push(e);
    }
    Some(out)
}

// ---------------------------------------------------------------------------------------------------------------------
// CraftEngine
// ---------------------------------------------------------------------------------------------------------------------

fn is_namespaced_map(y: Option<&Yaml>) -> bool {
    y.and_then(Yaml::as_mapping).is_some_and(|m| m.keys().any(|k| k.as_str().is_some_and(|s| s.contains(':'))))
}

fn looks_like_craftengine(y: &Yaml, path: &str) -> bool {
    let lower = path.to_lowercase();
    (lower.contains("craftengine") && (get(y, "items").is_some() || get(y, "blocks").is_some() || get(y, "furniture").is_some()))
        || ["items", "blocks", "furniture"].iter().any(|k| is_namespaced_map(get(y, k)))
}

fn craftengine_entries(y: &Yaml, files: &BTreeMap<String, Vec<u8>>, models_by_item: &mut BTreeMap<String, String>) -> Vec<Entry> {
    let mut out = Vec::new();
    let resolve_model = |raw: &str| resolve(files, "models", "json", raw, &[]);
    if let Some(items) = get(y, "items").and_then(Yaml::as_mapping) {
        for (k, def) in items {
            let (Some(id), Some(_)) = (k.as_str(), def.as_mapping()) else { continue };
            let Some(mut e) = new_entry("craftengine", id) else { continue };
            e.material = material(text(get(def, "material")));
            e.cmd = num(get(def, "custom-model-data")).or_else(|| num(get(def, "custom_model_data")));
            let model_path = find_model_path(def);
            e.model = model_path.as_deref().and_then(resolve_model);
            if e.model.is_none() {
                let ns = id.split(':').next().unwrap_or("");
                let short = id.rsplit(':').next().unwrap_or(id);
                e.texture = resolve(files, "textures", "png", &format!("{ns}:item/{short}"), &[]);
            }
            let data = get(def, "data");
            e.name = clean_name(data.and_then(|d| text(first(d, &["item-name", "item_name", "custom-name", "custom_name"]))));
            e.lore = lore_lines(data.map(|d| pack_import::list(get(d, "lore"))).unwrap_or_default());
            e.kind = if has_key(def, "food") || has_key(def, "consumable") { "food" } else { infer_item_kind(&e.material) }.into();
            if e.kind == "food" {
                e.extras.insert("nutrition".into(), json!(find_num(def, &["nutrition"]).unwrap_or(4.0).clamp(0.0, 20.0) as i64));
                e.extras.insert("saturation".into(), json!(find_num(def, &["saturation"]).unwrap_or(2.0).clamp(0.0, 20.0)));
                e.material = ensure_edible(e.material);
            }
            if let Some(m) = &e.model {
                models_by_item.insert(id.to_string(), m.clone());
            }
            if has_key(def, "behavior") || has_key(def, "behaviors") {
                e.warnings.push("Item behaviors (scripted actions) are not imported".into());
            }
            out.push(e);
        }
    }
    if let Some(blocks) = get(y, "blocks").and_then(Yaml::as_mapping) {
        for (k, def) in blocks {
            let (Some(id), Some(_)) = (k.as_str(), def.as_mapping()) else { continue };
            let Some(mut e) = new_entry("craftengine", id) else { continue };
            e.kind = "block".into();
            e.model = find_model_path(def).as_deref().and_then(resolve_model);
            if let Some(h) = find_num(def, &["hardness", "break_time"]) {
                e.extras.insert("hardness".into(), json!(h));
            }
            if let Some(l) = find_num(def, &["luminance", "light", "light_level"]) {
                e.extras.insert("light".into(), json!(l as i64));
            }
            e.name = clean_name(get(def, "settings").and_then(|s| text(get(s, "item-name"))));
            if has_key(def, "events") || has_key(def, "behavior") {
                e.warnings.push("Block events and behaviors are not imported".into());
            }
            out.push(e);
        }
    }
    if let Some(furn) = get(y, "furniture").and_then(Yaml::as_mapping) {
        for (k, def) in furn {
            let (Some(id), Some(_)) = (k.as_str(), def.as_mapping()) else { continue };
            let Some(mut e) = new_entry("craftengine", id) else { continue };
            e.kind = "decoration".into();
            // The furniture shows an item; borrow that item's model.
            let item_ref = find_item_ref(def);
            e.model = item_ref.as_deref().and_then(|r| models_by_item.get(r).cloned()).or_else(|| find_model_path(def).as_deref().and_then(resolve_model));
            e.extras.insert("seat".into(), json!(has_key(def, "seats") || has_key(def, "seat")));
            e.name = clean_name(get(def, "settings").and_then(|s| text(get(s, "item-name"))));
            if e.model.is_none() {
                e.warnings.push("Could not find the model this furniture shows (it shows an item defined elsewhere); import the item's file too".into());
            }
            out.push(e);
        }
    }
    out
}

fn find_item_ref(y: &Yaml) -> Option<String> {
    match y {
        Yaml::Mapping(m) => {
            for (k, v) in m {
                if k.as_str().is_some_and(|s| s.eq_ignore_ascii_case("item")) {
                    if let Some(s) = v.as_str() {
                        return Some(s.to_string());
                    }
                }
            }
            m.values().find_map(find_item_ref)
        }
        Yaml::Sequence(s) => s.iter().find_map(find_item_ref),
        _ => None,
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// MythicMobs
// ---------------------------------------------------------------------------------------------------------------------

fn looks_like_mythic(y: &Yaml, path: &str) -> bool {
    let lower = path.to_lowercase();
    let path_hint = lower.contains("mythicmobs") || lower.contains("/mobs/") || lower.starts_with("mobs/");
    y.as_mapping().is_some_and(|m| {
        m.values().any(|d| {
            d.as_mapping().is_some() && get(d, "type").and_then(Yaml::as_str).is_some_and(|t| t.chars().all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit()))
                && (path_hint || get(d, "health").is_some() || get(d, "display").is_some() || get(d, "skills").is_some())
        })
    })
}

fn mythic_model_id(def: &Yaml) -> Option<String> {
    if let Some(me) = get(def, "modelengine") {
        if let Some(m) = text(first(me, &["model", "id"])) {
            return Some(m);
        }
    }
    let dump = serde_yaml::to_string(def).unwrap_or_default();
    let idx = dump.find("mid=").or_else(|| dump.find("model="))?;
    let tail = &dump[idx..];
    let start = tail.find('=')? + 1;
    let id: String = tail[start..].chars().take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-')).collect();
    (!id.is_empty()).then_some(id)
}

fn parse_drop(line: &str) -> Option<Value> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    let name = *parts.first()?;
    if name.eq_ignore_ascii_case("exp") || name.eq_ignore_ascii_case("money") || name.contains('{') {
        return None;
    }
    let (min, max) = match parts.get(1) {
        Some(a) => match a.split_once('-') {
            Some((lo, hi)) => (lo.parse::<i64>().ok()?, hi.parse::<i64>().ok()?),
            None => {
                let n = a.parse::<i64>().ok()?;
                (n, n)
            }
        },
        None => (1, 1),
    };
    let chance = parts.get(2).and_then(|c| c.parse::<f64>().ok()).unwrap_or(1.0).clamp(0.0, 1.0);
    // A bare name is either a vanilla material or one of our custom items; the game side tries custom ids first, then materials.
    let item = name.trim_start_matches("minecraft:").to_lowercase();
    Some(json!({"item": item, "min": min.max(0), "max": max.max(min).min(64), "chance": chance}))
}

fn mythic_entries(y: &Yaml) -> Vec<Entry> {
    let Some(map) = y.as_mapping() else { return vec![] };
    let mut out = Vec::new();
    for (k, def) in map {
        let (Some(id), Some(_)) = (k.as_str(), def.as_mapping()) else { continue };
        let Some(ty) = text(get(def, "type")) else { continue };
        let Some(mut e) = new_entry("mythicmobs", id) else { continue };
        e.kind = "mob".into();
        e.name = clean_name(text(first(def, &["display", "displayname"])));
        e.extras.insert("entity".into(), json!(ty.to_uppercase()));
        e.extras.insert("health".into(), json!(find_num(def, &["health"]).unwrap_or(20.0).clamp(1.0, 1024.0)));
        e.extras.insert("damage".into(), json!(find_num(def, &["damage"]).unwrap_or(2.0).clamp(0.0, 1024.0)));
        if let Some(a) = find_num(def, &["armor"]) {
            e.extras.insert("armor".into(), json!(a.clamp(0.0, 30.0)));
        }
        if let Some(s) = find_num(def, &["movementspeed", "movement_speed"]) {
            e.extras.insert("speed".into(), json!(s.clamp(0.0, 2.0)));
        }
        let drops: Vec<Value> = pack_import::list(get(def, "drops")).iter().filter_map(|l| parse_drop(l)).take(32).collect();
        e.extras.insert("drops".into(), Value::Array(drops));
        if let Some(m) = mythic_model_id(def) {
            e.extras.insert("model_id".into(), json!(m));
        }
        let skills = get(def, "skills").and_then(Yaml::as_sequence).map_or(0, Vec::len);
        let ai = ["aigoalselectors", "aitargetselectors", "aiNearestTargetSelector".to_lowercase().as_str()].iter().filter(|k| get(def, k).is_some()).count();
        if skills > 0 {
            e.warnings.push(format!("{skills} skill line(s) only run in MythicMobs and are not imported"));
        }
        if ai > 0 {
            e.warnings.push("Custom AI goals are not imported; the mob uses its normal vanilla behaviour".into());
        }
        out.push(e);
    }
    out
}

// ---------------------------------------------------------------------------------------------------------------------
// Assembly
// ---------------------------------------------------------------------------------------------------------------------

fn stem_of(path: &str) -> String {
    let file = path.rsplit('/').next().unwrap_or(path);
    let file = file.rsplit_once('.').map_or(file, |x| x.0);
    item_key(file).unwrap_or_else(|| "model".into())
}

/// Collapse `thing_stage_1 … thing_stage_4` style entries into one crop with ordered stages.
fn group_crops(entries: &mut Vec<Entry>) {
    fn split(id: &str) -> Option<(String, u32)> {
        let (head, digits) = id.rsplit_once(|c: char| !c.is_ascii_digit()).map(|(h, d)| (h, d)).or(None)?;
        let _ = head;
        let n: u32 = id.trim_start_matches(|c: char| !c.is_ascii_digit() || false).parse().ok().or_else(|| digits.parse().ok())?;
        let base_end = id.len() - id.chars().rev().take_while(|c| c.is_ascii_digit()).count();
        let base = id[..base_end].trim_end_matches(['_', '-']).to_string();
        let base = base.trim_end_matches("stage").trim_end_matches("growth").trim_end_matches("age").trim_end_matches(['_', '-']).to_string();
        (!base.is_empty()).then_some((base, n))
    }
    let mut groups: BTreeMap<(String, &'static str), Vec<(u32, usize)>> = BTreeMap::new();
    for (i, e) in entries.iter().enumerate() {
        if e.kind == "crop" || (e.id.contains("stage") || e.id.contains("growth") || e.id.contains("_age")) && ITEM_KINDS.contains(&e.kind.as_str()) {
            if let Some((base, n)) = split(&e.id) {
                if e.texture.is_some() || e.model.is_some() {
                    groups.entry((base, e.framework)).or_default().push((n, i));
                }
            }
        }
    }
    let mut remove = BTreeSet::new();
    let mut crops = Vec::new();
    for ((base, framework), mut members) in groups {
        if members.len() < 2 {
            continue;
        }
        members.sort();
        let stages: Vec<Value> = members
            .iter()
            .map(|(_, i)| json!({"texture": entries[*i].texture, "model": entries[*i].model}))
            .collect();
        let head = &entries[members[0].1];
        let mut crop = new_entry(framework, &base).unwrap();
        crop.kind = "crop".into();
        crop.material = head.material.clone();
        crop.name = head.name.clone().map(|n| n.replace(|c: char| c.is_ascii_digit(), "").trim().to_string()).filter(|n| !n.is_empty());
        crop.texture = head.texture.clone();
        crop.model = head.model.clone();
        crop.extras.insert("stages".into(), Value::Array(stages));
        crop.extras.insert("growth_seconds".into(), json!(600));
        crop.extras.insert("seed_items".into(), json!(1));
        crop.warnings.push("Growth time is set to 10 minutes; change it after importing".into());
        crops.push(crop);
        remove.extend(members.iter().map(|(_, i)| *i));
    }
    if !remove.is_empty() {
        let mut idx = 0;
        entries.retain(|_| {
            let keep = !remove.contains(&idx);
            idx += 1;
            keep
        });
        entries.extend(crops);
    }
}

pub fn analyze(zip_bytes: &[u8]) -> Result<Analysis, String> {
    let c = collect(zip_bytes)?;
    let mut out = Analysis { skipped: c.skipped, files: c.files, ..Analysis::default() };
    let mut entries: Vec<Entry> = Vec::new();

    // ModelEngine blueprints become static models in their own namespace.
    let mut blueprint_ids = BTreeSet::new();
    for (path, bytes) in &c.blueprints {
        let mut id = stem_of(path);
        while !blueprint_ids.insert(id.clone()) {
            id.push('2');
        }
        match crate::bbmodel::convert(bytes, "modelengine", &id) {
            Ok(conv) => {
                out.files.insert(format!("assets/modelengine/models/{id}.json"), conv.model);
                for (name, png) in conv.textures {
                    out.files.insert(format!("assets/modelengine/textures/{id}/{name}.png"), png);
                }
                let mut e = new_entry("modelengine", &id).unwrap();
                e.kind = "npc".into();
                e.model = Some(format!("modelengine:{id}"));
                e.scale = conv.scale_hint;
                e.extras.insert("height".into(), json!((conv.height_blocks * 100.0).round() / 100.0));
                e.extras.insert("width".into(), json!((conv.width_blocks * 100.0).round() / 100.0));
                e.extras.insert("blueprint".into(), json!(id));
                e.warnings.push("Animations are not imported: the model is shown in its resting pose".into());
                e.warnings.extend(conv.warnings);
                entries.push(e);
            }
            Err(msg) => out.skipped.push(format!("{path}: {msg}")),
        }
    }

    // Repair references first so every later lookup sees consistent names.
    let repaired = pack_import::repair_models(&mut out.files);
    if repaired > 0 {
        out.notes.push(format!("Fixed {repaired} texture/model references that were missing their namespace, so models find their textures"));
    }

    // YAML: classify each file, most specific framework first.
    let mut ce_models: BTreeMap<String, String> = BTreeMap::new();
    let (mut ce_files, mut other) = (Vec::new(), Vec::new());
    for (path, y) in &c.yamls {
        if looks_like_craftengine(y, path) {
            ce_files.push((path, y));
        } else {
            other.push((path, y));
        }
    }
    // CraftEngine items must be read before furniture so furniture can find their models.
    for (_, y) in &ce_files {
        if get(y, "items").is_some() {
            let only_items = Yaml::Mapping([(Yaml::String("items".into()), get(y, "items").unwrap().clone())].into_iter().collect());
            entries.extend(craftengine_entries(&only_items, &out.files, &mut ce_models));
        }
    }
    for (_, y) in &ce_files {
        let rest: serde_yaml::Mapping = ["blocks", "furniture"].iter().filter_map(|k| get(y, k).map(|v| (Yaml::String((*k).into()), v.clone()))).collect();
        if !rest.is_empty() {
            entries.extend(craftengine_entries(&Yaml::Mapping(rest), &out.files, &mut ce_models));
        }
    }
    for (path, y) in other {
        let lower = path.to_lowercase();
        if looks_like_mythic(y, path) {
            entries.extend(mythic_entries(y));
        } else if let Some(found) = itemsadder_entries(y, &out.files) {
            entries.extend(found);
        } else if looks_like_id_map(y) {
            let fw = if lower.contains("nexo") { "nexo" } else if lower.contains("oraxen") { "oraxen" } else if has_key(y, "custom_block") || has_key(y, "components") { "nexo" } else { "oraxen" };
            entries.extend(id_map_entries(y, &out.files, framework_static(fw)));
        }
    }

    // Link MythicMobs to the ModelEngine models they use.
    let blueprint_entries: BTreeMap<String, usize> = entries.iter().enumerate().filter(|(_, e)| e.framework == "modelengine").map(|(i, e)| (e.id.clone(), i)).collect();
    let mut absorbed = BTreeSet::new();
    let snapshot = entries.clone();
    for e in entries.iter_mut().filter(|e| e.framework == "mythicmobs") {
        let Some(mid) = e.extras.get("model_id").and_then(Value::as_str).map(str::to_string) else { continue };
        let key = item_key(&mid).unwrap_or_default();
        match blueprint_entries.get(&key) {
            Some(&i) => {
                let bp = &snapshot[i];
                e.model = bp.model.clone();
                e.scale = bp.scale;
                for k in ["height", "width"] {
                    if let Some(v) = bp.extras.get(k) {
                        e.extras.insert(k.into(), v.clone());
                    }
                }
                absorbed.insert(i);
            }
            None => e.warnings.push(format!("It uses the ModelEngine model “{mid}”, which is not in this zip. Add its .bbmodel to the zip or import it separately")),
        }
    }
    if !absorbed.is_empty() {
        let mut idx = 0;
        entries.retain(|_| {
            let keep = !absorbed.contains(&idx);
            idx += 1;
            keep
        });
        out.notes.push(format!("{} ModelEngine model(s) were attached to the MythicMobs mobs that use them", absorbed.len()));
    }

    group_crops(&mut entries);

    // Unique ids, titles, and missing-texture checks.
    let mut used: BTreeSet<String> = BTreeSet::new();
    for e in &mut entries {
        let base = e.id.clone();
        let mut n = 2;
        while !used.insert(e.id.clone()) {
            e.id = format!("{}_{n}", base.chars().take(28).collect::<String>());
            n += 1;
        }
        e.key = format!("{}:{}", e.framework, e.id);
        finish_title(e);
        e.missing = missing_for(e, &out.files);
    }
    // Entries nobody can see are not useful (except mobs, which are vanilla entities without a model).
    let before = entries.len();
    entries.retain(|e| e.kind == "mob" || e.texture.is_some() || e.model.is_some() || e.extras.contains_key("stages"));
    let dropped = before - entries.len();
    if dropped > 0 {
        out.notes.push(format!("{dropped} entries had no texture or model in the zip and were left out"));
    }
    entries.sort_by(|a, b| (a.framework, &a.id).cmp(&(b.framework, &b.id)));

    let mut counts: BTreeMap<&'static str, usize> = BTreeMap::new();
    for e in &entries {
        *counts.entry(e.framework).or_default() += 1;
    }
    out.detected = counts.into_iter().map(|(f, n)| Detected { framework: f, label: label(f), entries: n }).collect();
    out.entries = entries;
    if out.entries.is_empty() && out.files.is_empty() {
        return Err("Nothing recognisable in this zip. It should hold ItemsAdder, Nexo, Oraxen, CraftEngine, ModelEngine (.bbmodel) or MythicMobs files — zip the plugin's folder (or just its items/ and pack/ or contents/ folders)".into());
    }
    Ok(out)
}

fn missing_for(e: &Entry, files: &BTreeMap<String, Vec<u8>>) -> Vec<String> {
    let mut refs: Vec<&str> = e.model.iter().map(String::as_str).collect();
    if let Some(stages) = e.extras.get("stages").and_then(Value::as_array) {
        refs.extend(stages.iter().filter_map(|s| s["model"].as_str()));
    }
    let mut models = BTreeMap::new();
    for r in refs {
        if let Some((ns, p)) = r.split_once(':') {
            let key = format!("assets/{ns}/models/{p}.json");
            if let Some(j) = files.get(&key) {
                models.insert(key, j.clone());
            }
        }
    }
    pack_import::problems(&models, &|p| files.contains_key(p)).into_iter().map(|p| p.reference).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};

    pub fn zip_of(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut z = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let o = zip::write::SimpleFileOptions::default();
        for (n, d) in files {
            z.start_file(*n, o).unwrap();
            z.write_all(d).unwrap();
        }
        z.finish().unwrap().into_inner()
    }
    const PNG: &[u8] = b"\x89PNG\r\n\x1a\nxxxx";
    const BB_TEX: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";

    fn by<'a>(a: &'a Analysis, id: &str) -> &'a Entry {
        a.entries.iter().find(|e| e.id == id).unwrap_or_else(|| panic!("no entry {id}: {:?}", a.entries.iter().map(|e| &e.id).collect::<Vec<_>>()))
    }

    #[test]
    fn oraxen_and_nexo_style_items_get_the_right_kind_and_numbers() {
        let yml = "ruby_sword:\n  itemname: '&cRuby Sword'\n  material: DIAMOND_SWORD\n  Pack:\n    textures: [default/ruby_sword]\n    custom_model_data: 1001\n\
bench:\n  itemname: 'Workbench'\n  material: PAPER\n  Pack:\n    model: default/bench\n  Mechanics:\n    furniture:\n      seat:\n        height: 0.5\n      scale: 1.5\n    glow:\n      x: 1\n\
vault:\n  itemname: 'Vault'\n  material: PAPER\n  Pack:\n    model: default/vault\n  Mechanics:\n    furniture:\n      storage:\n        rows: 4\n\
ruby_ore:\n  itemname: 'Ruby Ore'\n  material: PAPER\n  Pack:\n    textures: [default/ruby_ore]\n  Mechanics:\n    noteblock:\n      hardness: 3\n      light: 5\n\
cake:\n  itemname: 'Cake'\n  material: PAPER\n  Pack:\n    textures: [default/cake]\n  Components:\n    food:\n      nutrition: 8\n      saturation: 4\n";
        let zip = zip_of(&[
            ("plugins/Nexo/items/all.yml", yml.as_bytes()),
            ("plugins/Nexo/pack/textures/default/ruby_sword.png", PNG),
            ("plugins/Nexo/pack/textures/default/ruby_ore.png", PNG),
            ("plugins/Nexo/pack/textures/default/cake.png", PNG),
            ("plugins/Nexo/pack/models/default/bench.json", br#"{"textures":{"0":"default/ruby_ore"}}"#),
            ("plugins/Nexo/pack/models/default/vault.json", br#"{"textures":{"0":"default/cake"}}"#),
        ]);
        let a = analyze(&zip).unwrap();
        assert_eq!(a.detected.len(), 1);
        assert_eq!(a.detected[0].framework, "nexo");
        let sword = by(&a, "ruby_sword");
        assert_eq!((sword.kind.as_str(), sword.cmd, sword.name.as_deref()), ("weapon", Some(1001), Some("&cRuby Sword")));
        let bench = by(&a, "bench");
        assert_eq!(bench.kind, "decoration");
        assert_eq!(bench.extras["seat"], true);
        assert_eq!(bench.scale, 1.5);
        assert!(bench.warnings.iter().any(|w| w.contains("glow")), "{:?}", bench.warnings);
        assert_eq!((by(&a, "vault").kind.as_str(), by(&a, "vault").extras["rows"].as_i64()), ("chest", Some(4)));
        let ore = by(&a, "ruby_ore");
        assert_eq!((ore.kind.as_str(), ore.extras["hardness"].as_f64(), ore.extras["light"].as_i64()), ("block", Some(3.0), Some(5)));
        let cake = by(&a, "cake");
        assert_eq!((cake.kind.as_str(), cake.extras["nutrition"].as_i64(), cake.material.as_str()), ("food", Some(8), "minecraft:bread"));
    }

    #[test]
    fn itemsadder_kinds_from_behaviours_and_properties() {
        let yml = r#"info:
  namespace: med
items:
  chair:
    display_name: Chair
    resource: {material: PAPER, model_path: item/chair}
    behaviours:
      furniture:
        sit: {sit_height: 0.4}
  stone_block:
    display_name: Stone
    resource: {material: PAPER, model_path: item/stone_block}
    specific_properties:
      block:
        placed_model: {type: REAL_NOTE}
  pie:
    display_name: Pie
    resource: {material: PAPER, textures: [item/pie.png]}
    consumable: {nutrition: 6}
    events:
      eat: {}
"#;
        let zip = zip_of(&[
            ("contents/med/configs/a.yml", yml.as_bytes()),
            ("contents/med/models/item/chair.json", br#"{"textures":{"0":"med:item/pie"}}"#),
            ("contents/med/models/item/stone_block.json", br#"{"textures":{"0":"med:item/pie"}}"#),
            ("contents/med/textures/item/pie.png", PNG),
        ]);
        let a = analyze(&zip).unwrap();
        assert_eq!((by(&a, "chair").kind.as_str(), by(&a, "chair").extras["seat"].as_bool()), ("decoration", Some(true)));
        assert_eq!(by(&a, "stone_block").kind, "block");
        let pie = by(&a, "pie");
        assert_eq!((pie.kind.as_str(), pie.extras["nutrition"].as_i64()), ("food", Some(6)));
        assert!(pie.warnings.iter().any(|w| w.contains("events")));
        assert_eq!(a.detected[0].framework, "itemsadder");
    }

    #[test]
    fn craftengine_items_blocks_and_furniture_share_models() {
        let yml = "items:\n  gems:ruby:\n    material: paper\n    custom-model-data: 5000\n    data:\n      item-name: '<red>Ruby'\n    model:\n      type: minecraft:model\n      path: gems:item/ruby\n\
blocks:\n  gems:ruby_block:\n    settings:\n      hardness: 4\n    state:\n      model:\n        path: gems:block/ruby_block\n\
furniture:\n  gems:ruby_lamp:\n    placement:\n      ground:\n        elements:\n          - item: gems:ruby\n";
        let zip = zip_of(&[
            ("plugins/CraftEngine/resources/gems/configuration/a.yml", yml.as_bytes()),
            ("plugins/CraftEngine/resources/gems/resourcepack/assets/gems/models/item/ruby.json", br#"{"textures":{"0":"gems:item/ruby"}}"#),
            ("plugins/CraftEngine/resources/gems/resourcepack/assets/gems/models/block/ruby_block.json", br#"{"textures":{"0":"gems:item/ruby"}}"#),
            ("plugins/CraftEngine/resources/gems/resourcepack/assets/gems/textures/item/ruby.png", PNG),
        ]);
        let a = analyze(&zip).unwrap();
        assert_eq!(a.detected[0].framework, "craftengine");
        assert_eq!((by(&a, "ruby").kind.as_str(), by(&a, "ruby").cmd, by(&a, "ruby").model.as_deref()), ("item", Some(5000), Some("gems:item/ruby")));
        assert_eq!((by(&a, "ruby_block").kind.as_str(), by(&a, "ruby_block").extras["hardness"].as_f64()), ("block", Some(4.0)));
        let lamp = by(&a, "ruby_lamp");
        assert_eq!((lamp.kind.as_str(), lamp.model.as_deref()), ("decoration", Some("gems:item/ruby")));
    }

    #[test]
    fn modelengine_blueprints_and_mythicmobs_link_up() {
        let bb = format!(r#"{{"resolution":{{"width":16,"height":16}},"textures":[{{"name":"skin.png","source":"data:image/png;base64,{BB_TEX}"}}],
            "elements":[{{"type":"cube","from":[-4,0,-4],"to":[4,32,4],"faces":{{"north":{{"uv":[0,0,8,16],"texture":0}}}}}}]}}"#);
        let mobs = "Bone_Knight:\n  Type: SKELETON\n  Display: '&7Bone Knight'\n  Health: 80\n  Damage: 6\n  Options:\n    MovementSpeed: 0.3\n  Drops:\n    - diamond 1-2 0.5\n    - exp 10\n  Skills:\n    - model{mid=knight;n=body} @self ~onSpawn\n    - effect:particles{p=flame} @self ~onTimer:20\n\
Plain_Zombie:\n  Type: ZOMBIE\n  Health: 30\n  AIGoalSelectors:\n    - clear\n";
        let zip = zip_of(&[("plugins/ModelEngine/blueprints/knight.bbmodel", bb.as_bytes()), ("plugins/MythicMobs/Mobs/bosses.yml", mobs.as_bytes())]);
        let a = analyze(&zip).unwrap();
        let knight = by(&a, "bone_knight");
        assert_eq!(knight.kind, "mob");
        assert_eq!(knight.model.as_deref(), Some("modelengine:knight"));
        assert_eq!((knight.extras["entity"].as_str(), knight.extras["health"].as_f64(), knight.extras["speed"].as_f64()), (Some("SKELETON"), Some(80.0), Some(0.3)));
        assert_eq!(knight.extras["drops"].as_array().unwrap().len(), 1, "exp lines are not item drops");
        assert_eq!(knight.extras["drops"][0]["item"], "diamond");
        assert!(knight.warnings.iter().any(|w| w.contains("skill")));
        assert!(a.entries.iter().all(|e| e.id != "knight"), "the blueprint is absorbed by the mob that uses it");
        assert!(a.files.contains_key("assets/modelengine/models/knight.json"));
        assert!(a.files.contains_key("assets/modelengine/textures/knight/skin.png"));
        let zombie = by(&a, "plain_zombie");
        assert!(zombie.model.is_none() && zombie.warnings.iter().any(|w| w.contains("AI")), "{:?}", zombie.warnings);
        let fws: Vec<_> = a.detected.iter().map(|d| d.framework).collect();
        assert_eq!(fws, ["mythicmobs"]);
    }

    #[test]
    fn a_lone_blueprint_is_an_npc_and_missing_blueprints_are_reported() {
        let bb = format!(r#"{{"textures":[{{"name":"t","source":"data:image/png;base64,{BB_TEX}"}}],"elements":[{{"type":"cube","from":[0,0,0],"to":[8,8,8],"faces":{{"north":{{"uv":[0,0,8,8],"texture":0}}}}}}]}}"#);
        let a = analyze(&zip_of(&[("blueprints/Villager Guy.bbmodel", bb.as_bytes())])).unwrap();
        assert_eq!((a.entries[0].kind.as_str(), a.entries[0].id.as_str()), ("npc", "villager_guy"));
        let mobs = "Boss:\n  Type: WITHER_SKELETON\n  Health: 500\n  ModelEngine:\n    Model: ghost\n";
        let a = analyze(&zip_of(&[("MythicMobs/Mobs/boss.yml", mobs.as_bytes())])).unwrap();
        assert!(a.entries[0].warnings.iter().any(|w| w.contains("ghost")), "{:?}", a.entries[0].warnings);
    }

    #[test]
    fn staged_items_collapse_into_a_crop() {
        let yml = "info: {namespace: farm}\nitems:\n  wheat_stage_1:\n    display_name: Wheat\n    resource: {material: PAPER, textures: [item/w1.png]}\n  wheat_stage_2:\n    display_name: Wheat\n    resource: {material: PAPER, textures: [item/w2.png]}\n  wheat_stage_3:\n    display_name: Wheat\n    resource: {material: PAPER, textures: [item/w3.png]}\n  hoe:\n    display_name: Hoe\n    resource: {material: IRON_HOE, textures: [item/w1.png]}\n";
        let zip = zip_of(&[
            ("contents/farm/configs/f.yml", yml.as_bytes()),
            ("contents/farm/textures/item/w1.png", PNG),
            ("contents/farm/textures/item/w2.png", PNG),
            ("contents/farm/textures/item/w3.png", PNG),
        ]);
        let a = analyze(&zip).unwrap();
        let crop = by(&a, "wheat");
        assert_eq!(crop.kind, "crop");
        assert_eq!(crop.extras["stages"].as_array().unwrap().len(), 3);
        assert_eq!(by(&a, "hoe").kind, "tool");
        assert!(a.entries.iter().all(|e| !e.id.contains("stage")));
    }

    #[test]
    fn unrecognisable_zips_are_refused_with_directions() {
        assert!(analyze(b"junk").is_err());
        let err = analyze(&zip_of(&[("notes.txt", b"hi")])).unwrap_err();
        assert!(err.contains("ModelEngine"), "{err}");
    }
}
