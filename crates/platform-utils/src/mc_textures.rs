//! Minecraft's own item, block, HUD and effect textures, taken from a vanilla client jar.
//!
//! Nothing from Mojang is shipped with or committed to Velora. The launcher reads the jar the player already downloaded; the panel
//! downloads the official client from Mojang's servers (or accepts an uploaded jar on hosts without internet) and copies the
//! textures into its own data folder. Both use this module so the folder layout, the safety checks and the lookup rules are the
//! same everywhere.
//!
//! Layout under the textures folder: `item/`, `block/`, `effect/`, `font/` (flat) and `gui/` (nested sprite tree),
//! plus a `.source` marker recording which jar the files came from.

use crate::http::{download_one, get_json, Download};
use crate::meta::{mojang_manifest, VersionManifest};
use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const MARKER: &str = ".source";
/// Bump when the set of extracted folders changes, so older folders are rebuilt from the same jar.
const REVISION: u32 = 2;
const PREFIX: &str = "assets/minecraft/textures/";

/// (path inside the jar, folder in the output, nested sprite tree, biggest single file)
const SOURCES: [(&str, &str, bool, u64); 5] = [
    ("item/", "item", false, 512 * 1024),
    ("block/", "block", false, 512 * 1024),
    ("mob_effect/", "effect", false, 128 * 1024),
    ("gui/sprites/", "gui", true, 256 * 1024),
    ("font/", "font", false, 512 * 1024),
];

/// Folders the item lookup searches, and the ones callers may ask for by name.
pub const KINDS: [&str; 5] = ["item", "block", "effect", "gui", "font"];

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Status {
    pub ready: bool,
    pub count: usize,
    /// The version the textures came from, e.g. `1.21.1`.
    pub version: Option<String>,
}

fn name_ok(segment: &str) -> bool {
    !segment.is_empty() && segment.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'.' | b'-')) && !segment.starts_with('.')
}

fn count_files(dir: &Path) -> usize {
    let Ok(entries) = fs::read_dir(dir) else { return 0 };
    entries.flatten().map(|e| if e.path().is_dir() { count_files(&e.path()) } else { 1 }).sum()
}

fn marker(root: &Path) -> Option<String> {
    fs::read_to_string(root.join(MARKER)).ok()
}

/// What the textures folder holds right now.
pub fn status(root: &Path) -> Status {
    let marker = marker(root);
    let count = KINDS.iter().map(|k| count_files(&root.join(k))).sum();
    let items = fs::read_dir(root.join("item")).map(|d| d.count()).unwrap_or(0);
    let version = marker.as_deref().and_then(|m| m.split(':').next()).filter(|v| *v != "unknown").map(str::to_string);
    Status { ready: items > 0 && marker.is_some(), count, version }
}

/// Where the current textures came from: `"mojang"`, `"upload"` or `None` when they were taken from a local game install.
pub fn origin(root: &Path) -> Option<String> {
    marker(root)?.split(':').nth(3).map(str::to_string)
}

/// Extract `jar` into `root` unless the folder already came from it. Returns true when it did work.
pub fn ensure(root: &Path, id: &str, jar: &Path, size: u64, origin: Option<&str>) -> Result<bool> {
    let want = format!("{id}:{size}:{REVISION}{}", origin.map(|o| format!(":{o}")).unwrap_or_default());
    if marker(root).as_deref() == Some(want.as_str()) && status(root).ready {
        return Ok(false);
    }
    extract(jar, root).with_context(|| format!("reading textures from {}", jar.display()))?;
    fs::write(root.join(MARKER), want)?;
    Ok(true)
}

