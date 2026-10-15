use anyhow::{bail, Context, Result};
use std::collections::HashSet;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

pub(super) fn normalize(path: &str, internal: bool) -> Result<String> {
    let normalized = path.replace('\\', "/");
    for component in normalized.split('/') {
        let folded = component.to_lowercase();
        let stem = folded.split('.').next().unwrap_or_default();
        let device = matches!(stem, "con" | "prn" | "aux" | "nul")
            || (stem.len() == 4 && (stem.starts_with("com") || stem.starts_with("lpt")) && matches!(stem.as_bytes()[3], b'1'..=b'9'));
        if component.is_empty()
            || component == "."
            || component == ".."
            || component.ends_with(['.', ' '])
            || component.chars().any(|character| character.is_control() || ":*?\"<>|".contains(character))
            || device
        {
            bail!("unsafe sync path: {path}");
        }
    }
    if !internal && normalized.split('/').next().unwrap().eq_ignore_ascii_case(".scopenet") {
        bail!("reserved sync path: {path}");
    }
    Ok(normalized)
}

pub(super) fn unique(paths: &[String]) -> Result<()> {
    let mut seen = HashSet::new();
    for path in paths {
        if !seen.insert(path.to_lowercase()) {
            bail!("duplicate or case-aliased sync path: {path}");
        }
    }
    for path in paths {
        let folded = path.to_lowercase();
        for (position, _) in folded.match_indices('/') {
            if seen.contains(&folded[..position]) {
                bail!("overlapping sync paths: {path}");
            }
        }
    }
    Ok(())
}

pub(super) fn metadata(path: &Path) -> Result<Option<std::fs::Metadata>> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) => Ok(Some(metadata)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("inspecting {}", path.display())),
    }
}

pub(super) fn inspect(game: &Path, relative: &str, directory: bool) -> Result<PathBuf> {
    let mut path = game.to_path_buf();
    if let Some(metadata) = metadata(game)? {
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            bail!("unsafe game root: {}", game.display());
        }
    }
    let components: Vec<_> = relative.split('/').collect();
    for (index, component) in components.iter().enumerate() {
        path.push(component);
        if let Some(metadata) = metadata(&path)? {
            if metadata.file_type().is_symlink() {
                bail!("symlink in sync path: {}", path.display());
            }
            let final_component = index + 1 == components.len();
            if (!final_component || directory) && !metadata.is_dir() {
                bail!("expected directory: {}", path.display());
            }
            if final_component && !directory && !metadata.is_file() {
                bail!("expected regular file: {}", path.display());
            }
        }
    }
    Ok(path)
}

pub(super) fn fingerprint(path: &Path) -> Result<Option<Vec<(PathBuf, Option<String>)>>> {
    let Some(metadata) = metadata(path)? else { return Ok(None) };
    if metadata.file_type().is_symlink() {
        bail!("symlink in sync snapshot: {}", path.display());
    }
    if metadata.is_file() {
        return Ok(Some(vec![(PathBuf::new(), Some(crate::http::sha1_file(path)?))]));
    }
    if !metadata.is_dir() {
        bail!("unsupported sync entry: {}", path.display());
    }
    let mut snapshot = vec![(PathBuf::new(), None)];
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let contents = fingerprint(&entry.path())?.context("entry disappeared during sync snapshot")?;
        for (relative, checksum) in contents {
            snapshot.push((PathBuf::from(entry.file_name()).join(relative), checksum));
        }
    }
    snapshot.sort();
    Ok(Some(snapshot))
}
