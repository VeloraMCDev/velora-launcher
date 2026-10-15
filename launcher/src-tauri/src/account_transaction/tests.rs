use anyhow::{bail, Result};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const BEFORE_SECRETS: &[u8] = b"synthetic-encrypted-before";
const AFTER_SECRETS: &[u8] = b"synthetic-encrypted-after";
const CHILD_ACTION: &str = "VELORA_ACCOUNT_TRANSACTION_TEST_ACTION";

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let directory = std::env::temp_dir().join(format!("velora-account-transaction-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&directory).unwrap();
        Self(fs::canonicalize(directory).unwrap())
    }

    fn seed(&self) {
        fs::write(self.0.join("accounts.json"), accounts_bytes(false)).unwrap();
        fs::write(self.0.join("secrets.bin"), BEFORE_SECRETS).unwrap();
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn accounts_bytes(after: bool) -> Vec<u8> {
    let identifier = if after { "00000000-0000-4000-8000-000000000002" } else { "00000000-0000-4000-8000-000000000001" };
    let value = serde_json::json!({
        "accounts": [{
            "id": identifier,
            "kind": "offline",
            "username": if after { "FixtureAfter" } else { "FixtureBefore" },
            "uuid": identifier,
            "panel_url": null,
            "role": null
        }],
        "active": identifier
    });
    let accounts: crate::accounts::AccountsFile = serde_json::from_value(value).unwrap();
    serde_json::to_vec_pretty(&accounts).unwrap()
}

fn validate_synthetic_encrypted(bytes: &[u8]) -> Result<()> {
    if bytes != BEFORE_SECRETS && bytes != AFTER_SECRETS {
        bail!("not a synthetic encrypted transaction fixture");
    }
    Ok(())
}

fn commit_after(directory: &Path) -> Result<()> {
    super::commit(directory, &accounts_bytes(true), AFTER_SECRETS, validate_synthetic_encrypted)
}

fn interrupt(directory: &Path, action: &str, point: &str) {
    let child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "account_transaction::tests::crash_child", "--nocapture", "--test-threads=1"])
        .env("VELORA_ACCOUNT_TRANSACTION_ROOT", directory)
        .env("VELORA_ACCOUNT_TRANSACTION_FAULT", point)
        .env(CHILD_ACTION, action)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    let mut child = ChildGuard(child);
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            assert_eq!(status.code(), Some(77), "{action} did not exit at production fault hook {point}");
            return;
        }
        if Instant::now() >= deadline {
            child.0.kill().unwrap();
            let status = child.0.wait().unwrap();
            panic!("{action} child exceeded deadline at {point}; killed and reaped: {status}");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn crash_child() {
    let Some(action) = std::env::var_os(CHILD_ACTION) else { return };
    let directory = PathBuf::from(std::env::var_os("VELORA_ACCOUNT_TRANSACTION_ROOT").unwrap());
    assert!(directory.is_absolute());
    assert!(directory.is_dir());
    match action.to_str().unwrap() {
        "commit" => commit_after(&directory).unwrap(),
        "recover" => super::recover(&directory, validate_synthetic_encrypted).unwrap(),
        "io_commit" | "io_recover" => {
            let result =
                if action == "io_commit" { commit_after(&directory) } else { super::recover(&directory, validate_synthetic_encrypted) };
            let expect_error = std::env::var("VELORA_ACCOUNT_TRANSACTION_EXPECT_ERROR").unwrap() == "true";
            assert_eq!(result.is_err(), expect_error, "unexpected transaction result: {result:?}");
            return;
        }
        other => panic!("unknown synthetic child action {other}"),
    }
    panic!("production fault hook did not terminate the child");
}

fn io_failure(directory: &Path, action: &str, points: &str, expect_error: bool) {
    let child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "account_transaction::tests::crash_child", "--nocapture", "--test-threads=1"])
        .env("VELORA_ACCOUNT_TRANSACTION_ROOT", directory)
        .env("VELORA_ACCOUNT_TRANSACTION_IO_FAULT", points)
        .env("VELORA_ACCOUNT_TRANSACTION_EXPECT_ERROR", expect_error.to_string())
        .env(CHILD_ACTION, action)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    let mut child = ChildGuard(child);
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            assert!(status.success(), "{action} failed at injected I/O boundaries {points}: {status}");
            return;
        }
        if Instant::now() >= deadline {
            child.0.kill().unwrap();
            let status = child.0.wait().unwrap();
            panic!("{action} exceeded deadline at {points}; killed and reaped: {status}");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn failed_initial_journal_write_retains_invalid_evidence_without_store_changes() {
    let directory = TestDirectory::new();
    directory.seed();
    io_failure(&directory.0, "io_commit", "journal_write", true);
    assert_pair(&directory.0, false);
    assert_eq!(fs::read(directory.0.join(super::JOURNAL_NAME)).unwrap(), b"");
    assert_recovery_refuses_without_writes(&directory.0);
    assert!(super::ensure_clean(&directory.0).is_err());
}

#[test]
fn failed_prepared_synchronization_retains_recoverable_before_pair() {
    for point in ["journal_file_sync", "prepared_directory_sync"] {
        let directory = TestDirectory::new();
        directory.seed();
        io_failure(&directory.0, "io_commit", point, true);
        assert_pair(&directory.0, false);
        assert!(super::ensure_clean(&directory.0).is_err());
        super::recover(&directory.0, validate_synthetic_encrypted).unwrap();
        assert_pair(&directory.0, false);
        assert_clean(&directory.0);
    }
}

#[test]
fn failed_publication_or_marker_rolls_back_pair_before_returning_error() {
    for point in ["publish_secrets_write", "publish_accounts_write", "committed_marker_write"] {
        let directory = TestDirectory::new();
        directory.seed();
        io_failure(&directory.0, "io_commit", point, true);
        assert_pair(&directory.0, false);
        assert_clean(&directory.0);
    }
}

#[test]
fn failed_publication_and_rollback_preserve_journal_for_restart() {
    let directory = TestDirectory::new();
    directory.seed();
    io_failure(&directory.0, "io_commit", "publish_accounts_write,recovery_secrets_write", true);
    assert_eq!(fs::read(directory.0.join("accounts.json")).unwrap(), accounts_bytes(false));
    assert_eq!(fs::read(directory.0.join("secrets.bin")).unwrap(), AFTER_SECRETS);
    assert!(super::ensure_clean(&directory.0).is_err());
    super::recover(&directory.0, validate_synthetic_encrypted).unwrap();
    assert_pair(&directory.0, false);
    assert_clean(&directory.0);
}

#[test]
fn failed_image_synchronization_leaves_known_evidence_for_retry() {
    let directory = TestDirectory::new();
    directory.seed();
    io_failure(&directory.0, "io_commit", "image_directory_sync", true);
    assert_pair(&directory.0, false);
    assert!(super::ensure_clean(&directory.0).is_err());
    super::recover(&directory.0, validate_synthetic_encrypted).unwrap();
    assert_pair(&directory.0, false);
    assert_clean(&directory.0);
}

#[test]
fn failed_recovery_write_preserves_known_mixed_pair_until_retry() {
    for point in ["recovery_secrets_write", "recovery_accounts_write"] {
        let directory = TestDirectory::new();
        directory.seed();
        interrupt(&directory.0, "commit", "accounts_published");
        let journal = fs::read(directory.0.join(super::JOURNAL_NAME)).unwrap();
        io_failure(&directory.0, "io_recover", point, true);
        assert_eq!(fs::read(directory.0.join(super::JOURNAL_NAME)).unwrap(), journal);
        assert_eq!(fs::read(directory.0.join("accounts.json")).unwrap(), accounts_bytes(true));
        assert_eq!(
            fs::read(directory.0.join("secrets.bin")).unwrap(),
            if point == "recovery_secrets_write" { AFTER_SECRETS } else { BEFORE_SECRETS }
        );
        super::recover(&directory.0, validate_synthetic_encrypted).unwrap();
        assert_pair(&directory.0, false);
        assert_clean(&directory.0);
    }
}

#[test]
fn final_sync_or_cleanup_failure_returns_published_pair_with_recovery_fence() {
    for point in ["committed_directory_sync", "cleanup_remove"] {
        let directory = TestDirectory::new();
        directory.seed();
        io_failure(&directory.0, "io_commit", point, false);
        assert_pair(&directory.0, true);
        let journal: serde_json::Value = serde_json::from_slice(&fs::read(directory.0.join(super::JOURNAL_NAME)).unwrap()).unwrap();
        assert_eq!(journal["phase"], "committed");
        assert!(super::ensure_clean(&directory.0).is_err());
        super::recover(&directory.0, validate_synthetic_encrypted).unwrap();
        assert_pair(&directory.0, true);
        assert_clean(&directory.0);
    }
}

#[test]
fn cleanup_sync_failure_keeps_published_pair_and_removed_journal() {
    let directory = TestDirectory::new();
    directory.seed();
    io_failure(&directory.0, "io_commit", "cleanup_directory_sync", false);
    assert_pair(&directory.0, true);
    assert_clean(&directory.0);
    super::recover(&directory.0, validate_synthetic_encrypted).unwrap();
    assert_pair(&directory.0, true);
}

fn assert_pair(directory: &Path, after: bool) {
    assert_eq!(fs::read(directory.join("accounts.json")).unwrap(), accounts_bytes(after));
    assert_eq!(fs::read(directory.join("secrets.bin")).unwrap(), if after { AFTER_SECRETS } else { BEFORE_SECRETS });
}

fn assert_clean(directory: &Path) {
    assert!(!directory.join(super::JOURNAL_NAME).exists());
    super::ensure_clean(directory).unwrap();
}

#[derive(Debug, PartialEq, Eq)]
enum Evidence {
    Directory,
    File(Vec<u8>),
    Symlink(PathBuf),
}

fn snapshot(directory: &Path) -> BTreeMap<PathBuf, Evidence> {
    fn visit(root: &Path, directory: &Path, evidence: &mut BTreeMap<PathBuf, Evidence>) {
        for entry in fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            let metadata = fs::symlink_metadata(&path).unwrap();
            let relative = path.strip_prefix(root).unwrap().to_path_buf();
            if metadata.file_type().is_symlink() {
                evidence.insert(relative, Evidence::Symlink(fs::read_link(&path).unwrap()));
            } else if metadata.is_dir() {
                evidence.insert(relative, Evidence::Directory);
                visit(root, &path, evidence);
            } else {
                assert!(metadata.is_file(), "unexpected synthetic evidence type: {}", path.display());
                evidence.insert(relative, Evidence::File(fs::read(&path).unwrap()));
            }
        }
    }
    let mut evidence = BTreeMap::new();
    visit(directory, directory, &mut evidence);
    evidence
}

