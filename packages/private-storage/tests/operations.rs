use serde_json::json;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
    SqlitePool,
};
use velora_experiences_storage::{begin_operation, ensure_balance, finish_operation, StorageError};

async fn pool() -> SqlitePool {
    let pool = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
    schema(&pool).await;
    pool
}
async fn schema(pool: &SqlitePool) {
    sqlx::raw_sql("CREATE TABLE economy_operations(server_id INTEGER,operation_id TEXT,response TEXT,PRIMARY KEY(server_id,operation_id)); CREATE TABLE server_economy(server_id INTEGER,uuid TEXT,username TEXT,balance REAL,updated_at TEXT,PRIMARY KEY(server_id,uuid));").execute(pool).await.unwrap();
}

#[tokio::test]
async fn ids_use_original_byte_limit_and_invalid_id_precedes_store_outage() {
    let pool = pool().await;
    for id in ["", &"x".repeat(101), &"汉".repeat(34)] {
        assert!(matches!(begin_operation(&pool, 1, id).await, Err(StorageError::InvalidOperationId)));
    }
    let (tx, replay) = begin_operation(&pool, 1, &"x".repeat(100)).await.unwrap();
    assert!(replay.is_none());
    tx.rollback().await.unwrap();
    pool.close().await;
    assert!(matches!(begin_operation(&pool, 1, "").await, Err(StorageError::InvalidOperationId)));
    assert!(matches!(begin_operation(&pool, 1, "synthetic").await, Err(StorageError::Database(_))));
}

#[tokio::test]
async fn commit_replays_exact_response_and_keeps_server_scopes_separate() {
    let pool = pool().await;
    let response = json!({"ok":true,"amount":12.5,"opaque":{"items":[null,"retained"]}});
    let (mut tx, replay) = begin_operation(&pool, 1, "synthetic").await.unwrap();
    assert!(replay.is_none());
    ensure_balance(&mut tx, 1, "synthetic-uuid", "Example", 12.5, || "original-time".into()).await.unwrap();
    assert_eq!(finish_operation(tx, 1, "synthetic", response.clone()).await.unwrap(), response);
    let (tx, replayed) = begin_operation(&pool, 1, "synthetic").await.unwrap();
    assert_eq!(replayed, Some(response));
    tx.rollback().await.unwrap();
    let (tx, other) = begin_operation(&pool, 2, "synthetic").await.unwrap();
    assert!(other.is_none());
    tx.rollback().await.unwrap();
    let row: (String, f64, String) =
        sqlx::query_as("SELECT username,balance,updated_at FROM server_economy WHERE server_id=1").fetch_one(&pool).await.unwrap();
    assert_eq!(row, ("Example".into(), 12.5, "original-time".into()));
}

#[tokio::test]
async fn failed_response_commit_rolls_back_balance_and_operation_reservation() {
    let pool = pool().await;
    sqlx::raw_sql("CREATE TRIGGER reject_commit BEFORE UPDATE ON economy_operations BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;")
        .execute(&pool)
        .await
        .unwrap();
    let (mut tx, _) = begin_operation(&pool, 1, "synthetic").await.unwrap();
    ensure_balance(&mut tx, 1, "synthetic-uuid", "Example", 25.0, || "original-time".into()).await.unwrap();
    assert!(matches!(finish_operation(tx, 1, "synthetic", json!({"ok":true})).await, Err(StorageError::Database(_))));
    assert_eq!(sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM server_economy").fetch_one(&pool).await.unwrap(), 0);
    assert_eq!(sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM economy_operations").fetch_one(&pool).await.unwrap(), 0);
}

#[tokio::test]
async fn insert_only_balance_preserves_existing_fields_but_evaluates_supplied_clock() {
    let pool = pool().await;
    let mut tx = pool.begin().await.unwrap();
    let mut clocks = 0;
    ensure_balance(&mut tx, 1, "synthetic-uuid", "Original", 25.0, || {
        clocks += 1;
        "original-time".into()
    })
    .await
    .unwrap();
    ensure_balance(&mut tx, 1, "synthetic-uuid", "Changed", 999.0, || {
        clocks += 1;
        "changed-time".into()
    })
    .await
    .unwrap();
    assert_eq!(clocks, 2);
    tx.commit().await.unwrap();
    let row: (String, f64, String) =
        sqlx::query_as("SELECT username,balance,updated_at FROM server_economy").fetch_one(&pool).await.unwrap();
    assert_eq!(row, ("Original".into(), 25.0, "original-time".into()));
}

#[tokio::test]
async fn malformed_cached_json_and_same_ids_in_two_instance_pools_are_not_fallbacks() {
    let first = pool().await;
    let second = pool().await;
    sqlx::query("INSERT INTO economy_operations VALUES(1,'synthetic','not-json')").execute(&first).await.unwrap();
    assert!(matches!(begin_operation(&first, 1, "synthetic").await, Err(StorageError::Json(_))));
    let (tx, replay) = begin_operation(&second, 1, "synthetic").await.unwrap();
    assert!(replay.is_none());
    finish_operation(tx, 1, "synthetic", json!({"scope":"second"})).await.unwrap();
    assert_eq!(sqlx::query_scalar::<_, String>("SELECT response FROM economy_operations").fetch_one(&first).await.unwrap(), "not-json");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn competing_writers_lock_before_read_and_apply_one_operation() {
    let directory = tempfile::tempdir().unwrap();
    let pool = SqlitePoolOptions::new()
        .max_connections(2)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(directory.path().join("synthetic.sqlite"))
                .create_if_missing(true)
                .journal_mode(SqliteJournalMode::Wal)
                .busy_timeout(std::time::Duration::from_secs(10)),
        )
        .await
        .unwrap();
    schema(&pool).await;
    let mut tasks = Vec::new();
    for _ in 0..2 {
        let pool = pool.clone();
        tasks.push(tokio::spawn(async move {
            let (mut tx, replay) = begin_operation(&pool, 1, "same-operation").await.unwrap();
            if let Some(replay) = replay {
                return replay;
            }
            ensure_balance(&mut tx, 1, "synthetic-uuid", "Example", 0.0, || "time".into()).await.unwrap();
            sqlx::query("UPDATE server_economy SET balance=balance+10 WHERE server_id=1").execute(&mut *tx).await.unwrap();
            finish_operation(tx, 1, "same-operation", json!({"applied":10})).await.unwrap()
        }));
    }
    for task in tasks {
        assert_eq!(task.await.unwrap(), json!({"applied":10}));
    }
    assert_eq!(sqlx::query_scalar::<_, f64>("SELECT balance FROM server_economy").fetch_one(&pool).await.unwrap(), 10.0);
    assert_eq!(sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM economy_operations").fetch_one(&pool).await.unwrap(), 1);
    pool.close().await;
}
