//! Import Oraxen and ItemsAdder resource packs.
//!
//! Both plugins ship a normal resource pack (`assets/<namespace>/…`, in ItemsAdder's case under `contents/<pack>/resourcepack/`,
//! in Oraxen's under `pack/`) plus YAML files that describe each item. This module reads a zip of either, keeps every texture,
//! model, sound, font and language file, and turns the item definitions into SCOPENET custom items (a vanilla base item, a
//! Custom Model Data number and a texture or model reference). It is pure — the route decides what to store.

use serde_json::{json, Value};
use serde_yaml::Value as Yaml;
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Read};

pub const MAX_ZIP_ENTRIES: usize = 20_000;
pub(crate) const MAX_ASSET: u64 = 2 * 1024 * 1024;
const MAX_YAML: u64 = 1024 * 1024;
const DIRS: [&str; 7] = ["textures", "models", "sounds", "font", "items", "lang", "atlases"];
/// First number handed to items whose pack doesn't pin one (both plugins assign these themselves).
pub const AUTO_MODEL_DATA_START: i64 = 100_000;

#[derive(Debug, Default)]
pub struct Imported {
    /// "oraxen", "itemsadder", or "resourcepack" when there are no item definitions.
    pub source: &'static str,
    pub files: BTreeMap<String, Vec<u8>>,
    pub items: Vec<ImportedItem>,
    pub skipped: Vec<String>,
    /// Things the importer did on its own (for example references it repaired).
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ImportedItem {
    pub key: String,
    pub title: String,
    /// `minecraft:paper`
    pub material: String,
    pub custom_model_data: i64,
    pub texture: Option<String>,
    pub model: Option<String>,
    pub name: Option<String>,
    pub lore: Vec<String>,
    pub unbreakable: bool,
    /// Textures the item's model names that are not in the pack (the item would render black and magenta).
    pub missing: Vec<String>,
}

impl ImportedItem {
    /// The spec stored in `custom_items`.
    pub fn spec(&self) -> Value {
        let mut spec = json!({"item": self.material, "amount": 1, "custom_model_data": self.custom_model_data});
        if let Some(t) = &self.texture {
            spec["texture"] = json!(t);
        }
        if let Some(m) = &self.model {
            spec["model"] = json!(m);
        }
        if let Some(n) = &self.name {
            spec["name"] = json!(n);
        }
        if !self.lore.is_empty() {
            spec["lore"] = json!(self.lore);
        }
        if self.unbreakable {
            spec["unbreakable"] = json!(true);
        }
        spec
    }
}

/// The first texture a model uses, as an `assets/<ns>/textures/<path>.png` path (`ns:path` references or bare paths).
pub fn model_texture_path(model_json: &[u8], model_ns: &str) -> Option<String> {
    let v: Value = serde_json::from_slice(model_json).ok()?;
    let textures = v.get("textures")?.as_object()?;
    let pick = ["layer0", "all", "particle", "0"].iter().find_map(|k| textures.get(*k)).or_else(|| textures.values().find(|t| t.as_str().is_some_and(|s| !s.starts_with('#'))))?;
    let reference = pick.as_str()?.to_lowercase();
    let (ns, path) = reference.split_once(':').unwrap_or(("minecraft", reference.as_str()));
    let ns = if reference.contains(':') || ns != "minecraft" { ns } else { model_ns };
    Some(format!("assets/{ns}/textures/{path}.png"))
}

/// A reference as Minecraft reads it: lowercase, `minecraft` when no namespace is given.
pub(crate) fn split_ref(raw: &str) -> (String, String) {
    let r = raw.trim().to_lowercase();
    match r.split_once(':') {
        Some((ns, p)) => (ns.to_string(), p.to_string()),
        None => ("minecraft".to_string(), r),
    }
}

/// Paths under these `minecraft:` folders are the game's own files, which a pack does not have to contain.
pub(crate) fn vanilla(ns: &str, path: &str) -> bool {
    ns == "minecraft"
        && ["item/", "block/", "entity/", "particle/", "gui/", "misc/", "environment/", "builtin/", "trims/", "mob_effect/"].iter().any(|p| path.starts_with(p))
}

fn model_files(files: &BTreeMap<String, Vec<u8>>) -> Vec<String> {
    files.keys().filter(|k| k.starts_with("assets/") && k.contains("/models/") && k.ends_with(".json")).cloned().collect()
}

/// Source packs often name textures and parents without the namespace they were packed under (Oraxen/ItemsAdder fix this up when
/// they build their own pack). Minecraft resolves a bare name in `minecraft:`, so such a model renders black and magenta. For every
/// reference that does not resolve, point it at the namespace that really holds the file. Returns how many references were fixed.
pub fn repair_models(files: &mut BTreeMap<String, Vec<u8>>) -> usize {
    let mut fixed = 0;
    let exists_anywhere = |files: &BTreeMap<String, Vec<u8>>, folder: &str, ext: &str, rel: &str, own: &str| -> Option<String> {
        let tail = format!("/{folder}/{rel}.{ext}");
        if files.contains_key(&format!("assets/{own}{tail}")) {
            return Some(own.to_string());
        }
        files.keys().find_map(|k| k.strip_prefix("assets/")?.strip_suffix(&tail).filter(|ns| !ns.contains('/')).map(String::from))
    };
    for path in model_files(files) {
        let own = path.strip_prefix("assets/").and_then(|r| r.split_once('/')).map_or("minecraft", |x| x.0).to_string();
        let Some(mut json) = files.get(&path).and_then(|b| serde_json::from_slice::<Value>(b).ok()) else { continue };
        let mut changed = false;
        if let Some(parent) = json.get("parent").and_then(Value::as_str).map(String::from) {
            let (ns, rel) = split_ref(&parent);
            if !parent.starts_with("builtin/") && !vanilla(&ns, &rel) && !files.contains_key(&format!("assets/{ns}/models/{rel}.json")) {
                if let Some(found) = exists_anywhere(files, "models", "json", &rel, &own) {
                    json["parent"] = Value::String(format!("{found}:{rel}"));
                    changed = true;
                    fixed += 1;
                }
            }
        }
        if let Some(textures) = json.get_mut("textures").and_then(Value::as_object_mut) {
            for (_, value) in textures.iter_mut() {
                let Some(raw) = value.as_str().map(String::from) else { continue };
                if raw.starts_with('#') {
                    continue;
                }
                let (ns, rel) = split_ref(&raw);
                if vanilla(&ns, &rel) || files.contains_key(&format!("assets/{ns}/textures/{rel}.png")) {
                    continue;
                }
                if let Some(found) = exists_anywhere(&files, "textures", "png", &rel, &own) {
                    *value = Value::String(format!("{found}:{rel}"));
                    changed = true;
                    fixed += 1;
                }
            }
        }
        if changed {
            if let Ok(bytes) = serde_json::to_vec(&json) {
                files.insert(path, bytes);
            }
        }
    }
    fixed
}

/// One reference a model makes that the pack cannot satisfy.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Problem {
    pub model: String,
    pub kind: &'static str,
    pub reference: String,
}

