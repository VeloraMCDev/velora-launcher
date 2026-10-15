use super::{safety, transaction, SyncState};
use std::fs;
use std::path::Path;
use std::process::Command;

fn prepare(game: &Path) -> Vec<u8> {
    fs::create_dir_all(game.join("mods")).unwrap();
    fs::create_dir_all(game.join("saves/synthetic")).unwrap();
    fs::create_dir_all(game.join(".scopenet")).unwrap();
    fs::write(game.join("mods/managed.jar"), b"previous").unwrap();
    fs::write(game.join("saves/synthetic/level.dat"), b"player-owned").unwrap();
    let bytes = serde_json::to_vec_pretty(&SyncState { revision: 7, managed: vec!["mods/managed.jar".into()], clean_epoch: 2 }).unwrap();
    fs::write(super::state_path(game), &bytes).unwrap();
    bytes
}

fn interrupt(game: &Path, point: &str) {
    let status = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "sync::recovery_tests::interrupted_process_fixture", "--nocapture"])
        .env("VELORA_SYNTHETIC_SYNC_RECOVERY_ROOT", game)
        .env("VELORA_SYNTHETIC_SYNC_RECOVERY_POINT", point)
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(77));
}

fn batch_fault_hook(event: &str, source: &Path, destination: &Path) -> anyhow::Result<()> {
    let point = std::env::var("VELORA_SYNTHETIC_SYNC_RECOVERY_POINT")?;
    if event == "before_plan_persist" && point == "batch-persist-failure" {
        fs::write(source.parent().unwrap().parent().unwrap().join("journal.tmp"), b"interrupted synthetic journal write")?;
    }
    if event == "before_move" && source.file_name().is_some_and(|name| name == "1") {
        match point.as_str() {
            "batch-execute-source" => fs::write(source, b"foreign staged bytes")?,
            "batch-execute-occupied" => fs::write(destination, b"foreign destination bytes")?,
            "batch-execute-missing" => fs::remove_file(source)?,
            _ => {}
        }
    }
    if source.file_name().is_some_and(|name| name == "2")
        && ((event == "after_reverse" && point == "batch-reverse-before-persist")
            || (event == "after_reverse_persist" && point == "batch-reverse-after-persist"))
    {
        std::process::exit(77);
    }
    Ok(())
}

