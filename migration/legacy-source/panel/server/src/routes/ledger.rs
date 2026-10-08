//! Money movement shared by buy orders and contracts: take from or give to a player and leave a line in the transaction list
//! with a named account (the order escrow, the contract board) on the other side.

use super::economy::ensure_balance;
use crate::error::{AppError, AppResult};
use chrono::Utc;
use sqlx::{Sqlite, SqliteConnection, Transaction};

pub(crate) type Tx = Transaction<'static, Sqlite>;

pub(crate) fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// A named account on the other side of a transaction line.
#[derive(Clone, Copy)]
pub(crate) struct Account(pub &'static str, pub &'static str);

pub(crate) const ORDERS: Account = Account("orders", "Buy order escrow");
pub(crate) const CONTRACTS: Account = Account("contracts", "Contract board");

async fn log_tx(conn: &mut SqliteConnection, server: i64, from: (&str, &str), to: (&str, &str), amount: f64, description: &str) -> AppResult<()> {
    sqlx::query("INSERT INTO economy_transactions (server_id, from_uuid, from_name, to_uuid, to_name, amount, description, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(server).bind(from.0).bind(from.1).bind(to.0).bind(to.1).bind(amount).bind(description).bind(Utc::now().to_rfc3339())
        .execute(&mut *conn).await?;
    Ok(())
}

/// Take `amount` from a player. Fails when they cannot afford it.
pub(crate) async fn debit(tx: &mut Tx, economy: i64, server: i64, uuid: &str, name: &str, amount: f64, to: Account, description: &str) -> AppResult<f64> {
    ensure_balance(tx, economy, uuid, name).await?;
    let balance: Option<f64> = sqlx::query_scalar("UPDATE server_economy SET balance = balance - ?1, updated_at = ?2 WHERE server_id = ?3 AND uuid = ?4 AND balance - ?1 >= 0 RETURNING balance")
        .bind(amount).bind(Utc::now().to_rfc3339()).bind(economy).bind(uuid).fetch_optional(&mut **tx).await?;
    let Some(balance) = balance else { return Err(AppError::bad_request("You can't afford that.")) };
    log_tx(&mut **tx, server, (uuid, name), (to.0, to.1), amount, description).await?;
    Ok(balance)
}

/// Give `amount` to a player.
pub(crate) async fn credit(tx: &mut Tx, economy: i64, server: i64, uuid: &str, name: &str, amount: f64, from: Account, description: &str) -> AppResult<f64> {
    ensure_balance(tx, economy, uuid, name).await?;
    let balance: f64 = sqlx::query_scalar("UPDATE server_economy SET balance = balance + ?, updated_at = ? WHERE server_id = ? AND uuid = ? RETURNING balance")
        .bind(amount).bind(Utc::now().to_rfc3339()).bind(economy).bind(uuid).fetch_one(&mut **tx).await?;
    log_tx(&mut **tx, server, (from.0, from.1), (uuid, name), amount, description).await?;
    Ok(balance)
}

/// Money is on its own pool as far as one player's balance goes; read it back for a response.
pub(crate) async fn balance_of(db: &sqlx::SqlitePool, economy: i64, uuid: &str) -> Option<f64> {
    sqlx::query_scalar("SELECT balance FROM server_economy WHERE server_id = ? AND uuid = ?").bind(economy).bind(uuid).fetch_optional(db).await.ok().flatten()
}

/// "DIAMOND_PICKAXE" or "minecraft:diamond_pickaxe" becomes "Diamond Pickaxe".
pub(crate) fn pretty(item_id: &str) -> String {
    let base = item_id.rsplit(':').next().unwrap_or(item_id);
    base.split(['_', '-', ' ']).filter(|w| !w.is_empty()).map(|w| {
        let mut c = w.chars();
        c.next().map(|f| f.to_uppercase().collect::<String>() + &c.as_str().to_lowercase()).unwrap_or_default()
    }).collect::<Vec<_>>().join(" ")
}

/// Item ids are stored the way the plugins send them: vanilla items as "DIAMOND", modded ones as "modid:item".
pub(crate) fn normalize_item(id: &str) -> AppResult<String> {
    let id = id.trim();
    if id.is_empty() || id.len() > 64 || !id.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | ':' | '.' | '/' | '-')) {
        return Err(AppError::bad_request("That isn't a valid item id."));
    }
    let lower = id.to_ascii_lowercase();
    Ok(match lower.strip_prefix("minecraft:") {
        Some(rest) => rest.to_ascii_uppercase(),
        None if lower.contains(':') => lower,
        None => id.to_ascii_uppercase(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_ids_are_normalized_the_way_the_plugins_send_them() {
        assert_eq!(normalize_item("diamond").unwrap(), "DIAMOND");
        assert_eq!(normalize_item("minecraft:Cobblestone").unwrap(), "COBBLESTONE");
        assert_eq!(normalize_item("Create:Brass_Ingot").unwrap(), "create:brass_ingot");
        assert!(normalize_item("").is_err() && normalize_item("a b").is_err() && normalize_item(&"x".repeat(80)).is_err());
        assert_eq!(pretty("DIAMOND_PICKAXE"), "Diamond Pickaxe");
        assert_eq!(pretty("create:brass_ingot"), "Brass Ingot");
    }
}