fn assert_recovery_refuses_without_writes(directory: &Path) {
    let evidence = snapshot(directory);
    for _ in 0..2 {
        assert!(super::recover(directory, validate_synthetic_encrypted).is_err());
        assert_eq!(snapshot(directory), evidence, "failed recovery must preserve every evidence file");
    }
}

#[test]
fn successful_commit_publishes_both_stores() {
    let directory = TestDirectory::new();
    directory.seed();
    commit_after(&directory.0).unwrap();
    assert_pair(&directory.0, true);
    assert_clean(&directory.0);
    let evidence = snapshot(&directory.0);
    super::recover(&directory.0, validate_synthetic_encrypted).unwrap();
    assert_eq!(snapshot(&directory.0), evidence);
}

#[test]
fn commit_crashes_restore_prior_pair_unless_committed() {
    assert_eq!(super::JOURNAL_NAME, ".account-transaction.json");
    for point in ["prepared", "secrets_published", "accounts_published", "committed"] {
        let directory = TestDirectory::new();
        directory.seed();
        interrupt(&directory.0, "commit", point);
        assert!(directory.0.join(super::JOURNAL_NAME).is_file(), "missing journal at {point}");
        super::recover(&directory.0, validate_synthetic_encrypted).unwrap();
        assert_pair(&directory.0, point == "committed");
        assert_clean(&directory.0);
        let evidence = snapshot(&directory.0);
        super::recover(&directory.0, validate_synthetic_encrypted).unwrap();
        assert_eq!(snapshot(&directory.0), evidence, "repeated recovery changed evidence at {point}");
    }
}

