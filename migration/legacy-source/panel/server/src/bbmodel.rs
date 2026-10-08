//! Blockbench `.bbmodel` (ModelEngine blueprints) to a single vanilla item model in its rest pose.
//!
//! ModelEngine animates bones on the server; here every cube of every bone is merged into one static model, which a display
//! entity can show. Textures are embedded in the file as data URIs and come back as PNG files. Anything vanilla models cannot
//! express (meshes, multi-axis rotations, rotations other than 22.5° steps) is approximated or skipped and reported in `warnings`.

use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Map, Value};

pub struct Converted {
    /// Vanilla model JSON referring to `textures` by `{namespace}:{folder}/{name}`.
    pub model: Vec<u8>,
    /// (file stem, PNG bytes): to be stored at `assets/{namespace}/textures/{folder}/{stem}.png`.
    pub textures: Vec<(String, Vec<u8>)>,
    /// Multiply the rendered size by this to restore the model's real size (models bigger than vanilla's 3x3x3 block limit are shrunk).
    pub scale_hint: f64,
    /// Height of the model in blocks after `scale_hint`, handy for hitboxes.
    pub height_blocks: f64,
    pub width_blocks: f64,
    pub warnings: Vec<String>,
}

fn nums(v: &Value, n: usize) -> Option<Vec<f64>> {
    let a = v.as_array()?;
    (a.len() >= n).then(|| a.iter().take(n).map(|x| x.as_f64().unwrap_or(0.0)).collect())
}

