//! Everything the 3D viewer in the admin panel needs to draw an item or model: parent models merged, texture slots resolved
//! and embedded as data URIs. Pure: files are read through a closure, so it works on an uploaded zip and on stored assets alike.

use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Map, Value};

const MAX_TEXTURE: usize = 192 * 1024;

fn split(raw: &str, default_ns: &str) -> (String, String) {
    let r = raw.trim().to_lowercase();
    match r.split_once(':') {
        Some((ns, p)) => (ns.to_string(), p.to_string()),
        None => (default_ns.to_string(), r),
    }
}

fn data_uri(bytes: Vec<u8>) -> Option<String> {
    (bytes.starts_with(b"\x89PNG") && bytes.len() <= MAX_TEXTURE).then(|| format!("data:image/png;base64,{}", STANDARD.encode(bytes)))
}

fn texture_uri(get: &dyn Fn(&str) -> Option<Vec<u8>>, reference: &str, own_ns: &str) -> Option<String> {
    let (ns, p) = split(reference, "minecraft");
    get(&format!("assets/{ns}/textures/{p}.png"))
        .or_else(|| get(&format!("assets/{own_ns}/textures/{p}.png")))
        .and_then(data_uri)
}

/// A drawable description of `texture` / `model`, or `None` when there is nothing to show.
///
/// * `{"mode":"elements","elements":[…],"textures":{"0":"data:…"}}` — a real model (cuboids with UVs).
/// * `{"mode":"flat","layers":["data:…"]}` — a generated item (a sprite with thickness).
pub fn view(get: &dyn Fn(&str) -> Option<Vec<u8>>, texture: Option<&str>, model: Option<&str>) -> Option<Value> {
    if let Some(model) = model {
        let (ns, rel) = split(model, "minecraft");
        let mut chain: Vec<Value> = Vec::new();
        let mut parent_name: Option<String> = None;
        let mut cur = Some((ns.clone(), rel));
        while let Some((n, r)) = cur.take() {
            if chain.len() >= 8 {
                break;
            }
            let Some(json) = get(&format!("assets/{n}/models/{r}.json")).and_then(|b| serde_json::from_slice::<Value>(&b).ok()) else {
                if chain.is_empty() {
                    return None;
                }
                parent_name = Some(format!("{n}:{r}"));
                break;
            };
            if let Some(p) = json.get("parent").and_then(Value::as_str) {
                let (pn, pr) = split(p, "minecraft");
                if pn == "minecraft" && (pr.starts_with("item/") || pr.starts_with("block/") || pr.starts_with("builtin/")) && get(&format!("assets/{pn}/models/{pr}.json")).is_none() {
                    parent_name = Some(format!("{pn}:{pr}"));
                } else {
                    cur = Some((pn, pr));
                }
            }
            chain.push(json);
        }
        // Textures: children override parents.
        let mut slots: Map<String, Value> = Map::new();
        for m in chain.iter().rev() {
            if let Some(t) = m.get("textures").and_then(Value::as_object) {
                for (k, v) in t {
                    slots.insert(k.clone(), v.clone());
                }
            }
        }
        let resolve_slot = |key: &str| -> Option<String> {
            let mut v = slots.get(key)?.as_str()?.to_string();
            for _ in 0..8 {
                if let Some(next) = v.strip_prefix('#') {
                    v = slots.get(next)?.as_str()?.to_string();
                } else {
                    break;
                }
            }
            (!v.starts_with('#')).then_some(v)
        };
        let mut textures = Map::new();
        for key in slots.keys() {
            if let Some(r) = resolve_slot(key) {
                textures.insert(key.clone(), texture_uri(get, &r, &ns).map_or(Value::Null, Value::String));
            }
        }
        let elements = chain.iter().find_map(|m| m.get("elements").and_then(Value::as_array).filter(|e| !e.is_empty())).cloned();
        if let Some(elements) = elements {
            return Some(json!({"mode": "elements", "elements": elements, "textures": textures}));
        }
        // Parentless-of-elements: vanilla shapes.
        let p = parent_name.unwrap_or_default();
        let tex = |k: &str| format!("#{k}");
        let cube = |faces: [(&str, &str); 6]| {
            let mut f = Map::new();
            for (face, slot) in faces {
                f.insert(face.into(), json!({"uv": [0, 0, 16, 16], "texture": tex(slot)}));
            }
            json!({"mode": "elements", "elements": [{"from": [0, 0, 0], "to": [16, 16, 16], "faces": Value::Object(f)}], "textures": textures})
        };
        if p.ends_with("block/cube_all") {
            return Some(cube([("north", "all"), ("east", "all"), ("south", "all"), ("west", "all"), ("up", "all"), ("down", "all")]));
        }
        if p.ends_with("block/cube_column") || p.ends_with("block/cube_column_horizontal") {
            return Some(cube([("north", "side"), ("east", "side"), ("south", "side"), ("west", "side"), ("up", "end"), ("down", "end")]));
        }
        if p.ends_with("block/cube_bottom_top") {
            return Some(cube([("north", "side"), ("east", "side"), ("south", "side"), ("west", "side"), ("up", "top"), ("down", "bottom")]));
        }
        if p.ends_with("block/cube") {
            return Some(cube([("north", "north"), ("east", "east"), ("south", "south"), ("west", "west"), ("up", "up"), ("down", "down")]));
        }
        let layers: Vec<Value> = (0..5).filter_map(|i| textures.get(&format!("layer{i}")).and_then(Value::as_str).map(|s| json!(s))).collect();
        if !layers.is_empty() {
            return Some(json!({"mode": "flat", "layers": layers}));
        }
        if let Some(first) = textures.values().find_map(Value::as_str) {
            return Some(json!({"mode": "flat", "layers": [first]}));
        }
        return None;
    }
    let uri = texture_uri(get, texture?, "minecraft")?;
    Some(json!({"mode": "flat", "layers": [uri]}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn files() -> BTreeMap<String, Vec<u8>> {
        [
            ("assets/me/textures/knight/skin.png", b"\x89PNGskin".to_vec()),
            ("assets/me/textures/flat.png", b"\x89PNGflat".to_vec()),
            ("assets/me/models/knight.json", br##"{"textures":{"0":"me:knight/skin","particle":"#0"},"elements":[{"from":[0,0,0],"to":[4,4,4],"faces":{"north":{"uv":[0,0,4,4],"texture":"#0"}}}]}"##.to_vec()),
            ("assets/me/models/child.json", br#"{"parent":"me:knight","textures":{"0":"me:flat"}}"#.to_vec()),
            ("assets/me/models/sprite.json", br#"{"parent":"item/generated","textures":{"layer0":"me:flat"}}"#.to_vec()),
            ("assets/me/models/crate.json", br#"{"parent":"minecraft:block/cube_all","textures":{"all":"me:flat"}}"#.to_vec()),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect()
    }

    #[test]
    fn views_cover_elements_inheritance_sprites_and_vanilla_cubes() {
        let f = files();
        let get = |p: &str| f.get(p).cloned();
        let v = view(&get, None, Some("me:knight")).unwrap();
        assert_eq!(v["mode"], "elements");
        assert!(v["textures"]["0"].as_str().unwrap().starts_with("data:image/png;base64,"));
        assert!(v["textures"]["particle"].is_string(), "# references resolve to the real texture");
        // A child keeps its parent's geometry but swaps textures.
        let c = view(&get, None, Some("me:child")).unwrap();
        assert_eq!(c["elements"], v["elements"]);
        assert_ne!(c["textures"]["0"], v["textures"]["0"]);
        // item/generated is a flat sprite.
        let s = view(&get, None, Some("me:sprite")).unwrap();
        assert_eq!((s["mode"].as_str(), s["layers"].as_array().unwrap().len()), (Some("flat"), 1));
        // block/cube_all becomes a cube with the texture on every side.
        let cube = view(&get, None, Some("me:crate")).unwrap();
        assert_eq!(cube["elements"][0]["faces"]["up"]["texture"], "#all");
        // A bare texture is a sprite; unknown models show nothing.
        assert_eq!(view(&get, Some("me:flat"), None).unwrap()["mode"], "flat");
        assert!(view(&get, None, Some("me:nope")).is_none());
        assert!(view(&get, Some("me:nope"), None).is_none());
    }
}