#[test]
fn interrupted_recovery_is_idempotent() {
    for commit_point in ["accounts_published", "committed"] {
        for recovery_point in ["recovery_secrets", "recovery_accounts"] {
            let directory = TestDirectory::new();
            directory.seed();
            interrupt(&directory.0, "commit", commit_point);
            interrupt(&directory.0, "recover", recovery_point);
            assert!(directory.0.join(super::JOURNAL_NAME).is_file());
            super::recover(&directory.0, validate_synthetic_encrypted).unwrap();
            assert_pair(&directory.0, commit_point == "committed");
            assert_clean(&directory.0);
            let evidence = snapshot(&directory.0);
            super::recover(&directory.0, validate_synthetic_encrypted).unwrap();
            assert_eq!(snapshot(&directory.0), evidence);
        }
    }
}

#[test]
fn changed_store_evidence_is_preserved_before_any_recovery_writes() {
    for point in ["prepared", "secrets_published", "accounts_published", "committed"] {
        for filename in ["accounts.json", "secrets.bin"] {
            let directory = TestDirectory::new();
            directory.seed();
            interrupt(&directory.0, "commit", point);
            fs::write(directory.0.join(filename), b"externally-changed-synthetic-evidence").unwrap();
            assert_recovery_refuses_without_writes(&directory.0);
        }
    }
}