#[test]
fn interrupted_process_fixture() {
    let Some(root) = std::env::var_os("VELORA_SYNTHETIC_SYNC_RECOVERY_ROOT") else { return };
    let game = std::path::PathBuf::from(root);
    let point = std::env::var("VELORA_SYNTHETIC_SYNC_RECOVERY_POINT").unwrap();
    if point == "ownership" {
        assert!(transaction::claim(&game).is_err());
        std::process::exit(77);
    }
    let _ownership = transaction::claim(&game).unwrap();
    if point.starts_with("quarantine") {
        fs::write(game.join("mods/player.txt"), b"player-mod-folder-file").unwrap();
    }
    if point.starts_with("partial-directory") {
        fs::create_dir_all(game.join("server-resource-packs/nested")).unwrap();
        fs::write(game.join("server-resource-packs/first"), b"first cache file").unwrap();
        fs::write(game.join("server-resource-packs/nested/second"), b"second cache file").unwrap();
    }
    let workspace = game.join(".scopenet/staged-sync");
    fs::create_dir(&workspace).unwrap();
    let mut publication = transaction::Publication::start(&game, &workspace).unwrap();
    fs::create_dir(workspace.join("downloads")).unwrap();
    fs::write(workspace.join("downloads/0"), b"replacement").unwrap();
    fs::File::open(workspace.join("downloads/0")).unwrap().sync_all().unwrap();
    let state = SyncState {
        revision: 8,
        managed: if point.starts_with("batch-") {
            vec!["mods/managed.jar".into(), "new/nested/second.jar".into(), "new/nested/third.jar".into()]
        } else {
            vec!["mods/managed.jar".into()]
        },
        clean_epoch: if point.starts_with("quarantine") { 3 } else { 2 },
    };
    transaction::write_state(&workspace.join("state.json"), &state).unwrap();
    publication.prepare_state(&workspace.join("state.json")).unwrap();
    if point == "staging" {
        std::process::exit(77);
    }
    publication.move_path(&game.join("mods/managed.jar"), &workspace.join("rollback/mods/managed.jar")).unwrap();
    if point == "pending" {
        fs::rename(workspace.join("rollback/mods/managed.jar"), game.join("mods/managed.jar")).unwrap();
        std::process::exit(77);
    }
    if point == "original" {
        std::process::exit(77);
    }
    if point.starts_with("batch-") {
        fs::write(workspace.join("downloads/1"), b"second replacement").unwrap();
        fs::write(workspace.join("downloads/2"), b"third replacement").unwrap();
        for index in 1..=2 {
            fs::File::open(workspace.join("downloads").join(index.to_string())).unwrap().sync_all().unwrap();
        }
        let paths = vec![
            (workspace.join("downloads/0"), game.join("mods/managed.jar")),
            (workspace.join("downloads/1"), game.join("new/nested/second.jar")),
            (workspace.join("downloads/2"), game.join("new/nested/third.jar")),
        ];
        if point.starts_with("batch-execute-") || point.starts_with("batch-reverse-") || point == "batch-persist-failure" {
            publication.hook = Some(batch_fault_hook);
            let result = publication.move_paths(&paths);
            if point.starts_with("batch-reverse-") {
                result.unwrap();
                publication.rollback().unwrap();
                panic!("rollback interruption hook did not exit");
            }
            let error = result.unwrap_err();
            if point == "batch-persist-failure" {
                assert_eq!(error.downcast_ref::<std::io::Error>().unwrap().kind(), std::io::ErrorKind::AlreadyExists);
                for ((source, destination), payload) in
                    paths.iter().zip([b"replacement".as_slice(), b"second replacement", b"third replacement"])
                {
                    assert_eq!(fs::read(source).unwrap(), payload);
                    assert!(!destination.exists());
                }
                let journal: serde_json::Value = serde_json::from_slice(&fs::read(workspace.join("journal.json")).unwrap()).unwrap();
                assert_eq!(journal["moves"].as_array().unwrap().len(), 1);
                let directories = journal["directories"].as_array().unwrap();
                for relative in ["new", "new/nested"] {
                    assert!(directories.iter().any(|value| value.as_str() == Some(relative)));
                }
                assert_eq!(fs::read(workspace.join("rollback/mods/managed.jar")).unwrap(), b"previous");
            } else {
                assert_eq!(error.to_string(), "planned publication paths changed; refusing mutation");
                assert_eq!(fs::read(&paths[0].1).unwrap(), b"replacement");
                assert!(paths[2].0.exists());
                assert!(!paths[2].1.exists());
                if point != "batch-execute-occupied" {
                    assert!(!paths[1].1.exists());
                }
            }
            std::process::exit(77);
        }
        publication.plan_moves(&paths).unwrap();
        let applied = match point.as_str() {
            "batch-pending" => 0,
            "batch-partial" | "batch-rollback" | "batch-tampered" | "batch-occupied" => 1,
            "batch-two" => 2,
            _ => paths.len(),
        };
        for (source, destination) in paths.iter().take(applied) {
            fs::rename(source, destination).unwrap();
            transaction::flush_directory(source.parent().unwrap()).unwrap();
            transaction::flush_directory(destination.parent().unwrap()).unwrap();
        }
        if point == "batch-rollback" {
            fs::rename(&paths[0].1, &paths[0].0).unwrap();
        }
        if point == "batch-tampered" {
            fs::write(&paths[2].0, b"foreign staged bytes").unwrap();
        }
        if point == "batch-occupied" {
            fs::write(&paths[2].1, b"foreign destination bytes").unwrap();
        }
        if point == "batch-committed" {
            fs::rename(workspace.join("state.json"), super::state_path(&game)).unwrap();
            transaction::flush_directory(&game.join(".scopenet")).unwrap();
        }
        std::process::exit(77);
    }
    if point.starts_with("quarantine") {
        publication.move_path(&game.join("mods/player.txt"), &game.join(".scopenet/removed/3/mods/player.txt")).unwrap();
        if point == "quarantine" {
            std::process::exit(77);
        }
    }
    publication.move_path(&workspace.join("downloads/0"), &game.join("mods/managed.jar")).unwrap();
    if point.starts_with("partial-directory") {
        publication.move_path(&game.join("server-resource-packs"), &workspace.join("rollback/server-resource-packs")).unwrap();
    }
    if point == "replacement" {
        std::process::exit(77);
    }
    if point == "rolled-back" {
        publication.rollback().unwrap();
        fs::remove_file(workspace.join("state.json")).unwrap();
        std::process::exit(77);
    }
    fs::rename(workspace.join("state.json"), super::state_path(&game)).unwrap();
    transaction::flush_directory(&game.join(".scopenet")).unwrap();
    if point == "partial-cleanup" {
        publication.authorize_cleanup().unwrap();
        fs::remove_dir_all(workspace.join("rollback")).unwrap();
    }
    if point.starts_with("partial-directory") {
        publication.authorize_cleanup().unwrap();
        fs::remove_file(workspace.join("rollback/server-resource-packs/first")).unwrap();
        if point == "partial-directory-tampered" {
            fs::write(workspace.join("rollback/server-resource-packs/nested/second"), b"foreign backup file").unwrap();
        }
    }
    std::process::exit(77);
}

