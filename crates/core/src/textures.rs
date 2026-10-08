//! Item and block textures, taken from the player's own Minecraft client jar.
//!
//! The launcher already downloads the client to run the game. Its jar holds every item and block texture, so the launcher copies
//! those out once into `<data>/minecraft/textures/` — nothing from Mojang is shipped with or redistributed by SCOPENET. The folder
//! lives outside the version folders and the launcher's own files, so it survives launcher updates; `ensure` only does work when
//! the folder is missing or was built from a different jar.

use crate::paths::Layout;
use anyhow::{Context, Result};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const KINDS: [&str; 2] = ["item", "block"];
const MARKER: &str = ".source";
/// Never extract a single file bigger than this (real textures are a few KB).
const MAX_FILE: u64 = 512 * 1024;

pub fn dir(layout: &Layout) -> PathBuf {
    layout.root.join("textures")
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Status {
    pub ready: bool,
    pub count: usize,
    /// The version the textures came from, e.g. `1.21.1`.
    pub version: Option<String>,
}

/// What the textures folder holds right now.
pub fn status(layout: &Layout) -> Status {
    let root = dir(layout);
    let marker = fs::read_to_string(root.join(MARKER)).ok();
    let count = KINDS.iter().map(|k| fs::read_dir(root.join(k)).map(|d| d.count()).unwrap_or(0)).sum();
    let version = marker.as_deref().and_then(|m| m.split(':').next()).map(str::to_string);
    Status { ready: count > 0 && marker.is_some(), count, version }
}

/// `1.21.1` or `1.20` style ids: the plain vanilla client of a release.
fn is_release(id: &str) -> bool {
    let mut parts = id.split('.');
    let ok = |p: Option<&str>| p.is_some_and(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()));
    ok(parts.next()) && ok(parts.next()) && parts.next().map_or(true, |p| ok(Some(p))) && parts.next().is_none()
}

/// The newest downloaded vanilla client jar, if any: (version id, path, size).
fn newest_client(layout: &Layout) -> Option<(String, PathBuf, u64)> {
    let mut found: Vec<(std::time::SystemTime, String, PathBuf, u64)> = Vec::new();
    for entry in fs::read_dir(layout.versions()).ok()?.flatten() {
        let id = entry.file_name().to_string_lossy().to_string();
        if !is_release(&id) {
            continue;
        }
        let jar = layout.version_jar(&id);
        if let Ok(meta) = fs::metadata(&jar) {
            if meta.is_file() && meta.len() > 1_000_000 {
                found.push((meta.modified().unwrap_or(std::time::UNIX_EPOCH), id, jar, meta.len()));
            }
        }
    }
    // Newest release wins; ties go to the most recently written jar.
    found.sort_by(|a, b| {
        let key = |id: &str| id.split('.').map(|p| p.parse::<u64>().unwrap_or(0)).collect::<Vec<_>>();
        key(&a.1).cmp(&key(&b.1)).then(a.0.cmp(&b.0))
    });
    found.pop().map(|(_, id, jar, size)| (id, jar, size))
}

/// Extract the textures if they are missing or come from another jar. Returns true when it did work.
pub fn ensure(layout: &Layout) -> Result<bool> {
    let Some((id, jar, size)) = newest_client(layout) else { return Ok(false) };
    let want = format!("{id}:{size}");
    let root = dir(layout);
    if fs::read_to_string(root.join(MARKER)).ok().as_deref() == Some(want.as_str()) && status(layout).ready {
        return Ok(false);
    }
    extract(&jar, &root).with_context(|| format!("reading textures from {}", jar.display()))?;
    fs::write(root.join(MARKER), want)?;
    Ok(true)
}

/// Copy `assets/minecraft/textures/{item,block}/<name>.png` out of the jar into `out/{item,block}/`.
pub fn extract(jar: &Path, out: &Path) -> Result<usize> {
    let mut zip = zip::ZipArchive::new(File::open(jar)?)?;
    // Build next to the final folder and swap in, so a crash can't leave half a set behind.
    let staging = out.with_extension("building");
    let _ = fs::remove_dir_all(&staging);
    for k in KINDS {
        fs::create_dir_all(staging.join(k))?;
    }
    let mut n = 0;
    for i in 0..zip.len() {
        let mut file = zip.by_index(i)?;
        let name = file.name().to_string();
        let Some(rest) = name.strip_prefix("assets/minecraft/textures/") else { continue };
        let Some((kind, leaf)) = rest.split_once('/') else { continue };
        if !KINDS.contains(&kind) || leaf.contains('/') || !leaf.ends_with(".png") || file.size() > MAX_FILE {
            continue;
        }
        if !leaf.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'.' | b'-')) {
            continue;
        }
        let mut bytes = Vec::with_capacity(file.size() as usize);
        file.read_to_end(&mut bytes)?;
        File::create(staging.join(kind).join(leaf))?.write_all(&bytes)?;
        n += 1;
    }
    if n == 0 {
        let _ = fs::remove_dir_all(&staging);
        anyhow::bail!("that jar has no item textures");
    }
    let _ = fs::remove_dir_all(out);
    fs::rename(&staging, out)?;
    Ok(n)
}

