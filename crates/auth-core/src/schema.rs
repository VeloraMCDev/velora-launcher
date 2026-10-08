//! Fresh authority stores. Existing mixed application databases require explicit import.
use anyhow::{bail, Result};
use sqlx::{SqliteConnection, SqlitePool};
use std::collections::BTreeSet;

pub const VERSION: i64 = 2;
const REQUIRED_OBJECTS: &[&str] = &[
    "immutable_player_uuid",
    "reserve_name_insert",
    "reserve_name_update",
    "remember_name_insert",
    "remember_name_update",
    "users_uuid",
    "ygg_tokens_user",
    "password_resets_user",
];
pub const TABLES: &[&str] = &[
    "velora_auth_schema",
    "users",
    "groups",
    "user_groups",
    "reserved_usernames",
    "capes",
    "ygg_tokens",
    "ygg_sessions",
    "player_keys",
    "account_connections",
    "oauth_attempts",
    "password_resets",
    "launcher_sessions",
];

// A new owner-local migration series. Released legacy migrations are not rewritten.
const INITIAL: &str = r#"
CREATE TABLE velora_auth_schema (id INTEGER PRIMARY KEY CHECK(id = 1), version INTEGER NOT NULL);
CREATE TABLE users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT NOT NULL UNIQUE COLLATE NOCASE,
    password_hash TEXT NOT NULL,
    email TEXT,
    role TEXT NOT NULL DEFAULT 'player',
    status TEXT NOT NULL DEFAULT 'active',
    created_at TEXT NOT NULL,
    last_login TEXT,
    uuid TEXT NOT NULL DEFAULT '',
    skin_hash TEXT,
    skin_model TEXT NOT NULL DEFAULT 'classic',
    cape_id INTEGER,
    status_reason TEXT,
    auth_version INTEGER NOT NULL DEFAULT 0,
    email_optout INTEGER NOT NULL DEFAULT 0
);
CREATE UNIQUE INDEX users_uuid ON users(uuid);
CREATE TABLE groups (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE COLLATE NOCASE,
    color TEXT NOT NULL DEFAULT '#7c5cff',
    luckperms_group TEXT NOT NULL DEFAULT '',
    discord_role TEXT NOT NULL DEFAULT ''
);
CREATE TABLE user_groups (
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    group_id INTEGER NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    PRIMARY KEY (user_id, group_id)
);
CREATE TABLE reserved_usernames (name TEXT PRIMARY KEY COLLATE NOCASE, uuid TEXT NOT NULL);
CREATE TRIGGER immutable_player_uuid BEFORE UPDATE OF uuid ON users
WHEN OLD.uuid <> '' AND NEW.uuid <> OLD.uuid
BEGIN SELECT RAISE(ABORT, 'player UUID is immutable'); END;
CREATE TRIGGER reserve_name_insert BEFORE INSERT ON users
WHEN EXISTS(SELECT 1 FROM reserved_usernames WHERE name = NEW.username AND uuid <> NEW.uuid)
BEGIN SELECT RAISE(ABORT, 'username reserved'); END;
CREATE TRIGGER reserve_name_update BEFORE UPDATE OF username ON users
WHEN EXISTS(SELECT 1 FROM reserved_usernames WHERE name = NEW.username AND uuid <> NEW.uuid)
BEGIN SELECT RAISE(ABORT, 'username reserved'); END;
CREATE TRIGGER remember_name_insert AFTER INSERT ON users
WHEN NEW.uuid <> ''
BEGIN INSERT OR IGNORE INTO reserved_usernames VALUES (NEW.username, NEW.uuid); END;
CREATE TRIGGER remember_name_update AFTER UPDATE OF username, uuid ON users
WHEN NEW.uuid <> ''
BEGIN INSERT OR IGNORE INTO reserved_usernames VALUES (NEW.username, NEW.uuid); END;
CREATE TABLE capes (
    id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, hash TEXT NOT NULL,
    visibility TEXT NOT NULL DEFAULT 'public', allowed_groups TEXT NOT NULL DEFAULT '[]', created_at TEXT NOT NULL
);
CREATE TABLE ygg_tokens (
    access_token TEXT PRIMARY KEY, client_token TEXT NOT NULL,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE, created_at TEXT NOT NULL, expires_at TEXT NOT NULL
);
CREATE INDEX ygg_tokens_user ON ygg_tokens(user_id);
CREATE TABLE ygg_sessions (
    server_id TEXT PRIMARY KEY, user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE, ip TEXT, created_at TEXT NOT NULL
);
CREATE TABLE player_keys (
    user_id INTEGER PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE, private_pem TEXT NOT NULL,
    public_pem TEXT NOT NULL, signature_v1 TEXT NOT NULL, signature_v2 TEXT NOT NULL, expires_at TEXT NOT NULL, refreshed_after TEXT NOT NULL
);
CREATE TABLE account_connections (
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE, provider TEXT NOT NULL,
    provider_id TEXT NOT NULL, display_name TEXT NOT NULL DEFAULT '', created_at TEXT NOT NULL,
    PRIMARY KEY(provider, provider_id), UNIQUE(user_id, provider)
);
CREATE TABLE oauth_attempts (
    state TEXT PRIMARY KEY, kind TEXT NOT NULL, user_id INTEGER REFERENCES users(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL, expires_at TEXT NOT NULL, result TEXT, consumed_at TEXT
);
CREATE TABLE password_resets (
    token_hash TEXT PRIMARY KEY, user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    expires_at TEXT NOT NULL, used_at TEXT
);
CREATE INDEX password_resets_user ON password_resets(user_id);
INSERT INTO velora_auth_schema(id, version) VALUES (1, 1);
"#;

const LAUNCHER_SESSIONS: &str = r#"
CREATE TABLE launcher_sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER REFERENCES users(id) ON DELETE CASCADE,
    ip TEXT NOT NULL,
    username TEXT,
    created_at TEXT NOT NULL
);
CREATE INDEX launcher_sessions_user ON launcher_sessions(user_id, created_at);
CREATE INDEX launcher_sessions_ip ON launcher_sessions(ip, created_at);
UPDATE velora_auth_schema SET version=2 WHERE id=1;
"#;