/// Textures and parents that models name but the pack lacks (vanilla `minecraft:item/…`-style references excepted).
pub fn problems(models: &BTreeMap<String, Vec<u8>>, has: &dyn Fn(&str) -> bool) -> Vec<Problem> {
    let mut out = Vec::new();
    for (path, bytes) in models {
        let Ok(json) = serde_json::from_slice::<Value>(bytes) else { continue };
        let name = path.trim_start_matches("assets/").replacen("/models/", ":", 1).trim_end_matches(".json").to_string();
        if let Some(parent) = json.get("parent").and_then(Value::as_str) {
            let (ns, rel) = split_ref(parent);
            if !vanilla(&ns, &rel) && !has(&format!("assets/{ns}/models/{rel}.json")) {
                out.push(Problem { model: name.clone(), kind: "parent model", reference: format!("{ns}:{rel}") });
            }
        }
        if let Some(textures) = json.get("textures").and_then(Value::as_object) {
            for value in textures.values().filter_map(Value::as_str).filter(|t| !t.starts_with('#')) {
                let (ns, rel) = split_ref(value);
                if !vanilla(&ns, &rel) && !has(&format!("assets/{ns}/textures/{rel}.png")) {
                    out.push(Problem { model: name.clone(), kind: "texture", reference: format!("{ns}:{rel}") });
                }
            }
        }
    }
    out
}