#[test]
fn missing_published_store_is_preserved_before_any_recovery_writes() {
    for filename in ["accounts.json", "secrets.bin"] {
        let directory = TestDirectory::new();
        directory.seed();
        interrupt(&directory.0, "commit", "accounts_published");
        fs::remove_file(directory.0.join(filename)).unwrap();
        assert_recovery_refuses_without_writes(&directory.0);
    }
}

#[test]
fn corrupt_journal_fails_closed_and_preserves_evidence() {
    for corrupt in [b"{truncated".as_slice(), b"{}".as_slice()] {
        let directory = TestDirectory::new();
        directory.seed();
        interrupt(&directory.0, "commit", "accounts_published");
        fs::write(directory.0.join(super::JOURNAL_NAME), corrupt).unwrap();
        assert_recovery_refuses_without_writes(&directory.0);
        let evidence = snapshot(&directory.0);
        assert!(commit_after(&directory.0).is_err());
        assert!(super::ensure_clean(&directory.0).is_err());
        assert_eq!(snapshot(&directory.0), evidence);
    }
}

#[test]
fn unknown_journal_version_fails_closed_and_preserves_evidence() {
    let directory = TestDirectory::new();
    directory.seed();
    interrupt(&directory.0, "commit", "accounts_published");
    let journal_path = directory.0.join(super::JOURNAL_NAME);
    let mut journal: serde_json::Value = serde_json::from_slice(&fs::read(&journal_path).unwrap()).unwrap();
    assert!(journal.get("version").is_some(), "production journal must declare its schema version");
    journal["version"] = serde_json::json!(u32::MAX);
    fs::write(journal_path, serde_json::to_vec(&journal).unwrap()).unwrap();
    assert_recovery_refuses_without_writes(&directory.0);
    let evidence = snapshot(&directory.0);
    assert!(commit_after(&directory.0).is_err());
    assert!(super::ensure_clean(&directory.0).is_err());
    assert_eq!(snapshot(&directory.0), evidence);
}

#[test]
fn journal_cannot_redirect_publication_to_arbitrary_filenames() {
    for field in ["accounts_path", "secrets_path"] {
        let directory = TestDirectory::new();
        directory.seed();
        interrupt(&directory.0, "commit", "accounts_published");
        let journal_path = directory.0.join(super::JOURNAL_NAME);
        let mut journal: serde_json::Value = serde_json::from_slice(&fs::read(&journal_path).unwrap()).unwrap();
        journal[field] = serde_json::json!("../synthetic-other-store");
        fs::write(journal_path, serde_json::to_vec(&journal).unwrap()).unwrap();
        assert_recovery_refuses_without_writes(&directory.0);
    }
}

#[test]
fn rejected_recovery_validation_preserves_both_stores_and_journal() {
    let directory = TestDirectory::new();
    directory.seed();
    interrupt(&directory.0, "commit", "accounts_published");
    let evidence = snapshot(&directory.0);
    assert!(super::recover(&directory.0, |_| bail!("synthetic validation rejection")).is_err());
    assert_eq!(snapshot(&directory.0), evidence);
    super::recover(&directory.0, validate_synthetic_encrypted).unwrap();
    assert_pair(&directory.0, false);
    assert_clean(&directory.0);
}

