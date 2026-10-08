//! Yggdrasil credential storage. The host retains HTTP/profile/IP adapters.
use crate::identity::IdentityRecord as UserRow;
use rand::RngCore;
use sqlx::SqlitePool;
const TOKEN_DAYS: i64 = 30;
const MAX_TOKENS_PER_USER: i64 = 10;
fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn random_token() -> String {
    let mut b = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut b);
    hex::encode(b)
}

/// Issue a game access token for `user_id`.
pub async fn issue_token(pool: &SqlitePool, user_id: i64, client_token: Option<String>) -> Result<(String, String), sqlx::Error> {
    let access = random_token();
    let client = client_token.filter(|c| !c.is_empty() && c.len() <= 128).unwrap_or_else(random_token);
    let issued_at = chrono::Utc::now();
    sqlx::query("DELETE FROM ygg_tokens WHERE expires_at < ?").bind(now()).execute(pool).await?;
    sqlx::query("INSERT INTO ygg_tokens (access_token, client_token, user_id, created_at, expires_at) VALUES (?, ?, ?, ?, ?)")
        .bind(&access)
        .bind(&client)
        .bind(user_id)
        .bind(now())
        .bind((issued_at + chrono::Duration::days(TOKEN_DAYS)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
        .execute(pool)
        .await?;
    // Keep only the newest few sessions per account.
    sqlx::query(
        "DELETE FROM ygg_tokens WHERE user_id = ? AND access_token NOT IN
         (SELECT access_token FROM ygg_tokens WHERE user_id = ? ORDER BY created_at DESC LIMIT ?)",
    )
    .bind(user_id)
    .bind(user_id)
    .bind(MAX_TOKENS_PER_USER)
    .execute(pool)
    .await?;
    Ok((access, client))
}

/// The active account behind a valid token.
pub async fn token_user(pool: &SqlitePool, access: &str, client: Option<&str>) -> Result<Option<UserRow>, sqlx::Error> {
    let row: Option<(i64, String)> =
        sqlx::query_as("SELECT user_id, client_token FROM ygg_tokens WHERE access_token = ? AND expires_at > ?")
            .bind(access)
            .bind(now())
            .fetch_optional(pool)
            .await?;
    let Some((user_id, client_token)) = row else { return Ok(None) };
    if client.is_some_and(|c| !c.is_empty() && c != client_token) {
        return Ok(None);
    }
    let user: Option<UserRow> = sqlx::query_as("SELECT * FROM users WHERE id = ?").bind(user_id).fetch_optional(pool).await?;
    Ok(user.filter(|u| u.status == "active"))
}

pub async fn client_token(pool: &SqlitePool, access: &str) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT client_token FROM ygg_tokens WHERE access_token = ?").bind(access).fetch_optional(pool).await
}
pub async fn invalidate(pool: &SqlitePool, access: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM ygg_tokens WHERE access_token = ?").bind(access).execute(pool).await?;
    Ok(())
}
pub async fn signout(pool: &SqlitePool, user_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM ygg_tokens WHERE user_id = ?").bind(user_id).execute(pool).await?;
    Ok(())
}
pub async fn join(pool: &SqlitePool, server_id: &str, user_id: i64, ip: Option<String>, cutoff: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM ygg_sessions WHERE created_at < ?").bind(cutoff).execute(pool).await?;
    sqlx::query("INSERT OR REPLACE INTO ygg_sessions (server_id, user_id, ip, created_at) VALUES (?, ?, ?, ?)")
        .bind(server_id)
        .bind(user_id)
        .bind(ip)
        .bind(now())
        .execute(pool)
        .await?;
    Ok(())
}
pub async fn joined(pool: &SqlitePool, server_id: &str, cutoff: &str) -> Result<Option<(i64, Option<String>)>, sqlx::Error> {
    sqlx::query_as("SELECT user_id, ip FROM ygg_sessions WHERE server_id = ? AND created_at >= ?")
        .bind(server_id)
        .bind(cutoff)
        .fetch_optional(pool)
        .await
}
pub async fn active_user(pool: &SqlitePool, user_id: i64) -> Result<Option<UserRow>, sqlx::Error> {
    sqlx::query_as("SELECT * FROM users WHERE id = ? AND status = 'active'").bind(user_id).fetch_optional(pool).await
}

#[cfg(test)]
mod tests {
    use super::*;
    async fn fixture() -> (SqlitePool, i64) {
        let pool = crate::identity_store::tests::synthetic_store().await;
        sqlx::raw_sql("CREATE TABLE ygg_tokens (access_token TEXT PRIMARY KEY, client_token TEXT NOT NULL, user_id INTEGER NOT NULL, created_at TEXT NOT NULL, expires_at TEXT NOT NULL);
            CREATE TABLE ygg_sessions (server_id TEXT PRIMARY KEY, user_id INTEGER NOT NULL, ip TEXT, created_at TEXT NOT NULL);").execute(&pool).await.unwrap();
        let id = crate::identity_store::insert(
            &pool,
            crate::identity_store::NewAccount {
                username: "ExamplePlayer",
                password_hash: "synthetic-hash",
                email: None,
                role: "member",
                status: "active",
                created_at: "2026-01-01T00:00:00Z",
                uuid: "01234567-89ab-4def-8123-456789abcdef",
            },
        )
        .await
        .unwrap();
        (pool, id)
    }
    #[tokio::test]
    async fn token_lifetime_client_matching_revocation_and_invalidation() {
        let (pool, id) = fixture().await;
        let (access, client) = issue_token(&pool, id, Some("launcher-instance".into())).await.unwrap();
        assert_eq!(access.len(), 32);
        assert!(access.bytes().all(|b| b.is_ascii_hexdigit()));
        assert_eq!(client, "launcher-instance");
        assert_eq!(client_token(&pool, &access).await.unwrap(), Some(client.clone()));
        assert_eq!(token_user(&pool, &access, Some(&client)).await.unwrap().unwrap().id, id);
        assert!(token_user(&pool, &access, Some("wrong-client")).await.unwrap().is_none());
        assert!(token_user(&pool, &access, Some("")).await.unwrap().is_some());
        let (created, expires): (String, String) = sqlx::query_as("SELECT created_at, expires_at FROM ygg_tokens WHERE access_token=?")
            .bind(&access)
            .fetch_one(&pool)
            .await
            .unwrap();
        let lifetime = (chrono::DateTime::parse_from_rfc3339(&expires).unwrap() - chrono::DateTime::parse_from_rfc3339(&created).unwrap())
            .num_seconds();
        assert!((30 * 86400 - 2..=30 * 86400).contains(&lifetime));
        sqlx::query("UPDATE users SET status='disabled' WHERE id=?").bind(id).execute(&pool).await.unwrap();
        assert!(token_user(&pool, &access, None).await.unwrap().is_none());
        sqlx::query("UPDATE users SET status='active' WHERE id=?").bind(id).execute(&pool).await.unwrap();
        sqlx::query("UPDATE ygg_tokens SET expires_at='2000-01-01T00:00:00Z' WHERE access_token=?")
            .bind(&access)
            .execute(&pool)
            .await
            .unwrap();
        assert!(token_user(&pool, &access, None).await.unwrap().is_none());
        let (fresh, generated) = issue_token(&pool, id, Some("x".repeat(129))).await.unwrap();
        assert_eq!(generated.len(), 32);
        assert!(client_token(&pool, &access).await.unwrap().is_none());
        invalidate(&pool, &fresh).await.unwrap();
        invalidate(&pool, &fresh).await.unwrap();
        assert!(token_user(&pool, &fresh, None).await.unwrap().is_none());
    }
    #[tokio::test]
    async fn account_token_cap_and_signout_preserve_other_accounts() {
        let (pool, id) = fixture().await;
        let (other, _) = issue_token(&pool, id + 1, None).await.unwrap();
        for _ in 0..12 {
            issue_token(&pool, id, None).await.unwrap();
        }
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM ygg_tokens WHERE user_id=?").bind(id).fetch_one(&pool).await.unwrap();
        assert_eq!(count, 10);
        signout(&pool, id).await.unwrap();
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM ygg_tokens WHERE user_id=?").bind(id).fetch_one(&pool).await.unwrap();
        assert_eq!(count, 0);
        assert!(client_token(&pool, &other).await.unwrap().is_some());
    }
    #[tokio::test]
    async fn join_replacement_expiration_live_account_check_and_outage() {
        let (pool, id) = fixture().await;
        let cutoff = "2000-01-01T00:00:00Z";
        join(&pool, "synthetic-server", id, Some("192.0.2.1".into()), cutoff).await.unwrap();
        assert_eq!(joined(&pool, "synthetic-server", cutoff).await.unwrap(), Some((id, Some("192.0.2.1".into()))));
        join(&pool, "synthetic-server", id, None, cutoff).await.unwrap();
        assert_eq!(joined(&pool, "synthetic-server", cutoff).await.unwrap(), Some((id, None)));
        assert!(joined(&pool, "synthetic-server", "9999-01-01T00:00:00Z").await.unwrap().is_none());
        assert!(active_user(&pool, id).await.unwrap().is_some());
        sqlx::query("UPDATE users SET status='pending' WHERE id=?").bind(id).execute(&pool).await.unwrap();
        assert!(active_user(&pool, id).await.unwrap().is_none());
        sqlx::query("UPDATE ygg_sessions SET created_at='1999-01-01T00:00:00Z'").execute(&pool).await.unwrap();
        join(&pool, "next-server", id, None, cutoff).await.unwrap();
        assert!(joined(&pool, "synthetic-server", cutoff).await.unwrap().is_none());
        pool.close().await;
        assert!(token_user(&pool, "cached", None).await.is_err());
        assert!(joined(&pool, "next-server", cutoff).await.is_err());
        assert!(active_user(&pool, id).await.is_err());
        assert!(issue_token(&pool, id, None).await.is_err());
    }
}