/// Every texture a model refers to (`#name` references to other slots excluded) as `assets/<ns>/textures/<path>.png` paths.
pub fn model_texture_paths(model_json: &[u8], model_ns: &str) -> Vec<String> {
    let Ok(v) = serde_json::from_slice::<Value>(model_json) else { return vec![] };
    let Some(textures) = v.get("textures").and_then(Value::as_object) else { return vec![] };
    textures
        .values()
        .filter_map(Value::as_str)
        .filter(|t| !t.starts_with('#'))
        .map(|t| {
            let t = t.to_lowercase();
            match t.split_once(':') {
                Some((ns, path)) => format!("assets/{ns}/textures/{path}.png"),
                None => format!("assets/{model_ns}/textures/{t}.png"),
            }
        })
        .collect()
}

/// PNG bytes that show what an item looks like: its texture, or the first texture of its model. `get` reads a pack file by path.
pub fn preview_png(get: impl Fn(&str) -> Option<Vec<u8>>, texture: Option<&str>, model: Option<&str>) -> Option<Vec<u8>> {
    if let Some((ns, path)) = texture.and_then(|t| t.split_once(':')) {
        if let Some(bytes) = get(&format!("assets/{ns}/textures/{path}.png")) {
            return Some(bytes);
        }
    }
    let (ns, path) = model?.split_once(':')?;
    let json = get(&format!("assets/{ns}/models/{path}.json"))?;
    let tex = model_texture_path(&json, ns)?;
    get(&tex).or_else(|| {
        // Oraxen models often name textures without the namespace they were packed under: try the model's own, then minecraft.
        let rest = tex.split_once("/textures/")?.1;
        get(&format!("assets/{ns}/textures/{rest}")).or_else(|| get(&format!("assets/minecraft/textures/{rest}")))
    })
}

/// Where a zip entry lands in a resource pack, if it belongs in one.
fn pack_path(entry: &str) -> Option<String> {
    let parts: Vec<&str> = entry.split('/').filter(|p| !p.is_empty()).collect();
    if parts.iter().any(|p| *p == ".." || *p == ".") {
        return None;
    }
    let ext_ok = ["png", "json", "ogg", "mcmeta"].iter().any(|e| entry.to_ascii_lowercase().ends_with(&format!(".{e}")));
    if !ext_ok {
        return None;
    }
    let n = parts.len();
    // …/assets/<ns>/<dir>/… (built packs, ItemsAdder `resourcepack/assets/…`)
    for i in 0..n.saturating_sub(3) {
        if parts[i] == "assets" && DIRS.contains(&parts[i + 2]) {
            return Some(parts[i..].join("/"));
        }
    }
    // …/assets/<ns>/sounds.json
    if n >= 3 && parts[n - 3] == "assets" && parts[n - 1] == "sounds.json" {
        return Some(parts[n - 3..].join("/"));
    }
    // …/resourcepack/<ns>/<dir>/… (ItemsAdder's short form)
    for i in 0..n.saturating_sub(3) {
        if parts[i] == "resourcepack" && DIRS.contains(&parts[i + 2]) {
            return Some(format!("assets/{}", parts[i + 1..].join("/")));
        }
    }
    // contents/<ns>/<dir>/… (ItemsAdder's own layout: textures and models next to configs/)
    for i in 0..n.saturating_sub(3) {
        if parts[i] == "contents" && parts[i + 1] != "resourcepack" && DIRS.contains(&parts[i + 2]) {
            return Some(format!("assets/{}", parts[i + 1..].join("/")));
        }
    }
    // pack/<dir>/… (Oraxen's source folder: vanilla namespace)
    for i in 0..n.saturating_sub(2) {
        if parts[i] == "pack" && DIRS.contains(&parts[i + 1]) {
            return Some(format!("assets/minecraft/{}", parts[i + 1..].join("/")));
        }
    }
    None
}

fn valid_asset_path(path: &str) -> bool {
    let parts: Vec<&str> = path.split('/').collect();
    (parts.len() >= 4 || parts.len() == 3 && parts[2] == "sounds.json")
        && parts[0] == "assets"
        && parts.len() >= 3
        && parts[1..].iter().all(|p| {
            !p.is_empty() && p.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "_.-".contains(c))
        })
}

