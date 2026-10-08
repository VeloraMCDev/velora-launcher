//! Authority-owned SQL behind supplied-pool and open-transaction compatibility adapters.
//! Import/migrations and service cutover are separate; this never opens a foreign database.
use crate::identity::IdentityRecord;
use sqlx::SqlitePool;

pub struct NewAccount<'a> {
    pub username: &'a str,
    pub password_hash: &'a str,
    pub email: Option<&'a str>,
    pub role: &'a str,
    pub status: &'a str,
    pub created_at: &'a str,
    pub uuid: &'a str,
}

/// Callers still validate supplied names and hash passwords before insertion.
pub async fn insert(pool: &SqlitePool, account: NewAccount<'_>) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar(
        "INSERT INTO users (username, password_hash, email, role, status, created_at, uuid) VALUES (?, ?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(account.username)
    .bind(account.password_hash)
    .bind(account.email.map(str::trim).filter(|email| !email.is_empty()))
    .bind(account.role)
    .bind(account.status)
    .bind(account.created_at)
    .bind(account.uuid)
    .fetch_one(pool)
    .await
}

pub async fn has_admin(pool: &SqlitePool) -> Result<bool, sqlx::Error> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE role = 'admin'").fetch_one(pool).await?;
    Ok(count > 0)
}

/// One SQL statement arbitrates concurrent bootstrap attempts. Never promote or overwrite an existing account.
pub async fn insert_first_admin(pool: &SqlitePool, account: NewAccount<'_>) -> Result<Option<i64>, sqlx::Error> {
    sqlx::query_scalar(
        "INSERT INTO users (username,password_hash,email,role,status,created_at,uuid)
        SELECT ?,?,NULL,'admin','active',?,? WHERE NOT EXISTS(SELECT 1 FROM users WHERE role='admin') RETURNING id",
    )
    .bind(account.username)
    .bind(account.password_hash)
    .bind(account.created_at)
    .bind(account.uuid)
    .fetch_optional(pool)
    .await
}

pub async fn find_by_name(pool: &SqlitePool, name: &str) -> Result<Option<IdentityRecord>, sqlx::Error> {
    sqlx::query_as("SELECT * FROM users WHERE username = ?").bind(name.trim()).fetch_optional(pool).await
}

pub async fn find_by_id(pool: &SqlitePool, id: i64) -> Result<Option<IdentityRecord>, sqlx::Error> {
    sqlx::query_as("SELECT * FROM users WHERE id = ?").bind(id).fetch_optional(pool).await
}

pub async fn groups(pool: &SqlitePool, user_id: i64) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT g.name FROM groups g JOIN user_groups ug ON ug.group_id = g.id WHERE ug.user_id = ? ORDER BY g.name")
        .bind(user_id)
        .fetch_all(pool)
        .await
}

pub async fn find_by_uuid(pool: &SqlitePool, uuid: &str) -> Result<Option<IdentityRecord>, sqlx::Error> {
    sqlx::query_as("SELECT * FROM users WHERE uuid = ?").bind(uuid).fetch_optional(pool).await
}
pub async fn find_by_login(pool: &SqlitePool, login: &str) -> Result<Option<IdentityRecord>, sqlx::Error> {
    sqlx::query_as("SELECT * FROM users WHERE username = ? OR (email IS NOT NULL AND email = ?)")
        .bind(login.trim())
        .bind(login.trim())
        .fetch_optional(pool)
        .await
}
pub async fn clear_skin(pool: &SqlitePool, user_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE users SET skin_hash = NULL WHERE id = ?").bind(user_id).execute(pool).await?;
    Ok(())
}
pub async fn set_skin_model(pool: &SqlitePool, user_id: i64, model: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE users SET skin_model = ? WHERE id = ?").bind(model).bind(user_id).execute(pool).await?;
    Ok(())
}

