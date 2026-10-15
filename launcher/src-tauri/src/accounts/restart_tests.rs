use super::*;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("velora-account-restart-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&root).unwrap();
        Self(root)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

fn response(username: &str) -> AuthResponse {
    AuthResponse {
        token: format!("synthetic-panel-{username}"),
        user: velora_shared::PublicUser {
            id: 7,
            username: username.into(),
            uuid: "synthetic-stable-uuid".into(),
            role: "player".into(),
            groups: vec![],
        },
        pending: false,
        yggdrasil: Some(velora_shared::YggdrasilTokens {
            access_token: format!("synthetic-game-{username}"),
            client_token: "synthetic-client".into(),
        }),
    }
}

fn child(root: &std::path::Path, point: &str, action: &str) {
    let mut process = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "accounts::restart_tests::restart_child", "--nocapture"])
        .env("VELORA_ACCOUNT_TRANSACTION_ROOT", root)
        .env("VELORA_ACCOUNT_TRANSACTION_FAULT", point)
        .env("VELORA_ACCOUNT_RESTART_ACTION", action)
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        if let Some(status) = process.try_wait().unwrap() {
            assert_eq!(status.code(), Some(77), "{point}: {action}");
            return;
        }
        if Instant::now() >= deadline {
            process.kill().unwrap();
            process.wait().unwrap();
            panic!("synthetic account interruption child timed out");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn io_child(root: &std::path::Path, points: &str, expect_error: bool, pending: bool) {
    let mut process = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "accounts::restart_tests::restart_child", "--nocapture"])
        .env("VELORA_ACCOUNT_TRANSACTION_ROOT", root)
        .env("VELORA_ACCOUNT_TRANSACTION_IO_FAULT", points)
        .env("VELORA_ACCOUNT_TRANSACTION_EXPECT_ERROR", expect_error.to_string())
        .env("VELORA_ACCOUNT_TRANSACTION_EXPECT_PENDING", pending.to_string())
        .env("VELORA_ACCOUNT_RESTART_ACTION", "io_save")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        if let Some(status) = process.try_wait().unwrap() {
            assert!(status.success(), "encrypted account I/O fixture failed at {points}: {status}");
            return;
        }
        if Instant::now() >= deadline {
            process.kill().unwrap();
            process.wait().unwrap();
            panic!("synthetic account I/O failure child timed out");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn restart_child() {
    let Some(root) = std::env::var_os("VELORA_ACCOUNT_TRANSACTION_ROOT") else { return };
    let state = AppState::try_new(PathBuf::from(root)).unwrap();
    match std::env::var("VELORA_ACCOUNT_RESTART_ACTION").unwrap().as_str() {
        "save" => {
            save_panel_account(&state, "https://synthetic.invalid", response("After")).unwrap();
        }
        "remove" => {
            let id = state.accounts.read().unwrap().active().unwrap().id.clone();
            remove(&state, &id).unwrap();
        }
        "recover" => {}
        "io_save" => {
            let original = state.accounts.read().unwrap().active().unwrap().clone();
            let result = save_panel_account(&state, "https://synthetic.invalid", response("After"));
            let expect_error = std::env::var("VELORA_ACCOUNT_TRANSACTION_EXPECT_ERROR").unwrap() == "true";
            let pending = std::env::var("VELORA_ACCOUNT_TRANSACTION_EXPECT_PENDING").unwrap() == "true";
            assert_eq!(result.is_err(), expect_error, "unexpected encrypted account result: {result:?}");
            let current = state.accounts.read().unwrap().active().unwrap().clone();
            assert_eq!(current.id, original.id);
            assert_eq!(current.username, if expect_error { "Before" } else { "After" });
            assert_eq!(state.data_dir.join(crate::account_transaction::JOURNAL_NAME).exists(), pending);
            if pending {
                let accounts = std::fs::read(state.data_dir.join("accounts.json")).unwrap();
                let secrets = std::fs::read(state.data_dir.join("secrets.bin")).unwrap();
                assert!(state.secrets.get(&original.id).panel_token.is_none());
                assert!(state.secrets.set(&original.id, Secret::default()).is_err());
                assert!(select(&state, original.id.clone()).is_err());
                assert!(add_offline(&state, "Blocked").is_err());
                assert_eq!(std::fs::read(state.data_dir.join("accounts.json")).unwrap(), accounts);
                assert_eq!(std::fs::read(state.data_dir.join("secrets.bin")).unwrap(), secrets);
            } else {
                assert_eq!(state.secrets.get(&original.id).panel_token.as_deref(), Some("synthetic-panel-Before"));
            }
            return;
        }
        _ => panic!("unknown synthetic interruption action"),
    }
    panic!("requested interruption was not reached");
}

#[test]
fn encrypted_publication_and_rollback_io_errors_recover_original_identity() {
    for (points, pending) in [
        ("publish_accounts_write", false),
        ("committed_marker_write", false),
        ("publish_accounts_write,recovery_secrets_write", true),
        ("image_directory_sync", true),
    ] {
        let fixture = Fixture::new();
        let state = AppState::new(fixture.0.clone());
        let original = save_panel_account(&state, "https://synthetic.invalid", response("Before")).unwrap();
        let before_accounts = std::fs::read(fixture.0.join("accounts.json")).unwrap();
        let before_secrets = std::fs::read(fixture.0.join("secrets.bin")).unwrap();
        let key = std::fs::read(fixture.0.join("secrets.key")).unwrap();
        drop(state);
        io_child(&fixture.0, points, true, pending);
        let reopened = AppState::try_new(fixture.0.clone()).unwrap();
        assert_eq!(reopened.accounts.read().unwrap().active(), Some(&original));
        assert_eq!(reopened.secrets.get(&original.id).panel_token.as_deref(), Some("synthetic-panel-Before"));
        assert_eq!(std::fs::read(fixture.0.join("accounts.json")).unwrap(), before_accounts);
        assert_eq!(std::fs::read(fixture.0.join("secrets.bin")).unwrap(), before_secrets);
        assert_eq!(std::fs::read(fixture.0.join("secrets.key")).unwrap(), key);
        assert!(!fixture.0.join(crate::account_transaction::JOURNAL_NAME).exists());
    }
}

#[test]
fn encrypted_committed_sync_and_cleanup_errors_fence_live_reads_until_restart() {
    for point in ["committed_directory_sync", "cleanup_remove"] {
        let fixture = Fixture::new();
        let state = AppState::new(fixture.0.clone());
        let original = save_panel_account(&state, "https://synthetic.invalid", response("Before")).unwrap();
        let key = std::fs::read(fixture.0.join("secrets.key")).unwrap();
        drop(state);
        io_child(&fixture.0, point, false, true);
        let accounts = std::fs::read(fixture.0.join("accounts.json")).unwrap();
        let secrets = std::fs::read(fixture.0.join("secrets.bin")).unwrap();
        let reopened = AppState::try_new(fixture.0.clone()).unwrap();
        let current = reopened.accounts.read().unwrap().active().unwrap().clone();
        assert_eq!(current.id, original.id);
        assert_eq!(current.username, "After");
        assert_eq!(reopened.secrets.get(&current.id).panel_token.as_deref(), Some("synthetic-panel-After"));
        assert_eq!(std::fs::read(fixture.0.join("accounts.json")).unwrap(), accounts);
        assert_eq!(std::fs::read(fixture.0.join("secrets.bin")).unwrap(), secrets);
        assert_eq!(std::fs::read(fixture.0.join("secrets.key")).unwrap(), key);
        assert!(!fixture.0.join(crate::account_transaction::JOURNAL_NAME).exists());
    }
}

#[test]
fn encrypted_account_replacement_recovers_before_accounts_are_loaded() {
    for point in ["prepared", "secrets_published", "accounts_published", "committed"] {
        let fixture = Fixture::new();
        let state = AppState::new(fixture.0.clone());
        let original = save_panel_account(&state, "https://synthetic.invalid", response("Before")).unwrap();
        let before_accounts = std::fs::read(fixture.0.join("accounts.json")).unwrap();
        let before_secrets = std::fs::read(fixture.0.join("secrets.bin")).unwrap();
        let key = std::fs::read(fixture.0.join("secrets.key")).unwrap();
        std::fs::write(fixture.0.join("skin_profiles.json"), b"[]").unwrap();
        drop(state);
        child(&fixture.0, point, "save");
        assert!(fixture.0.join(crate::account_transaction::JOURNAL_NAME).exists());
        let journal = std::fs::read_to_string(fixture.0.join(crate::account_transaction::JOURNAL_NAME)).unwrap();
        for token in
            ["synthetic-panel-Before", "synthetic-panel-After", "synthetic-game-Before", "synthetic-game-After", "synthetic-client"]
        {
            assert!(!journal.contains(token));
        }
        let reopened = AppState::try_new(fixture.0.clone()).unwrap();
        let current = reopened.accounts.read().unwrap().active().unwrap().clone();
        assert_eq!(current.id, original.id);
        let expected = if point == "committed" { "After" } else { "Before" };
        assert_eq!(current.username, expected);
        assert_eq!(reopened.secrets.get(&current.id).panel_token.as_deref(), Some(format!("synthetic-panel-{expected}").as_str()));
        if point != "committed" {
            assert_eq!(std::fs::read(fixture.0.join("accounts.json")).unwrap(), before_accounts);
            assert_eq!(std::fs::read(fixture.0.join("secrets.bin")).unwrap(), before_secrets);
        }
        assert_eq!(std::fs::read(fixture.0.join("secrets.key")).unwrap(), key);
        assert_eq!(std::fs::read(fixture.0.join("skin_profiles.json")).unwrap(), b"[]");
        assert!(!fixture.0.join(crate::account_transaction::JOURNAL_NAME).exists());
    }
}

#[test]
fn interrupted_first_sign_in_and_logout_retain_coherent_pair() {
    for action in ["save", "remove"] {
        for point in ["prepared", "secrets_published", "accounts_published", "committed"] {
            let fixture = Fixture::new();
            let state = AppState::new(fixture.0.clone());
            let original =
                (action == "remove").then(|| save_panel_account(&state, "https://synthetic.invalid", response("Before")).unwrap());
            drop(state);
            child(&fixture.0, point, action);
            let reopened = AppState::try_new(fixture.0.clone()).unwrap();
            let current = reopened.accounts.read().unwrap().active().cloned();
            if action == "save" && point == "committed" {
                let account = current.unwrap();
                assert_eq!(account.username, "After");
                assert_eq!(reopened.secrets.get(&account.id).panel_token.as_deref(), Some("synthetic-panel-After"));
            } else if action == "remove" && point != "committed" {
                let original = original.unwrap();
                assert_eq!(current, Some(original.clone()));
                assert_eq!(reopened.secrets.get(&original.id).panel_token.as_deref(), Some("synthetic-panel-Before"));
            } else {
                assert!(current.is_none());
                if let Some(account) = original {
                    assert!(reopened.secrets.get(&account.id).panel_token.is_none());
                } else {
                    assert!(!fixture.0.join("accounts.json").exists());
                    assert!(!fixture.0.join("secrets.bin").exists());
                }
            }
        }
    }
}

#[test]
fn interrupted_encrypted_rollback_can_resume_without_regenerating_keys() {
    for point in ["recovery_secrets", "recovery_accounts"] {
        let fixture = Fixture::new();
        let state = AppState::new(fixture.0.clone());
        let original = save_panel_account(&state, "https://synthetic.invalid", response("Before")).unwrap();
        let key = std::fs::read(fixture.0.join("secrets.key")).unwrap();
        drop(state);
        child(&fixture.0, "accounts_published", "save");
        child(&fixture.0, point, "recover");
        let reopened = AppState::try_new(fixture.0.clone()).unwrap();
        assert_eq!(reopened.accounts.read().unwrap().active(), Some(&original));
        assert_eq!(reopened.secrets.get(&original.id).panel_token.as_deref(), Some("synthetic-panel-Before"));
        assert_eq!(std::fs::read(fixture.0.join("secrets.key")).unwrap(), key);
    }
}

#[test]
fn damaged_or_locked_recovery_blocks_startup_without_new_keys_or_overwrites() {
    let fixture = Fixture::new();
    let journal = fixture.0.join(crate::account_transaction::JOURNAL_NAME);
    std::fs::write(&journal, b"synthetic damaged journal").unwrap();
    assert!(AppState::try_new(fixture.0.clone()).is_err());
    assert!(!fixture.0.join("secrets.key").exists());
    assert_eq!(std::fs::read(&journal).unwrap(), b"synthetic damaged journal");

    let fixture = Fixture::new();
    let state = AppState::new(fixture.0.clone());
    save_panel_account(&state, "https://synthetic.invalid", response("Before")).unwrap();
    drop(state);
    child(&fixture.0, "accounts_published", "save");
    std::fs::remove_file(fixture.0.join("secrets.key")).unwrap();
    let accounts = std::fs::read(fixture.0.join("accounts.json")).unwrap();
    let secrets = std::fs::read(fixture.0.join("secrets.bin")).unwrap();
    let journal = std::fs::read(fixture.0.join(crate::account_transaction::JOURNAL_NAME)).unwrap();
    assert!(AppState::try_new(fixture.0.clone()).is_err());
    assert!(!fixture.0.join("secrets.key").exists());
    assert_eq!(std::fs::read(fixture.0.join("accounts.json")).unwrap(), accounts);
    assert_eq!(std::fs::read(fixture.0.join("secrets.bin")).unwrap(), secrets);
    assert_eq!(std::fs::read(fixture.0.join(crate::account_transaction::JOURNAL_NAME)).unwrap(), journal);
}

#[test]
fn unresolved_journal_blocks_live_account_and_credential_changes() {
    let fixture = Fixture::new();
    let state = AppState::new(fixture.0.clone());
    let original = save_panel_account(&state, "https://synthetic.invalid", response("Before")).unwrap();
    child(&fixture.0, "accounts_published", "save");
    let accounts = std::fs::read(fixture.0.join("accounts.json")).unwrap();
    let secrets = std::fs::read(fixture.0.join("secrets.bin")).unwrap();
    assert!(state.secrets.get(&original.id).panel_token.is_none());
    assert!(state.secrets.set(&original.id, Secret::default()).is_err());
    assert!(select(&state, original.id.clone()).is_err());
    assert!(add_offline(&state, "Other").is_err());
    assert_eq!(std::fs::read(fixture.0.join("accounts.json")).unwrap(), accounts);
    assert_eq!(std::fs::read(fixture.0.join("secrets.bin")).unwrap(), secrets);
    drop(state);
    let reopened = AppState::try_new(fixture.0.clone()).unwrap();
    assert_eq!(reopened.accounts.read().unwrap().active(), Some(&original));
    assert_eq!(reopened.secrets.get(&original.id).panel_token.as_deref(), Some("synthetic-panel-Before"));
}

#[cfg(unix)]
#[test]
fn startup_rejects_symlink_root_before_creating_or_loading_credential_key() {
    let fixture = Fixture::new();
    let target = fixture.0.join("unrelated-store");
    std::fs::create_dir(&target).unwrap();
    std::fs::write(target.join("sentinel"), b"synthetic preserved").unwrap();
    let link = fixture.0.join("linked-store");
    std::os::unix::fs::symlink(&target, &link).unwrap();
    assert!(AppState::try_new(link.clone()).is_err());
    assert!(crate::secrets::Secrets::try_open(&link).is_err());
    assert!(!target.join("secrets.key").exists());
    assert!(!target.join("secrets.bin").exists());
    assert_eq!(std::fs::read_dir(&target).unwrap().count(), 1);
    assert_eq!(std::fs::read(target.join("sentinel")).unwrap(), b"synthetic preserved");
}