/// Everything a framework zip holds that we care about, read once.
#[derive(Default)]
pub struct Collected {
    /// Resource-pack files by `assets/<ns>/…` path.
    pub files: BTreeMap<String, Vec<u8>>,
    /// Parsed YAML files with their path inside the zip.
    pub yamls: Vec<(String, Yaml)>,
    /// Blockbench `.bbmodel` files (ModelEngine blueprints) with their path inside the zip.
    pub blueprints: Vec<(String, Vec<u8>)>,
    pub skipped: Vec<String>,
}

pub fn collect(zip_bytes: &[u8]) -> Result<Collected, String> {
    let mut zip = zip::ZipArchive::new(Cursor::new(zip_bytes)).map_err(|_| "That isn't a zip file".to_string())?;
    if zip.len() > MAX_ZIP_ENTRIES {
        return Err("That zip has too many files".into());
    }
    let mut out = Collected::default();
    for i in 0..zip.len() {
        let Ok(mut f) = zip.by_index(i) else { continue };
        if f.is_dir() {
            continue;
        }
        let name = f.name().replace('\\', "/");
        let lower = name.to_ascii_lowercase();
        if lower.ends_with(".yml") || lower.ends_with(".yaml") {
            if f.size() <= MAX_YAML {
                let mut text = String::new();
                if f.read_to_string(&mut text).is_ok() {
                    if let Ok(y) = serde_yaml::from_str::<Yaml>(&text) {
                        out.yamls.push((name, y));
                    }
                }
            }
            continue;
        }
        if lower.ends_with(".bbmodel") {
            if f.size() <= 24 * 1024 * 1024 {
                let mut bytes = Vec::with_capacity(f.size() as usize);
                if f.read_to_end(&mut bytes).is_ok() {
                    out.blueprints.push((name, bytes));
                }
            } else {
                out.skipped.push(format!("{name} (model over 24 MiB)"));
            }
            continue;
        }
        let Some(path) = pack_path(&name) else { continue };
        let path = path.to_ascii_lowercase();
        if !valid_asset_path(&path) {
            out.skipped.push(format!("{name} (unsupported file name — use lowercase letters, numbers, _ . -)"));
            continue;
        }
        if f.size() > MAX_ASSET {
            out.skipped.push(format!("{name} (over 2 MiB)"));
            continue;
        }
        let mut bytes = Vec::with_capacity(f.size() as usize);
        if f.read_to_end(&mut bytes).is_err() || bytes.is_empty() {
            continue;
        }
        if (path.ends_with(".mcmeta") || path.ends_with(".json")) && !serde_json::from_slice::<Value>(&bytes).is_ok_and(|v| v.is_object()) {
            out.skipped.push(format!("{name} (not a JSON object)"));
            continue;
        }
        if path.ends_with(".png") && !bytes.starts_with(b"\x89PNG") {
            out.skipped.push(format!("{name} (not a PNG)"));
            continue;
        }
        if path.ends_with(".ogg") && !bytes.starts_with(b"OggS") {
            out.skipped.push(format!("{name} (not Ogg audio)"));
            continue;
        }
        out.files.insert(path, bytes);
    }
    Ok(out)
}