/// A texture as PNG bytes. Item ids are written like `minecraft:diamond_sword`, `DIAMOND_SWORD` or `diamond_sword`; a few names
/// differ between the item id and the texture file, and blocks without an item texture (stone, oak_log…) use their block texture.
pub fn find(layout: &Layout, id: &str) -> Option<Vec<u8>> {
    let name = id.rsplit(':').next()?.to_ascii_lowercase();
    if name.is_empty() || !name.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_') {
        return None;
    }
    let root = dir(layout);
    let candidates = [
        root.join("item").join(format!("{name}.png")),
        root.join("block").join(format!("{name}.png")),
        root.join("block").join(format!("{name}_front.png")),
        root.join("block").join(format!("{name}_side.png")),
        root.join("block").join(format!("{name}_top.png")),
        root.join("item").join(format!("{}.png", alias(&name))),
    ];
    candidates.iter().find_map(|p| fs::read(p).ok())
}

fn alias(name: &str) -> String {
    match name {
        "oak_log" => "oak_log".into(),
        "grass_block" => "grass_block_side".into(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    fn fake_jar(path: &Path, files: &[(&str, &[u8])]) {
        let mut zip = zip::ZipWriter::new(File::create(path).unwrap());
        let opts = zip::write::SimpleFileOptions::default();
        for (name, data) in files {
            zip.start_file(*name, opts).unwrap();
            zip.write_all(data).unwrap();
        }
        // Pad past the "real jar" size check.
        zip.start_file("padding.bin", zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored)).unwrap();
        zip.write_all(&vec![0u8; 1_100_000]).unwrap();
        zip.finish().unwrap();
    }

    fn setup(dir: &Path, version: &str, files: &[(&str, &[u8])]) -> Layout {
        let layout = Layout::new(dir.join("minecraft"));
        fs::create_dir_all(layout.version_dir(version)).unwrap();
        fake_jar(&layout.version_jar(version), files);
        layout
    }

    #[test]
    fn release_ids_only() {
        assert!(is_release("1.21.1") && is_release("1.20") && is_release("26.3"));
        assert!(!is_release("fabric-loader-0.16.9-1.21.1") && !is_release("1.21.1-forge-52.0.0") && !is_release("24w14a") && !is_release("1.."));
    }

    #[test]
    fn extracts_once_survives_reruns_and_updates_with_a_new_jar() {
        let t = tempfile::tempdir().unwrap();
        let files: &[(&str, &[u8])] = &[
            ("assets/minecraft/textures/item/diamond_sword.png", b"sword"),
            ("assets/minecraft/textures/block/stone.png", b"stone"),
            ("assets/minecraft/textures/block/oak_log_top.png", b"top"),
            ("assets/minecraft/textures/entity/zombie.png", b"nope"),
            ("assets/minecraft/textures/item/../../evil.png", b"evil"),
            ("assets/minecraft/textures/item/Bad Name.png", b"x"),
            ("assets/minecraft/lang/en_us.json", b"{}"),
        ];
        let layout = setup(t.path(), "1.21.1", files);
        assert!(!status(&layout).ready, "nothing before the first extraction");
        assert!(ensure(&layout).unwrap(), "first run extracts");
        let s = status(&layout);
        assert_eq!((s.ready, s.count, s.version.as_deref()), (true, 3, Some("1.21.1")));
        assert_eq!(find(&layout, "minecraft:diamond_sword").unwrap(), b"sword");
        assert_eq!(find(&layout, "DIAMOND_SWORD").unwrap(), b"sword");
        assert_eq!(find(&layout, "stone").unwrap(), b"stone");
        assert_eq!(find(&layout, "oak_log").unwrap(), b"top", "blocks fall back to their top texture");
        assert!(find(&layout, "zombie").is_none() && find(&layout, "../evil").is_none());
        assert!(!layout.root.join("evil.png").exists() && !dir(&layout).join("item").join("Bad Name.png").exists());

        // A launcher update or restart doesn't redo the work…
        assert!(!ensure(&layout).unwrap());
        // …and a deleted folder is rebuilt from the jar that is still there.
        fs::remove_dir_all(dir(&layout)).unwrap();
        assert!(ensure(&layout).unwrap());
        assert!(status(&layout).ready);

        // A newer game version replaces them.
        fs::create_dir_all(layout.version_dir("1.21.4")).unwrap();
        fake_jar(&layout.version_jar("1.21.4"), &[("assets/minecraft/textures/item/apple.png", b"apple")]);
        assert!(ensure(&layout).unwrap());
        assert_eq!(status(&layout).version.as_deref(), Some("1.21.4"));
        assert_eq!(find(&layout, "apple").unwrap(), b"apple");
        assert!(find(&layout, "diamond_sword").is_none());
    }

    #[test]
    fn no_client_yet_is_not_an_error() {
        let t = tempfile::tempdir().unwrap();
        let layout = Layout::new(t.path().join("minecraft"));
        assert!(!ensure(&layout).unwrap());
        assert!(!status(&layout).ready);
    }

    #[test]
    fn a_jar_without_textures_changes_nothing() {
        let t = tempfile::tempdir().unwrap();
        let layout = setup(t.path(), "1.21.1", &[("assets/minecraft/lang/en_us.json", b"{}")]);
        assert!(ensure(&layout).is_err());
        assert!(!status(&layout).ready);
    }
}
