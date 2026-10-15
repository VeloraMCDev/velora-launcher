use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

pub(crate) const JOURNAL_NAME: &str = ".account-transaction.json";
const MAX_IMAGE_BYTES: usize = 8 * 1024 * 1024;
const MAX_JOURNAL_BYTES: usize = 48 * 1024 * 1024;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Images {
    before: Option<String>,
    after: String,
}

impl Images {
    fn capture(before: Option<Vec<u8>>, after: &[u8]) -> Self {
        Self { before: before.map(|bytes| STANDARD.encode(bytes)), after: STANDARD.encode(after) }
    }

    fn decode(&self) -> Result<(Option<Vec<u8>>, Vec<u8>)> {
        let decode = |encoded: &str| -> Result<Vec<u8>> {
            anyhow::ensure!(encoded.len() <= MAX_IMAGE_BYTES.div_ceil(3) * 4, "account transaction image exceeds limit");
            let bytes = STANDARD.decode(encoded).context("invalid account transaction image")?;
            anyhow::ensure!(bytes.len() <= MAX_IMAGE_BYTES, "account transaction image exceeds limit");
            Ok(bytes)
        };
        Ok((self.before.as_deref().map(decode).transpose()?, decode(&self.after)?))
    }
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
enum Phase {
    Prepared,
    Committed,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    version: u32,
    phase: Phase,
    accounts: Images,
    secrets: Images,
}

pub(crate) fn validate_root(directory: &Path) -> Result<()> {
    anyhow::ensure!(fs::symlink_metadata(directory)?.file_type().is_dir(), "account transaction parent must be a directory");
    Ok(())
}

fn read_regular(path: &Path, limit: usize) -> Result<Option<Vec<u8>>> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            anyhow::ensure!(metadata.file_type().is_file(), "account transaction destination must be a regular file");
            anyhow::ensure!(metadata.len() <= limit as u64, "account transaction file exceeds limit");
            let mut bytes = Vec::new();
            File::open(path)?.take(limit as u64 + 1).read_to_end(&mut bytes)?;
            anyhow::ensure!(bytes.len() <= limit, "account transaction file exceeds limit");
            Ok(Some(bytes))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn sync_directory(directory: &Path, point: &str) -> Result<()> {
    io_fault(point, directory)?;
    #[cfg(unix)]
    File::open(directory)?.sync_all()?;
    #[cfg(not(unix))]
    let _ = directory;
    Ok(())
}

pub(crate) fn ensure_clean(root: &Path) -> Result<()> {
    validate_root(root)?;
    match fs::symlink_metadata(root.join(JOURNAL_NAME)) {
        Ok(_) => anyhow::bail!("saved account recovery is pending; restart before changing saved sign-ins"),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn validate_accounts(bytes: &[u8]) -> Result<()> {
    serde_json::from_slice::<crate::accounts::AccountsFile>(bytes).context("invalid saved account transaction image")?;
    Ok(())
}

fn write_image(root: &Path, name: &str, bytes: Option<&[u8]>, expected: Option<&[u8]>) -> Result<()> {
    let path = root.join(name);
    anyhow::ensure!(read_regular(&path, MAX_IMAGE_BYTES)?.as_deref() == expected, "saved account state changed during recovery");
    match bytes {
        Some(bytes) => crate::persistence::atomic_write_bytes(&path, bytes, |current| {
            anyhow::ensure!(Some(current) == expected, "saved account state changed during publication");
            Ok(())
        })?,
        None if expected.is_some() => fs::remove_file(&path)?,
        None => {}
    }
    sync_directory(root, "image_directory_sync")
}

fn cleanup(root: &Path, expected: &[u8]) -> Result<()> {
    let path = root.join(JOURNAL_NAME);
    anyhow::ensure!(read_regular(&path, MAX_JOURNAL_BYTES)?.as_deref() == Some(expected), "account recovery journal changed");
    io_fault("cleanup_remove", root)?;
    fs::remove_file(path)?;
    if sync_directory(root, "cleanup_directory_sync").is_err() {
        tracing::warn!("account journal cleanup directory synchronization failed");
    }
    Ok(())
}

pub(crate) fn recover(root: &Path, validate_secret: impl Fn(&[u8]) -> Result<()>) -> Result<()> {
    validate_root(root)?;
    let Some(raw) = read_regular(&root.join(JOURNAL_NAME), MAX_JOURNAL_BYTES)? else { return Ok(()) };
    let journal: Journal = serde_json::from_slice(&raw).context("invalid saved account recovery journal")?;
    anyhow::ensure!(journal.version == 1, "unsupported saved account recovery version");
    let (accounts_before, accounts_after) = journal.accounts.decode()?;
    let (secrets_before, secrets_after) = journal.secrets.decode()?;
    validate_accounts(&accounts_after)?;
    if let Some(bytes) = &accounts_before {
        validate_accounts(bytes)?;
    }
    validate_secret(&secrets_after)?;
    if let Some(bytes) = &secrets_before {
        validate_secret(bytes)?;
    }
    let current_accounts = read_regular(&root.join("accounts.json"), MAX_IMAGE_BYTES)?;
    let current_secrets = read_regular(&root.join("secrets.bin"), MAX_IMAGE_BYTES)?;
    anyhow::ensure!(
        current_accounts == accounts_before || current_accounts.as_deref() == Some(accounts_after.as_slice()),
        "saved account evidence differs from recovery journal"
    );
    anyhow::ensure!(
        current_secrets == secrets_before || current_secrets.as_deref() == Some(secrets_after.as_slice()),
        "saved credential evidence differs from recovery journal"
    );
    let committed = journal.phase == Phase::Committed;
    let desired_secrets = if committed { Some(secrets_after.as_slice()) } else { secrets_before.as_deref() };
    let desired_accounts = if committed { Some(accounts_after.as_slice()) } else { accounts_before.as_deref() };
    io_fault("recovery_secrets_write", root)?;
    write_image(root, "secrets.bin", desired_secrets, current_secrets.as_deref())?;
    fault("recovery_secrets", root);
    io_fault("recovery_accounts_write", root)?;
    write_image(root, "accounts.json", desired_accounts, current_accounts.as_deref())?;
    fault("recovery_accounts", root);
    cleanup(root, &raw)
}

pub(crate) fn commit(root: &Path, accounts: &[u8], encrypted: &[u8], validate_secret: impl Fn(&[u8]) -> Result<()>) -> Result<()> {
    ensure_clean(root)?;
    anyhow::ensure!(accounts.len() <= MAX_IMAGE_BYTES && encrypted.len() <= MAX_IMAGE_BYTES, "saved account transaction exceeds limit");
    validate_accounts(accounts)?;
    validate_secret(encrypted)?;
    let old_accounts = read_regular(&root.join("accounts.json"), MAX_IMAGE_BYTES)?;
    let old_secrets = read_regular(&root.join("secrets.bin"), MAX_IMAGE_BYTES)?;
    if let Some(bytes) = &old_accounts {
        validate_accounts(bytes)?;
    }
    if let Some(bytes) = &old_secrets {
        validate_secret(bytes)?;
    }
    let mut journal = Journal {
        version: 1,
        phase: Phase::Prepared,
        accounts: Images::capture(old_accounts.clone(), accounts),
        secrets: Images::capture(old_secrets.clone(), encrypted),
    };
    let prepared = serde_json::to_vec_pretty(&journal)?;
    anyhow::ensure!(prepared.len() <= MAX_JOURNAL_BYTES, "account transaction journal exceeds limit");
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(root.join(JOURNAL_NAME))?;
    io_fault("journal_write", root)?;
    file.write_all(&prepared)?;
    io_fault("journal_file_sync", root)?;
    file.sync_all()?;
    drop(file);
    sync_directory(root, "prepared_directory_sync")?;
    fault("prepared", root);
    let publication = (|| -> Result<()> {
        io_fault("publish_secrets_write", root)?;
        write_image(root, "secrets.bin", Some(encrypted), old_secrets.as_deref())?;
        fault("secrets_published", root);
        io_fault("publish_accounts_write", root)?;
        write_image(root, "accounts.json", Some(accounts), old_accounts.as_deref())?;
        fault("accounts_published", root);
        journal.phase = Phase::Committed;
        io_fault("committed_marker_write", root)?;
        crate::persistence::atomic_write_bytes(&root.join(JOURNAL_NAME), &serde_json::to_vec_pretty(&journal)?, |existing| {
            anyhow::ensure!(existing == prepared, "account transaction journal changed during commit");
            Ok(())
        })?;
        Ok(())
    })();
    if let Err(error) = publication {
        return match recover(root, validate_secret) {
            Ok(()) => Err(error),
            Err(rollback) => Err(error.context(format!("saved credential rollback failed: {rollback:#}"))),
        };
    }
    if sync_directory(root, "committed_directory_sync").is_err() {
        tracing::warn!("account commit directory synchronization failed; power-loss durability is not established");
        return Ok(());
    }
    fault("committed", root);
    if cleanup(root, &serde_json::to_vec_pretty(&journal)?).is_err() {
        tracing::warn!("committed account recovery journal retained; restart before another saved-account change");
    }
    Ok(())
}

fn fault(point: &str, root: &Path) {
    #[cfg(test)]
    if std::env::var_os("VELORA_ACCOUNT_TRANSACTION_ROOT").as_deref() == Some(root.as_os_str())
        && std::env::var("VELORA_ACCOUNT_TRANSACTION_FAULT").as_deref() == Ok(point)
    {
        std::process::exit(77);
    }
    #[cfg(not(test))]
    let _ = (point, root);
}

fn io_fault(point: &str, root: &Path) -> Result<()> {
    #[cfg(test)]
    if std::env::var_os("VELORA_ACCOUNT_TRANSACTION_ROOT").as_deref() == Some(root.as_os_str())
        && std::env::var("VELORA_ACCOUNT_TRANSACTION_IO_FAULT").is_ok_and(|points| points.split(',').any(|selected| selected == point))
    {
        return Err(std::io::Error::other(format!("synthetic account transaction I/O failure: {point}")).into());
    }
    #[cfg(not(test))]
    let _ = (point, root);
    Ok(())
}

#[cfg(test)]
mod tests;
