//! Rank title PNGs as in-game glyphs.
//!
//! Each title reward with an uploaded PNG gets a private-use character (U+F700 and up, a range Oraxen/ItemsAdder rarely touch).
//! The resource pack maps that character to the PNG through `assets/minecraft/font/default.json`, so a plain string containing it
//! — chat, tab list, scoreboard, any plugin's PlaceholderAPI output — shows the image. Slots are handed out once and never reused,
//! so a rank's character stays the same however the rewards are edited.

use crate::{error::AppResult, state::AppState, store};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub const FIRST: u32 = 0xF700;
const SLOTS: u32 = 0x200;
/// Height (in GUI pixels) a title is drawn at: one line of chat text plus a little.
const HEIGHT: u32 = 9;
const ASCENT: u32 = 8;

#[derive(Serialize, Deserialize, Default)]
struct Slots {
    map: BTreeMap<i64, u32>,
}

pub struct RankImage {
    pub reward_id: i64,
    pub glyph: char,
    pub bytes: Vec<u8>,
}

fn upload_name(url: &str) -> Option<&str> {
    let name = url.strip_prefix("/uploads/")?;
    (!name.is_empty() && name.len() <= 120 && name.chars().all(|c| c.is_ascii_alphanumeric() || "._-".contains(c)) && !name.starts_with('.')
        && name.to_ascii_lowercase().ends_with(".png"))
    .then_some(name)
}

async fn slot(state: &AppState, reward_id: i64) -> AppResult<Option<char>> {
    let mut slots: Slots = store::kv_get(state, "rank_glyph_slots").await?;
    if !slots.map.contains_key(&reward_id) {
        let next = slots.map.values().max().map_or(0, |m| m + 1);
        if next >= SLOTS {
            return Ok(None);
        }
        slots.map.insert(reward_id, next);
        store::kv_set(state, "rank_glyph_slots", &slots).await?;
    }
    Ok(char::from_u32(FIRST + slots.map[&reward_id]))
}

/// The glyph (as a one-character string) for the player's current global title, if that title has an uploaded PNG.
pub async fn active_global_glyph(state: &AppState, global_level: i64) -> AppResult<Option<String>> {
    let top: Option<(i64, Option<String>)> = sqlx::query_as(
        "SELECT id, json_extract(reward_data, '$.title_image') FROM level_rewards
         WHERE level_type='global' AND reward_type='title' AND level_req<=? ORDER BY level_req DESC, id DESC LIMIT 1",
    )
    .bind(global_level)
    .fetch_optional(&state.db)
    .await?;
    let Some((id, Some(image))) = top else { return Ok(None) };
    if upload_name(&image).is_none() {
        return Ok(None);
    }
    Ok(slot(state, id).await?.map(String::from))
}

/// Every title PNG that can go into the pack (valid upload, still on disk).
pub async fn images(state: &AppState) -> AppResult<Vec<RankImage>> {
    let rows: Vec<(i64, Option<String>)> =
        sqlx::query_as("SELECT id, json_extract(reward_data, '$.title_image') FROM level_rewards WHERE reward_type='title' ORDER BY id")
            .fetch_all(&state.db)
            .await?;
    let mut out = Vec::new();
    for (id, image) in rows {
        let Some(name) = image.as_deref().and_then(upload_name) else { continue };
        let Ok(bytes) = tokio::fs::read(state.cfg.uploads_dir().join(name)).await else { continue };
        if !bytes.starts_with(b"\x89PNG") || bytes.len() > 2 * 1024 * 1024 {
            continue;
        }
        if let Some(glyph) = slot(state, id).await? {
            out.push(RankImage { reward_id: id, glyph, bytes });
        }
    }
    Ok(out)
}

/// Adds the textures and font providers to a pack's file map, merging into any `font/default.json` already there.
pub fn add_to_pack(files: &mut BTreeMap<String, Vec<u8>>, images: Vec<RankImage>) -> Result<(), serde_json::Error> {
    if images.is_empty() {
        return Ok(());
    }
    let path = "assets/minecraft/font/default.json".to_string();
    let mut font: Value = match files.get(&path) {
        Some(bytes) => serde_json::from_slice(bytes).unwrap_or_else(|_| json!({})),
        None => json!({}),
    };
    if !font["providers"].is_array() {
        font["providers"] = json!([]);
    }
    for img in images {
        let file = format!("scopenet:rank/{}.png", img.reward_id);
        files.insert(format!("assets/scopenet/textures/rank/{}.png", img.reward_id), img.bytes);
        font["providers"].as_array_mut().unwrap().push(json!({
            "type": "bitmap", "file": file, "ascent": ASCENT, "height": HEIGHT, "chars": [img.glyph.to_string()],
        }));
    }
    files.insert(path, serde_json::to_vec(&font)?);
    Ok(())
}