#[test]
fn process_exit_before_commit_restores_original_bytes_and_preserves_player_data() {
    for point in [
        "staging",
        "pending",
        "original",
        "replacement",
        "rolled-back",
        "batch-pending",
        "batch-partial",
        "batch-applied",
        "batch-rollback",
        "batch-two",
        "batch-reverse-before-persist",
        "batch-reverse-after-persist",
        "batch-persist-failure",
    ] {
        let directory = tempfile::tempdir().unwrap();
        let original = prepare(directory.path());
        interrupt(directory.path(), point);
        if point.starts_with("batch-reverse-") {
            let workspace = directory.path().join(".scopenet/staged-sync");
            let journal: serde_json::Value = serde_json::from_slice(&fs::read(workspace.join("journal.json")).unwrap()).unwrap();
            let expected = if point == "batch-reverse-before-persist" { 4 } else { 3 };
            assert_eq!(journal["moves"].as_array().unwrap().len(), expected);
            assert_eq!(fs::read(workspace.join("downloads/2")).unwrap(), b"third replacement");
            assert!(!directory.path().join("new/nested/third.jar").exists());
            assert_eq!(fs::read(directory.path().join("new/nested/second.jar")).unwrap(), b"second replacement");
            assert_eq!(fs::read(directory.path().join("mods/managed.jar")).unwrap(), b"replacement");
        }
        let _ownership = transaction::claim(directory.path()).unwrap();
        transaction::recover(directory.path()).unwrap();
        assert_eq!(fs::read(super::state_path(directory.path())).unwrap(), original, "{point}");
        assert_eq!(fs::read(directory.path().join("mods/managed.jar")).unwrap(), b"previous", "{point}");
        assert_eq!(fs::read(directory.path().join("saves/synthetic/level.dat")).unwrap(), b"player-owned");
        assert!(!directory.path().join(".scopenet/staged-sync").exists());
        assert!(!directory.path().join("new").exists());
        transaction::recover(directory.path()).unwrap();
    }
}

#[test]
fn committed_batch_preserves_all_published_files() {
    let directory = tempfile::tempdir().unwrap();
    prepare(directory.path());
    interrupt(directory.path(), "batch-committed");
    let _ownership = transaction::claim(directory.path()).unwrap();
    transaction::recover(directory.path()).unwrap();
    assert_eq!(fs::read(directory.path().join("mods/managed.jar")).unwrap(), b"replacement");
    assert_eq!(fs::read(directory.path().join("new/nested/second.jar")).unwrap(), b"second replacement");
    assert_eq!(fs::read(directory.path().join("new/nested/third.jar")).unwrap(), b"third replacement");
    assert_eq!(super::load_state(directory.path()).revision, 8);
    assert_eq!(super::load_state(directory.path()).managed, ["mods/managed.jar", "new/nested/second.jar", "new/nested/third.jar"]);
    assert!(!directory.path().join(".scopenet/staged-sync").exists());
}

