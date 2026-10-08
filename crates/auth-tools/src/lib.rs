//! Offline file operations. No HTTP service, credential writer cutover or signing-key generation.
pub mod assets;
use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::fs::{File, OpenOptions};
use std::io::Read;
use std::path::{Path, PathBuf};

fn digest(path: &Path) -> Result<String> {
    let mut source = File::open(path)?;
    let mut hash = Sha256::new();
    let mut buffer = [0; 65536];
    loop {
        let count = source.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    Ok(hex::encode(hash.finalize()))
}

fn sidecar(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(suffix);
    PathBuf::from(name)
}

fn check_checkpoint(path: &Path) -> Result<()> {
    if !path.is_file() {
        bail!("checkpoint must be an existing regular file");
    }
    let path = path.canonicalize()?;
    for suffix in ["-wal", "-shm", "-journal"] {
        if sidecar(&path, suffix).exists() {
            bail!("checkpoint has SQLite sidecars; supply a consistent, closed backup, not a live database");
        }
    }
    Ok(())
}

async fn new_store(path: &Path) -> Result<SqlitePool> {
    for suffix in ["-wal", "-shm", "-journal"] {
        if sidecar(path, suffix).exists() {
            bail!("destination has existing SQLite sidecars");
        }
    }
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path).context("destination must be a new file in an existing protected directory")?;
    let connection = SqliteConnectOptions::new().filename(path).foreign_keys(true);
    let pool = SqlitePoolOptions::new().max_connections(1).connect_with(connection).await?;
    velora_auth_core::schema::initialize(&pool).await?;
    Ok(pool)
}

/// Create an empty owner-local store, never overwrite an existing file.
pub async fn init_new(path: &Path) -> Result<serde_json::Value> {
    let store = new_store(path).await?;
    store.close().await;
    Ok(serde_json::json!({"operation":"init", "schema_version":velora_auth_core::schema::VERSION}))
}

/// Preserve a closed SQLite file byte-for-byte, including mixed private schema/data.
/// Output is protected backup material, never a public extraction or live backup operation.
pub async fn checkpoint_new(source: &Path, target: &Path) -> Result<serde_json::Value> {
    check_checkpoint(source)?;
    let source = source.canonicalize()?;
    let before = digest(&source)?;
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    for suffix in ["-wal", "-shm", "-journal"] {
        if sidecar(target, suffix).exists() {
            bail!("checkpoint destination has existing SQLite sidecars");
        }
    }
    let mut output = options.open(target).context("checkpoint destination must be a new file in a protected directory")?;
    let bytes = std::io::copy(&mut File::open(&source)?, &mut output)?;
    output.sync_all()?;
    drop(output);
    check_checkpoint(&source)?;
    if digest(&source)? != before || digest(target)? != before {
        bail!("checkpoint bytes changed during copying; inspect the preserved output and do not import it");
    }
    let connection = SqliteConnectOptions::new().filename(target).read_only(true).immutable(true).pragma("query_only", "ON");
    let pool = SqlitePoolOptions::new().max_connections(1).connect_with(connection).await?;
    let checks: Vec<String> = sqlx::query_scalar("PRAGMA integrity_check").fetch_all(&pool).await?;
    let version: i64 = sqlx::query_scalar("PRAGMA user_version").fetch_one(&pool).await?;
    pool.close().await;
    if checks != ["ok"] {
        bail!("checkpoint failed SQLite integrity validation; inspect protected output");
    }
    check_checkpoint(&source)?;
    if digest(&source)? != before || digest(target)? != before {
        bail!("checkpoint changed during validation; do not import the preserved output");
    }
    Ok(serde_json::json!({"operation":"checkpoint", "sha256":before, "bytes":bytes, "sqlite_user_version":version}))
}

