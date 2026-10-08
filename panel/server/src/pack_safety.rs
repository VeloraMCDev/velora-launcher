//! Keeps the server resource pack from disturbing how the *world* looks.
//!
//! Imported packs (Oraxen, ItemsAdder, Nexo ...) are built to be the only pack on a server, so they often carry more than custom
//! items: replacements for vanilla block models, block states, block textures and colour maps, shader files, and atlas definitions
//! that stitch whole folders (or everything) into the block atlas. On a client that is also loading the player's own packs and mods,
//! those files are what turn grass solid green, strip the texture off one facing of a stair, or paint the whole world with one
//! texture: the block atlas and the models baked against it no longer agree. A single truncated PNG that reaches the atlas does the
//! same. Nothing here changes how custom items, blocks or NPCs look; it only removes what could change vanilla blocks.

use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::io::Cursor;

/// Something taken out of the pack, and why.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Removed {
    pub path: String,
    pub reason: String,
}

/// Folders of the `minecraft` namespace that decide how vanilla blocks, terrain and entities are drawn.
const VANILLA_WORLD: &[(&str, &str)] = &[
    ("blockstates/", "replaces how a vanilla block picks its model"),
    ("models/block/", "replaces a vanilla block model"),
    ("textures/block/", "replaces a vanilla block texture"),
    ("textures/colormap/", "replaces the grass and foliage tint maps"),
    ("textures/entity/", "replaces a vanilla mob texture"),
    ("textures/environment/", "replaces the sky, sun, moon or clouds"),
    ("textures/particle/", "replaces a vanilla particle"),
    ("textures/misc/", "replaces a vanilla world texture"),
    ("particles/", "replaces a vanilla particle definition"),
    ("shaders/", "replaces a core shader, which breaks rendering with shader mods"),
    ("post_effect/", "replaces a post-processing effect"),
    ("equipment/", "replaces vanilla armour rendering"),
];

/// Atlases packs may add sprites to. Every other atlas file is dropped.
const ATLASES: &[&str] = &["blocks", "items"];

const MAX_SIDE: u32 = 4096;

fn png_ok(bytes: &[u8], full: bool) -> Result<(u32, u32), String> {
    use image::ImageDecoder;
    let decoder = image::codecs::png::PngDecoder::new(Cursor::new(bytes)).map_err(|_| "not a readable PNG".to_string())?;
    let (w, h) = decoder.dimensions();
    if w == 0 || h == 0 {
        return Err("empty image".into());
    }
    if w > MAX_SIDE || h > MAX_SIDE {
        return Err(format!("{w}x{h} is larger than {MAX_SIDE}px, which can overflow the texture atlas"));
    }
    if full {
        image::load_from_memory_with_format(bytes, image::ImageFormat::Png).map_err(|_| "damaged PNG (cannot be fully decoded)".to_string())?;
    }
    Ok((w, h))
}

/// A directory source that would pull in far more than a pack's own sprites.
fn risky_source(source: &Value) -> Option<&'static str> {
    let kind = source["type"].as_str().unwrap_or("").trim_start_matches("minecraft:");
    match kind {
        "single" => source["resource"].as_str().filter(|r| !r.is_empty()).map(|_| None).unwrap_or(Some("a single source without a resource")),
        "directory" => {
            let dir = source["source"].as_str().unwrap_or("").trim_matches('/');
            if dir.is_empty() || dir == "." || dir.contains("..") {
                Some("a directory source that would stitch every texture in the pack")
            } else if matches!(dir, "block" | "item") && source["prefix"].as_str().is_some_and(|p| p.is_empty()) {
                Some("a directory source that renames vanilla sprites")
            } else {
                None
            }
        }
        "filter" | "unstitch" | "paletted_permutations" => None,
        _ => Some("an unknown source type"),
    }
}

