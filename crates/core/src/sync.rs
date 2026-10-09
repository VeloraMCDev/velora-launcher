//! Keeps an instance's mods/configs in sync with what the admin published.
//!
//! Files the panel sends are "managed": they get updated and removed when
//! the admin changes the instance. Anything the player adds themselves is
//! never touched.

use crate::http::{self, Download};
use crate::paths::safe_join;
use crate::progress::{self, Reporter, Stage};
use anyhow::Result;
use velora_shared::FileEntry;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SyncState {
    pub revision: i64,
    pub managed: Vec<String>,
    /// The admin's "clean update" number this instance was last cleaned for.
    #[serde(default)]
    pub clean_epoch: i64,
}

fn state_path(game_dir: &Path) -> PathBuf {
    game_dir.join(".scopenet").join("state.json")
}

pub fn load_state(game_dir: &Path) -> SyncState {
    std::fs::read(state_path(game_dir)).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

fn save_state(game_dir: &Path, state: &SyncState) -> Result<()> {
    let path = state_path(game_dir);
    std::fs::create_dir_all(path.parent().unwrap())?;
    std::fs::write(path, serde_json::to_vec_pretty(state)?)?;
    Ok(())
}

pub fn resolve_url(base: &str, url: &str) -> String {
    if url.starts_with("http://") || url.starts_with("https://") {
        url.to_string()
    } else {
        format!("{}/{}", base.trim_end_matches('/'), url.trim_start_matches('/'))
    }
}

#[derive(Debug, Default, Serialize, Clone)]
pub struct SyncReport {
    pub downloaded: usize,
    pub removed: usize,
    /// Files and folders cleared by a clean update (managed files, every mod jar, the server resource-pack cache).
    pub cleaned: usize,
}

/// A clean update: delete everything the panel ever put here, move any mod the player added to `.scopenet/removed/` (so a stray jar
/// cannot conflict with the new modpack, yet is never lost), and empty the server resource-pack cache. Worlds, screenshots, options
/// and the rest of the game folder stay untouched.
fn clean(game_dir: &Path, state: &SyncState, epoch: i64) -> usize {
    let mut cleaned = 0;
    for old in &state.managed {
        if let Some(path) = safe_join(game_dir, old) {
            if std::fs::remove_file(&path).is_ok() {
                cleaned += 1;
            }
        }
    }
    let mods = game_dir.join("mods");
    if let Ok(entries) = std::fs::read_dir(&mods) {
        let keep = game_dir.join(".scopenet").join("removed").join(epoch.to_string()).join("mods");
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            std::fs::create_dir_all(&keep).ok();
            if std::fs::rename(&path, keep.join(entry.file_name())).is_ok() {
                cleaned += 1;
            }
        }
    }
    let packs = game_dir.join("server-resource-packs");
    if packs.is_dir() && std::fs::remove_dir_all(&packs).is_ok() {
        cleaned += 1;
    }
    cleaned
}