/// The compatibility host supplies an open transaction so its audit/session cleanup stays atomic.
/// Name validation, password verification and the live server check happen before this mutation.
pub async fn rename_in_transaction(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, user_id: i64, name: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE users SET username=?, auth_version=auth_version+1 WHERE id=?").bind(name).bind(user_id).execute(&mut **tx).await?;
    Ok(())
}
pub async fn revoke_game_sessions_in_transaction(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, user_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM ygg_tokens WHERE user_id=?").bind(user_id).execute(&mut **tx).await?;
    sqlx::query("DELETE FROM ygg_sessions WHERE user_id=?").bind(user_id).execute(&mut **tx).await?;
    Ok(())
}
pub async fn set_skin(pool: &SqlitePool, user_id: i64, hash: &str, model: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE users SET skin_hash = ?, skin_model = ? WHERE id = ?").bind(hash).bind(model).bind(user_id).execute(pool).await?;
    Ok(())
}
pub async fn set_cape(pool: &SqlitePool, user_id: i64, cape_id: Option<i64>) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE users SET cape_id = ? WHERE id = ?").bind(cape_id).bind(user_id).execute(pool).await?;
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) async fn synthetic_store() -> SqlitePool {
        let pool = sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        // Synthetic projection fixture, not a production migration or imported account database.
        sqlx::raw_sql(
            "CREATE TABLE users (id INTEGER PRIMARY KEY, auth_version INTEGER NOT NULL DEFAULT 0,
            username TEXT NOT NULL COLLATE NOCASE UNIQUE, password_hash TEXT NOT NULL, email TEXT,
            role TEXT NOT NULL, status TEXT NOT NULL, created_at TEXT NOT NULL, last_login TEXT,
            uuid TEXT NOT NULL UNIQUE, skin_hash TEXT, skin_model TEXT NOT NULL DEFAULT 'classic',
            cape_id INTEGER, status_reason TEXT);
            CREATE TABLE groups (id INTEGER PRIMARY KEY, name TEXT NOT NULL);
            CREATE TABLE user_groups (user_id INTEGER, group_id INTEGER);",
        )
        .execute(&pool)
        .await
        .unwrap();
        pool
    }

    #[tokio::test]
    async fn owned_queries_preserve_projection_normalization_groups_and_live_revocation() {
        let pool = synthetic_store().await;
        let account = || NewAccount {
            username: "ExamplePlayer",
            password_hash: "synthetic-hash",
            email: Some(" example@example.invalid "),
            role: "member",
            status: "active",
            created_at: "2026-01-01T00:00:00Z",
            uuid: "01234567-89ab-4def-8123-456789abcdef",
        };
        let id = insert(&pool, account()).await.unwrap();
        assert!(insert(&pool, account()).await.is_err());
        let user = find_by_name(&pool, " exampleplayer ").await.unwrap().unwrap();
        assert_eq!(user.id, id);
        assert_eq!(user.email.as_deref(), Some("example@example.invalid"));
        assert_eq!(user.uuid, account().uuid);
        assert_eq!(user.auth_version, 0);
        assert!(find_by_id(&pool, id + 99).await.unwrap().is_none());
        assert!(groups(&pool, id).await.unwrap().is_empty());
        sqlx::raw_sql(
            "INSERT INTO groups VALUES (1, 'Readers'), (2, 'Builders');
            INSERT INTO user_groups VALUES (1, 1), (1, 2);",
        )
        .execute(&pool)
        .await
        .unwrap();
        assert_eq!(groups(&pool, id).await.unwrap(), ["Builders", "Readers"]);
        sqlx::query("UPDATE users SET status='disabled', auth_version=auth_version+1 WHERE id=?").bind(id).execute(&pool).await.unwrap();
        let fresh = find_by_id(&pool, id).await.unwrap().unwrap();
        assert_eq!((fresh.status.as_str(), fresh.auth_version), ("disabled", 1));
        assert!(crate::admission::session_admission(
            0,
            Some(crate::admission::AccountStatus { status: &fresh.status, auth_version: fresh.auth_version })
        )
        .is_err());
    }

    #[tokio::test]
    async fn authority_unavailability_propagates_instead_of_returning_an_active_account() {
        let pool = synthetic_store().await;
        pool.close().await;
        assert!(matches!(find_by_id(&pool, 1).await, Err(sqlx::Error::PoolClosed)));
        assert!(groups(&pool, 1).await.is_err());
    }

    #[tokio::test]
    async fn login_uuid_and_cosmetic_queries_preserve_identity_and_field_updates() {
        let pool = synthetic_store().await;
        let uuid = "01234567-89ab-4def-8123-456789abcdef";
        let id = insert(
            &pool,
            NewAccount {
                username: "ExamplePlayer",
                password_hash: "synthetic-hash",
                email: Some("example@example.invalid"),
                role: "member",
                status: "active",
                created_at: "2026-01-01T00:00:00Z",
                uuid,
            },
        )
        .await
        .unwrap();
        for login in [" exampleplayer ", " example@example.invalid "] {
            assert_eq!(find_by_login(&pool, login).await.unwrap().unwrap().id, id);
        }
        assert_eq!(find_by_uuid(&pool, uuid).await.unwrap().unwrap().id, id);
        assert!(find_by_login(&pool, "missing").await.unwrap().is_none());
        assert!(find_by_uuid(&pool, "missing").await.unwrap().is_none());
        set_skin(&pool, id, "synthetic-hash", "slim").await.unwrap();
        set_cape(&pool, id, Some(42)).await.unwrap();
        let user = find_by_id(&pool, id).await.unwrap().unwrap();
        assert_eq!(user.skin_hash.as_deref(), Some("synthetic-hash"));
        assert_eq!(user.skin_model, "slim");
        assert_eq!(user.cape_id, Some(42));
        clear_skin(&pool, id).await.unwrap();
        set_cape(&pool, id, None).await.unwrap();
        let user = find_by_id(&pool, id).await.unwrap().unwrap();
        assert!(user.skin_hash.is_none());
        assert_eq!(user.skin_model, "slim");
        assert!(user.cape_id.is_none());
        assert_eq!(user.uuid, uuid);
        set_skin_model(&pool, id, "classic").await.unwrap();
        let user = find_by_id(&pool, id).await.unwrap().unwrap();
        assert_eq!(user.skin_model, "classic");
        assert!(user.skin_hash.is_none());
        assert_eq!(user.uuid, uuid);
        pool.close().await;
        assert!(find_by_login(&pool, "ExamplePlayer").await.is_err());
        assert!(set_skin(&pool, id, "hash", "classic").await.is_err());
        assert!(set_cape(&pool, id, None).await.is_err());
    }
}

