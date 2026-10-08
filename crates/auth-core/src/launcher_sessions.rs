//! Launcher admission records behind supplied-store compatibility adapters.
//! Panel retains event observations. No database is opened or schema adopted here.
use sqlx::{SqliteConnection, SqlitePool};
use std::net::IpAddr;

fn canonical_ip(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V4(v4) => IpAddr::V4(v4),
        IpAddr::V6(v6) => v6.to_ipv4_mapped().map(IpAddr::V4).unwrap_or(IpAddr::V6(v6)),
    }
}

pub fn ips_match(session: &str, request: &str) -> bool {
    match (session.trim().parse::<IpAddr>(), request.trim().parse::<IpAddr>()) {
        (Ok(a), Ok(b)) => canonical_ip(a) == canonical_ip(b),
        _ => false,
    }
}

/// Record verified identity first. The caller then computes the prune cutoff and
/// writes telemetry; later failures must not roll back this legacy autocommit.
pub async fn record(pool: &SqlitePool, user_id: i64, ip: &str, username: &str, created_at: &str) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO launcher_sessions (user_id, ip, username, created_at) VALUES (?, ?, ?, ?)")
        .bind(user_id)
        .bind(ip)
        .bind(username)
        .bind(created_at)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn prune(pool: &SqlitePool, cutoff: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM launcher_sessions WHERE created_at < ?").bind(cutoff).execute(pool).await?;
    Ok(())
}

/// Legacy anonymous rows form a separate admission bucket, never a user fallback.
pub async fn verify_ip(pool: &SqlitePool, user_id: Option<i64>, request_ip: Option<&str>, since: &str) -> Result<bool, sqlx::Error> {
    let request = match request_ip {
        Some(ip) if !ip.trim().is_empty() => ip.trim(),
        _ => return Ok(false),
    };
    let sessions: Vec<String> = match user_id {
        Some(id) => {
            sqlx::query_scalar("SELECT ip FROM launcher_sessions WHERE user_id = ? AND created_at >= ? ORDER BY id DESC LIMIT 50")
                .bind(id)
                .bind(since)
                .fetch_all(pool)
                .await?
        }
        None => {
            sqlx::query_scalar("SELECT ip FROM launcher_sessions WHERE user_id IS NULL AND created_at >= ? ORDER BY id DESC LIMIT 50")
                .bind(since)
                .fetch_all(pool)
                .await?
        }
    };
    Ok(sessions.iter().any(|ip| ips_match(ip, request)))
}

