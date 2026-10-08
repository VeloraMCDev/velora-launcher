//! Explicit authority-only import from a caller-supplied, offline read-only checkpoint.
use anyhow::{bail, Context, Result};
use sqlx::{Row, SqlitePool};

struct Table {
    name: &'static str,
    columns: &'static str,
    integers: &'static [&'static str],
}
// Fixed projections exclude activity, private gameplay and all legacy triggers.
const TABLES: &[Table] = &[
    Table { name: "reserved_usernames", columns: "name,uuid", integers: &[] },
    Table { name: "users", columns: "id,username,password_hash,email,role,status,created_at,last_login,uuid,skin_hash,skin_model,cape_id,status_reason,auth_version,email_optout", integers: &["id", "cape_id", "auth_version", "email_optout"] },
    Table { name: "groups", columns: "id,name,color,luckperms_group,discord_role", integers: &["id"] },
    Table { name: "user_groups", columns: "user_id,group_id", integers: &["user_id", "group_id"] },
    Table { name: "capes", columns: "id,name,hash,visibility,allowed_groups,created_at", integers: &["id"] },
    Table { name: "ygg_tokens", columns: "access_token,client_token,user_id,created_at,expires_at", integers: &["user_id"] },
    Table { name: "ygg_sessions", columns: "server_id,user_id,ip,created_at", integers: &["user_id"] },
    Table { name: "player_keys", columns: "user_id,private_pem,public_pem,signature_v1,signature_v2,expires_at,refreshed_after", integers: &["user_id"] },
    Table { name: "account_connections", columns: "user_id,provider,provider_id,display_name,created_at", integers: &["user_id"] },
    Table { name: "oauth_attempts", columns: "state,kind,user_id,created_at,expires_at,result,consumed_at", integers: &["user_id"] },
    Table { name: "password_resets", columns: "token_hash,user_id,expires_at,used_at", integers: &["user_id"] },
    Table { name: "launcher_sessions", columns: "id,user_id,ip,username,created_at", integers: &["id", "user_id"] },
];

