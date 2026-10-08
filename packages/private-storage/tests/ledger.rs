use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use velora_experiences_storage::{
    ensure_balance,
    ledger::{self, Movement, CONTRACTS, ORDERS},
    StorageError,
};

async fn pool() -> SqlitePool {
    let pool = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
    sqlx::raw_sql("CREATE TABLE server_economy(server_id INTEGER,uuid TEXT,username TEXT,balance REAL,updated_at TEXT,PRIMARY KEY(server_id,uuid));CREATE TABLE economy_transactions(server_id INTEGER,from_uuid TEXT,from_name TEXT,to_uuid TEXT,to_name TEXT,amount REAL,description TEXT,created_at TEXT);").execute(&pool).await.unwrap();
    pool
}
fn movement() -> Movement<'static> {
    Movement {
        economy: 100,
        server: 7,
        uuid: "synthetic-uuid",
        name: "Example",
        amount: 10.0,
        counterparty: ORDERS,
        description: "Synthetic escrow",
    }
}

#[tokio::test]
async fn debit_credit_keep_economy_scope_named_accounts_and_separate_update_log_clocks() {
    let pool = pool().await;
    let mut tx = pool.begin().await.unwrap();
    ensure_balance(&mut tx, 100, "synthetic-uuid", "Example", 25.0, || "initial".into()).await.unwrap();
    let mut ticks = 0;
    let balance = ledger::debit(&mut tx, movement(), || {
        ticks += 1;
        format!("tick-{ticks}")
    })
    .await
    .unwrap();
    assert_eq!(balance, 15.0);
    assert_eq!(ticks, 2);
    let credited = Movement { amount: 5.0, counterparty: CONTRACTS, ..movement() };
    assert_eq!(
        ledger::credit(&mut tx, credited, || {
            ticks += 1;
            format!("tick-{ticks}")
        })
        .await
        .unwrap(),
        20.0
    );
    assert_eq!(ticks, 4);
    tx.commit().await.unwrap();
    let balance: (f64, String) =
        sqlx::query_as("SELECT balance,updated_at FROM server_economy WHERE server_id=100").fetch_one(&pool).await.unwrap();
    assert_eq!(balance, (20.0, "tick-3".into()));
    type LedgerRow = (i64, String, String, String, String, f64, String, String);
    let rows: Vec<LedgerRow> = sqlx::query_as(
        "SELECT server_id,from_uuid,from_name,to_uuid,to_name,amount,description,created_at FROM economy_transactions ORDER BY rowid",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        rows[0],
        (
            7,
            "synthetic-uuid".into(),
            "Example".into(),
            "orders".into(),
            "Buy order escrow".into(),
            10.0,
            "Synthetic escrow".into(),
            "tick-2".into()
        )
    );
    assert_eq!(
        rows[1],
        (
            7,
            "contracts".into(),
            "Contract board".into(),
            "synthetic-uuid".into(),
            "Example".into(),
            5.0,
            "Synthetic escrow".into(),
            "tick-4".into()
        )
    );
    assert_eq!(ledger::balance_of(&pool, 100, "synthetic-uuid").await, Some(20.0));
    assert_eq!(ledger::balance_of(&pool, 7, "synthetic-uuid").await, None);
}

#[tokio::test]
async fn insufficient_funds_and_log_failure_keep_transaction_rollback() {
    let pool = pool().await;
    let mut tx = pool.begin().await.unwrap();
    ensure_balance(&mut tx, 100, "synthetic-uuid", "Example", 5.0, || "initial".into()).await.unwrap();
    tx.commit().await.unwrap();
    let mut tx = pool.begin().await.unwrap();
    let mut clocks = 0;
    let error = ledger::debit(&mut tx, movement(), || {
        clocks += 1;
        "denied".into()
    })
    .await
    .unwrap_err();
    assert!(matches!(error, StorageError::BadRequest("You can't afford that.")));
    assert_eq!(clocks, 1);
    tx.rollback().await.unwrap();
    sqlx::raw_sql("CREATE TRIGGER reject_log BEFORE INSERT ON economy_transactions BEGIN SELECT RAISE(ABORT,'synthetic failure');END;")
        .execute(&pool)
        .await
        .unwrap();
    let mut tx = pool.begin().await.unwrap();
    let attempt = Movement { amount: 1.0, ..movement() };
    assert!(matches!(ledger::debit(&mut tx, attempt, || "changed".into()).await, Err(StorageError::Database(_))));
    tx.rollback().await.unwrap();
    assert_eq!(ledger::balance_of(&pool, 100, "synthetic-uuid").await, Some(5.0));
    assert_eq!(sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM economy_transactions").fetch_one(&pool).await.unwrap(), 0);
}

#[tokio::test]
async fn balance_read_failures_keep_original_none_and_missing_credit_keeps_sql_error() {
    let pool = pool().await;
    let mut tx = pool.begin().await.unwrap();
    assert!(matches!(ledger::credit(&mut tx, movement(), || "time".into()).await, Err(StorageError::Database(_))));
    tx.rollback().await.unwrap();
    pool.close().await;
    assert_eq!(ledger::balance_of(&pool, 100, "synthetic-uuid").await, None);
}