/// Remove everything from `files` that could change vanilla world rendering. With `allow_vanilla` the pack keeps its block
/// overrides (an admin who really wants a retextured world), but atlases and PNGs are still checked.
pub fn sanitize(files: &mut BTreeMap<String, Vec<u8>>, allow_vanilla: bool) -> Vec<Removed> {
    let mut removed: Vec<Removed> = Vec::new();

    if !allow_vanilla {
        let doomed: Vec<(String, String)> = files
            .keys()
            .filter_map(|k| {
                let rest = k.strip_prefix("assets/minecraft/")?;
                VANILLA_WORLD.iter().find(|(prefix, _)| rest.starts_with(prefix)).map(|(_, why)| (k.clone(), why.to_string()))
            })
            .collect();
        for (path, reason) in doomed {
            files.remove(&path);
            removed.push(Removed { path, reason });
        }
    }

    // Atlas definitions: keep only the two atlases custom sprites need, and only well-formed, narrow sources.
    let atlas_paths: Vec<String> = files.keys().filter(|k| k.starts_with("assets/") && k.contains("/atlases/") && k.ends_with(".json")).cloned().collect();
    for path in atlas_paths {
        let name = path.rsplit('/').next().unwrap_or("").trim_end_matches(".json").to_string();
        let namespace_ok = path.starts_with("assets/minecraft/atlases/");
        if !namespace_ok || !ATLASES.contains(&name.as_str()) {
            files.remove(&path);
            removed.push(Removed { path, reason: "an atlas definition other than blocks or items".into() });
            continue;
        }
        let Some(mut doc) = files.get(&path).and_then(|b| serde_json::from_slice::<Value>(b).ok()) else {
            files.remove(&path);
            removed.push(Removed { path, reason: "unreadable atlas definition".into() });
            continue;
        };
        let sources = doc["sources"].as_array().cloned().unwrap_or_default();
        let before = sources.len();
        let mut kept: Vec<Value> = Vec::new();
        for s in sources {
            match risky_source(&s) {
                Some(why) => removed.push(Removed { path: path.clone(), reason: format!("removed {why}") }),
                None if kept.contains(&s) => {}
                None => kept.push(s),
            }
        }
        if kept.is_empty() {
            files.remove(&path);
            if before > 0 {
                removed.push(Removed { path, reason: "atlas definition had nothing safe left".into() });
            }
        } else {
            doc["sources"] = Value::Array(kept);
            if let Ok(b) = serde_json::to_vec(&doc) {
                files.insert(path, b);
            }
        }
    }

    // Images: header check for everything, a full decode for the textures models and atlases pull into the block atlas.
    let referenced = referenced_textures(files);
    let pngs: Vec<String> = files.keys().filter(|k| k.ends_with(".png") && k.contains("/textures/")).cloned().collect();
    for path in pngs {
        let full = referenced.contains(&path);
        let verdict = files.get(&path).map(|b| png_ok(b, full)).unwrap_or(Ok((1, 1)));
        if let Err(reason) = verdict {
            files.remove(&path);
            files.remove(&format!("{path}.mcmeta"));
            removed.push(Removed { path, reason });
        }
    }
    removed
}

/// `assets/<ns>/textures/<path>.png` for every texture a model or atlas source names.
fn referenced_textures(files: &BTreeMap<String, Vec<u8>>) -> std::collections::BTreeSet<String> {
    let mut out = std::collections::BTreeSet::new();
    for (path, bytes) in files {
        if path.contains("/models/") && path.ends_with(".json") {
            let own = path.strip_prefix("assets/").and_then(|r| r.split_once('/')).map_or("minecraft", |x| x.0).to_string();
            out.extend(crate::pack_import::model_texture_paths(bytes, &own));
        }
    }
    out
}

