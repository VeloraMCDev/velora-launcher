//! Private economy transaction/idempotency primitives; no public redistribution grant.
pub mod ledger;
use serde_json::Value;
#[derive(Debug)]
pub enum StorageError {
    BadRequest(&'static str),
    InvalidOperationId,
    Database(sqlx::Error),
    Json(serde_json::Error),
}
pub type StorageResult<T> = Result<T, StorageError>;
impl From<sqlx::Error> for StorageError {
    fn from(value: sqlx::Error) -> Self {
        Self::Database(value)
    }
}
impl From<serde_json::Error> for StorageError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}
impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidOperationId => write!(f, "Invalid operation ID"),
            Self::BadRequest(message) => write!(f, "{message}"),
            Self::Database(error) => write!(f, "database error: {error}"),
            Self::Json(error) => write!(f, "invalid JSON: {error}"),
        }
    }
}
impl std::error::Error for StorageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidOperationId | Self::BadRequest(_) => None,
            Self::Database(error) => Some(error),
            Self::Json(error) => Some(error),
        }
    }
}
pub async fn begin_operation(
    pool: &sqlx::SqlitePool,
    server_id: i64,
    operation_id: &str,
) -> StorageResult<(sqlx::Transaction<'static, sqlx::Sqlite>, Option<Value>)> {
    if operation_id.is_empty() || operation_id.len() > 100 {
        return Err(StorageError::InvalidOperationId);
    }
    let mut tx = pool.begin().await?;
    // The first statement obtains the write lock before reading any balances.
    let inserted = sqlx::query("INSERT OR IGNORE INTO economy_operations(server_id, operation_id, response) VALUES (?, ?, '')")
        .bind(server_id)
        .bind(operation_id)
        .execute(&mut *tx)
        .await?
        .rows_affected();
    let previous = if inserted == 0 {
        let raw: String = sqlx::query_scalar("SELECT response FROM economy_operations WHERE server_id = ? AND operation_id = ?")
            .bind(server_id)
            .bind(operation_id)
            .fetch_one(&mut *tx)
            .await?;
        Some(serde_json::from_str(&raw)?)
    } else {
        None
    };
    Ok((tx, previous))
}

pub async fn finish_operation(
    mut tx: sqlx::Transaction<'_, sqlx::Sqlite>,
    server_id: i64,
    operation_id: &str,
    response: Value,
) -> StorageResult<Value> {
    sqlx::query("UPDATE economy_operations SET response = ? WHERE server_id = ? AND operation_id = ?")
        .bind(response.to_string())
        .bind(server_id)
        .bind(operation_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(response)
}

pub async fn ensure_balance(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    server_id: i64,
    uuid: &str,
    name: &str,
    start: f64,
    clock: impl FnOnce() -> String + Send,
) -> StorageResult<()> {
    sqlx::query("INSERT INTO server_economy(server_id, uuid, username, balance, updated_at) VALUES (?, ?, ?, ?, ?) ON CONFLICT(server_id, uuid) DO NOTHING")
        .bind(server_id).bind(uuid).bind(name).bind(start).bind(clock()).execute(&mut **tx).await?;
    Ok(())
}
