//! Game assets (sounds, languages, textures) — content-addressed objects.

use crate::http::{self, Download};
use crate::paths::Layout;
use crate::version::AssetIndexRef;
use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub const RESOURCES: &str = "https://resources.download.minecraft.net";

#[derive(Debug, Deserialize)]
pub struct AssetIndex {
    pub objects: HashMap<String, AssetObject>,
    #[serde(default)]
    pub map_to_resources: bool,
    #[serde(default, rename = "virtual")]
    pub is_virtual: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AssetObject {
    pub hash: String,
    pub size: u64,
}

pub fn object_path(layout: &Layout, hash: &str) -> PathBuf {
    layout.assets().join("objects").join(&hash[..2]).join(hash)
}

/// Load (downloading if needed) the asset index.
pub async fn load_index(client: &reqwest::Client, layout: &Layout, index: &AssetIndexRef) -> Result<AssetIndex> {
    let path = layout.assets().join("indexes").join(format!("{}.json", index.id));
    if !http::file_ok(&path, index.sha1.as_deref(), index.size, true) {
        http::download_one(client, &Download::new(index.url.clone(), path.clone(), index.sha1.clone(), index.size), &|_| {}).await?;
    }
    Ok(serde_json::from_slice(&std::fs::read(&path)?)?)
}

/// Downloads needed for missing/corrupt objects.
pub fn missing_objects(layout: &Layout, index: &AssetIndex, deep: bool) -> Vec<Download> {
    let mut seen = std::collections::HashSet::new();
    index
        .objects
        .values()
        .filter(|o| o.hash.len() > 2 && seen.insert(o.hash.clone()))
        .filter_map(|o| {
            let path = object_path(layout, &o.hash);
            if http::file_ok(&path, Some(&o.hash), Some(o.size), deep) {
                None
            } else {
                Some(Download::new(format!("{RESOURCES}/{}/{}", &o.hash[..2], o.hash), path, Some(o.hash.clone()), Some(o.size)))
            }
        })
        .collect()
}

/// Pre-1.7 versions read assets by name, not hash. Copy them into the
/// virtual (or `resources/`) directory. Returns the directory to pass as
/// `game_assets`.
pub fn materialize_legacy(layout: &Layout, index_id: &str, index: &AssetIndex, game_dir: &Path) -> Result<Option<PathBuf>> {
    if !index.is_virtual && !index.map_to_resources {
        return Ok(None);
    }
    let target = if index.map_to_resources { game_dir.join("resources") } else { layout.assets().join("virtual").join(index_id) };
    for (name, obj) in &index.objects {
        let Some(dest) = crate::paths::safe_join(&target, name) else { continue };
        if dest.exists() {
            continue;
        }
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(object_path(layout, &obj.hash), &dest).ok();
    }
    Ok(Some(target))
}
