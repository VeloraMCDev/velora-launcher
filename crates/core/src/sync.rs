//! Keeps an instance's mods/configs in sync with what the admin published.
//!
//! Files the panel sends are "managed": they get updated and removed when
//! the admin changes the instance. Anything the player adds themselves is
//! never touched.

use crate::http::{self, Download};
use crate::progress::{self, Reporter, Stage};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use velora_shared::FileEntry;

mod safety;
mod transaction;

#[cfg(test)]
mod recovery_tests;

#[cfg(test)]
mod safety_tests;

#[cfg(test)]
mod performance_tests;

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

#[cfg(test)]
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

/// Sync `files` into `game_dir`.
///
/// Fast path: when the revision hasn't changed we only check that files
/// exist and match the declared nonzero size.
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
    sync_with_publication(client, panel_base, game_dir, revision, clean_epoch, files, concurrency, deep, reporter, |source, destination| {
        std::fs::rename(source, destination)
    })
    .await
}

#[allow(clippy::too_many_arguments)]
async fn sync_with_publication(
    client: &reqwest::Client,
    panel_base: &str,
    game_dir: &Path,
    revision: i64,
    clean_epoch: i64,
    files: &[FileEntry],
    concurrency: usize,
    deep: bool,
    reporter: &Reporter,
    publish_state: impl FnOnce(&Path, &Path) -> std::io::Result<()>,
) -> Result<SyncReport> {
    let wanted_paths: Vec<String> = files.iter().map(|entry| safety::normalize(&entry.path, false)).collect::<Result<_>>()?;
    for entry in files {
        if entry.sha1.len() != 40 || !entry.sha1.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            bail!("invalid manifest checksum for {}", entry.path);
        }
    }
    safety::unique(&wanted_paths)?;
    safety::inspect(game_dir, ".scopenet", true)?;
    for relative in &wanted_paths {
        safety::inspect(game_dir, relative, false)?;
    }
    let _ownership = transaction::claim(game_dir)?;
    transaction::recover(game_dir)?;
    safety::inspect(game_dir, ".scopenet/state.json", false)?;
    let state = match std::fs::read(state_path(game_dir)) {
        Ok(bytes) => serde_json::from_slice::<SyncState>(&bytes).context("invalid sync state; refusing mutation")?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => SyncState::default(),
        Err(error) => return Err(error).context("reading sync state"),
    };
    let old_paths: Vec<String> = state.managed.iter().map(|path| safety::normalize(path, false)).collect::<Result<_>>()?;
    safety::unique(&old_paths)?;
    let old_set: HashSet<_> = old_paths.iter().cloned().collect();
    let wanted: HashSet<_> = wanted_paths.iter().cloned().collect();
    let mut combined: Vec<_> = old_set.union(&wanted).cloned().collect();
    combined.sort();
    safety::unique(&combined)?;
    let cleaning = state.revision > 0 && clean_epoch > state.clean_epoch;
    let changed = state.revision != revision || deep || cleaning;
    let mut snapshots = Vec::new();
    for path in &combined {
        let destination = safety::inspect(game_dir, path, false)?;
        let snapshot = safety::fingerprint(&destination)?;
        if wanted.contains(path) && !old_set.contains(path) && snapshot.is_some() {
            bail!("unknown user file conflicts with manifest: {path}");
        }
        snapshots.push((destination, snapshot));
    }
    snapshots.push((state_path(game_dir), safety::fingerprint(&state_path(game_dir))?));
    let mut quarantine = Vec::new();
    let mut packs = None;
    if cleaning {
        let mods = safety::inspect(game_dir, "mods", true)?;
        if safety::metadata(&mods)?.is_some() {
            for entry in std::fs::read_dir(mods)? {
                let entry = entry?;
                let name = entry.file_name().into_string().map_err(|_| anyhow::anyhow!("non-UTF8 mod filename"))?;
                let relative = safety::normalize(&format!("mods/{name}"), false)?;
                let metadata = std::fs::symlink_metadata(entry.path())?;
                if metadata.file_type().is_symlink() {
                    bail!("symlink in clean-update mods: {relative}");
                }
                if !metadata.is_file() || old_set.contains(&relative) {
                    continue;
                }
                let backup = format!(".scopenet/removed/{clean_epoch}/mods/{name}");
                let destination = safety::inspect(game_dir, &backup, false)?;
                if safety::metadata(&destination)?.is_some() {
                    bail!("clean-update backup already exists: {backup}");
                }
                snapshots.push((entry.path(), safety::fingerprint(&entry.path())?));
                snapshots.push((destination.clone(), None));
                quarantine.push((entry.path(), destination));
            }
        }
        let cache = safety::inspect(game_dir, "server-resource-packs", true)?;
        snapshots.push((cache.clone(), safety::fingerprint(&cache)?));
        if safety::metadata(&cache)?.is_some() {
            if wanted_paths.iter().any(|path| path == "server-resource-packs" || path.starts_with("server-resource-packs/"))
                || old_paths.iter().any(|path| path.starts_with("server-resource-packs/"))
            {
                bail!("managed paths overlap clean-update resource-pack cache");
            }
            packs = Some(cache);
        }
    }
    if state.revision == revision
        && clean_epoch <= state.clean_epoch
        && old_set == wanted
        && safety::metadata(&state_path(game_dir))?.is_some()
        && files
            .iter()
            .zip(&wanted_paths)
            .all(|(entry, relative)| http::file_ok(&game_dir.join(relative), Some(&entry.sha1), Some(entry.size), deep))
    {
        progress::stage(reporter, Stage::Files, "Checking instance files");
        return Ok(SyncReport::default());
    }
    let workspace = safety::inspect(game_dir, ".scopenet/staged-sync", true)?;
    if safety::metadata(&workspace)?.is_some() {
        bail!("unfinished or concurrent sync at {}; manual review required", workspace.display());
    }
    std::fs::create_dir_all(workspace.parent().unwrap())?;
    std::fs::create_dir(&workspace).context("claiming sync staging directory")?;
    let mut publication = transaction::Publication::start(game_dir, &workspace)?;
    let mut preserve_workspace = false;
    let result = async {
        progress::stage(reporter, Stage::Files, if changed { "Staging instance files" } else { "Checking instance files" });
        std::fs::create_dir(workspace.join("downloads"))?;
        let mut downloads = Vec::new();
        let mut replacements = Vec::new();
        for (index, (entry, relative)) in files.iter().zip(&wanted_paths).enumerate() {
            let destination = game_dir.join(relative);
            if cleaning || !http::file_ok(&destination, Some(&entry.sha1), Some(entry.size), changed) {
                let staged = workspace.join("downloads").join(index.to_string());
                downloads.push(Download::new(
                    resolve_url(panel_base, &entry.url),
                    staged.clone(),
                    Some(entry.sha1.clone()),
                    Some(entry.size),
                ));
                replacements.push((staged, destination, entry));
            }
        }
        let downloaded = downloads.len();
        http::download_all(client, downloads, concurrency.max(1), Stage::Files, reporter).await?;
        for (staged, _, entry) in &replacements {
            safety::inspect(&workspace, &format!("downloads/{}", staged.file_name().unwrap().to_string_lossy()), false)?;
            if !http::file_ok(staged, Some(&entry.sha1), Some(entry.size), true) {
                bail!("staged verification failed: {}", entry.path);
            }
            std::fs::File::open(staged)?.sync_all()?;
        }
        let mut managed = wanted_paths.clone();
        managed.sort();
        let next = SyncState { revision, managed, clean_epoch: clean_epoch.max(state.clean_epoch) };
        let next_state = workspace.join("state.json");
        transaction::write_state(&next_state, &next)?;
        publication.prepare_state(&next_state)?;
        for (path, snapshot) in &snapshots {
            if safety::fingerprint(path)? != *snapshot {
                bail!("sync tree changed during staging: {}", path.display());
            }
        }
        for relative in &combined {
            safety::inspect(game_dir, relative, false)?;
        }
        safety::inspect(game_dir, ".scopenet/state.json", false)?;
        for (_, destination) in &quarantine {
            let relative = destination.strip_prefix(game_dir)?.to_str().context("invalid quarantine path")?;
            safety::inspect(game_dir, &relative.replace('\\', "/"), false)?;
        }
        let apply = (|| -> Result<SyncReport> {
            let mut report = SyncReport { downloaded, ..SyncReport::default() };
            for relative in &old_paths {
                let original = game_dir.join(relative);
                let replacing = replacements.iter().any(|(_, destination, _)| destination == &original);
                if (cleaning || !wanted.contains(relative) || replacing) && safety::metadata(&original)?.is_some() {
                    publication.move_path(&original, &workspace.join("rollback").join(relative))?;
                    if cleaning {
                        report.cleaned += 1;
                    } else if !wanted.contains(relative) {
                        report.removed += 1;
                    }
                }
            }
            for (source, destination) in &quarantine {
                publication.move_path(source, destination)?;
                report.cleaned += 1;
            }
            if let Some(cache) = &packs {
                publication.move_path(cache, &workspace.join("rollback/server-resource-packs"))?;
                report.cleaned += 1;
            }
            let replacement_paths =
                replacements.iter().map(|(staged, destination, _)| (staged.clone(), destination.clone())).collect::<Vec<_>>();
            publication.move_paths(&replacement_paths)?;
            publish_state(&next_state, &state_path(game_dir)).context("atomically publishing sync state")?;
            Ok(report)
        })();
        match apply {
            Ok(report) => match transaction::flush_directory(state_path(game_dir).parent().unwrap()) {
                Ok(()) => Ok(report),
                Err(error) => {
                    preserve_workspace = true;
                    Err(error).context("sync state published but directory flush failed; retain staging for recovery")
                }
            },
            Err(error) => match publication.rollback() {
                Ok(()) => Err(error).context("publication failed; previous tree restored"),
                Err(rollback) => {
                    preserve_workspace = true;
                    Err(error).context(format!("rollback failed: {rollback:#}; retain {} for manual recovery", workspace.display()))
                }
            },
        }
    }
    .await;
    if preserve_workspace {
        result
    } else {
        transaction::finish(&workspace, result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::regression_tests::{local_client, LocalHttp};

    pub(super) fn write_fixture(game: &Path, path: &str, body: &[u8]) {
        let destination = game.join(path);
        std::fs::create_dir_all(destination.parent().unwrap()).unwrap();
        std::fs::write(destination, body).unwrap();
    }

    pub(super) fn entry(path: &str, url: &str, body: &[u8]) -> FileEntry {
        FileEntry { path: path.into(), url: url.into(), sha1: http::sha1_bytes(body), size: body.len() as u64 }
    }

    pub(super) fn previous_state(game: &Path) -> Vec<u8> {
        write_fixture(game, "mods/managed.jar", b"previous");
        write_fixture(game, "config/dropped.toml", b"old config");
        write_fixture(game, "mods/player.jar", b"player mod");
        save_state(
            game,
            &SyncState { revision: 7, managed: vec!["mods/managed.jar".into(), "config/dropped.toml".into()], clean_epoch: 2 },
        )
        .unwrap();
        std::fs::read(state_path(game)).unwrap()
    }

    #[tokio::test]
    async fn failed_download_keeps_revision_state_dropped_files_and_unknown_files() {
        let directory = tempfile::tempdir().unwrap();
        let game = directory.path();
        let persisted = previous_state(game);
        let server = LocalHttp::serve(b"wrong", 5, false).await;
        let files = [entry("mods/managed.jar", &server.url, b"right")];
        assert!(sync(&local_client(), &server.url, game, 8, 2, &files, 1, false, &progress::noop()).await.is_err());
        assert_eq!(std::fs::read(state_path(game)).unwrap(), persisted);
        assert_eq!(load_state(game).revision, 7);
        assert_eq!(std::fs::read(game.join("mods/managed.jar")).unwrap(), b"previous");
        assert_eq!(std::fs::read(game.join("config/dropped.toml")).unwrap(), b"old config");
        assert_eq!(std::fs::read(game.join("mods/player.jar")).unwrap(), b"player mod");
    }

    #[tokio::test]
    async fn interrupted_sync_does_not_advance_persisted_revision() {
        let directory = tempfile::tempdir().unwrap();
        let game = directory.path();
        let persisted = previous_state(game);
        let server = LocalHttp::serve(b"partial", 100, true).await;
        let files = [entry("mods/managed.jar", &server.url, b"replacement")];
        let received = std::sync::Arc::new(tokio::sync::Notify::new());
        let signal = received.clone();
        let reporter: Reporter = std::sync::Arc::new(move |event| {
            if matches!(event, progress::Event::Progress { done, .. } if done > 0) {
                signal.notify_one();
            }
        });
        let game_path = game.to_path_buf();
        let base = server.url.clone();
        let task = tokio::spawn(async move { sync(&local_client(), &base, &game_path, 8, 2, &files, 1, false, &reporter).await });
        tokio::time::timeout(std::time::Duration::from_secs(3), received.notified()).await.unwrap();
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        assert_eq!(std::fs::read(state_path(game)).unwrap(), persisted);
        assert_eq!(std::fs::read(game.join("mods/managed.jar")).unwrap(), b"previous");
        assert_eq!(std::fs::read(game.join("config/dropped.toml")).unwrap(), b"old config");
        assert_eq!(std::fs::read(game.join("mods/player.jar")).unwrap(), b"player mod");
    }

    #[tokio::test]
    async fn successful_update_persists_sorted_ownership_and_is_idempotent() {
        let directory = tempfile::tempdir().unwrap();
        let game = directory.path();
        previous_state(game);
        write_fixture(game, "config/player.toml", b"player config");
        let server = LocalHttp::serve(b"updated", 7, false).await;
        let files = [entry("mods/managed.jar", &server.url, b"updated"), entry("config/new.toml", &server.url, b"updated")];
        let report = sync(&local_client(), &server.url, game, 8, 2, &files, 2, false, &progress::noop()).await.unwrap();
        assert_eq!(report.downloaded, 2);
        assert_eq!(report.removed, 1);
        assert_eq!(report.cleaned, 0);
        assert_eq!(std::fs::read(game.join("mods/managed.jar")).unwrap(), b"updated");
        assert_eq!(std::fs::read(game.join("config/new.toml")).unwrap(), b"updated");
        assert!(!game.join("config/dropped.toml").exists());
        assert_eq!(std::fs::read(game.join("mods/player.jar")).unwrap(), b"player mod");
        assert_eq!(std::fs::read(game.join("config/player.toml")).unwrap(), b"player config");
        let state: SyncState = serde_json::from_slice(&std::fs::read(state_path(game)).unwrap()).unwrap();
        assert_eq!(state.revision, 8);
        assert_eq!(state.clean_epoch, 2);
        assert_eq!(state.managed, ["config/new.toml", "mods/managed.jar"]);
        drop(server);
        let again = sync(&local_client(), "http://unused", game, 8, 2, &files, 2, false, &progress::noop()).await.unwrap();
        assert_eq!(again.downloaded, 0);
        assert_eq!(again.removed, 0);
    }

    #[tokio::test]
    async fn deep_verification_repairs_truncation_at_same_revision() {
        let directory = tempfile::tempdir().unwrap();
        let game = directory.path();
        previous_state(game);
        write_fixture(game, "mods/managed.jar", b"cut");
        let server = LocalHttp::serve(b"previous", 8, false).await;
        let files = [entry("mods/managed.jar", &server.url, b"previous")];
        let report = sync(&local_client(), &server.url, game, 7, 2, &files, 1, true, &progress::noop()).await.unwrap();
        assert_eq!(report.downloaded, 1);
        assert_eq!(std::fs::read(game.join("mods/managed.jar")).unwrap(), b"previous");
        assert_eq!(load_state(game).revision, 7);
    }

    #[tokio::test]
    async fn successful_clean_update_preserves_non_mod_user_files_and_records_epoch() {
        let directory = tempfile::tempdir().unwrap();
        let game = directory.path();
        previous_state(game);
        for path in ["config/player.toml", "saves/synthetic/level.dat", "screenshots/synthetic.png", "options.txt"] {
            write_fixture(game, path, b"user data");
        }
        write_fixture(game, "server-resource-packs/cached", b"cache");
        let server = LocalHttp::serve(b"updated", 7, false).await;
        let files = [entry("mods/managed.jar", &server.url, b"updated")];
        let report = sync(&local_client(), &server.url, game, 8, 3, &files, 1, false, &progress::noop()).await.unwrap();
        assert_eq!(report.cleaned, 4);
        assert_eq!(report.downloaded, 1);
        assert_eq!(std::fs::read(game.join(".scopenet/removed/3/mods/player.jar")).unwrap(), b"player mod");
        assert!(!game.join("server-resource-packs").exists());
        for path in ["config/player.toml", "saves/synthetic/level.dat", "screenshots/synthetic.png", "options.txt"] {
            assert_eq!(std::fs::read(game.join(path)).unwrap(), b"user data");
        }
        assert_eq!(load_state(game).clean_epoch, 3);
        drop(server);
        let again = sync(&local_client(), "http://unused", game, 8, 3, &files, 1, false, &progress::noop()).await.unwrap();
        assert_eq!(again.cleaned, 0);
        assert_eq!(again.downloaded, 0);
    }

    #[tokio::test]
    async fn fast_path_must_repair_truncated_file() {
        let directory = tempfile::tempdir().unwrap();
        let game = directory.path();
        previous_state(game);
        write_fixture(game, "mods/managed.jar", b"cut");
        let server = LocalHttp::serve(b"previous", 8, false).await;
        let files = [entry("mods/managed.jar", &server.url, b"previous")];
        sync(&local_client(), &server.url, game, 7, 2, &files, 1, false, &progress::noop()).await.unwrap();
        assert_eq!(std::fs::read(game.join("mods/managed.jar")).unwrap(), b"previous");
    }

    #[tokio::test]
    async fn failed_batch_must_restore_already_published_files() {
        let directory = tempfile::tempdir().unwrap();
        let game = directory.path();
        let persisted = previous_state(game);
        let server = LocalHttp::serve(b"updated", 7, false).await;
        let files = [entry("mods/managed.jar", &server.url, b"updated"), entry("mods/broken.jar", &server.url, b"expected")];
        assert!(sync(&local_client(), &server.url, game, 8, 2, &files, 1, false, &progress::noop()).await.is_err());
        assert_eq!(std::fs::read(state_path(game)).unwrap(), persisted);
        assert_eq!(std::fs::read(game.join("mods/managed.jar")).unwrap(), b"previous");
        assert!(!game.join("mods/broken.jar").exists());
    }

    #[tokio::test]
    async fn failed_clean_update_must_restore_previous_tree() {
        let directory = tempfile::tempdir().unwrap();
        let game = directory.path();
        let persisted = previous_state(game);
        write_fixture(game, "server-resource-packs/cached", b"cache");
        let server = LocalHttp::serve(b"wrong", 5, false).await;
        let files = [entry("mods/managed.jar", &server.url, b"right")];
        assert!(sync(&local_client(), &server.url, game, 8, 3, &files, 1, false, &progress::noop()).await.is_err());
        assert_eq!(std::fs::read(state_path(game)).unwrap(), persisted);
        assert_eq!(std::fs::read(game.join("mods/managed.jar")).unwrap(), b"previous");
        assert_eq!(std::fs::read(game.join("mods/player.jar")).unwrap(), b"player mod");
        assert_eq!(std::fs::read(game.join("server-resource-packs/cached")).unwrap(), b"cache");
    }

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
        for (path, body) in [
            ("mods/managed.jar", "m"),
            ("mods/player-added.jar", "p"),
            ("config/mod.toml", "c"),
            ("saves/world/level.dat", "w"),
            ("options.txt", "o"),
            ("server-resource-packs/abc", "x"),
        ] {
            std::fs::create_dir_all(game.join(path).parent().unwrap()).unwrap();
            std::fs::write(game.join(path), body).unwrap();
        }
        save_state(game, &SyncState { revision: 4, managed: vec!["mods/managed.jar".into(), "config/mod.toml".into()], clean_epoch: 0 })
            .unwrap();
        let client = crate::http::client();
        let report = sync(&client, "http://unused", game, 5, 1, &[], 4, false, &crate::progress::noop()).await.unwrap();
        assert_eq!(report.cleaned, 4, "two managed files, one stray jar, the pack cache");
        assert!(
            !game.join("mods/managed.jar").exists()
                && !game.join("config/mod.toml").exists()
                && !game.join("server-resource-packs").exists()
        );
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
        let report =
            sync(&crate::http::client(), "http://unused", dir.path(), 1, 3, &[], 4, false, &crate::progress::noop()).await.unwrap();
        assert_eq!(report.cleaned, 0);
        assert!(dir.path().join("mods/mine.jar").exists());
    }
}