#[cfg(test)]
mod rename_tests {
    use super::*;
    #[tokio::test]
    async fn rename_revocation_stays_atomic_with_host_cleanup_and_preserves_other_users() {
        let pool = tests::synthetic_store().await;
        sqlx::raw_sql(
            "INSERT INTO users(id,username,password_hash,role,status,created_at,uuid) VALUES
            (1,'Before','synthetic','member','active','fixture','uuid-one'),
            (2,'Other','synthetic','member','active','fixture','uuid-two');
            CREATE TABLE ygg_tokens (user_id INTEGER, value TEXT);
            CREATE TABLE ygg_sessions (user_id INTEGER, value TEXT);
            CREATE TABLE host_audit (detail TEXT NOT NULL);
            INSERT INTO ygg_tokens VALUES(1,'one'),(2,'two');
            INSERT INTO ygg_sessions VALUES(1,'one'),(2,'two');",
        )
        .execute(&pool)
        .await
        .unwrap();
        // Simulate a host-side audit failure after authority mutations. Roll back everything.
        let mut tx = pool.begin().await.unwrap();
        rename_in_transaction(&mut tx, 1, "After").await.unwrap();
        revoke_game_sessions_in_transaction(&mut tx, 1).await.unwrap();
        assert!(sqlx::query("INSERT INTO host_audit VALUES(NULL)").execute(&mut *tx).await.is_err());
        tx.rollback().await.unwrap();
        let original = find_by_id(&pool, 1).await.unwrap().unwrap();
        assert_eq!((original.username.as_str(), original.auth_version, original.uuid.as_str()), ("Before", 0, "uuid-one"));
        for table in ["ygg_tokens", "ygg_sessions"] {
            let count: i64 = sqlx::query_scalar(&format!("SELECT count(*) FROM {table}")).fetch_one(&pool).await.unwrap();
            assert_eq!(count, 2);
        }
        let mut tx = pool.begin().await.unwrap();
        rename_in_transaction(&mut tx, 1, "After").await.unwrap();
        revoke_game_sessions_in_transaction(&mut tx, 1).await.unwrap();
        sqlx::query("INSERT INTO host_audit VALUES('synthetic rename')").execute(&mut *tx).await.unwrap();
        tx.commit().await.unwrap();
        let renamed = find_by_id(&pool, 1).await.unwrap().unwrap();
        assert_eq!((renamed.username.as_str(), renamed.auth_version, renamed.uuid.as_str()), ("After", 1, "uuid-one"));
        assert_eq!(find_by_id(&pool, 2).await.unwrap().unwrap().username, "Other");
        for table in ["ygg_tokens", "ygg_sessions"] {
            let ids: Vec<i64> = sqlx::query_scalar(&format!("SELECT user_id FROM {table}")).fetch_all(&pool).await.unwrap();
            assert_eq!(ids, [2]);
        }
        // A uniqueness conflict cannot replace another identity or revoke its sessions.
        let mut tx = pool.begin().await.unwrap();
        assert!(rename_in_transaction(&mut tx, 1, "Other").await.is_err());
        tx.rollback().await.unwrap();
        assert_eq!(find_by_id(&pool, 1).await.unwrap().unwrap().auth_version, 1);
    }
}