#[test]
fn changed_pending_batch_evidence_is_retained_for_review() {
    for point in ["batch-tampered", "batch-occupied"] {
        let directory = tempfile::tempdir().unwrap();
        let original = prepare(directory.path());
        interrupt(directory.path(), point);
        let _ownership = transaction::claim(directory.path()).unwrap();
        assert!(transaction::recover(directory.path()).is_err());
        assert_eq!(fs::read(super::state_path(directory.path())).unwrap(), original);
        assert_eq!(fs::read(directory.path().join("mods/managed.jar")).unwrap(), b"replacement");
        assert!(directory.path().join(".scopenet/staged-sync/rollback/mods/managed.jar").exists());
        let foreign = if point == "batch-tampered" { ".scopenet/staged-sync/downloads/2" } else { "new/nested/third.jar" };
        assert!(fs::read(directory.path().join(foreign)).unwrap().starts_with(b"foreign"));
    }
}

#[test]
fn execution_guard_rejects_changed_source_occupied_destination_and_missing_evidence() {
    for point in ["batch-execute-source", "batch-execute-occupied", "batch-execute-missing"] {
        let directory = tempfile::tempdir().unwrap();
        let original = prepare(directory.path());
        interrupt(directory.path(), point);
        let _ownership = transaction::claim(directory.path()).unwrap();
        assert!(transaction::recover(directory.path()).is_err());
        let workspace = directory.path().join(".scopenet/staged-sync");
        let preserved = safety::fingerprint(&workspace).unwrap();
        assert!(transaction::recover(directory.path()).is_err());
        assert_eq!(safety::fingerprint(&workspace).unwrap(), preserved);
        assert_eq!(fs::read(super::state_path(directory.path())).unwrap(), original);
        assert_eq!(fs::read(directory.path().join("mods/managed.jar")).unwrap(), b"replacement");
        assert_eq!(fs::read(directory.path().join(".scopenet/staged-sync/rollback/mods/managed.jar")).unwrap(), b"previous");
        assert_eq!(fs::read(directory.path().join("saves/synthetic/level.dat")).unwrap(), b"player-owned");
        match point {
            "batch-execute-source" => {
                assert_eq!(fs::read(directory.path().join(".scopenet/staged-sync/downloads/1")).unwrap(), b"foreign staged bytes")
            }
            "batch-execute-occupied" => {
                assert_eq!(fs::read(directory.path().join("new/nested/second.jar")).unwrap(), b"foreign destination bytes")
            }
            _ => {
                assert!(!directory.path().join(".scopenet/staged-sync/downloads/1").exists());
                assert!(!directory.path().join("new/nested/second.jar").exists());
            }
        }
    }
}

#[test]
fn overlapping_batch_is_rejected_before_recording_moves_or_creating_parents() {
    let directory = tempfile::tempdir().unwrap();
    prepare(directory.path());
    let workspace = directory.path().join(".scopenet/staged-sync");
    fs::create_dir(&workspace).unwrap();
    let mut publication = transaction::Publication::start(directory.path(), &workspace).unwrap();
    fs::create_dir(workspace.join("downloads")).unwrap();
    fs::write(workspace.join("downloads/0"), b"replacement").unwrap();
    fs::write(workspace.join("downloads/1"), b"second replacement").unwrap();
    let journal = fs::read(workspace.join("journal.json")).unwrap();
    for destinations in [["new/first", "new/first"], ["new/first", "new/first/nested"], ["new/First", "new/first"]] {
        let paths = destinations
            .iter()
            .enumerate()
            .map(|(index, relative)| (workspace.join("downloads").join(index.to_string()), directory.path().join(relative)))
            .collect::<Vec<_>>();
        assert!(publication.plan_moves(&paths).is_err());
        assert_eq!(fs::read(workspace.join("journal.json")).unwrap(), journal);
        assert!(!directory.path().join("new").exists());
    }
    let source = workspace.join("downloads/0");
    let duplicate_sources = vec![(source.clone(), directory.path().join("new/first")), (source, directory.path().join("new/second"))];
    assert!(publication.plan_moves(&duplicate_sources).is_err());
    assert_eq!(fs::read(workspace.join("journal.json")).unwrap(), journal);
    assert!(!directory.path().join("new").exists());
    publication.rollback().unwrap();
    transaction::recover(directory.path()).unwrap();
}

