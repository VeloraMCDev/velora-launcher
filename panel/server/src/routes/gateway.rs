//! Platform-wide gateway configuration. Gateway credentials never reach the browser.
use crate::{
    auth::AdminUser,
    error::{AppError, AppResult},
    state::AppState,
};
use axum::{extract::State, http::HeaderMap, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::HashSet, net::Ipv4Addr};

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub revision: u64,
    pub routes: Vec<Route>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Route {
    pub id: String,
    pub experience: String,
    pub hostname: String,
    pub address: String,
    pub port: u16,
    pub enabled: bool,
}
fn validate(s: &mut Snapshot) -> AppResult<()> {
    if s.routes.len() > 128 {
        return Err(AppError::bad_request("Maximum 128 routes"));
    }
    let mut hosts = HashSet::new();
    let mut ids = HashSet::new();
    for r in &mut s.routes {
        let hostname = r.hostname.to_ascii_lowercase();
        r.hostname = hostname.strip_suffix('.').unwrap_or(&hostname).to_owned();
        if r.hostname.len() > 253
            || !r.hostname.contains('.')
            || r.hostname.split('.').any(|p| {
                p.is_empty()
                    || p.len() > 63
                    || p.starts_with('-')
                    || p.ends_with('-')
                    || !p.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
            })
        {
            return Err(AppError::bad_request("Invalid DNS hostname (no port or wildcard)"));
        }
        let ip: Ipv4Addr = r.address.parse().map_err(|_| AppError::bad_request("Backend must be a private IPv4 tunnel address"))?;
        let a = ip.octets();
        if !(ip.is_private() || (a[0] == 100 && (64..=127).contains(&a[1]))) || r.port == 0 {
            return Err(AppError::bad_request("Backend must use RFC1918 or CGNAT tunnel IPv4 and a nonzero port"));
        }
        if r.id.is_empty()
            || r.id.len() > 64
            || !r.id.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
            || r.experience.trim().is_empty()
            || r.experience.len() > 120
            || !hosts.insert(r.hostname.clone())
            || !ids.insert(r.id.clone())
        {
            return Err(AppError::bad_request("Invalid or duplicate route ID/hostname/experience"));
        }
    }
    Ok(())
}
async fn read(db: &sqlx::SqlitePool, key: &str) -> AppResult<Option<String>> {
    Ok(sqlx::query_scalar("SELECT value FROM kv WHERE key = ?").bind(key).fetch_optional(db).await?)
}
async fn snapshot(db: &sqlx::SqlitePool) -> AppResult<Snapshot> {
    Ok(read(db, "gateway.routes").await?.map(|v| serde_json::from_str(&v)).transpose()?.unwrap_or_default())
}
pub async fn overview(_: AdminUser, State(s): State<AppState>) -> AppResult<Json<Value>> {
    let status: Value = read(&s.platform_db, "gateway.status").await?.map(|v| serde_json::from_str(&v)).transpose()?.unwrap_or(Value::Null);
    Ok(Json(
        json!({"snapshot": snapshot(&s.platform_db).await?, "status": status, "public_endpoint": std::env::var("VELORA_GATEWAY_PUBLIC_ENDPOINT").ok()}),
    ))
}
pub async fn save(_: AdminUser, State(s): State<AppState>, Json(mut next): Json<Snapshot>) -> AppResult<Json<Snapshot>> {
    validate(&mut next)?;
    // SQLite serializes writers; compare-and-swap rejects stale browser edits.
    let old = read(&s.platform_db, "gateway.routes").await?;
    let current: Snapshot = old.as_ref().map(|v| serde_json::from_str(v)).transpose()?.unwrap_or_default();
    if current.revision != next.revision {
        return Err(AppError::conflict("Routes changed; refresh before saving"));
    }
    next.revision = next.revision.checked_add(1).ok_or_else(|| AppError::conflict("Revision exhausted"))?;
    let value = serde_json::to_string(&next)?;
    let changed = if let Some(old) = old {
        sqlx::query("UPDATE kv SET value = ? WHERE key = 'gateway.routes' AND value = ?")
            .bind(&value)
            .bind(old)
            .execute(&s.platform_db)
            .await?
            .rows_affected()
    } else {
        sqlx::query("INSERT OR IGNORE INTO kv(key,value) VALUES('gateway.routes',?)")
            .bind(&value)
            .execute(&s.platform_db)
            .await?
            .rows_affected()
    };
    if changed != 1 {
        return Err(AppError::conflict("Routes changed; refresh before saving"));
    }
    Ok(Json(next))
}
fn authorize(headers: &HeaderMap) -> AppResult<()> {
    let token = std::env::var("VELORA_GATEWAY_TOKEN").unwrap_or_default();
    let supplied = headers.get("authorization").and_then(|v| v.to_str().ok()).and_then(|v| v.strip_prefix("Bearer ")).unwrap_or("");
    if token.len() < 32 || !credential_matches(&token, supplied) {
        return Err(AppError::unauthorized("Invalid gateway credential"));
    }
    Ok(())
}
fn credential_matches(expected: &str, supplied: &str) -> bool {
    use ring::hmac;
    let message = b"velora-gateway-control-plane";
    let tag = hmac::sign(&hmac::Key::new(hmac::HMAC_SHA256, supplied.as_bytes()), message);
    hmac::verify(&hmac::Key::new(hmac::HMAC_SHA256, expected.as_bytes()), message, tag.as_ref()).is_ok()
}
pub async fn pull(State(s): State<AppState>, headers: HeaderMap) -> AppResult<Json<Snapshot>> {
    authorize(&headers)?;
    Ok(Json(snapshot(&s.platform_db).await?))
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub revision: u64,
    pub connections: u32,
    pub routes: Vec<RouteHealth>,
    #[serde(default)]
    pub events: Vec<ConnectionEvent>,
    pub error: Option<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteHealth {
    pub id: String,
    pub reachable: bool,
    #[serde(default)]
    pub enabled: bool,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectionEvent {
    pub at: String,
    pub hostname: String,
    pub kind: String,
}
pub async fn report(State(s): State<AppState>, headers: HeaderMap, Json(report): Json<Report>) -> AppResult<Json<Value>> {
    authorize(&headers)?;
    if report.routes.len() > 128
        || report.events.len() > 50
        || report.routes.iter().any(|r| r.id.len() > 64)
        || report.events.iter().any(|e| e.hostname.len() > 253 || e.at.len() > 40 || !matches!(e.kind.as_str(), "assigned" | "closed"))
        || report.error.as_ref().is_some_and(|e| e.len() > 512)
    {
        return Err(AppError::bad_request("Report too large"));
    }
    let value = serde_json::to_string(&json!({"received_at": chrono::Utc::now(), "report": report}))?;
    sqlx::query("INSERT INTO kv(key,value) VALUES('gateway.status',?) ON CONFLICT(key) DO UPDATE SET value=excluded.value")
        .bind(value)
        .execute(&s.platform_db)
        .await?;
    Ok(Json(json!({"ok": true})))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn credentials_are_exact() {
        assert!(credential_matches("synthetic-a", "synthetic-a"));
        assert!(!credential_matches("synthetic-a", "synthetic-b"));
        assert!(!credential_matches("synthetic-a", ""));
    }
    #[test]
    fn route_validation() {
        let mut s = Snapshot {
            revision: 0,
            routes: vec![Route {
                id: "smp".into(),
                experience: "SMP".into(),
                hostname: "SMP.velora.scopedd.lol.".into(),
                address: "100.64.0.2".into(),
                port: 25565,
                enabled: true,
            }],
        };
        assert!(validate(&mut s).is_ok());
        assert_eq!(s.routes[0].hostname, "smp.velora.scopedd.lol");
        s.routes.push(s.routes[0].clone());
        assert!(validate(&mut s).is_err());
        s.routes.pop();
        s.routes[0].address = "8.8.8.8".into();
        assert!(validate(&mut s).is_err());
    }
}