/// Counts contain no credentials or player data; keep checkpoint material protected.
#[derive(Debug, PartialEq, Eq)]
pub struct ImportCounts(pub Vec<(&'static str, usize)>);

/// Import only into an initialized, empty destination, in one transaction.
/// The source must be a closed, consistent backup opened with query_only enabled.
/// This does not open files, copy signing assets, stop writers or perform cutover.
pub async fn import_checkpoint(source: &SqlitePool, destination: &SqlitePool) -> Result<ImportCounts> {
    crate::schema::initialize(destination).await?;
    let mut checkpoint = source.begin().await?;
    let read_only: i64 = sqlx::query_scalar("PRAGMA query_only").fetch_one(&mut *checkpoint).await?;
    if read_only != 1 {
        bail!("import source must be an offline checkpoint with query_only enabled on every connection");
    }
    let mut target = destination.begin().await?;
    for table in TABLES {
        let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {}", table.name)).fetch_one(&mut *target).await?;
        if count != 0 {
            bail!("import destination must be empty; refusing to merge or overwrite existing authority records");
        }
    }
    let blank: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE uuid = '' OR uuid IS NULL").fetch_one(&mut *checkpoint).await?;
    if blank != 0 {
        bail!("checkpoint has uninitialized player UUIDs; complete legacy identity preparation before taking the checkpoint");
    }
    let mut counts = Vec::new();
    for table in TABLES {
        let select = format!("SELECT {} FROM {} ORDER BY {}", table.columns, table.name, table.columns);
        let records =
            sqlx::query(&select).fetch_all(&mut *checkpoint).await.with_context(|| format!("reading checkpoint table {}", table.name))?;
        let columns: Vec<&str> = table.columns.split(',').collect();
        let placeholders = vec!["?"; columns.len()].join(",");
        let insert = format!("INSERT INTO {} ({}) VALUES ({})", table.name, table.columns, placeholders);
        for record in &records {
            let mut query = sqlx::query(&insert);
            for column in &columns {
                query = if table.integers.contains(column) {
                    query.bind(record.try_get::<Option<i64>, _>(*column)?)
                } else {
                    query.bind(record.try_get::<Option<String>, _>(*column)?)
                };
            }
            query.execute(&mut *target).await.with_context(|| format!("importing checkpoint table {}", table.name))?;
        }
        counts.push((table.name, records.len()));
    }
    // Preserve high-water marks: deleted account/group/cape IDs must not be reused.
    for name in ["users", "groups", "capes", "launcher_sessions"] {
        let sequences: Vec<i64> =
            sqlx::query_scalar("SELECT seq FROM sqlite_sequence WHERE name=?").bind(name).fetch_all(&mut *checkpoint).await?;
        if sequences.len() > 1 {
            bail!("checkpoint has duplicate sequence entries for {}", name);
        }
        let maximum: i64 = sqlx::query_scalar(&format!("SELECT COALESCE(MAX(id), 0) FROM {}", name)).fetch_one(&mut *target).await?;
        let sequence = sequences.first().copied().unwrap_or(0);
        if sequence < maximum || sequence < 0 {
            bail!("checkpoint sequence is invalid for {}", name);
        }
        sqlx::query("DELETE FROM sqlite_sequence WHERE name=?").bind(name).execute(&mut *target).await?;
        sqlx::query("INSERT INTO sqlite_sequence(name,seq) VALUES(?,?)").bind(name).bind(sequence).execute(&mut *target).await?;
    }
    // Verify exact projected values, including credentials/key bytes, before commit.
    for table in TABLES {
        let select = format!("SELECT {} FROM {} ORDER BY {}", table.columns, table.name, table.columns);
        let original = sqlx::query(&select).fetch_all(&mut *checkpoint).await?;
        let imported = sqlx::query(&select).fetch_all(&mut *target).await?;
        if original.len() != imported.len() {
            bail!("checkpoint count mismatch for {}", table.name);
        }
        for (before, after) in original.iter().zip(&imported) {
            for column in table.columns.split(',') {
                let equal = if table.integers.contains(&column) {
                    before.try_get::<Option<i64>, _>(column)? == after.try_get::<Option<i64>, _>(column)?
                } else {
                    before.try_get::<Option<String>, _>(column)? == after.try_get::<Option<String>, _>(column)?
                };
                if !equal {
                    bail!("checkpoint value mismatch for {}", table.name);
                }
            }
        }
    }
    let violations = sqlx::query("PRAGMA foreign_key_check").fetch_all(&mut *target).await?;
    if !violations.is_empty() {
        bail!("imported checkpoint violates authority foreign keys");
    }
    checkpoint.commit().await?;
    target.commit().await?;
    Ok(ImportCounts(counts))
}

#[cfg(test)]
mod tests {
    use super::*;
    type SessionProjection = (i64, Option<i64>, String, Option<String>, String);
    async fn pool() -> SqlitePool {
        let options = sqlx::sqlite::SqliteConnectOptions::new().in_memory(true).foreign_keys(true);
        sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect_with(options).await.unwrap()
    }
    async fn checkpoint() -> SqlitePool {
        let source = pool().await;
        crate::schema::initialize(&source).await.unwrap();
        sqlx::raw_sql("INSERT INTO users(id,username,password_hash,email,role,status,created_at,uuid,auth_version,email_optout) VALUES(42,'SyntheticPlayer','synthetic-argon2-bytes','player@example.invalid','admin','active','2026-01-01','01234567-89ab-4def-8123-456789abcdef',7,1);
            UPDATE users SET username='Renamed' WHERE id=42;
            INSERT INTO groups VALUES(9,'SyntheticGroup','#abcdef','synthetic-permission','synthetic-role');
            INSERT INTO user_groups VALUES(42,9);
            INSERT INTO capes VALUES(12,'SyntheticCape','synthetic-texture-hash','private','[\"SyntheticGroup\"]','2026-01-01');
            UPDATE users SET cape_id=12,skin_hash='synthetic-skin-hash',skin_model='slim',status_reason='Synthetic reason',last_login='2026-02-01' WHERE id=42;
            INSERT INTO player_keys VALUES(42,'synthetic-private\nkey','synthetic-public\nkey','synthetic-v1','synthetic-v2','2999-01-01','2998-01-01');
            INSERT INTO ygg_tokens VALUES('synthetic-game-token','synthetic-client',42,'2026-01-01','2999-01-01');
            INSERT INTO ygg_sessions VALUES('synthetic-join',42,NULL,'2026-01-01');
            INSERT INTO password_resets VALUES('synthetic-hashed-reset',42,'2999-01-01',NULL);
            INSERT INTO account_connections VALUES(42,'discord','synthetic-provider-id','SyntheticDisplay','2026-01-01');
            INSERT INTO oauth_attempts VALUES('synthetic-oauth-state','link',42,'2026-01-01','2999-01-01','synthetic-result',NULL);
            INSERT INTO users(id,username,password_hash,created_at,uuid) VALUES(99,'DeletedSyntheticPlayer','synthetic-hash','2026-01-01','99999999-89ab-4def-8123-456789abcdef');
            DELETE FROM users WHERE id=99;
            INSERT INTO launcher_sessions(id,user_id,ip,username,created_at) VALUES
                (123,42,' ::ffff:203.0.113.8 ','HistoricalName','2026-01-02'),
                (124,NULL,'2001:db8::1',NULL,'2026-01-01'),
                (999,NULL,'198.51.100.1','DeletedSession','2026-01-01');
            DELETE FROM launcher_sessions WHERE id=999;
            CREATE TABLE private_gameplay(id INTEGER); INSERT INTO private_gameplay VALUES(99);
            PRAGMA query_only=ON;").execute(&source).await.unwrap();
        source
    }
    #[tokio::test]
    async fn imports_exact_identity_history_credentials_preferences_and_keys_without_gameplay() {
        let source = checkpoint().await;
        let target = pool().await;
        let counts = import_checkpoint(&source, &target).await.unwrap();
        assert!(counts.0.contains(&("reserved_usernames", 3)));
        assert!(counts.0.contains(&("launcher_sessions", 2)));
        let sessions: Vec<SessionProjection> =
            sqlx::query_as("SELECT id,user_id,ip,username,created_at FROM launcher_sessions ORDER BY id").fetch_all(&target).await.unwrap();
        assert_eq!(
            sessions,
            vec![
                (123, Some(42), " ::ffff:203.0.113.8 ".into(), Some("HistoricalName".into()), "2026-01-02".into()),
                (124, None, "2001:db8::1".into(), None, "2026-01-01".into())
            ]
        );
        assert!(crate::launcher_sessions::verify_ip(&target, Some(42), Some("203.0.113.8"), "2026-01-02").await.unwrap());
        assert!(crate::launcher_sessions::verify_ip(&target, None, Some("2001:db8::1"), "2026-01-01").await.unwrap());
        let identity: (i64, String, String, i64, i64) =
            sqlx::query_as("SELECT id,uuid,password_hash,auth_version,email_optout FROM users").fetch_one(&target).await.unwrap();
        assert_eq!(identity, (42, "01234567-89ab-4def-8123-456789abcdef".into(), "synthetic-argon2-bytes".into(), 7, 1));
        assert_eq!(
            sqlx::query_scalar::<_, String>("SELECT private_pem FROM player_keys").fetch_one(&target).await.unwrap(),
            "synthetic-private\nkey"
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM sqlite_master WHERE name='private_gameplay'")
                .fetch_one(&target)
                .await
                .unwrap(),
            0
        );
        assert_eq!(sqlx::query_scalar::<_, i64>("SELECT id FROM private_gameplay").fetch_one(&source).await.unwrap(), 99);
        assert!(sqlx::query("UPDATE users SET uuid='changed'").execute(&target).await.is_err());
        assert!(import_checkpoint(&source, &target).await.unwrap_err().to_string().contains("must be empty"));
        let next: i64 = sqlx::query_scalar("INSERT INTO users(username,password_hash,created_at,uuid) VALUES('NextSyntheticPlayer','synthetic-hash','2026-01-01','88888888-89ab-4def-8123-456789abcdef') RETURNING id")
            .fetch_one(&target).await.unwrap();
        assert_eq!(next, 100);
        let session_id: i64 =
            sqlx::query_scalar("INSERT INTO launcher_sessions(ip,created_at) VALUES('203.0.113.9','2026-01-03') RETURNING id")
                .fetch_one(&target)
                .await
                .unwrap();
        assert_eq!(session_id, 1000);
    }
    #[tokio::test]
    async fn writable_source_and_incomplete_checkpoint_are_refused_with_no_partial_import() {
        let writable = pool().await;
        crate::schema::initialize(&writable).await.unwrap();
        let target = pool().await;
        assert!(import_checkpoint(&writable, &target).await.unwrap_err().to_string().contains("query_only"));
        let source = checkpoint().await;
        sqlx::raw_sql("PRAGMA query_only=OFF; DROP TABLE password_resets; PRAGMA query_only=ON;").execute(&source).await.unwrap();
        assert!(import_checkpoint(&source, &target).await.is_err());
        assert_eq!(sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users").fetch_one(&target).await.unwrap(), 0);
        assert_eq!(sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM reserved_usernames").fetch_one(&target).await.unwrap(), 0);
    }
    #[tokio::test]
    async fn unprepared_identity_invalid_sequences_and_orphaned_records_abort_the_whole_import() {
        for damage in [
            "DROP TRIGGER immutable_player_uuid; UPDATE users SET uuid='';",
            "UPDATE sqlite_sequence SET seq=1 WHERE name='users';",
            "UPDATE sqlite_sequence SET seq=1 WHERE name='launcher_sessions';",
            "PRAGMA foreign_keys=OFF; INSERT INTO launcher_sessions(user_id,ip,created_at) VALUES(666,'203.0.113.9','2026-01-01');",
            "PRAGMA foreign_keys=OFF; INSERT INTO user_groups VALUES(42,666);",
        ] {
            let source = checkpoint().await;
            sqlx::raw_sql(&format!("PRAGMA query_only=OFF; {damage} PRAGMA query_only=ON;")).execute(&source).await.unwrap();
            let target = pool().await;
            assert!(import_checkpoint(&source, &target).await.is_err());
            for table in TABLES {
                let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {}", table.name)).fetch_one(&target).await.unwrap();
                assert_eq!(count, 0, "{} retains partial import", table.name);
            }
        }
    }
    #[tokio::test]
    async fn version_one_checkpoint_without_launcher_history_is_not_silently_imported() {
        let source = checkpoint().await;
        sqlx::raw_sql(
            "PRAGMA query_only=OFF; DROP TABLE launcher_sessions; UPDATE velora_auth_schema SET version=1; PRAGMA query_only=ON;",
        )
        .execute(&source)
        .await
        .unwrap();
        let target = pool().await;
        assert!(import_checkpoint(&source, &target).await.unwrap_err().to_string().contains("launcher_sessions"));
        assert_eq!(sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users").fetch_one(&target).await.unwrap(), 0);
        assert_eq!(sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM reserved_usernames").fetch_one(&target).await.unwrap(), 0);
    }
}