/// The game version a client jar says it is (`version.json` at its root).
pub fn jar_version(jar: &Path) -> Option<String> {
    #[derive(Deserialize)]
    struct V {
        id: String,
    }
    let mut zip = zip::ZipArchive::new(File::open(jar).ok()?).ok()?;
    let mut file = zip.by_name("version.json").ok()?;
    if file.size() > 64 * 1024 {
        return None;
    }
    let mut text = String::new();
    file.read_to_string(&mut text).ok()?;
    let id = serde_json::from_str::<V>(&text).ok()?.id;
    // Version ids end up in a marker and folder names: keep them boring.
    (id.len() <= 40 && id.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_' | b'+'))).then_some(id)
}

/// Copy the texture folders out of the jar into `out`.
pub fn extract(jar: &Path, out: &Path) -> Result<usize> {
    let mut zip = zip::ZipArchive::new(File::open(jar)?).context("that file is not a jar/zip archive")?;
    // Build next to the final folder and swap in, so a crash can't leave half a set behind.
    let staging = out.with_extension("building");
    let _ = fs::remove_dir_all(&staging);
    fs::create_dir_all(&staging)?;
    let mut n = 0;
    let mut items = 0;
    for i in 0..zip.len() {
        let mut file = zip.by_index(i)?;
        let name = file.name().to_string();
        let Some(rest) = name.strip_prefix(PREFIX) else { continue };
        let Some((src, kind, nested, cap)) = SOURCES.iter().find(|(p, ..)| rest.starts_with(p)).map(|(p, k, nested, cap)| (*p, *k, *nested, *cap)) else { continue };
        let rel = &rest[src.len()..];
        if !rel.ends_with(".png") || file.size() > cap || file.is_dir() {
            continue;
        }
        let segments: Vec<&str> = rel.split('/').collect();
        if (!nested && segments.len() != 1) || segments.len() > 6 || !segments.iter().all(|s| name_ok(s)) {
            continue;
        }
        let mut bytes = Vec::with_capacity(file.size() as usize);
        file.read_to_end(&mut bytes)?;
        let dest = segments.iter().fold(staging.join(kind), |p, s| p.join(s));
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        File::create(dest)?.write_all(&bytes)?;
        n += 1;
        items += usize::from(kind == "item");
    }
    if items == 0 {
        let _ = fs::remove_dir_all(&staging);
        bail!("that jar has no item textures — use a vanilla Minecraft client jar");
    }
    let _ = fs::remove_dir_all(out);
    fs::rename(&staging, out)?;
    Ok(n)
}

/// Remove every extracted file.
pub fn clear(root: &Path) -> Result<()> {
    if root.exists() {
        fs::remove_dir_all(root)?;
    }
    Ok(())
}

/// A texture as PNG bytes. Item ids are written like `minecraft:diamond_sword`, `DIAMOND_SWORD` or `diamond_sword`; blocks without
/// an item texture (stone, oak_log…) use their block texture.
pub fn find_item(root: &Path, id: &str) -> Option<Vec<u8>> {
    let name = id.rsplit(':').next()?.to_ascii_lowercase();
    if name.is_empty() || !name.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_') {
        return None;
    }
    let block = |suffix: &str| root.join("block").join(format!("{name}{suffix}.png"));
    let candidates = [
        root.join("item").join(format!("{name}.png")),
        block(""),
        block("_front"),
        block("_side"),
        block("_top"),
        root.join("item").join(format!("{}.png", if name == "grass_block" { "grass_block_side".to_string() } else { name.clone() })),
    ];
    candidates.iter().find_map(|p| fs::read(p).ok())
}

/// The path of one extracted file (`kind` = `gui`, `font`…; `rel` = `hud/heart/full.png`), or `None` when the request tries anything
/// other than a plain relative path inside a known folder.
pub fn file(root: &Path, kind: &str, rel: &str) -> Option<PathBuf> {
    if !KINDS.contains(&kind) || !rel.ends_with(".png") {
        return None;
    }
    let segments: Vec<&str> = rel.split('/').collect();
    if segments.len() > 6 || !segments.iter().all(|s| name_ok(s)) {
        return None;
    }
    let path = segments.iter().fold(root.join(kind), |p, s| p.join(s));
    path.is_file().then_some(path)
}

/// Every item id with a texture, sorted — the panel's item pickers list these.
pub fn item_names(root: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(root.join("item"))
        .map(|d| d.flatten().filter_map(|e| e.file_name().to_string_lossy().strip_suffix(".png").map(str::to_string)).collect())
        .unwrap_or_default();
    names.sort();
    names
}

#[derive(Deserialize)]
struct VersionDoc {
    downloads: Option<DocDownloads>,
}
#[derive(Deserialize)]
struct DocDownloads {
    client: Option<DocFile>,
}
#[derive(Deserialize)]
struct DocFile {
    url: String,
    sha1: String,
    size: u64,
}

