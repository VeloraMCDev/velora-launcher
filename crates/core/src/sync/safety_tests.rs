use super::tests::{entry, previous_state, write_fixture};
use super::*;
use crate::http::regression_tests::{local_client, LocalHttp};

#[tokio::test]
async fn invalid_checksums_and_unsafe_paths_fail_before_any_mutation() {
    let directory = tempfile::tempdir().unwrap();
    let game = directory.path();
    let persisted = previous_state(game);
    for checksum in ["", "not-a-checksum", "gggggggggggggggggggggggggggggggggggggggg"] {
        let mut file = entry("mods/managed.jar", "http://unused", b"updated");
        file.sha1 = checksum.into();
        assert!(sync(&local_client(), "http://unused", game, 8, 3, &[file], 1, false, &progress::noop()).await.is_err());
    }
    for path in
        ["../outside", "/rooted", "C:\\outside", ".scopenet/state.json", ".SCOPENET/other", "mods/../escape", "mods/CON.jar", "mods/a."]
    {
        let file = entry(path, "http://unused", b"updated");
        assert!(sync(&local_client(), "http://unused", game, 8, 3, &[file], 1, false, &progress::noop()).await.is_err());
    }
    for paths in [["mods/a.jar", "mods/a.jar"], ["mods/A.jar", "mods/a.jar"], ["config", "config/file"], ["mods\\a.jar", "mods/a.jar"]] {
        let files: Vec<_> = paths.into_iter().map(|path| entry(path, "http://unused", b"updated")).collect();
        assert!(sync(&local_client(), "http://unused", game, 8, 3, &files, 1, false, &progress::noop()).await.is_err());
    }
    assert_eq!(std::fs::read(state_path(game)).unwrap(), persisted);
    assert_eq!(std::fs::read(game.join("mods/managed.jar")).unwrap(), b"previous");
    assert_eq!(std::fs::read(game.join("mods/player.jar")).unwrap(), b"player mod");
    assert!(!game.join(".scopenet/staged-sync").exists());
}

#[tokio::test]
async fn corrupt_state_and_unknown_file_collision_are_preserved() {
    let directory = tempfile::tempdir().unwrap();
    let game = directory.path();
    let persisted = previous_state(game);
    let files = [entry("mods/player.jar", "http://unused", b"updated")];
    assert!(sync(&local_client(), "http://unused", game, 8, 3, &files, 1, false, &progress::noop()).await.is_err());
    assert_eq!(std::fs::read(state_path(game)).unwrap(), persisted);
    write_fixture(game, ".scopenet/state.json", b"synthetic corrupt JSON");
    assert!(sync(&local_client(), "http://unused", game, 8, 3, &[], 1, false, &progress::noop()).await.is_err());
    assert_eq!(std::fs::read(state_path(game)).unwrap(), b"synthetic corrupt JSON");
    assert_eq!(std::fs::read(game.join("mods/player.jar")).unwrap(), b"player mod");
    assert!(!game.join(".scopenet/staged-sync").exists());
}

#[cfg(unix)]
#[tokio::test]
async fn symlink_destination_does_not_write_outside_instance() {
    let directory = tempfile::tempdir().unwrap();
    let game = directory.path().join("game");
    let outside = directory.path().join("outside");
    std::fs::create_dir(&game).unwrap();
    std::fs::create_dir(&outside).unwrap();
    std::fs::write(outside.join("sentinel"), b"outside data").unwrap();
    std::os::unix::fs::symlink(&outside, game.join("mods")).unwrap();
    let files = [entry("mods/escape.jar", "http://unused", b"updated")];
    assert!(sync(&local_client(), "http://unused", &game, 1, 0, &files, 1, false, &progress::noop()).await.is_err());
    assert_eq!(std::fs::read(outside.join("sentinel")).unwrap(), b"outside data");
    assert!(!outside.join("escape.jar").exists());
    assert!(!game.join(".scopenet").exists());
}

#[tokio::test]
async fn final_state_publication_failure_rolls_back_clean_tree_and_quarantine() {
    let directory = tempfile::tempdir().unwrap();
    let game = directory.path();
    let persisted = previous_state(game);
    write_fixture(game, "server-resource-packs/cached", b"cache");
    let server = LocalHttp::serve(b"updated", 7, false).await;
    let files = [entry("mods/managed.jar", &server.url, b"updated"), entry("config/new.toml", &server.url, b"updated")];
    let error = sync_with_publication(&local_client(), &server.url, game, 8, 3, &files, 1, false, &progress::noop(), |_, _| {
        Err(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "synthetic state publication denied"))
    })
    .await
    .unwrap_err();
    assert!(format!("{error:#}").contains("previous tree restored"));
    assert_eq!(std::fs::read(state_path(game)).unwrap(), persisted);
    for (path, body) in [
        ("mods/managed.jar", b"previous".as_slice()),
        ("mods/player.jar", b"player mod"),
        ("config/dropped.toml", b"old config"),
        ("server-resource-packs/cached", b"cache"),
    ] {
        assert_eq!(std::fs::read(game.join(path)).unwrap(), body);
    }
    assert!(!game.join("config/new.toml").exists());
    assert!(!game.join(".scopenet/removed/3").exists());
    assert!(!game.join(".scopenet/staged-sync").exists());
}

#[tokio::test]
async fn occupied_rollback_target_retains_backup_and_blocks_another_sync() {
    let directory = tempfile::tempdir().unwrap();
    let game = directory.path();
    let persisted = previous_state(game);
    let server = LocalHttp::serve(b"updated", 7, false).await;
    let files = [entry("mods/managed.jar", &server.url, b"updated")];
    let error = sync_with_publication(&local_client(), &server.url, game, 8, 2, &files, 1, false, &progress::noop(), |_, _| {
        write_fixture(game, ".scopenet/staged-sync/downloads/0", b"foreign data");
        Err(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "synthetic state publication denied"))
    })
    .await
    .unwrap_err();
    assert!(format!("{error:#}").contains("rollback failed"));
    assert_eq!(std::fs::read(state_path(game)).unwrap(), persisted);
    assert_eq!(std::fs::read(game.join(".scopenet/staged-sync/rollback/mods/managed.jar")).unwrap(), b"previous");
    assert_eq!(std::fs::read(game.join(".scopenet/staged-sync/downloads/0")).unwrap(), b"foreign data");
    assert!(sync(&local_client(), &server.url, game, 8, 2, &files, 1, false, &progress::noop()).await.is_err());
}

#[tokio::test]
async fn changed_live_file_during_failed_commit_is_not_destroyed_by_rollback() {
    let directory = tempfile::tempdir().unwrap();
    let game = directory.path();
    previous_state(game);
    let server = LocalHttp::serve(b"updated", 7, false).await;
    let files = [entry("mods/managed.jar", &server.url, b"updated")];
    let error = sync_with_publication(&local_client(), &server.url, game, 8, 2, &files, 1, false, &progress::noop(), |_, _| {
        write_fixture(game, "mods/managed.jar", b"concurrent user data");
        Err(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "synthetic state publication denied"))
    })
    .await
    .unwrap_err();
    assert!(format!("{error:#}").contains("rollback source changed"));
    assert_eq!(std::fs::read(game.join("mods/managed.jar")).unwrap(), b"concurrent user data");
    assert_eq!(std::fs::read(game.join(".scopenet/staged-sync/rollback/mods/managed.jar")).unwrap(), b"previous");
}