pub fn parse(zip_bytes: &[u8]) -> Result<Imported, String> {
    let collected = collect(zip_bytes)?;
    let mut out = Imported { files: collected.files, skipped: collected.skipped, ..Imported::default() };
    let yamls = collected.yamls;
    if out.files.is_empty() {
        return Err("No resource-pack files found. Expected assets/…, an Oraxen pack/ folder or an ItemsAdder contents/…/resourcepack/ folder".into());
    }

    let repaired = repair_models(&mut out.files);
    if repaired > 0 {
        out.notes.push(format!("Fixed {repaired} texture/model references that were missing their namespace, so models find their textures"));
    }
    let mut items = Vec::new();
    let (mut oraxen, mut ia) = (false, false);
    for (file, yaml) in &yamls {
        if let Some(found) = itemsadder_items(yaml, &out.files) {
            ia |= !found.is_empty();
            items.extend(found);
        } else if let Some(found) = oraxen_items(yaml, &out.files) {
            oraxen |= !found.is_empty();
            items.extend(found);
        }
        let _ = file;
    }
    out.source = match (oraxen, ia) {
        (_, true) => "itemsadder",
        (true, _) => "oraxen",
        _ => "resourcepack",
    };

    // Skip repeated ids, then give every item without a pinned number a free one per base material.
    let mut seen = BTreeSet::new();
    let mut taken: BTreeSet<(String, i64)> = BTreeSet::new();
    items.retain(|it: &RawItem| seen.insert(it.key.clone()));
    for it in &items {
        if let Some(n) = it.cmd {
            taken.insert((it.material.clone(), n));
        }
    }
    let mut next: BTreeMap<String, i64> = BTreeMap::new();
    for it in items {
        let (texture, model) = (it.texture.clone(), it.model.clone());
        if texture.is_none() && model.is_none() {
            out.skipped.push(format!("item {} (no texture or model found in the pack)", it.key));
            continue;
        }
        let cmd = match it.cmd {
            Some(n) if (1..=16_777_215).contains(&n) => n,
            _ => {
                let c = next.entry(it.material.clone()).or_insert(AUTO_MODEL_DATA_START);
                while taken.contains(&(it.material.clone(), *c)) {
                    *c += 1;
                }
                taken.insert((it.material.clone(), *c));
                *c
            }
        };
        let missing = model
            .as_deref()
            .and_then(|m| m.split_once(':'))
            .map(|(ns, path)| {
                let key = format!("assets/{ns}/models/{path}.json");
                let models: BTreeMap<String, Vec<u8>> = out.files.get(&key).map(|j| (key.clone(), j.clone())).into_iter().collect();
                problems(&models, &|p| out.files.contains_key(p)).into_iter().map(|p| p.reference).collect::<Vec<_>>()
            })
            .unwrap_or_default();
        out.items.push(ImportedItem {
            missing,
            title: strip_formatting(it.name.as_deref().unwrap_or(&it.key)).chars().take(60).collect(),
            key: it.key,
            material: it.material,
            custom_model_data: cmd,
            texture,
            model,
            name: it.name,
            lore: it.lore,
            unbreakable: it.unbreakable,
        });
    }
    Ok(out)
}

struct RawItem {
    key: String,
    material: String,
    cmd: Option<i64>,
    texture: Option<String>,
    model: Option<String>,
    name: Option<String>,
    lore: Vec<String>,
    unbreakable: bool,
}