/// Download the official vanilla client for `version` (the newest release when `None`) into `dest`, checking Mojang's SHA-1.
/// Returns the version id and the file size.
pub async fn fetch_client_jar(client: &reqwest::Client, version: Option<&str>, dest: &Path) -> Result<(String, u64)> {
    let manifest: VersionManifest = mojang_manifest(client).await?;
    let id = version.map(str::to_string).unwrap_or(manifest.latest.release);
    let entry = manifest.versions.iter().find(|v| v.id == id).with_context(|| format!("Mojang has no version {id}"))?;
    let doc: VersionDoc = get_json(client, &entry.url).await?;
    let file = doc.downloads.and_then(|d| d.client).with_context(|| format!("{id} has no client download"))?;
    if !file.url.starts_with("https://") {
        bail!("refusing a non-https client URL");
    }
    download_one(client, &Download::new(file.url, dest.to_path_buf(), Some(file.sha1), Some(file.size)), &|_| {}).await?;
    Ok((id, file.size))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_jar(path: &Path, files: &[(&str, &[u8])]) {
        let mut zip = zip::ZipWriter::new(File::create(path).unwrap());
        let opts = zip::write::SimpleFileOptions::default();
        for (name, data) in files {
            zip.start_file(*name, opts).unwrap();
            zip.write_all(data).unwrap();
        }
        zip.finish().unwrap();
    }

    #[test]
    fn extracts_known_folders_and_rejects_anything_unsafe() {
        let t = tempfile::tempdir().unwrap();
        let jar = t.path().join("c.jar");
        fake_jar(
            &jar,
            &[
                ("version.json", br#"{"id":"1.21.1"}"#),
                ("assets/minecraft/textures/item/diamond_sword.png", b"sword"),
                ("assets/minecraft/textures/block/oak_log_top.png", b"top"),
                ("assets/minecraft/textures/mob_effect/speed.png", b"speed"),
                ("assets/minecraft/textures/gui/sprites/hud/heart/full.png", b"heart"),
                ("assets/minecraft/textures/gui/sprites/hud/../../evil.png", b"evil"),
                ("assets/minecraft/textures/item/Bad Name.png", b"x"),
                ("assets/minecraft/textures/item/sub/nested.png", b"x"),
                ("assets/minecraft/textures/entity/zombie.png", b"nope"),
                ("assets/minecraft/lang/en_us.json", b"{}"),
            ],
        );
        assert_eq!(jar_version(&jar).as_deref(), Some("1.21.1"));
        let root = t.path().join("tex");
        assert!(ensure(&root, "1.21.1", &jar, 10, Some("upload")).unwrap());
        assert!(!ensure(&root, "1.21.1", &jar, 10, Some("upload")).unwrap(), "same jar twice does no work");
        let s = status(&root);
        assert_eq!((s.ready, s.count, s.version.as_deref()), (true, 4, Some("1.21.1")));
        assert_eq!(origin(&root).as_deref(), Some("upload"));
        assert_eq!(find_item(&root, "minecraft:diamond_sword").unwrap(), b"sword");
        assert_eq!(find_item(&root, "OAK_LOG").unwrap(), b"top");
        assert_eq!(fs::read(file(&root, "gui", "hud/heart/full.png").unwrap()).unwrap(), b"heart");
        assert!(file(&root, "gui", "../item/diamond_sword.png").is_none());
        assert!(file(&root, "gui", "hud//full.png").is_none());
        assert!(file(&root, "secrets", "x.png").is_none());
        assert!(!root.join("evil.png").exists() && !root.join("item/Bad Name.png").exists() && !root.join("item/sub").exists());
        assert_eq!(item_names(&root), vec!["diamond_sword"]);
        clear(&root).unwrap();
        assert!(!status(&root).ready);
    }

    #[test]
    fn a_jar_without_item_textures_changes_nothing() {
        let t = tempfile::tempdir().unwrap();
        let jar = t.path().join("c.jar");
        fake_jar(&jar, &[("assets/minecraft/lang/en_us.json", b"{}")]);
        let root = t.path().join("tex");
        assert!(ensure(&root, "x", &jar, 1, None).is_err());
        assert!(!status(&root).ready && !root.exists());
        fs::write(t.path().join("junk"), b"not a zip").unwrap();
        assert!(extract(&t.path().join("junk"), &root).is_err());
    }
}