async fn validate_connection(connection: &mut SqliteConnection) -> Result<i64> {
    let foreign_keys: i64 = sqlx::query_scalar("PRAGMA foreign_keys").fetch_one(&mut *connection).await?;
    if foreign_keys != 1 {
        bail!("enable SQLite foreign keys on every Authentication pool connection before initialization");
    }
    let names: Vec<String> = sqlx::query_scalar("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'")
        .fetch_all(&mut *connection)
        .await?;
    if !names.iter().any(|name| name == "velora_auth_schema") {
        bail!("existing database is not an owned Authentication store; use the explicit import workflow");
    }
    let versions: Vec<(i64, i64)> = sqlx::query_as("SELECT id,version FROM velora_auth_schema").fetch_all(&mut *connection).await?;
    let version = match versions.as_slice() {
        [(1, version @ (1 | 2))] => *version,
        _ => bail!("unsupported Authentication schema version; upgrade or restore using its documented migration"),
    };
    let expected: BTreeSet<&str> = TABLES.iter().copied().filter(|name| version == 2 || *name != "launcher_sessions").collect();
    let found: BTreeSet<&str> = names.iter().map(String::as_str).collect();
    if found != expected {
        bail!("Authentication store has missing or foreign tables; refusing implicit schema changes");
    }
    let objects: Vec<String> =
        sqlx::query_scalar("SELECT name FROM sqlite_master WHERE type IN ('trigger','index')").fetch_all(&mut *connection).await?;
    if REQUIRED_OBJECTS.iter().any(|name| !objects.iter().any(|found| found == name)) {
        bail!("Authentication store has missing identity constraints; refusing implicit schema repair");
    }
    if version == 2 && ["launcher_sessions_user", "launcher_sessions_ip"].iter().any(|name| !objects.iter().any(|found| found == name)) {
        bail!("Authentication store has missing launcher constraints; refusing implicit schema repair");
    }
    Ok(version)
}

/// Validate a supported existing store without DDL, suitable for immutable backups.
/// Returns its actual version; never initializes or upgrades an empty/version-one file.
pub async fn validate(pool: &SqlitePool) -> Result<i64> {
    let mut tx = pool.begin().await?;
    let version = validate_connection(&mut tx).await?;
    tx.commit().await?;
    Ok(version)
}

