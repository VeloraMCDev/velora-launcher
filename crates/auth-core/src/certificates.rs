//! Minecraft player certificates; credential records are authority-local, not SDK DTOs.
use crate::keys::Keys;
use base64::Engine;
fn iso_millis(t: chrono::DateTime<chrono::Utc>) -> String {
    t.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

/// PEM exactly as Minecraft's `Crypt.rsaPublicKeyToString` writes it (MIME
/// base64: 76-char lines, CRLF) — the V1 signature covers this text.
fn mojang_pem(label: &str, der: &[u8]) -> String {
    let b64 = base64::engine::general_purpose::STANDARD.encode(der);
    let lines: Vec<&str> = b64.as_bytes().chunks(76).map(|c| std::str::from_utf8(c).unwrap()).collect();
    format!("-----BEGIN {label}-----\n{}\n-----END {label}-----\n", lines.join("\r\n"))
}

#[cfg_attr(feature = "sqlite", derive(sqlx::FromRow))]
pub struct PlayerKeyRow {
    pub private_pem: String,
    pub public_pem: String,
    pub signature_v1: String,
    pub signature_v2: String,
    pub expires_at: String,
    pub refreshed_after: String,
}

pub fn new_player_key() -> anyhow::Result<rsa::RsaPrivateKey> {
    Ok(rsa::RsaPrivateKey::new(&mut rand::thread_rng(), 2048)?)
}
pub fn certify(signing: &Keys, key: &rsa::RsaPrivateKey, account_uuid: &str) -> anyhow::Result<PlayerKeyRow> {
    use rsa::pkcs8::{EncodePrivateKey, EncodePublicKey};
    let public_der = rsa::RsaPublicKey::from(key).to_public_key_der().map_err(anyhow::Error::from)?;
    let private_der = key.to_pkcs8_der().map_err(anyhow::Error::from)?;
    let now = chrono::Utc::now();
    let expires = now + chrono::Duration::hours(48);
    let refreshed_after = now + chrono::Duration::hours(40);
    let public_pem = mojang_pem("RSA PUBLIC KEY", public_der.as_bytes());

    // V2 (1.19.1+): uuid msb, uuid lsb, expiry millis (big-endian), key DER.
    let uuid = uuid::Uuid::parse_str(account_uuid).map_err(anyhow::Error::from)?;
    let mut v2 = Vec::with_capacity(24 + public_der.as_bytes().len());
    v2.extend_from_slice(uuid.as_bytes());
    v2.extend_from_slice(&expires.timestamp_millis().to_be_bytes());
    v2.extend_from_slice(public_der.as_bytes());
    // V1 (1.19.0): expiry millis as text + the PEM text.
    let v1 = format!("{}{}", expires.timestamp_millis(), public_pem);

    Ok(PlayerKeyRow {
        private_pem: mojang_pem("RSA PRIVATE KEY", private_der.as_bytes()),
        signature_v1: signing.sign_b64(v1.as_bytes()),
        signature_v2: signing.sign_b64(&v2),
        public_pem,
        expires_at: iso_millis(expires),
        refreshed_after: iso_millis(refreshed_after),
    })
}

pub fn fresh(row: &PlayerKeyRow, now: chrono::DateTime<chrono::Utc>) -> bool {
    row.refreshed_after > iso_millis(now)
}
#[cfg(feature = "sqlite")]
pub async fn load(pool: &sqlx::SqlitePool, user_id: i64) -> Result<Option<PlayerKeyRow>, sqlx::Error> {
    sqlx::query_as("SELECT * FROM player_keys WHERE user_id = ?").bind(user_id).fetch_optional(pool).await
}
#[cfg(feature = "sqlite")]
pub async fn store(pool: &sqlx::SqlitePool, user_id: i64, row: &PlayerKeyRow) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT OR REPLACE INTO player_keys (user_id, private_pem, public_pem, signature_v1, signature_v2, expires_at, refreshed_after)
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(user_id)
    .bind(&row.private_pem)
    .bind(&row.public_pem)
    .bind(&row.signature_v1)
    .bind(&row.signature_v2)
    .bind(&row.expires_at)
    .bind(&row.refreshed_after)
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pem_matches_java_mime_layout() {
        let pem = mojang_pem("RSA PUBLIC KEY", &[7u8; 100]);
        assert!(pem.starts_with("-----BEGIN RSA PUBLIC KEY-----\n"));
        assert!(pem.ends_with("\n-----END RSA PUBLIC KEY-----\n"));
        assert!(pem.contains("\r\n"), "76-column MIME lines separated by CRLF");
    }
    use rsa::{
        pkcs1v15::{Signature, VerifyingKey},
        pkcs8::DecodePublicKey,
        signature::Verifier,
        traits::PublicKeyParts,
    };
    #[test]
    fn legacy_pem_and_both_certificate_payloads_verify_against_the_authority() {
        let signing = Keys::from_private(rsa::RsaPrivateKey::new(&mut rand::thread_rng(), 1024).unwrap()).unwrap();
        let key = new_player_key().unwrap();
        assert_eq!(key.n().bits(), 2048);
        let uuid = "01234567-89ab-4def-8123-456789abcdef";
        let row = certify(&signing, &key, uuid).unwrap();
        assert!(row.private_pem.starts_with("-----BEGIN RSA PRIVATE KEY-----\n"));
        assert!(row.public_pem.starts_with("-----BEGIN RSA PUBLIC KEY-----\n"));
        assert!(row.public_pem.contains("\r\n"));
        let expires = chrono::DateTime::parse_from_rfc3339(&row.expires_at).unwrap();
        let refresh = chrono::DateTime::parse_from_rfc3339(&row.refreshed_after).unwrap();
        assert_eq!((expires - refresh).num_hours(), 8);
        let public = rsa::RsaPublicKey::from_public_key_pem(&signing.public_pem).unwrap();
        let verifying = VerifyingKey::<sha1::Sha1>::new(public);
        let verify = |bytes: &[u8], encoded: &str| {
            let signature = Signature::try_from(base64::engine::general_purpose::STANDARD.decode(encoded).unwrap().as_slice()).unwrap();
            assert!(verifying.verify(bytes, &signature).is_ok());
        };
        verify(format!("{}{}", expires.timestamp_millis(), row.public_pem).as_bytes(), &row.signature_v1);
        let encoded: String = row.public_pem.lines().filter(|line| !line.starts_with("-----")).map(str::trim).collect();
        let der = base64::engine::general_purpose::STANDARD.decode(encoded).unwrap();
        let mut payload = uuid::Uuid::parse_str(uuid).unwrap().as_bytes().to_vec();
        payload.extend_from_slice(&expires.timestamp_millis().to_be_bytes());
        payload.extend_from_slice(&der);
        verify(&payload, &row.signature_v2);
        assert!(certify(&signing, &key, "invalid-uuid").is_err());
    }
    fn synthetic_row() -> PlayerKeyRow {
        PlayerKeyRow {
            private_pem: "synthetic-private".into(),
            public_pem: "synthetic-public".into(),
            signature_v1: "v1".into(),
            signature_v2: "v2".into(),
            expires_at: "2026-01-03T00:00:00.000Z".into(),
            refreshed_after: "2026-01-02T16:00:00.000Z".into(),
        }
    }
    #[test]
    fn cached_certificate_refresh_boundary_is_strict() {
        let row = synthetic_row();
        let at = chrono::DateTime::parse_from_rfc3339(&row.refreshed_after).unwrap().with_timezone(&chrono::Utc);
        assert!(fresh(&row, at - chrono::Duration::milliseconds(1)));
        assert!(!fresh(&row, at));
        assert!(!fresh(&row, at + chrono::Duration::milliseconds(1)));
    }
    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn certificate_storage_preserves_key_bytes_replacement_and_outage_errors() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        sqlx::query("CREATE TABLE player_keys (user_id INTEGER PRIMARY KEY, private_pem TEXT, public_pem TEXT, signature_v1 TEXT, signature_v2 TEXT, expires_at TEXT, refreshed_after TEXT)").execute(&pool).await.unwrap();
        assert!(load(&pool, 1).await.unwrap().is_none());
        let mut row = synthetic_row();
        store(&pool, 1, &row).await.unwrap();
        let stored = load(&pool, 1).await.unwrap().unwrap();
        assert_eq!(stored.private_pem, row.private_pem);
        assert_eq!(stored.public_pem, row.public_pem);
        assert_eq!(stored.signature_v1, row.signature_v1);
        assert_eq!(stored.signature_v2, row.signature_v2);
        assert_eq!(stored.expires_at, row.expires_at);
        assert_eq!(stored.refreshed_after, row.refreshed_after);
        row.private_pem = "synthetic-replacement".into();
        store(&pool, 1, &row).await.unwrap();
        assert_eq!(load(&pool, 1).await.unwrap().unwrap().private_pem, row.private_pem);
        pool.close().await;
        assert!(load(&pool, 1).await.is_err());
        assert!(store(&pool, 1, &row).await.is_err());
    }
}
