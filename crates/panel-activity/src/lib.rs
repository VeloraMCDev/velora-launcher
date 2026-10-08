//! Platform metadata writes and compatibility report projections.
//! No credentials, authorization middleware, gameplay implementation or schema adoption.
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

/// Identity/policy/timestamp are supplied by an authenticated caller.
pub struct Record<'a> {
    pub username: &'a str,
    pub uuid: &'a str,
    pub source: &'a str,
    pub kind: &'a str,
    pub detail: Option<&'a str>,
    pub created_at: &'a str,
}

/// Metadata-only callers must not supply passwords, tokens, chat or command arguments.
pub async fn record(pool: &SqlitePool, record: Record<'_>) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO events (username, uuid, source, kind, detail, created_at) VALUES (?, ?, ?, ?, ?, ?)")
        .bind(record.username)
        .bind(record.uuid)
        .bind(record.source)
        .bind(record.kind)
        .bind(record.detail.map(|detail| detail.chars().filter(|c| !c.is_control()).take(256).collect::<String>()))
        .bind(record.created_at)
        .execute(pool)
        .await?;
    Ok(())
}

#[derive(Deserialize, Default)]
pub struct Filter {
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub player: String,
    #[serde(default)]
    pub offset: u32,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct Entry {
    pub id: i64,
    pub source: String,
    pub server: Option<String>,
    pub uuid: Option<String>,
    pub name: Option<String>,
    pub kind: String,
    pub detail: Option<String>,
    pub created_at: String,
}

/// Callers guard this report with live administrator authorization.
/// The supplied pool exposes the current platform/report projections, not credentials.
pub async fn list(pool: &SqlitePool, filter: &Filter) -> Result<Vec<Entry>, sqlx::Error> {
    sqlx::query_as(
        "SELECT * FROM (
        SELECT id, source, NULL AS server, uuid, username AS name, kind, detail, created_at FROM events
        UNION ALL
        SELECT e.id, 'server' AS source, s.name AS server, e.uuid, e.name, e.kind, e.detail, e.created_at
        FROM server_events e JOIN game_servers s ON s.id=e.server_id
    ) WHERE (?='' OR source=?) AND (?='' OR name=? COLLATE NOCASE OR uuid=?)
    ORDER BY created_at DESC, source, id DESC LIMIT 100 OFFSET ?",
    )
    .bind(&filter.source)
    .bind(&filter.source)
    .bind(&filter.player)
    .bind(&filter.player)
    .bind(&filter.player)
    .bind(filter.offset.min(100_000))
    .fetch_all(pool)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    async fn pool() -> SqlitePool {
        let options = sqlx::sqlite::SqliteConnectOptions::new().in_memory(true);
        let pool = sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect_with(options).await.unwrap();
        // Minimal read projections, not an imported mixed schema or private migrations.
        sqlx::raw_sql("CREATE TABLE events(id INTEGER PRIMARY KEY AUTOINCREMENT, username TEXT, uuid TEXT, source TEXT NOT NULL, kind TEXT NOT NULL, detail TEXT, created_at TEXT NOT NULL);
            CREATE TABLE game_servers(id INTEGER PRIMARY KEY,name TEXT);
            CREATE TABLE server_events(id INTEGER,server_id INTEGER,uuid TEXT,name TEXT,kind TEXT,detail TEXT,created_at TEXT);
            INSERT INTO game_servers VALUES(7,'SyntheticServer');
            INSERT INTO events VALUES(9,'Original','synthetic-uuid','auth','login',NULL,'2026-01-01');
            INSERT INTO events VALUES(11,NULL,NULL,'legacy-unknown','opaque-kind','opaque legacy value','2026-01-02');
            INSERT INTO server_events VALUES(9,7,'synthetic-uuid','Original','unknown-server-kind',NULL,'2026-01-01');
            INSERT INTO server_events VALUES(99,999,'synthetic-uuid','Original','orphan',NULL,'2999-01-01');")
            .execute(&pool).await.unwrap();
        pool
    }
    #[tokio::test]
    async fn report_keeps_unknown_payloads_nullable_fields_ids_filters_and_join_behavior() {
        let pool = pool().await;
        let entries = list(&pool, &Filter::default()).await.unwrap();
        assert_eq!(
            serde_json::to_value(&entries).unwrap(),
            serde_json::json!([
                {"id":11,"source":"legacy-unknown","server":null,"uuid":null,"name":null,"kind":"opaque-kind","detail":"opaque legacy value","created_at":"2026-01-02"},
                {"id":9,"source":"auth","server":null,"uuid":"synthetic-uuid","name":"Original","kind":"login","detail":null,"created_at":"2026-01-01"},
                {"id":9,"source":"server","server":"SyntheticServer","uuid":"synthetic-uuid","name":"Original","kind":"unknown-server-kind","detail":null,"created_at":"2026-01-01"}
            ])
        );
        for player in ["original", "synthetic-uuid"] {
            assert_eq!(list(&pool, &Filter { player: player.into(), ..Default::default() }).await.unwrap().len(), 2);
        }
        assert_eq!(
            list(&pool, &Filter { source: "server".into(), player: "ORIGINAL".into(), offset: 0 }).await.unwrap()[0].server.as_deref(),
            Some("SyntheticServer")
        );
        assert!(list(&pool, &Filter { player: "' OR 1=1 --".into(), ..Default::default() }).await.unwrap().is_empty());
        assert_eq!(list(&pool, &Filter { offset: 1, ..Default::default() }).await.unwrap()[0].source, "auth");
    }
    #[tokio::test]
    async fn metadata_sanitization_is_unicode_aware_and_outages_propagate() {
        let pool = pool().await;
        let detail = format!("\n\t{}\0", "é".repeat(300));
        record(
            &pool,
            Record {
                username: "Verified",
                uuid: "synthetic-uuid-two",
                source: "panel",
                kind: "account_or_admin_change",
                detail: Some(&detail),
                created_at: "2026-01-03",
            },
        )
        .await
        .unwrap();
        let entries = list(&pool, &Filter { source: "panel".into(), ..Default::default() }).await.unwrap();
        assert_eq!(entries[0].detail.as_deref(), Some("é".repeat(256).as_str()));
        assert_eq!(entries[0].uuid.as_deref(), Some("synthetic-uuid-two"));
        pool.close().await;
        assert!(list(&pool, &Filter::default()).await.is_err());
        assert!(record(
            &pool,
            Record {
                username: "Verified",
                uuid: "synthetic-uuid-two",
                source: "auth",
                kind: "login",
                detail: None,
                created_at: "2026-01-03"
            }
        )
        .await
        .is_err());
    }
    #[tokio::test]
    async fn ordering_and_bounded_pagination_keep_legacy_limits() {
        let pool = pool().await;
        for id in 100..205 {
            sqlx::query("INSERT INTO events(id,source,kind,created_at) VALUES(?,'panel','synthetic','2026-01-03')")
                .bind(id)
                .execute(&pool)
                .await
                .unwrap();
        }
        let page = list(&pool, &Filter::default()).await.unwrap();
        assert_eq!(page.len(), 100);
        assert_eq!((page[0].id, page[99].id), (204, 105));
        let remaining = list(&pool, &Filter { offset: 100, ..Default::default() }).await.unwrap();
        assert_eq!(remaining.len(), 8);
        assert!(list(&pool, &Filter { offset: u32::MAX, ..Default::default() }).await.unwrap().is_empty());
    }
}