/// Sync `files` into `game_dir`.
///
/// Fast path: when the revision hasn't changed we only check that files
/// exist, so launching an up-to-date instance costs a few `stat` calls.
#[allow(clippy::too_many_arguments)]
pub async fn sync(
    client: &reqwest::Client,
    panel_base: &str,
    game_dir: &Path,
    revision: i64,
    clean_epoch: i64,
    files: &[FileEntry],
    concurrency: usize,
    deep: bool,
    reporter: &Reporter,
) -> Result<SyncReport> {
    std::fs::create_dir_all(game_dir)?;
    let mut state = load_state(game_dir);
    // A first install has nothing to clean; every later one wipes once per "clean update" the admin presses.
    let cleaning = state.revision > 0 && clean_epoch > state.clean_epoch;
    let cleaned = if cleaning {
        progress::stage(reporter, Stage::Files, "Clearing old files");
        let n = clean(game_dir, &state, clean_epoch);
        state.managed.clear();
        n
    } else {
        0
    };
    let changed = state.revision != revision || deep || cleaning;
    progress::stage(reporter, Stage::Files, if changed { "Syncing instance files" } else { "Checking instance files" });

    let mut wanted = HashSet::new();
    let mut downloads = Vec::new();
    for f in files {
        let Some(dest) = safe_join(game_dir, &f.path) else {
            tracing::warn!("skipping unsafe path from panel: {}", f.path);
            continue;
        };
        wanted.insert(f.path.replace('\\', "/"));
        let ok = if changed { http::file_ok(&dest, Some(&f.sha1), Some(f.size), true) } else { dest.exists() };
        if !ok {
            downloads.push(Download::new(resolve_url(panel_base, &f.url), dest, Some(f.sha1.clone()), Some(f.size)));
        }
    }
    let downloaded = downloads.len();
    http::download_all(client, downloads, concurrency, Stage::Files, reporter).await?;

    let mut removed = 0;
    for old in &state.managed {
        if !wanted.contains(old) {
            if let Some(path) = safe_join(game_dir, old) {
                if std::fs::remove_file(&path).is_ok() {
                    removed += 1;
                }
            }
        }
    }

    let mut managed: Vec<String> = wanted.into_iter().collect();
    managed.sort();
    save_state(game_dir, &SyncState { revision, managed, clean_epoch: clean_epoch.max(state.clean_epoch) })?;
    Ok(SyncReport { downloaded, removed, cleaned })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls() {
        assert_eq!(resolve_url("https://panel.example/", "/files/a/mods/x.jar"), "https://panel.example/files/a/mods/x.jar");
        assert_eq!(resolve_url("https://panel.example", "https://cdn.modrinth.com/x.jar"), "https://cdn.modrinth.com/x.jar");
    }

    #[tokio::test]
    async fn removes_files_the_admin_dropped() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path();
        std::fs::create_dir_all(game.join("mods")).unwrap();
        std::fs::write(game.join("mods/old.jar"), b"old").unwrap();
        std::fs::write(game.join("mods/mine.jar"), b"player added").unwrap();
        save_state(game, &SyncState { revision: 1, managed: vec!["mods/old.jar".into()], clean_epoch: 0 }).unwrap();

        let client = crate::http::client();
        let report = sync(&client, "http://unused", game, 2, 0, &[], 4, false, &crate::progress::noop()).await.unwrap();
        assert_eq!(report.removed, 1);
        assert!(!game.join("mods/old.jar").exists());
        assert!(game.join("mods/mine.jar").exists(), "player files are never touched");
        assert_eq!(load_state(game).revision, 2);
    }

    #[tokio::test]
    async fn a_clean_update_clears_old_files_once_and_keeps_the_players_own_things() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path();
        for (path, body) in [("mods/managed.jar", "m"), ("mods/player-added.jar", "p"), ("config/mod.toml", "c"), ("saves/world/level.dat", "w"), ("options.txt", "o"), ("server-resource-packs/abc", "x")] {
            std::fs::create_dir_all(game.join(path).parent().unwrap()).unwrap();
            std::fs::write(game.join(path), body).unwrap();
        }
        save_state(game, &SyncState { revision: 4, managed: vec!["mods/managed.jar".into(), "config/mod.toml".into()], clean_epoch: 0 }).unwrap();
        let client = crate::http::client();
        let report = sync(&client, "http://unused", game, 5, 1, &[], 4, false, &crate::progress::noop()).await.unwrap();
        assert_eq!(report.cleaned, 4, "two managed files, one stray jar, the pack cache");
        assert!(!game.join("mods/managed.jar").exists() && !game.join("config/mod.toml").exists() && !game.join("server-resource-packs").exists());
        assert!(!game.join("mods/player-added.jar").exists());
        assert!(game.join(".scopenet/removed/1/mods/player-added.jar").exists(), "stray mods are moved aside, not destroyed");
        assert!(game.join("saves/world/level.dat").exists() && game.join("options.txt").exists());
        assert_eq!(load_state(game).clean_epoch, 1);
        // The same epoch again does nothing.
        std::fs::write(game.join("mods/new.jar"), b"n").unwrap();
        let again = sync(&client, "http://unused", game, 5, 1, &[], 4, false, &crate::progress::noop()).await.unwrap();
        assert_eq!(again.cleaned, 0);
        assert!(game.join("mods/new.jar").exists());
    }

    #[tokio::test]
    async fn a_first_install_has_nothing_to_clean() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("mods")).unwrap();
        std::fs::write(dir.path().join("mods/mine.jar"), b"x").unwrap();
        let report = sync(&crate::http::client(), "http://unused", dir.path(), 1, 3, &[], 4, false, &crate::progress::noop()).await.unwrap();
        assert_eq!(report.cleaned, 0);
        assert!(dir.path().join("mods/mine.jar").exists());
    }
}