fn stem(raw: &str) -> String {
    let base = raw.rsplit(['/', '\\']).next().unwrap_or(raw);
    let base = base.strip_suffix(".png").unwrap_or(base);
    let s: String = base.to_lowercase().chars().map(|c| if c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' { c } else { '_' }).collect();
    let s = s.trim_matches('_').to_string();
    if s.is_empty() { "texture".into() } else { s }
}

/// Convert a model. `namespace`/`folder` decide where its textures will live (`assets/<namespace>/textures/<folder>/`).
pub fn convert(bytes: &[u8], namespace: &str, folder: &str) -> Result<Converted, String> {
    let root: Value = serde_json::from_slice(bytes).map_err(|_| "Not a valid .bbmodel file".to_string())?;
    let elements = root.get("elements").and_then(Value::as_array).ok_or("This .bbmodel has no elements")?;
    let mut warnings = Vec::new();

    // Textures: embedded data URIs.
    let res_w = root["resolution"]["width"].as_f64().unwrap_or(16.0).max(1.0);
    let res_h = root["resolution"]["height"].as_f64().unwrap_or(16.0).max(1.0);
    let mut textures: Vec<(String, Vec<u8>)> = Vec::new();
    let mut sizes: Vec<(f64, f64)> = Vec::new();
    for (i, t) in root.get("textures").and_then(Value::as_array).into_iter().flatten().enumerate() {
        let src = t["source"].as_str().unwrap_or("");
        let data = src.strip_prefix("data:image/png;base64,").and_then(|b| STANDARD.decode(b.trim()).ok());
        let Some(png) = data.filter(|d| d.starts_with(b"\x89PNG") && d.len() <= 2 * 1024 * 1024) else {
            warnings.push(format!("texture {} is not an embedded PNG under 2 MiB", t["name"].as_str().unwrap_or("?")));
            textures.push((format!("missing_{i}"), Vec::new()));
            sizes.push((res_w, res_h));
            continue;
        };
        let mut name = stem(t["name"].as_str().unwrap_or(&format!("texture_{i}")));
        while textures.iter().any(|(n, _)| *n == name) {
            name.push('_');
        }
        textures.push((name, png));
        sizes.push((t["uv_width"].as_f64().filter(|v| *v > 0.0).unwrap_or(res_w), t["uv_height"].as_f64().filter(|v| *v > 0.0).unwrap_or(res_h)));
    }

    // Bounding box over all visible cubes, to centre the model and fit vanilla's coordinate limits.
    let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
    let mut cubes = Vec::new();
    let mut skipped = 0;
    let mut approximated = 0;
    for e in elements {
        if e["type"].as_str().is_some_and(|t| t != "cube") || e["export"] == json!(false) || e["visibility"] == json!(false) {
            skipped += 1;
            continue;
        }
        let (Some(from), Some(to)) = (nums(&e["from"], 3), nums(&e["to"], 3)) else { skipped += 1; continue };
        let inflate = e["inflate"].as_f64().unwrap_or(0.0);
        let (from, to): (Vec<f64>, Vec<f64>) = (from.iter().map(|v| v - inflate).collect(), to.iter().map(|v| v + inflate).collect());
        for k in 0..3 {
            lo[k] = lo[k].min(from[k].min(to[k]));
            hi[k] = hi[k].max(from[k].max(to[k]));
        }
        cubes.push((e, from, to));
    }
    if cubes.is_empty() {
        return Err("This model has no cubes (only meshes or hidden parts)".into());
    }
    if skipped > 0 {
        warnings.push(format!("{skipped} parts (meshes or hidden elements) were skipped"));
    }

    // Centre on the block (x/z → 8) and sit on the floor (y → 0); shrink when bigger than vanilla can hold (-16..32 per axis).
    let (cx, cz) = ((lo[0] + hi[0]) / 2.0, (lo[2] + hi[2]) / 2.0);
    let (shift_x, shift_y, shift_z) = (8.0 - cx, -lo[1], 8.0 - cz);
    let half_x = (hi[0] - lo[0]) / 2.0;
    let half_z = (hi[2] - lo[2]) / 2.0;
    let height = hi[1] - lo[1];
    let need = half_x.max(half_z).max(0.0);
    let f = [if need > 24.0 { 24.0 / need } else { 1.0 }, if height > 48.0 { 48.0 / height } else { 1.0 }].into_iter().fold(1.0_f64, f64::min);
    // The model keeps its aspect: positions scale about (8, 0, 8).
    let place = |p: &[f64]| -> [f64; 3] { [8.0 + (p[0] + shift_x - 8.0) * f, (p[1] + shift_y) * f, 8.0 + (p[2] + shift_z - 8.0) * f] };

    let faces_names = ["north", "east", "south", "west", "up", "down"];
    let mut out_elements = Vec::new();
    for (e, from, to) in &cubes {
        let (a, b) = (place(from), place(to));
        let (min, max) = ([a[0].min(b[0]), a[1].min(b[1]), a[2].min(b[2])], [a[0].max(b[0]), a[1].max(b[1]), a[2].max(b[2])]);
        let mut el = Map::new();
        el.insert("from".into(), json!([round(min[0]), round(min[1]), round(min[2])]));
        el.insert("to".into(), json!([round(max[0]), round(max[1]), round(max[2])]));

        if let Some(r) = nums(&e["rotation"], 3).filter(|r| r.iter().any(|v| v.abs() > 0.001)) {
            let axis_name = ["x", "y", "z"][(0..3).max_by(|a, b| r[*a].abs().partial_cmp(&r[*b].abs()).unwrap()).unwrap()];
            let origin = nums(&e["origin"], 3).unwrap_or_else(|| vec![0.0; 3]);
            let o = place(&origin);
            let axis = (0..3).max_by(|a, b| r[*a].abs().partial_cmp(&r[*b].abs()).unwrap()).unwrap();
            if r.iter().filter(|v| v.abs() > 0.001).count() > 1 {
                approximated += 1;
            }
            let snapped = (r[axis] / 22.5).round() * 22.5;
            if (snapped - r[axis]).abs() > 0.01 || snapped.abs() > 45.0 {
                approximated += 1;
            }
            let angle = snapped.clamp(-45.0, 45.0);
            if angle.abs() > 0.01 {
                el.insert("rotation".into(), json!({"origin": [round(o[0]), round(o[1]), round(o[2])], "axis": axis_name, "angle": angle}));
            }
        }

        let box_uv = e["box_uv"].as_bool().unwrap_or(false);
        let size = [to[0] - from[0], to[1] - from[1], to[2] - from[2]];
        let off = nums(&e["uv_offset"], 2).unwrap_or_else(|| vec![0.0, 0.0]);
        let mut faces = Map::new();
        for name in faces_names {
            let face = &e["faces"][name];
            let tex = face["texture"].as_u64().map(|t| t as usize).or_else(|| face["texture"].as_str().and_then(|s| s.parse().ok()));
            let Some(tex) = tex.filter(|t| *t < textures.len() && !textures[*t].1.is_empty()) else { continue };
            let uv = if box_uv {
                let (w, h, d) = (size[0].abs().floor(), size[1].abs().floor(), size[2].abs().floor());
                let (u, v) = (off[0], off[1]);
                match name {
                    "up" => vec![u + d, v, u + d + w, v + d],
                    "down" => vec![u + d + w, v, u + d + 2.0 * w, v + d],
                    "east" => vec![u, v + d, u + d, v + d + h],
                    "north" => vec![u + d, v + d, u + d + w, v + d + h],
                    "west" => vec![u + d + w, v + d, u + 2.0 * d + w, v + d + h],
                    _ => vec![u + 2.0 * d + w, v + d, u + 2.0 * d + 2.0 * w, v + d + h],
                }
            } else {
                match nums(&face["uv"], 4) {
                    Some(uv) => uv,
                    None => continue,
                }
            };
            let (tw, th) = sizes[tex];
            let mut f_obj = Map::new();
            f_obj.insert("uv".into(), json!([round(uv[0] * 16.0 / tw), round(uv[1] * 16.0 / th), round(uv[2] * 16.0 / tw), round(uv[3] * 16.0 / th)]));
            f_obj.insert("texture".into(), json!(format!("#{tex}")));
            if let Some(rot) = face["rotation"].as_i64().filter(|r| [90, 180, 270].contains(&r.rem_euclid(360))) {
                f_obj.insert("rotation".into(), json!(rot.rem_euclid(360)));
            }
            faces.insert(name.into(), Value::Object(f_obj));
        }
        if faces.is_empty() {
            continue;
        }
        el.insert("faces".into(), Value::Object(faces));
        out_elements.push(Value::Object(el));
    }
    if out_elements.is_empty() {
        return Err("None of this model's faces use an embedded texture".into());
    }
    if approximated > 0 {
        warnings.push(format!("{approximated} rotations were approximated to what vanilla models allow (one axis, 22.5° steps)"));
    }
    if f < 1.0 {
        warnings.push(format!("the model is larger than vanilla allows, so it is shown at {:.0}% and scaled back up in game", f * 100.0));
    }

    let mut tex_map = Map::new();
    for (i, (name, bytes)) in textures.iter().enumerate() {
        if !bytes.is_empty() {
            tex_map.insert(i.to_string(), json!(format!("{namespace}:{folder}/{name}")));
        }
    }
    if let Some((k, v)) = tex_map.iter().next() {
        let v = v.clone();
        let _ = k;
        tex_map.insert("particle".into(), v);
    }
    let model = json!({
        "ambientocclusion": false,
        "textures": Value::Object(tex_map),
        "elements": out_elements,
        "display": {"gui": {"rotation": [30, 225, 0], "scale": [0.6, 0.6, 0.6]}}
    });
    Ok(Converted {
        model: serde_json::to_vec(&model).map_err(|e| e.to_string())?,
        textures: textures.into_iter().filter(|(_, b)| !b.is_empty()).collect(),
        scale_hint: if f < 1.0 { 1.0 / f } else { 1.0 },
        height_blocks: height * f / 16.0 * if f < 1.0 { 1.0 / f } else { 1.0 },
        width_blocks: half_x.max(half_z) * 2.0 / 16.0,
        warnings,
    })
}

fn round(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    const PNG_B64: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";

    fn model(extra: &str) -> Vec<u8> {
        format!(
            r##"{{"resolution":{{"width":16,"height":16}},
            "textures":[{{"name":"Body Tex.png","source":"data:image/png;base64,{PNG_B64}"}}],
            "elements":[
              {{"type":"cube","from":[-4,0,-4],"to":[4,16,4],"origin":[0,0,0],"rotation":[0,45,0],"faces":{{"north":{{"uv":[0,0,8,16],"texture":0}},"up":{{"uv":[0,0,8,8],"texture":0}}}}}},
              {{"type":"cube","from":[-2,16,-2],"to":[2,20,2],"box_uv":true,"uv_offset":[0,0],"faces":{{"north":{{"texture":0}},"south":{{"texture":0}}}}}},
              {{"type":"mesh","name":"fancy"}}
              {extra}
            ]}}"##
        )
        .into_bytes()
    }

    #[test]
    fn a_blueprint_becomes_a_centred_vanilla_model_with_its_textures() {
        let c = convert(&model(""), "modelengine", "knight").unwrap();
        let m: Value = serde_json::from_slice(&c.model).unwrap();
        assert_eq!(c.textures.len(), 1);
        assert_eq!(c.textures[0].0, "body_tex");
        assert_eq!(m["textures"]["0"], "modelengine:knight/body_tex");
        let els = m["elements"].as_array().unwrap();
        assert_eq!(els.len(), 2, "the mesh is skipped");
        // 8 wide, centred on x=8, standing on y=0.
        assert_eq!((els[0]["from"][0].as_f64(), els[0]["to"][0].as_f64(), els[0]["from"][1].as_f64()), (Some(4.0), Some(12.0), Some(0.0)));
        assert_eq!(els[0]["rotation"]["axis"], "y");
        assert_eq!(els[0]["rotation"]["angle"], 45.0);
        // uv in 0..16 of the texture
        assert_eq!(els[0]["faces"]["north"]["uv"], json!([0.0, 0.0, 8.0, 16.0]));
        assert_eq!(c.scale_hint, 1.0);
        assert!(c.warnings.iter().any(|w| w.contains("skipped")));
        // box uv: north face of a 4x4x4 cube sits after the d=4 left column
        assert_eq!(els[1]["faces"]["north"]["uv"], json!([4.0, 4.0, 8.0, 8.0]));
    }

    #[test]
    fn huge_models_are_shrunk_and_the_hint_restores_their_size() {
        let big = br#"{"textures":[{"name":"t","source":"data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg=="}],
            "elements":[{"type":"cube","from":[-100,0,-100],"to":[100,200,100],"faces":{"north":{"uv":[0,0,16,16],"texture":0}}}]}"#;
        let c = convert(big, "modelengine", "boss").unwrap();
        let m: Value = serde_json::from_slice(&c.model).unwrap();
        let (lo, hi) = (m["elements"][0]["from"][0].as_f64().unwrap(), m["elements"][0]["to"][0].as_f64().unwrap());
        assert!(lo >= -16.0 && hi <= 32.0, "{lo} {hi}");
        assert!(c.scale_hint > 4.0);
        assert!((c.height_blocks - 200.0 / 16.0).abs() < 0.5, "{}", c.height_blocks);
    }

    #[test]
    fn rejects_models_without_geometry_or_textures() {
        assert!(convert(b"nope", "m", "x").is_err());
        assert!(convert(br#"{"elements":[]}"#, "m", "x").is_err());
        assert!(convert(br#"{"elements":[{"type":"cube","from":[0,0,0],"to":[1,1,1],"faces":{"north":{"uv":[0,0,1,1],"texture":0}}}]}"#, "m", "x").is_err());
    }
}