#[test]
fn first_save_crashes_remove_uncommitted_pair_or_retain_committed_pair() {
    for point in ["prepared", "secrets_published", "accounts_published", "committed"] {
        let directory = TestDirectory::new();
        interrupt(&directory.0, "commit", point);
        super::recover(&directory.0, validate_synthetic_encrypted).unwrap();
        if point == "committed" {
            assert_pair(&directory.0, true);
        } else {
            assert!(!directory.0.join("accounts.json").exists());
            assert!(!directory.0.join("secrets.bin").exists());
        }
        assert_clean(&directory.0);
        let evidence = snapshot(&directory.0);
        super::recover(&directory.0, validate_synthetic_encrypted).unwrap();
        assert_eq!(snapshot(&directory.0), evidence);
    }
}

#[test]
fn rejected_encrypted_candidate_does_not_change_stores() {
    let directory = TestDirectory::new();
    directory.seed();
    let evidence = snapshot(&directory.0);
    assert!(super::commit(&directory.0, &accounts_bytes(true), b"invalid-synthetic-encrypted", validate_synthetic_encrypted).is_err());
    assert_eq!(snapshot(&directory.0), evidence);
    assert_clean(&directory.0);
}

#[cfg(unix)]
#[test]
fn symlink_destinations_are_rejected_without_changing_targets() {
    for filename in ["accounts.json", "secrets.bin", super::JOURNAL_NAME] {
        let directory = TestDirectory::new();
        directory.seed();
        interrupt(&directory.0, "commit", "accounts_published");
        let outside = TestDirectory::new();
        let target = outside.0.join("synthetic-target");
        fs::rename(directory.0.join(filename), &target).unwrap();
        std::os::unix::fs::symlink(&target, directory.0.join(filename)).unwrap();
        let outside_evidence = snapshot(&outside.0);
        assert_recovery_refuses_without_writes(&directory.0);
        let evidence = snapshot(&directory.0);
        assert!(commit_after(&directory.0).is_err());
        assert!(super::ensure_clean(&directory.0).is_err());
        assert_eq!(snapshot(&directory.0), evidence);
        assert_eq!(snapshot(&outside.0), outside_evidence);
    }
}

#[cfg(unix)]
#[test]
fn fresh_commit_rejects_symlink_destinations_without_a_pending_journal() {
    for filename in ["accounts.json", "secrets.bin"] {
        let directory = TestDirectory::new();
        directory.seed();
        let outside = TestDirectory::new();
        let target = outside.0.join("synthetic-target");
        fs::rename(directory.0.join(filename), &target).unwrap();
        std::os::unix::fs::symlink(&target, directory.0.join(filename)).unwrap();
        let evidence = snapshot(&directory.0);
        let outside_evidence = snapshot(&outside.0);
        assert!(commit_after(&directory.0).is_err());
        assert_eq!(snapshot(&directory.0), evidence);
        assert_eq!(snapshot(&outside.0), outside_evidence);
    }
}

#[cfg(unix)]
#[test]
fn symlink_store_parent_is_rejected_without_writes() {
    let directory = TestDirectory::new();
    let store = directory.0.join("actual");
    fs::create_dir(&store).unwrap();
    fs::write(store.join("accounts.json"), accounts_bytes(false)).unwrap();
    fs::write(store.join("secrets.bin"), BEFORE_SECRETS).unwrap();
    interrupt(&store, "commit", "accounts_published");
    let alias_store = directory.0.join("alias");
    std::os::unix::fs::symlink(&store, &alias_store).unwrap();
    let evidence = snapshot(&directory.0);
    assert!(super::recover(&alias_store, validate_synthetic_encrypted).is_err());
    assert!(commit_after(&alias_store).is_err());
    assert!(super::ensure_clean(&alias_store).is_err());
    assert_eq!(snapshot(&directory.0), evidence);
    super::recover(&store, validate_synthetic_encrypted).unwrap();
    let clean_evidence = snapshot(&directory.0);
    assert!(commit_after(&alias_store).is_err());
    assert_eq!(snapshot(&directory.0), clean_evidence);
}