/// Participate in the caller's existing rename/audit transaction, including rollback.
pub async fn revoke(connection: &mut SqliteConnection, user_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM launcher_sessions WHERE user_id=?").bind(user_id).execute(connection).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    async fn record_and_prune(pool: &SqlitePool, user: i64, ip: &str, name: &str, time: &str, cutoff: &str) -> Result<(), sqlx::Error> {
        record(pool, user, ip, name, time).await?;
        prune(pool, cutoff).await
    }
    // Use the real owned schema, not a handcrafted reduced session projection.
    async fn pool() -> SqlitePool {
        let options = sqlx::sqlite::SqliteConnectOptions::new().in_memory(true).foreign_keys(true);
        let pool = sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect_with(options).await.unwrap();
        crate::schema::initialize(&pool).await.unwrap();
        sqlx::raw_sql(
            "INSERT INTO users(id,username,password_hash,created_at,uuid) VALUES
            (1,'FirstUser','synthetic-hash','2026-01-01','01234567-89ab-4def-8123-456789abcdef'),
            (2,'SecondUser','synthetic-hash','2026-01-01','11234567-89ab-4def-8123-456789abcdef');",
        )
        .execute(&pool)
        .await
        .unwrap();
        pool
    }
    async fn row(pool: &SqlitePool, user: Option<i64>, ip: &str, time: &str) {
        sqlx::query("INSERT INTO launcher_sessions(user_id,ip,created_at) VALUES(?,?,?)")
            .bind(user)
            .bind(ip)
            .bind(time)
            .execute(pool)
            .await
            .unwrap();
    }
    #[test]
    fn ip_comparison_retains_mapped_ipv4_but_not_distinct_loopbacks_or_invalid_values() {
        assert!(ips_match(" 203.0.113.9 ", "::ffff:203.0.113.9"));
        assert!(ips_match("2001:db8::1", "2001:0db8:0:0:0:0:0:1"));
        assert!(!ips_match("127.0.0.1", "::1"));
        assert!(!ips_match("203.0.113.9", "203.0.113.10"));
        assert!(!ips_match("invalid", "invalid"));
        assert!(!ips_match("", ""));
    }
    #[tokio::test]
    async fn admission_preserves_cutoff_identity_bucket_and_newest_fifty_by_id() {
        let pool = pool().await;
        row(&pool, None, "203.0.113.1", "2026-01-02").await;
        row(&pool, Some(2), "203.0.113.2", "2026-01-02").await;
        row(&pool, Some(1), "203.0.113.3", "2026-01-01").await;
        row(&pool, Some(1), "203.0.113.4", "2026-01-02").await;
        assert!(verify_ip(&pool, None, Some("203.0.113.1"), "2026-01-02").await.unwrap());
        assert!(!verify_ip(&pool, Some(1), Some("203.0.113.1"), "2026-01-02").await.unwrap());
        assert!(!verify_ip(&pool, Some(1), Some("203.0.113.2"), "2026-01-02").await.unwrap());
        assert!(!verify_ip(&pool, Some(1), Some("203.0.113.3"), "2026-01-02").await.unwrap());
        for _ in 0..49 {
            row(&pool, Some(1), "198.51.100.1", "2026-01-02").await;
        }
        assert!(verify_ip(&pool, Some(1), Some(" ::ffff:203.0.113.4 "), "2026-01-02").await.unwrap());
        row(&pool, Some(1), "198.51.100.2", "2026-01-02").await;
        assert!(!verify_ip(&pool, Some(1), Some("203.0.113.4"), "2026-01-02").await.unwrap());
        pool.close().await;
        assert!(!verify_ip(&pool, Some(1), Some(" "), "2026-01-02").await.unwrap());
        assert!(verify_ip(&pool, Some(1), Some("203.0.113.4"), "2026-01-02").await.is_err());
    }
    #[tokio::test]
    async fn recording_prunes_globally_with_strict_cutoff_and_preserves_exact_values() {
        let pool = pool().await;
        row(&pool, None, "203.0.113.1", "2026-01-01").await;
        row(&pool, Some(2), "203.0.113.2", "2026-01-02").await;
        record_and_prune(&pool, 1, " ::ffff:203.0.113.3 ", "Original", "2026-01-03", "2026-01-02").await.unwrap();
        let rows: Vec<(Option<i64>, String, Option<String>, String)> =
            sqlx::query_as("SELECT user_id,ip,username,created_at FROM launcher_sessions ORDER BY id").fetch_all(&pool).await.unwrap();
        assert_eq!(
            rows,
            vec![
                (Some(2), "203.0.113.2".into(), None, "2026-01-02".into()),
                (Some(1), " ::ffff:203.0.113.3 ".into(), Some("Original".into()), "2026-01-03".into())
            ]
        );
        assert!(record_and_prune(&pool, 99, "203.0.113.9", "Absent", "2026-01-03", "2026-01-03").await.is_err());
        assert!(verify_ip(&pool, Some(2), Some("203.0.113.2"), "2026-01-02").await.unwrap());
    }
    #[tokio::test]
    async fn prune_failure_retains_inserted_session_and_existing_history() {
        let pool = pool().await;
        row(&pool, None, "203.0.113.1", "2026-01-01").await;
        sqlx::raw_sql("CREATE TRIGGER refuse_prune BEFORE DELETE ON launcher_sessions BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;")
            .execute(&pool)
            .await
            .unwrap();
        assert!(record_and_prune(&pool, 1, "203.0.113.2", "Original", "2026-01-03", "2026-01-02").await.is_err());
        assert!(verify_ip(&pool, Some(1), Some("203.0.113.2"), "2026-01-03").await.unwrap());
        assert!(verify_ip(&pool, None, Some("203.0.113.1"), "2026-01-01").await.unwrap());
    }
    #[tokio::test]
    async fn revocation_obeys_caller_rollback_commit_and_user_cascade() {
        let pool = pool().await;
        row(&pool, Some(1), "203.0.113.1", "2026-01-02").await;
        row(&pool, Some(2), "203.0.113.2", "2026-01-02").await;
        row(&pool, None, "203.0.113.3", "2026-01-02").await;
        let mut tx = pool.begin().await.unwrap();
        revoke(&mut tx, 1).await.unwrap();
        // A failed host audit can abort the same transaction without losing admission.
        assert!(sqlx::query("INSERT INTO missing_audit VALUES(1)").execute(&mut *tx).await.is_err());
        tx.rollback().await.unwrap();
        assert!(verify_ip(&pool, Some(1), Some("203.0.113.1"), "2026-01-02").await.unwrap());
        let mut tx = pool.begin().await.unwrap();
        revoke(&mut tx, 1).await.unwrap();
        tx.commit().await.unwrap();
        assert!(!verify_ip(&pool, Some(1), Some("203.0.113.1"), "2026-01-02").await.unwrap());
        sqlx::query("DELETE FROM users WHERE id=2").execute(&pool).await.unwrap();
        assert!(!verify_ip(&pool, Some(2), Some("203.0.113.2"), "2026-01-02").await.unwrap());
        assert!(verify_ip(&pool, None, Some("203.0.113.3"), "2026-01-02").await.unwrap());
    }
}
