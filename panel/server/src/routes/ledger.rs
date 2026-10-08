//! Compatibility facade for private transaction-bound economy ledger behavior.
use super::economy::{ensure_balance, storage_error};
use crate::error::AppResult;
use velora_experiences_storage::ledger::{self, Movement};
pub(crate) use velora_experiences_storage::ledger::{balance_of, pretty, round2, Account, Tx, CONTRACTS, ORDERS};
pub(crate) fn normalize_item(id: &str) -> AppResult<String> {
    ledger::normalize_item(id).map_err(storage_error)
}
pub(crate) async fn debit(
    tx: &mut Tx,
    economy: i64,
    server: i64,
    uuid: &str,
    name: &str,
    amount: f64,
    to: Account,
    description: &str,
) -> AppResult<f64> {
    ensure_balance(tx, economy, uuid, name).await?;
    ledger::debit(tx, Movement { economy, server, uuid, name, amount, counterparty: to, description }, || chrono::Utc::now().to_rfc3339())
        .await
        .map_err(storage_error)
}
pub(crate) async fn credit(
    tx: &mut Tx,
    economy: i64,
    server: i64,
    uuid: &str,
    name: &str,
    amount: f64,
    from: Account,
    description: &str,
) -> AppResult<f64> {
    ensure_balance(tx, economy, uuid, name).await?;
    ledger::credit(tx, Movement { economy, server, uuid, name, amount, counterparty: from, description }, || {
        chrono::Utc::now().to_rfc3339()
    })
    .await
    .map_err(storage_error)
}