/// Import from a closed checkpoint; emit only file integrity hashes and table counts.
/// Directory permissions, checkpoint creation and external signing/texture assets remain operator responsibilities.
pub async fn import_new(source: &Path, target: &Path) -> Result<serde_json::Value> {
    check_checkpoint(source)?;
    let source = source.canonicalize()?;
    let before = digest(&source)?;
    let connection =
        SqliteConnectOptions::new().filename(&source).read_only(true).immutable(true).foreign_keys(true).pragma("query_only", "ON");
    let checkpoint = SqlitePoolOptions::new().max_connections(1).connect_with(connection).await?;
    // Read-only/query-only also blocks accidental writes through the source pool.
    let store = new_store(target).await?;
    let result = velora_auth_core::import::import_checkpoint(&checkpoint, &store).await;
    checkpoint.close().await;
    store.close().await;
    let counts = result?;
    check_checkpoint(&source)?;
    if digest(&source)? != before {
        bail!("checkpoint changed during import; do not deploy the destination; inspect the preserved files");
    }
    let tables: serde_json::Map<String, serde_json::Value> =
        counts.0.into_iter().map(|(name, count)| (name.into(), count.into())).collect();
    Ok(serde_json::json!({
        "operation":"import", "schema_version":velora_auth_core::schema::VERSION,
        "source_sha256":before, "destination_sha256":digest(target)?, "tables":tables
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn fresh_initialization_never_overwrites_existing_files_or_sidecars() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("synthetic store.sqlite");
        init_new(&path).await.unwrap();
        let before = digest(&path).unwrap();
        assert!(init_new(&path).await.is_err());
        assert_eq!(digest(&path).unwrap(), before);
        let other = folder.path().join("other.sqlite");
        std::fs::write(sidecar(&other, "-wal"), b"synthetic").unwrap();
        assert!(init_new(&other).await.is_err());
        assert!(!other.exists());
    }
    #[tokio::test]
    async fn file_import_preserves_source_bytes_and_refuses_live_checkpoint_sidecars() {
        let folder = tempfile::tempdir().unwrap();
        let source = folder.path().join("synthetic checkpoint.sqlite");
        let target = folder.path().join("synthetic authority.sqlite");
        let pool = new_store(&source).await.unwrap();
        sqlx::query("INSERT INTO users(id,username,password_hash,created_at,uuid) VALUES(42,'SyntheticPlayer','synthetic-hash','2026-01-01','01234567-89ab-4def-8123-456789abcdef')").execute(&pool).await.unwrap();
        sqlx::raw_sql(
            "INSERT INTO launcher_sessions(id,user_id,ip,username,created_at) VALUES
            (7,42,'203.0.113.8','HistoricalName','2026-01-01'),(8,NULL,'2001:db8::1',NULL,'2026-01-01');",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("CREATE TABLE private_gameplay(id INTEGER)").execute(&pool).await.unwrap();
        let mode: String = sqlx::query_scalar("PRAGMA journal_mode=WAL").fetch_one(&pool).await.unwrap();
        assert_eq!(mode, "wal");
        pool.close().await;
        let before = digest(&source).unwrap();
        let report = import_new(&source, &target).await.unwrap();
        assert_eq!(report["tables"]["users"], 1);
        assert_eq!(report["tables"]["launcher_sessions"], 2);
        assert_eq!(report["source_sha256"], before);
        assert_eq!(digest(&source).unwrap(), before);
        assert!(!report.to_string().contains("synthetic-hash"));
        assert!(!report.to_string().contains("SyntheticPlayer"));
        assert!(import_new(&source, &target).await.is_err());
        std::fs::write(sidecar(&source, "-wal"), b"synthetic").unwrap();
        let rejected = folder.path().join("rejected.sqlite");
        assert!(import_new(&source, &rejected).await.is_err());
        assert!(!rejected.exists());
    }
    #[tokio::test]
    async fn closed_checkpoint_is_byte_exact_including_private_records_and_legacy_version() {
        let folder = tempfile::tempdir().unwrap();
        let source = folder.path().join("synthetic legacy.sqlite");
        let target = folder.path().join("synthetic backup.sqlite");
        let pool = new_store(&source).await.unwrap();
        sqlx::raw_sql("CREATE TABLE private_gameplay(id INTEGER PRIMARY KEY, value TEXT); INSERT INTO private_gameplay VALUES(99,'synthetic-private-value'); PRAGMA user_version=77;")
            .execute(&pool).await.unwrap();
        pool.close().await;
        let original = std::fs::read(&source).unwrap();
        let report = checkpoint_new(&source, &target).await.unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), original);
        assert_eq!(std::fs::read(&source).unwrap(), original);
        assert_eq!(report["sqlite_user_version"], 77);
        assert!(!report.to_string().contains("synthetic-private-value"));
        assert!(checkpoint_new(&source, &target).await.is_err());
        assert_eq!(std::fs::read(&target).unwrap(), original);
    }
    #[tokio::test]
    async fn checkpoint_refuses_source_sidecars_before_output_and_detects_invalid_sqlite() {
        let folder = tempfile::tempdir().unwrap();
        let source = folder.path().join("synthetic.sqlite");
        let target = folder.path().join("rejected.sqlite");
        init_new(&source).await.unwrap();
        std::fs::write(sidecar(&source, "-journal"), b"synthetic").unwrap();
        assert!(checkpoint_new(&source, &target).await.is_err());
        assert!(!target.exists());
        let invalid = folder.path().join("invalid.sqlite");
        std::fs::write(&invalid, b"synthetic invalid SQLite bytes").unwrap();
        assert!(checkpoint_new(&invalid, &target).await.is_err());
        assert_eq!(std::fs::read(invalid).unwrap(), b"synthetic invalid SQLite bytes");
    }
}