#[test]
fn process_exit_after_commit_preserves_new_state_and_finishes_partial_cleanup() {
    for point in ["committed", "partial-cleanup"] {
        let directory = tempfile::tempdir().unwrap();
        prepare(directory.path());
        interrupt(directory.path(), point);
        let _ownership = transaction::claim(directory.path()).unwrap();
        transaction::recover(directory.path()).unwrap();
        assert_eq!(super::load_state(directory.path()).revision, 8);
        assert_eq!(fs::read(directory.path().join("mods/managed.jar")).unwrap(), b"replacement");
        assert_eq!(fs::read(directory.path().join("saves/synthetic/level.dat")).unwrap(), b"player-owned");
        assert!(!directory.path().join(".scopenet/staged-sync").exists());
    }
}

#[test]
fn live_ownership_is_exclusive_across_processes_and_released_on_drop() {
    let directory = tempfile::tempdir().unwrap();
    prepare(directory.path());
    let ownership = transaction::claim(directory.path()).unwrap();
    assert!(transaction::claim(directory.path()).is_err());
    interrupt(directory.path(), "ownership");
    drop(ownership);
    assert!(transaction::claim(directory.path()).is_ok());
}

#[test]
fn modified_published_file_blocks_recovery_without_deleting_backup() {
    let directory = tempfile::tempdir().unwrap();
    prepare(directory.path());
    interrupt(directory.path(), "replacement");
    fs::write(directory.path().join("mods/managed.jar"), b"external-change").unwrap();
    let _ownership = transaction::claim(directory.path()).unwrap();
    assert!(transaction::recover(directory.path()).is_err());
    assert_eq!(fs::read(directory.path().join("mods/managed.jar")).unwrap(), b"external-change");
    assert_eq!(fs::read(directory.path().join(".scopenet/staged-sync/rollback/mods/managed.jar")).unwrap(), b"previous");
}

#[test]
fn changed_committed_file_retains_backup_for_manual_review() {
    let directory = tempfile::tempdir().unwrap();
    prepare(directory.path());
    interrupt(directory.path(), "committed");
    fs::write(directory.path().join("mods/managed.jar"), b"external-change").unwrap();
    let _ownership = transaction::claim(directory.path()).unwrap();
    assert!(transaction::recover(directory.path()).is_err());
    assert_eq!(fs::read(directory.path().join(".scopenet/staged-sync/rollback/mods/managed.jar")).unwrap(), b"previous");
}

#[test]
fn pending_move_with_two_occupied_paths_retains_both_copies() {
    let directory = tempfile::tempdir().unwrap();
    prepare(directory.path());
    interrupt(directory.path(), "pending");
    let backup = directory.path().join(".scopenet/staged-sync/rollback/mods/managed.jar");
    fs::write(&backup, b"foreign-backup").unwrap();
    let _ownership = transaction::claim(directory.path()).unwrap();
    assert!(transaction::recover(directory.path()).is_err());
    assert_eq!(fs::read(directory.path().join("mods/managed.jar")).unwrap(), b"previous");
    assert_eq!(fs::read(backup).unwrap(), b"foreign-backup");
}

#[test]
fn interrupted_clean_update_recovers_non_jar_quarantine_and_preserves_committed_backup() {
    for point in ["quarantine", "quarantine-committed"] {
        let directory = tempfile::tempdir().unwrap();
        prepare(directory.path());
        interrupt(directory.path(), point);
        let _ownership = transaction::claim(directory.path()).unwrap();
        transaction::recover(directory.path()).unwrap();
        let retained = if point == "quarantine" { "mods/player.txt" } else { ".scopenet/removed/3/mods/player.txt" };
        assert_eq!(fs::read(directory.path().join(retained)).unwrap(), b"player-mod-folder-file");
        assert_eq!(super::load_state(directory.path()).revision, if point == "quarantine" { 7 } else { 8 });
        assert!(!directory.path().join(".scopenet/staged-sync").exists());
    }
}

