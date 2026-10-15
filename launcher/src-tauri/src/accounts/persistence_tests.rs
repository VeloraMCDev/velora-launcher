use super::*;

struct Directory(std::path::PathBuf);

impl Directory {
    fn new() -> Self {
        let directory = Self(std::env::temp_dir().join(format!("velora-account-fixture-{}", uuid::Uuid::new_v4())));
        std::fs::create_dir(&directory.0).unwrap();
        directory
    }
}

impl Drop for Directory {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

fn auth(username: &str, with_session: bool) -> AuthResponse {
    AuthResponse {
        token: "synthetic-panel-token".into(),
        user: velora_shared::PublicUser {
            id: 7,
            username: username.into(),
            uuid: "synthetic-stable-uuid".into(),
            role: "player".into(),
            groups: vec![],
        },
        pending: false,
        yggdrasil: with_session.then(|| velora_shared::YggdrasilTokens {
            access_token: "synthetic-game-token".into(),
            client_token: "synthetic-client-token".into(),
        }),
    }
}

#[test]
fn legacy_account_roundtrip_preserves_identity_and_selection() {
    let directory = Directory::new();
    let path = directory.0.join("accounts.json");
    std::fs::write(
        &path,
        br#"{"accounts":[{"id":"synthetic-id","kind":"offline","username":"Steve","uuid":"synthetic-uuid"}],"active":"synthetic-id"}"#,
    )
    .unwrap();
    let mut accounts = AccountsFile::load(&path);
    let original = accounts.active().unwrap().clone();
    AccountsFile::update(&mut accounts, &path, |candidate| {
        candidate.upsert(Account { id: "discarded-new-id".into(), ..original.clone() });
        Ok(())
    })
    .unwrap();
    let restored = AccountsFile::load(&path);
    assert_eq!(restored.active(), Some(&original));
    assert_eq!(restored.accounts.len(), 1);
    assert_eq!(std::fs::read_dir(&directory.0).unwrap().count(), 1);
}

#[test]
fn corrupt_accounts_preserve_disk_and_memory_on_failed_update() {
    let directory = Directory::new();
    let path = directory.0.join("accounts.json");
    let mut accounts = AccountsFile { accounts: vec![], active: Some("synthetic-current".into()) };
    std::fs::write(&path, b"synthetic invalid account JSON").unwrap();
    assert!(AccountsFile::load(&path).accounts.is_empty());
    assert!(AccountsFile::update(&mut accounts, &path, |candidate| {
        candidate.active = None;
        Ok(())
    })
    .is_err());
    assert_eq!(accounts.active.as_deref(), Some("synthetic-current"));
    assert_eq!(std::fs::read(&path).unwrap(), b"synthetic invalid account JSON");
}

#[test]
fn missing_game_session_cannot_replace_saved_account_or_credentials() {
    let directory = Directory::new();
    let state = AppState::new(directory.0.clone());
    let account = save_panel_account(&state, "https://synthetic.invalid", auth("Steve", true)).unwrap();
    let original_accounts = std::fs::read(directory.0.join("accounts.json")).unwrap();
    let original_credentials = std::fs::read(directory.0.join("secrets.bin")).unwrap();
    assert!(save_panel_account(&state, "https://synthetic.invalid", auth("Changed", false)).is_err());
    assert_eq!(state.accounts.read().unwrap().active(), Some(&account));
    assert_eq!(std::fs::read(directory.0.join("accounts.json")).unwrap(), original_accounts);
    assert_eq!(std::fs::read(directory.0.join("secrets.bin")).unwrap(), original_credentials);
}

#[test]
fn failed_account_publication_restores_saved_credentials_and_memory() {
    let directory = Directory::new();
    let state = AppState::new(directory.0.clone());
    let account = save_panel_account(&state, "https://synthetic.invalid", auth("Steve", true)).unwrap();
    let path = directory.0.join("accounts.json");
    std::fs::write(&path, b"synthetic concurrent corruption").unwrap();
    let mut replacement = auth("Changed", true);
    replacement.token = "synthetic-replacement-token".into();
    assert!(save_panel_account(&state, "https://synthetic.invalid", replacement).is_err());
    assert_eq!(state.accounts.read().unwrap().active(), Some(&account));
    assert_eq!(state.secrets.get(&account.id).panel_token.as_deref(), Some("synthetic-panel-token"));
    let reopened = crate::secrets::Secrets::open(&directory.0);
    assert_eq!(reopened.get(&account.id).panel_token.as_deref(), Some("synthetic-panel-token"));
    assert_eq!(std::fs::read(&path).unwrap(), b"synthetic concurrent corruption");
}

#[test]
fn credential_write_failure_never_commits_account_or_in_memory_secret() {
    let directory = Directory::new();
    let state = AppState::new(directory.0.clone());
    let account = save_panel_account(&state, "https://synthetic.invalid", auth("Steve", true)).unwrap();
    let original = std::fs::read(directory.0.join("accounts.json")).unwrap();
    let saved_credentials = std::fs::read(directory.0.join("secrets.bin")).unwrap();
    std::fs::remove_file(directory.0.join("secrets.bin")).unwrap();
    std::fs::create_dir(directory.0.join("secrets.bin")).unwrap();
    let mut replacement = auth("Changed", true);
    replacement.token = "synthetic-replacement-token".into();
    assert!(save_panel_account(&state, "https://synthetic.invalid", replacement).is_err());
    assert_eq!(state.accounts.read().unwrap().active(), Some(&account));
    assert_eq!(state.secrets.get(&account.id).panel_token.as_deref(), Some("synthetic-panel-token"));
    assert_eq!(std::fs::read(directory.0.join("accounts.json")).unwrap(), original);
    assert!(state.secrets.remove(&account.id).is_err());
    assert_eq!(state.secrets.get(&account.id).panel_token.as_deref(), Some("synthetic-panel-token"));
    std::fs::remove_dir(directory.0.join("secrets.bin")).unwrap();
    std::fs::write(directory.0.join("secrets.bin"), saved_credentials).unwrap();
    assert_eq!(crate::secrets::Secrets::open(&directory.0).get(&account.id).panel_token.as_deref(), Some("synthetic-panel-token"));
}

#[test]
fn offline_account_save_failure_keeps_last_selection() {
    let directory = Directory::new();
    let state = AppState::new(directory.0.clone());
    let first = add_offline(&state, "Steve").unwrap();
    std::fs::write(directory.0.join("accounts.json"), b"synthetic damaged accounts").unwrap();
    assert!(add_offline(&state, "Alex").is_err());
    assert_eq!(state.accounts.read().unwrap().active(), Some(&first));
    assert_eq!(state.accounts.read().unwrap().accounts.len(), 1);
}

#[test]
fn failed_selection_and_removal_preserve_saved_identity_and_credentials() {
    let directory = Directory::new();
    let state = AppState::new(directory.0.clone());
    let first = save_panel_account(&state, "https://synthetic.invalid", auth("Steve", true)).unwrap();
    let second = add_offline(&state, "Alex").unwrap();
    let path = directory.0.join("accounts.json");
    std::fs::write(&path, b"synthetic damaged accounts").unwrap();
    assert!(select(&state, first.id.clone()).is_err());
    assert!(remove(&state, &first.id).is_err());
    assert_eq!(state.accounts.read().unwrap().active(), Some(&second));
    assert_eq!(state.accounts.read().unwrap().accounts.len(), 2);
    assert_eq!(crate::secrets::Secrets::open(&directory.0).get(&first.id).panel_token.as_deref(), Some("synthetic-panel-token"));
    assert_eq!(std::fs::read(&path).unwrap(), b"synthetic damaged accounts");
}

#[test]
fn successful_rename_reuses_id_and_removal_reopens_with_next_selection() {
    let directory = Directory::new();
    let state = AppState::new(directory.0.clone());
    let first = save_panel_account(&state, "https://synthetic.invalid", auth("Steve", true)).unwrap();
    let renamed = save_panel_account(&state, "https://synthetic.invalid", auth("Changed", true)).unwrap();
    assert_eq!(renamed.id, first.id);
    let second = add_offline(&state, "Alex").unwrap();
    select(&state, renamed.id.clone()).unwrap();
    remove(&state, &renamed.id).unwrap();
    let restored = AccountsFile::load(&directory.0.join("accounts.json"));
    assert_eq!(restored.active(), Some(&second));
    assert_eq!(restored.accounts.len(), 1);
    assert!(crate::secrets::Secrets::open(&directory.0).get(&renamed.id).panel_token.is_none());
}

#[cfg(unix)]
#[test]
fn legacy_credential_temp_symlink_cannot_truncate_an_unrelated_file() {
    use std::os::unix::fs::symlink;
    let directory = Directory::new();
    let state = AppState::new(directory.0.clone());
    let unrelated = directory.0.join("synthetic-unrelated.txt");
    std::fs::write(&unrelated, b"synthetic unrelated content").unwrap();
    symlink(&unrelated, directory.0.join("secrets.bin.tmp")).unwrap();
    let first = save_panel_account(&state, "https://synthetic.invalid", auth("Steve", true)).unwrap();
    let renamed = save_panel_account(&state, "https://synthetic.invalid", auth("Changed", true)).unwrap();
    assert_eq!(first.id, renamed.id);
    assert_eq!(std::fs::read(&unrelated).unwrap(), b"synthetic unrelated content");
    assert!(std::fs::symlink_metadata(directory.0.join("secrets.bin.tmp")).unwrap().file_type().is_symlink());
    assert_eq!(crate::secrets::Secrets::open(&directory.0).get(&first.id).panel_token.as_deref(), Some("synthetic-panel-token"));
    assert!(!std::fs::read_dir(&directory.0).unwrap().any(|entry| entry
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with(".secrets.bin.")));
}

#[cfg(unix)]
#[test]
fn credential_destination_symlink_preserves_target_and_account_state() {
    use std::os::unix::fs::symlink;
    let directory = Directory::new();
    let state = AppState::new(directory.0.clone());
    let first = save_panel_account(&state, "https://synthetic.invalid", auth("Steve", true)).unwrap();
    let destination = directory.0.join("secrets.bin");
    let unrelated = directory.0.join("synthetic-saved-credentials.bin");
    std::fs::rename(&destination, &unrelated).unwrap();
    let original = std::fs::read(&unrelated).unwrap();
    symlink(&unrelated, &destination).unwrap();
    assert!(save_panel_account(&state, "https://synthetic.invalid", auth("Changed", true)).is_err());
    assert_eq!(state.accounts.read().unwrap().active(), Some(&first));
    assert_eq!(std::fs::read(&unrelated).unwrap(), original);
    assert!(std::fs::symlink_metadata(&destination).unwrap().file_type().is_symlink());
}

#[test]
fn failed_multi_store_rollback_reports_both_failures_and_retains_credentials_for_recovery() {
    let directory = Directory::new();
    let state = AppState::new(directory.0.clone());
    let first = save_panel_account(&state, "https://synthetic.invalid", auth("Steve", true)).unwrap();
    let destination = directory.0.join("secrets.bin");
    let recovery = directory.0.join("synthetic-retained-credentials.bin");
    let error = state
        .secrets
        .set_with_commit(&first.id, Secret { panel_token: Some("synthetic-replacement-token".into()), ..Secret::default() }, || {
            std::fs::rename(&destination, &recovery)?;
            std::fs::create_dir(&destination)?;
            bail!("synthetic account publication failed")
        })
        .unwrap_err();
    let message = format!("{error:#}");
    assert!(message.contains("saved credential rollback failed"));
    assert!(message.contains("synthetic account publication failed"));
    assert!(recovery.is_file());
    assert!(destination.is_dir());
    assert_eq!(state.secrets.get(&first.id).panel_token.as_deref(), Some("synthetic-panel-token"));
    assert_eq!(AccountsFile::load(&directory.0.join("accounts.json")).active(), Some(&first));
}

#[test]
fn failed_first_account_publication_restores_credential_file_absence() {
    let directory = Directory::new();
    let state = AppState::new(directory.0.clone());
    std::fs::write(directory.0.join("accounts.json"), b"synthetic damaged accounts").unwrap();
    assert!(!directory.0.join("secrets.bin").exists());
    assert!(save_panel_account(&state, "https://synthetic.invalid", auth("Steve", true)).is_err());
    assert!(!directory.0.join("secrets.bin").exists());
    assert!(state.accounts.read().unwrap().accounts.is_empty());
    assert_eq!(std::fs::read(directory.0.join("accounts.json")).unwrap(), b"synthetic damaged accounts");
}