/// Initialize an empty store or upgrade an intact owned version-one store.
/// Never adopt a mixed legacy database or reconstruct omitted session history.
/// The caller configures foreign keys on every pooled connection.
pub async fn initialize(pool: &SqlitePool) -> Result<()> {
    let mut tx = pool.begin().await?;
    let foreign_keys: i64 = sqlx::query_scalar("PRAGMA foreign_keys").fetch_one(&mut *tx).await?;
    if foreign_keys != 1 {
        bail!("enable SQLite foreign keys on every Authentication pool connection before initialization");
    }
    let names: Vec<String> =
        sqlx::query_scalar("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'").fetch_all(&mut *tx).await?;
    if names.is_empty() {
        sqlx::raw_sql(INITIAL).execute(&mut *tx).await?;
    }
    if validate_connection(&mut tx).await? == 1 {
        sqlx::raw_sql(LAUNCHER_SESSIONS).execute(&mut *tx).await?;
    }
    validate_connection(&mut tx).await?;
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity_store::{self, NewAccount};
    async fn pool() -> SqlitePool {
        let options = sqlx::sqlite::SqliteConnectOptions::new().in_memory(true).foreign_keys(true);
        sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect_with(options).await.unwrap()
    }
    async fn user(pool: &SqlitePool, name: &str, uuid: &str) -> Result<i64, sqlx::Error> {
        identity_store::insert(
            pool,
            NewAccount {
                username: name,
                password_hash: "synthetic-hash",
                email: None,
                role: "player",
                status: "active",
                created_at: "2026-01-01T00:00:00Z",
                uuid,
            },
        )
        .await
    }
    #[tokio::test]
    async fn fresh_store_is_repeatable_and_preserves_identity_constraints_and_cascades() {
        let pool = pool().await;
        initialize(&pool).await.unwrap();
        let id = user(&pool, "Original", "01234567-89ab-4def-8123-456789abcdef").await.unwrap();
        initialize(&pool).await.unwrap();
        sqlx::query("UPDATE users SET username='Renamed' WHERE id=?").bind(id).execute(&pool).await.unwrap();
        assert!(sqlx::query("UPDATE users SET uuid='changed' WHERE id=?").bind(id).execute(&pool).await.is_err());
        assert!(user(&pool, "original", "99999999-89ab-4def-8123-456789abcdef").await.is_err());
        sqlx::query("INSERT INTO password_resets VALUES ('synthetic-token-hash', ?, '2999-01-01T00:00:00Z', NULL)")
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM users WHERE id=?").bind(id).execute(&pool).await.unwrap();
        let resets: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM password_resets").fetch_one(&pool).await.unwrap();
        assert_eq!(resets, 0);
        assert!(user(&pool, "Renamed", "99999999-89ab-4def-8123-456789abcdef").await.is_err());
        assert!(user(&pool, "Original", "99999999-89ab-4def-8123-456789abcdef").await.is_err());
    }
    #[tokio::test]
    async fn version_one_validation_is_read_only_and_upgrade_preserves_identity_and_sequences() {
        let pool = pool().await;
        sqlx::raw_sql(INITIAL).execute(&pool).await.unwrap();
        let id = user(&pool, "Original", "01234567-89ab-4def-8123-456789abcdef").await.unwrap();
        sqlx::raw_sql("UPDATE sqlite_sequence SET seq=99 WHERE name='users'; PRAGMA user_version=77; PRAGMA query_only=ON;")
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(validate(&pool).await.unwrap(), 1);
        assert!(initialize(&pool).await.is_err());
        assert_eq!(validate(&pool).await.unwrap(), 1);
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM sqlite_master WHERE name='launcher_sessions'")
                .fetch_one(&pool)
                .await
                .unwrap(),
            0
        );
        sqlx::query("PRAGMA query_only=OFF").execute(&pool).await.unwrap();
        initialize(&pool).await.unwrap();
        assert_eq!(validate(&pool).await.unwrap(), 2);
        assert_eq!(sqlx::query_scalar::<_, i64>("PRAGMA user_version").fetch_one(&pool).await.unwrap(), 77);
        let identity: (i64, String, String, String) =
            sqlx::query_as("SELECT id,username,password_hash,uuid FROM users").fetch_one(&pool).await.unwrap();
        assert_eq!(identity, (id, "Original".into(), "synthetic-hash".into(), "01234567-89ab-4def-8123-456789abcdef".into()));
        assert_eq!(user(&pool, "NextUser", "11234567-89ab-4def-8123-456789abcdef").await.unwrap(), 100);
        initialize(&pool).await.unwrap();
        sqlx::query("PRAGMA query_only=ON").execute(&pool).await.unwrap();
        assert_eq!(validate(&pool).await.unwrap(), 2);
    }
    #[tokio::test]
    async fn damaged_old_store_and_missing_session_indices_are_refused_without_upgrade_or_repair() {
        let pool = pool().await;
        sqlx::raw_sql(INITIAL).execute(&pool).await.unwrap();
        sqlx::query("DROP TRIGGER immutable_player_uuid").execute(&pool).await.unwrap();
        assert!(initialize(&pool).await.is_err());
        assert_eq!(sqlx::query_scalar::<_, i64>("SELECT version FROM velora_auth_schema").fetch_one(&pool).await.unwrap(), 1);
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM sqlite_master WHERE name='launcher_sessions'")
                .fetch_one(&pool)
                .await
                .unwrap(),
            0
        );
        let fresh = super::tests::pool().await;
        initialize(&fresh).await.unwrap();
        sqlx::query("DROP INDEX launcher_sessions_user").execute(&fresh).await.unwrap();
        assert!(initialize(&fresh).await.unwrap_err().to_string().contains("launcher constraints"));
        assert!(validate(&fresh).await.is_err());
    }
    #[tokio::test]
    async fn legacy_mixed_and_future_stores_are_refused_without_creating_or_rewriting_tables() {
        let pool = pool().await;
        sqlx::raw_sql(
            "CREATE TABLE users (id INTEGER, username TEXT); INSERT INTO users VALUES (42, 'SyntheticLegacy'); PRAGMA user_version=77;",
        )
        .execute(&pool)
        .await
        .unwrap();
        assert!(initialize(&pool).await.unwrap_err().to_string().contains("explicit import"));
        let tables: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_master WHERE type='table'").fetch_one(&pool).await.unwrap();
        assert_eq!(tables, 1);
        let legacy: (i64, String) = sqlx::query_as("SELECT id, username FROM users").fetch_one(&pool).await.unwrap();
        assert_eq!(legacy, (42, "SyntheticLegacy".into()));
        let version: i64 = sqlx::query_scalar("PRAGMA user_version").fetch_one(&pool).await.unwrap();
        assert_eq!(version, 77);
        let fresh = super::tests::pool().await;
        initialize(&fresh).await.unwrap();
        sqlx::query("UPDATE velora_auth_schema SET version=999").execute(&fresh).await.unwrap();
        assert!(initialize(&fresh).await.unwrap_err().to_string().contains("unsupported"));
        assert_eq!(sqlx::query_scalar::<_, i64>("SELECT version FROM velora_auth_schema").fetch_one(&fresh).await.unwrap(), 999);
    }
    #[tokio::test]
    async fn incomplete_or_foreign_owned_stores_are_refused_instead_of_silently_repaired() {
        let pool = pool().await;
        initialize(&pool).await.unwrap();
        sqlx::query("DROP TRIGGER immutable_player_uuid").execute(&pool).await.unwrap();
        assert!(initialize(&pool).await.unwrap_err().to_string().contains("identity constraints"));
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM sqlite_master WHERE name='immutable_player_uuid'")
                .fetch_one(&pool)
                .await
                .unwrap(),
            0
        );
        sqlx::query("CREATE TABLE private_gameplay (id INTEGER)").execute(&pool).await.unwrap();
        assert!(initialize(&pool).await.is_err());
        sqlx::raw_sql("DROP TABLE private_gameplay; DROP TABLE password_resets;").execute(&pool).await.unwrap();
        assert!(initialize(&pool).await.is_err());
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM sqlite_master WHERE name='password_resets'").fetch_one(&pool).await.unwrap(),
            0
        );
    }
    #[tokio::test]
    async fn disabled_foreign_keys_are_rejected_before_creating_tables() {
        let options = sqlx::sqlite::SqliteConnectOptions::new().in_memory(true).foreign_keys(false);
        let pool = sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect_with(options).await.unwrap();
        assert!(initialize(&pool).await.unwrap_err().to_string().contains("foreign keys"));
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM sqlite_master WHERE type='table'").fetch_one(&pool).await.unwrap(),
            0
        );
    }
}