#[test]
fn modified_original_or_staged_state_preserves_recovery_evidence() {
    for relative in [".scopenet/state.json", ".scopenet/staged-sync/state.json"] {
        let directory = tempfile::tempdir().unwrap();
        prepare(directory.path());
        interrupt(directory.path(), "replacement");
        fs::write(directory.path().join(relative), b"externally changed state").unwrap();
        let _ownership = transaction::claim(directory.path()).unwrap();
        assert!(transaction::recover(directory.path()).is_err());
        assert_eq!(fs::read(directory.path().join(relative)).unwrap(), b"externally changed state");
        assert_eq!(fs::read(directory.path().join(".scopenet/staged-sync/rollback/mods/managed.jar")).unwrap(), b"previous");
    }
}

#[test]
fn interrupted_journal_replacement_discards_only_pending_temporary_metadata() {
    let directory = tempfile::tempdir().unwrap();
    let original = prepare(directory.path());
    interrupt(directory.path(), "replacement");
    fs::write(directory.path().join(".scopenet/staged-sync/journal.tmp"), b"partial metadata").unwrap();
    let _ownership = transaction::claim(directory.path()).unwrap();
    transaction::recover(directory.path()).unwrap();
    assert_eq!(fs::read(super::state_path(directory.path())).unwrap(), original);
    assert_eq!(fs::read(directory.path().join("mods/managed.jar")).unwrap(), b"previous");
    assert!(!directory.path().join(".scopenet/staged-sync").exists());
}

#[test]
fn interrupted_recursive_cleanup_accepts_only_unchanged_remaining_backup_entries() {
    for point in ["partial-directory", "partial-directory-tampered"] {
        let directory = tempfile::tempdir().unwrap();
        prepare(directory.path());
        interrupt(directory.path(), point);
        let _ownership = transaction::claim(directory.path()).unwrap();
        let result = transaction::recover(directory.path());
        if point == "partial-directory" {
            result.unwrap();
            assert!(!directory.path().join(".scopenet/staged-sync").exists());
        } else {
            assert!(result.is_err());
            assert_eq!(
                fs::read(directory.path().join(".scopenet/staged-sync/rollback/server-resource-packs/nested/second")).unwrap(),
                b"foreign backup file"
            );
        }
        assert_eq!(super::load_state(directory.path()).revision, 8);
        assert_eq!(fs::read(directory.path().join("mods/managed.jar")).unwrap(), b"replacement");
        assert_eq!(fs::read(directory.path().join("saves/synthetic/level.dat")).unwrap(), b"player-owned");
    }
}

#[test]
fn malformed_oversized_or_legacy_workspace_is_preserved() {
    for contents in [b"invalid".to_vec(), vec![b' '; 16 * 1024 * 1024 + 1]] {
        let directory = tempfile::tempdir().unwrap();
        prepare(directory.path());
        let workspace = directory.path().join(".scopenet/staged-sync");
        fs::create_dir(&workspace).unwrap();
        fs::write(workspace.join("journal.json"), &contents).unwrap();
        let _ownership = transaction::claim(directory.path()).unwrap();
        assert!(transaction::recover(directory.path()).is_err());
        assert_eq!(fs::read(workspace.join("journal.json")).unwrap(), contents);
    }
    let directory = tempfile::tempdir().unwrap();
    prepare(directory.path());
    fs::create_dir(directory.path().join(".scopenet/staged-sync")).unwrap();
    fs::write(directory.path().join(".scopenet/staged-sync/legacy-backup"), b"retained").unwrap();
    let _ownership = transaction::claim(directory.path()).unwrap();
    assert!(transaction::recover(directory.path()).is_err());
}

#[test]
fn unsafe_journal_paths_do_not_mutate_player_data() {
    let directory = tempfile::tempdir().unwrap();
    let original = prepare(directory.path());
    interrupt(directory.path(), "original");
    let journal = directory.path().join(".scopenet/staged-sync/journal.json");
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&journal).unwrap()).unwrap();
    value["moves"][0]["source"] = "../escape".into();
    fs::write(&journal, serde_json::to_vec(&value).unwrap()).unwrap();
    let _ownership = transaction::claim(directory.path()).unwrap();
    assert!(transaction::recover(directory.path()).is_err());
    assert_eq!(fs::read(super::state_path(directory.path())).unwrap(), original);
    assert_eq!(fs::read(directory.path().join("saves/synthetic/level.dat")).unwrap(), b"player-owned");
}