pub(crate) fn text(v: Option<&Yaml>) -> Option<String> {
    match v? {
        Yaml::String(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
        _ => None,
    }
}
pub(crate) fn list(v: Option<&Yaml>) -> Vec<String> {
    match v {
        Some(Yaml::Sequence(s)) => s.iter().filter_map(|x| text(Some(x))).collect(),
        Some(Yaml::String(s)) if !s.trim().is_empty() => vec![s.trim().to_string()],
        _ => vec![],
    }
}
pub(crate) fn num(v: Option<&Yaml>) -> Option<i64> {
    v.and_then(Yaml::as_i64)
}

/// Item keys must be `[a-z0-9_-]{1,32}`.
pub(crate) fn item_key(raw: &str) -> Option<String> {
    let k: String = raw
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' { c } else { '_' })
        .take(32)
        .collect();
    (!k.trim_matches('_').is_empty()).then_some(k)
}

pub(crate) fn material(raw: Option<String>) -> String {
    let m = raw.unwrap_or_else(|| "PAPER".into()).to_lowercase();
    let m = m.trim_start_matches("minecraft:");
    format!("minecraft:{m}")
}

/// Remove MiniMessage tags (`<gradient:…>`, `<#ff0000>`, `</bold>`) that Minecraft's legacy codes can't express.
pub fn strip_formatting(s: &str) -> String {
    let mut out = String::new();
    let mut depth = 0;
    for c in s.chars() {
        match c {
            '<' => depth += 1,
            '>' if depth > 0 => depth -= 1,
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    // Drop legacy colour codes for the title only; the name keeps them.
    let mut clean = String::new();
    let mut it = out.chars().peekable();
    while let Some(c) = it.next() {
        if (c == '&' || c == '§') && it.peek().is_some_and(|n| n.is_ascii_alphanumeric()) {
            it.next();
        } else {
            clean.push(c);
        }
    }
    clean.trim().to_string()
}

pub(crate) fn display(s: String) -> String {
    let mut out = String::new();
    let mut depth = 0;
    for c in s.chars() {
        match c {
            '<' => depth += 1,
            '>' if depth > 0 => depth -= 1,
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    let one_line: String = out.replace('\n', " ");
    one_line.trim().chars().take(100).collect()
}

pub(crate) fn lore_lines(raw: Vec<String>) -> Vec<String> {
    raw.into_iter().map(|l| display(l).chars().take(160).collect()).take(24).collect()
}

/// Find `assets/<ns>/<folder>/<rel>.<ext>` among the imported files and return it as `ns:rel`.
pub(crate) fn resolve(files: &BTreeMap<String, Vec<u8>>, folder: &str, ext: &str, rel: &str, prefer: &[&str]) -> Option<String> {
    let rel = rel.trim().trim_start_matches('/').to_lowercase();
    let (explicit, rel) = match rel.split_once(':') {
        Some((ns, r)) => (Some(ns.to_string()), r.to_string()),
        None => (None, rel),
    };
    let rel = rel.strip_suffix(&format!(".{ext}")).unwrap_or(&rel).to_string();
    let exists = |ns: &str| files.contains_key(&format!("assets/{ns}/{folder}/{rel}.{ext}"));
    let candidates = explicit.iter().map(String::as_str).chain(prefer.iter().copied()).chain(std::iter::once("minecraft"));
    for ns in candidates {
        if exists(ns) {
            return Some(format!("{ns}:{rel}"));
        }
    }
    let suffix = format!("/{folder}/{rel}.{ext}");
    files.keys().find_map(|k| {
        let ns = k.strip_prefix("assets/")?.strip_suffix(&suffix)?;
        (!ns.contains('/')).then(|| format!("{ns}:{rel}"))
    })
}

fn oraxen_items(y: &Yaml, files: &BTreeMap<String, Vec<u8>>) -> Option<Vec<RawItem>> {
    let map = y.as_mapping()?;
    let mut out = Vec::new();
    let mut looked_like_items = false;
    for (k, v) in map {
        let (Some(id), Some(def)) = (k.as_str(), v.as_mapping()) else { continue };
        let get = |name: &str| def.iter().find(|(k, _)| k.as_str().is_some_and(|s| s.eq_ignore_ascii_case(name))).map(|(_, v)| v);
        let pack = get("pack").and_then(Yaml::as_mapping);
        if pack.is_none() && get("material").is_none() {
            continue;
        }
        looked_like_items = true;
        let Some(pack) = pack else { continue };
        let p = |name: &str| pack.iter().find(|(k, _)| k.as_str().is_some_and(|s| s.eq_ignore_ascii_case(name))).map(|(_, v)| v);
        let Some(key) = item_key(id) else { continue };
        let model = text(p("model")).and_then(|m| resolve(files, "models", "json", &m, &[]));
        let mut tex = list(p("textures"));
        tex.extend(list(p("texture")));
        let texture = tex.first().and_then(|t| resolve(files, "textures", "png", t, &[]));
        out.push(RawItem {
            key,
            material: material(text(get("material"))),
            cmd: num(p("custom_model_data")),
            texture,
            model,
            name: text(get("displayname")).or_else(|| text(get("itemname"))).map(display),
            lore: lore_lines(list(get("lore"))),
            unbreakable: get("unbreakable").and_then(Yaml::as_bool).unwrap_or(false),
        });
    }
    looked_like_items.then_some(out)
}

fn itemsadder_items(y: &Yaml, files: &BTreeMap<String, Vec<u8>>) -> Option<Vec<RawItem>> {
    let root = y.as_mapping()?;
    let items = root.get("items")?.as_mapping()?;
    let namespace = root.get("info").and_then(|i| i.get("namespace")).and_then(Yaml::as_str).unwrap_or("").to_lowercase();
    let prefer: Vec<&str> = if namespace.is_empty() { vec![] } else { vec![namespace.as_str()] };
    let mut out = Vec::new();
    for (k, v) in items {
        let (Some(id), Some(def)) = (k.as_str(), v.as_mapping()) else { continue };
        let Some(res) = def.get("resource").and_then(Yaml::as_mapping) else { continue };
        let Some(key) = item_key(id) else { continue };
        let model = text(res.get("model_path")).and_then(|m| resolve(files, "models", "json", &m, &prefer));
        let mut tex = list(res.get("textures"));
        tex.extend(list(res.get("texture")));
        let texture = tex.first().and_then(|t| resolve(files, "textures", "png", t, &prefer));
        out.push(RawItem {
            key,
            material: material(text(res.get("material"))),
            cmd: num(res.get("model_id")).or_else(|| num(res.get("custom_model_data"))),
            texture,
            model,
            name: text(def.get("display_name")).map(display),
            lore: lore_lines(list(def.get("lore"))),
            unbreakable: false,
        });
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    #[test]
    fn models_naming_textures_the_pack_lacks_are_flagged() {
        let yml = "info:\n  namespace: med\nitems:\n  hammer:\n    display_name: Hammer\n    resource:\n      material: IRON_AXE\n      model_path: item/hammer\n";
        let model = br##"{"textures":{"0":"med:weapons/head","1":"med:weapons/handle","particle":"#0"}}"##;
        let zip = zip_of(&[
            ("contents/med/configs/i.yml", yml.as_bytes()),
            ("contents/med/resourcepack/med/models/item/hammer.json", model),
            ("contents/med/resourcepack/med/textures/weapons/head.png", PNG),
        ]);
        let got = parse(&zip).unwrap();
        assert_eq!(got.items[0].missing, vec!["med:weapons/handle".to_string()]);
        assert!(got.files.contains_key("assets/med/textures/weapons/head.png"));
    }

    use super::*;
    use std::io::Write;

    fn zip_of(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut z = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let o = zip::write::SimpleFileOptions::default();
        for (n, d) in files {
            z.start_file(*n, o).unwrap();
            z.write_all(d).unwrap();
        }
        z.finish().unwrap().into_inner()
    }
    const PNG: &[u8] = b"\x89PNG\r\n\x1a\nxxxx";

    #[test]
    fn maps_every_known_layout_and_rejects_the_rest() {
        assert_eq!(pack_path("assets/a/textures/item/x.png").as_deref(), Some("assets/a/textures/item/x.png"));
        assert_eq!(pack_path("Pack1/assets/a/models/x.json").as_deref(), Some("assets/a/models/x.json"));
        assert_eq!(pack_path("contents/ruby/resourcepack/ruby/textures/item/x.png").as_deref(), Some("assets/ruby/textures/item/x.png"));
        assert_eq!(pack_path("contents/ruby/resourcepack/assets/ruby/textures/x.png").as_deref(), Some("assets/ruby/textures/x.png"));
        assert_eq!(pack_path("plugins/Oraxen/pack/textures/default/ruby.png").as_deref(), Some("assets/minecraft/textures/default/ruby.png"));
        assert_eq!(pack_path("assets/a/shaders/x.json"), None);
        assert_eq!(pack_path("assets/a/textures/../../x.png"), None);
        assert_eq!(pack_path("readme.txt"), None);
    }

    #[test]
    fn imports_an_itemsadder_pack() {
        let yml = "info:\n  namespace: ruby\nitems:\n  ruby_sword:\n    display_name: \"&cRuby Sword\"\n    lore:\n      - \"&7Sharp\"\n    resource:\n      material: DIAMOND_SWORD\n      generate: true\n      textures:\n        - item/ruby_sword.png\n  ghost:\n    display_name: Ghost\n    resource:\n      material: PAPER\n      textures:\n        - item/missing.png\n";
        let zip = zip_of(&[
            ("contents/ruby/configs/items.yml", yml.as_bytes()),
            ("contents/ruby/resourcepack/ruby/textures/item/ruby_sword.png", PNG),
            ("contents/ruby/resourcepack/ruby/textures/item/Bad Name.png", PNG),
        ]);
        let got = parse(&zip).unwrap();
        assert_eq!(got.source, "itemsadder");
        assert_eq!(got.files.keys().collect::<Vec<_>>(), ["assets/ruby/textures/item/ruby_sword.png"]);
        assert_eq!(got.items.len(), 1);
        let it = &got.items[0];
        assert_eq!((it.key.as_str(), it.material.as_str(), it.custom_model_data), ("ruby_sword", "minecraft:diamond_sword", AUTO_MODEL_DATA_START));
        assert_eq!(it.texture.as_deref(), Some("ruby:item/ruby_sword"));
        assert_eq!((it.name.as_deref(), it.title.as_str()), (Some("&cRuby Sword"), "Ruby Sword"));
        assert_eq!(it.spec()["lore"][0], "&7Sharp");
        assert!(got.skipped.iter().any(|s| s.contains("Bad Name")) && got.skipped.iter().any(|s| s.contains("ghost")));
    }

    #[test]
    fn imports_an_oraxen_pack_and_keeps_pinned_model_data() {
        let yml = "ruby_helm:\n  displayname: \"<gradient:red:blue>Ruby Helm</gradient>\"\n  material: DIAMOND_HELMET\n  Pack:\n    generate_model: true\n    parent_model: item/generated\n    textures:\n      - default/ruby_helm\n    custom_model_data: 1001\nplain:\n  material: STICK\n  Pack:\n    model: default/plain_model\nauto_one:\n  material: DIAMOND_HELMET\n  Pack:\n    textures: [default/ruby_helm.png]\n";
        let zip = zip_of(&[
            ("plugins/Oraxen/items/helms.yml", yml.as_bytes()),
            ("plugins/Oraxen/pack/textures/default/ruby_helm.png", PNG),
            ("plugins/Oraxen/pack/models/default/plain_model.json", br#"{"parent":"item/generated","textures":{"layer0":"default/x"}}"#),
        ]);
        let got = parse(&zip).unwrap();
        assert_eq!(got.source, "oraxen");
        let by: BTreeMap<_, _> = got.items.iter().map(|i| (i.key.as_str(), i)).collect();
        assert_eq!(by["ruby_helm"].custom_model_data, 1001);
        assert_eq!(by["ruby_helm"].title, "Ruby Helm");
        assert_eq!(by["ruby_helm"].name.as_deref(), Some("Ruby Helm"));
        assert_eq!(by["ruby_helm"].texture.as_deref(), Some("minecraft:default/ruby_helm"));
        assert_eq!(by["plain"].model.as_deref(), Some("minecraft:default/plain_model"));
        assert_eq!(by["plain"].custom_model_data, AUTO_MODEL_DATA_START);
        assert_eq!(by["auto_one"].custom_model_data, AUTO_MODEL_DATA_START, "numbers are per base item");
        // Every spec passes the panel's own validation.
        for i in got.items {
            crate::routes::utilities::clean_item(&i.spec()).unwrap();
        }
    }

    #[test]
    fn previews_come_from_textures_or_from_the_models_first_texture() {
        let files: BTreeMap<String, Vec<u8>> = [
            ("assets/ruby/textures/item/gem.png", b"gem".to_vec()),
            ("assets/minecraft/textures/default/helm.png", b"helm".to_vec()),
            ("assets/ruby/models/item/gem3d.json", br#"{"textures":{"particle":"ruby:item/gem","layer0":"ruby:item/gem"}}"#.to_vec()),
            ("assets/minecraft/models/default/helm.json", br#"{"textures":{"layer0":"default/helm"}}"#.to_vec()),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
        let get = |p: &str| files.get(p).cloned();
        assert_eq!(preview_png(get, Some("ruby:item/gem"), None).unwrap(), b"gem");
        assert_eq!(preview_png(get, None, Some("ruby:item/gem3d")).unwrap(), b"gem");
        assert_eq!(preview_png(get, None, Some("minecraft:default/helm")).unwrap(), b"helm");
        assert!(preview_png(get, Some("ruby:item/nope"), None).is_none());
    }

    #[test]
    fn a_plain_pack_imports_only_files_and_junk_is_refused() {
        let got = parse(&zip_of(&[("assets/x/sounds.json", b"{}"), ("assets/x/sounds/a.ogg", b"OggSxx")])).unwrap();
        assert_eq!((got.source, got.files.len(), got.items.len()), ("resourcepack", 2, 0));
        assert!(parse(b"not a zip").is_err());
        assert!(parse(&zip_of(&[("notes.txt", b"hi")])).is_err());
    }
}