pub fn summary(removed: &[Removed]) -> Value {
    json!({ "count": removed.len(), "removed": removed })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png() -> Vec<u8> {
        let mut out = Vec::new();
        image::DynamicImage::new_rgba8(16, 16).write_to(&mut Cursor::new(&mut out), image::ImageFormat::Png).unwrap();
        out
    }
    fn pack(entries: &[(&str, Vec<u8>)]) -> BTreeMap<String, Vec<u8>> {
        entries.iter().map(|(k, v)| (k.to_string(), v.clone())).collect()
    }

    #[test]
    fn vanilla_block_overrides_are_stripped_but_custom_content_stays() {
        let mut f = pack(&[
            ("assets/minecraft/blockstates/oak_stairs.json", b"{}".to_vec()),
            ("assets/minecraft/models/block/stairs.json", b"{}".to_vec()),
            ("assets/minecraft/textures/block/grass_block_top.png", png()),
            ("assets/minecraft/textures/colormap/grass.png", png()),
            ("assets/minecraft/shaders/core/position.fsh", b"x".to_vec()),
            ("assets/minecraft/models/item/paper.json", b"{}".to_vec()),
            ("assets/minecraft/items/paper.json", b"{}".to_vec()),
            ("assets/minecraft/textures/item/paper.png", png()),
            ("assets/med/textures/block/bench.png", png()),
            ("assets/med/models/block/bench.json", b"{}".to_vec()),
        ]);
        let removed = sanitize(&mut f, false);
        assert_eq!(removed.len(), 5, "{removed:?}");
        for keep in ["assets/minecraft/models/item/paper.json", "assets/minecraft/items/paper.json", "assets/minecraft/textures/item/paper.png", "assets/med/textures/block/bench.png", "assets/med/models/block/bench.json"] {
            assert!(f.contains_key(keep), "{keep} must stay");
        }
        let mut g = pack(&[("assets/minecraft/textures/block/grass_block_top.png", png())]);
        assert!(sanitize(&mut g, true).is_empty(), "allowed on request");
        assert!(g.contains_key("assets/minecraft/textures/block/grass_block_top.png"));
    }

    #[test]
    fn atlases_keep_narrow_sources_only() {
        let atlas = json!({"sources": [
            {"type": "minecraft:directory", "source": "", "prefix": ""},
            {"type": "directory", "source": "/", "prefix": "x/"},
            {"type": "directory", "source": "block", "prefix": ""},
            {"type": "minecraft:single", "resource": "med:default/a"},
            {"type": "minecraft:single", "resource": "med:default/a"},
            {"type": "directory", "source": "med/weapons", "prefix": "med/weapons/"},
        ]});
        let mut f = pack(&[
            ("assets/minecraft/atlases/blocks.json", serde_json::to_vec(&atlas).unwrap()),
            ("assets/minecraft/atlases/gui.json", br#"{"sources":[{"type":"single","resource":"x"}]}"#.to_vec()),
            ("assets/minecraft/atlases/items.json", br#"{"sources":[{"type":"directory","source":"","prefix":""}]}"#.to_vec()),
        ]);
        let removed = sanitize(&mut f, false);
        let kept: Value = serde_json::from_slice(&f["assets/minecraft/atlases/blocks.json"]).unwrap();
        assert_eq!(kept["sources"].as_array().unwrap().len(), 2, "{kept}");
        assert!(!f.contains_key("assets/minecraft/atlases/gui.json") && !f.contains_key("assets/minecraft/atlases/items.json"), "{removed:?}");
    }

    #[test]
    fn broken_or_oversized_images_never_reach_the_atlas() {
        let mut cut = png();
        cut.truncate(cut.len() - 12);
        let big = {
            let mut out = Vec::new();
            image::DynamicImage::new_rgba8(5000, 4).write_to(&mut Cursor::new(&mut out), image::ImageFormat::Png).unwrap();
            out
        };
        let model = br#"{"parent":"minecraft:item/generated","textures":{"layer0":"med:default/cut"}}"#.to_vec();
        let mut f = pack(&[
            ("assets/med/textures/default/cut.png", cut),
            ("assets/med/textures/default/huge.png", big),
            ("assets/med/textures/default/ok.png", png()),
            ("assets/med/models/default/cut.json", model),
            ("assets/med/textures/default/notpng.png", b"hello".to_vec()),
        ]);
        let removed = sanitize(&mut f, false);
        assert_eq!(removed.len(), 3, "{removed:?}");
        assert!(f.contains_key("assets/med/textures/default/ok.png"));
    }
}