#[cfg(unix)]
#[test]
fn symlinked_workspace_or_lock_is_rejected() {
    let directory = tempfile::tempdir().unwrap();
    prepare(directory.path());
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("sentinel"), b"retained").unwrap();
    std::os::unix::fs::symlink(outside.path(), directory.path().join(".scopenet/staged-sync")).unwrap();
    assert!(transaction::recover(directory.path()).is_err());
    assert_eq!(fs::read(outside.path().join("sentinel")).unwrap(), b"retained");
    fs::remove_file(directory.path().join(".scopenet/staged-sync")).unwrap();
    std::os::unix::fs::symlink(outside.path().join("sentinel"), directory.path().join(".scopenet/sync.lock")).unwrap();
    assert!(transaction::claim(directory.path()).is_err());
}

#[test]
fn empty_workspace_cleanup_is_restart_safe() {
    let directory = tempfile::tempdir().unwrap();
    prepare(directory.path());
    fs::create_dir(directory.path().join(".scopenet/staged-sync")).unwrap();
    let _ownership = transaction::claim(directory.path()).unwrap();
    transaction::recover(directory.path()).unwrap();
    assert!(safety::metadata(&directory.path().join(".scopenet/staged-sync")).unwrap().is_none());
}

#[tokio::test]
async fn unchanged_sync_is_read_only_but_deep_corruption_still_requires_repair() {
    use crate::http::regression_tests::local_client;
    use std::time::{Duration, UNIX_EPOCH};
    let directory = tempfile::tempdir().unwrap();
    let game = directory.path();
    let original = prepare(game);
    let path = super::state_path(game);
    let timestamp = UNIX_EPOCH + Duration::from_secs(5);
    fs::File::open(&path).unwrap().set_times(fs::FileTimes::new().set_modified(timestamp)).unwrap();
    let files = [super::tests::entry("mods/managed.jar", "http://unused", b"previous")];
    for deep in [false, true] {
        let report = super::sync_with_publication(
            &local_client(),
            "http://unused",
            game,
            7,
            2,
            &files,
            1,
            deep,
            &crate::progress::noop(),
            |_, _| {
                panic!("unchanged state must not be republished");
            },
        )
        .await
        .unwrap();
        assert_eq!(report.downloaded, 0);
        assert_eq!(fs::read(&path).unwrap(), original);
        assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), timestamp);
        assert!(!game.join(".scopenet/staged-sync").exists());
    }
    fs::write(game.join("mods/managed.jar"), b"tampered").unwrap();
    assert!(super::sync(&local_client(), "http://unused", game, 7, 2, &files, 1, true, &crate::progress::noop()).await.is_err());
    assert_eq!(fs::read(&path).unwrap(), original);
    assert_eq!(fs::read(game.join("mods/managed.jar")).unwrap(), b"tampered");
    assert_eq!(fs::read(game.join("saves/synthetic/level.dat")).unwrap(), b"player-owned");
}

#[tokio::test]
async fn revision_zero_epoch_advance_is_persisted_without_cleaning_existing_files() {
    use crate::http::regression_tests::local_client;
    let directory = tempfile::tempdir().unwrap();
    let game = directory.path();
    prepare(game);
    fs::write(
        super::state_path(game),
        serde_json::to_vec_pretty(&SyncState { revision: 0, managed: vec!["mods/managed.jar".into()], clean_epoch: 2 }).unwrap(),
    )
    .unwrap();
    let files = [super::tests::entry("mods/managed.jar", "http://unused", b"previous")];
    let report = super::sync(&local_client(), "http://unused", game, 0, 3, &files, 1, false, &crate::progress::noop()).await.unwrap();
    assert_eq!(report.downloaded, 0);
    assert_eq!(report.cleaned, 0);
    assert_eq!(super::load_state(game).clean_epoch, 3);
    assert_eq!(fs::read(game.join("mods/managed.jar")).unwrap(), b"previous");
    assert_eq!(fs::read(game.join("saves/synthetic/level.dat")).unwrap(), b"player-owned");
    assert!(!game.join(".scopenet/staged-sync").exists());
}
