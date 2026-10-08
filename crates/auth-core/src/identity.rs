//! Authority-local account record, not a public SDK wire model.
//! SQLite mapping is opt-in while the compatible host owns the live database.
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "sqlite", derive(sqlx::FromRow))]
pub struct IdentityRecord {
    #[serde(skip)]
    pub auth_version: i64,
    pub id: i64,
    pub username: String,
    #[serde(skip)]
    pub password_hash: String,
    pub email: Option<String>,
    pub role: String,
    pub status: String,
    pub created_at: String,
    pub last_login: Option<String>,
    /// Dashed player UUID. Names can change; the identity remains fixed.
    pub uuid: String,
    pub skin_hash: Option<String>,
    pub skin_model: String,
    pub cape_id: Option<i64>,
    pub status_reason: Option<String>,
}

impl IdentityRecord {
    pub fn is_admin(&self) -> bool {
        self.role == "admin"
    }
}

/// Operator-supplied legacy exact-name and marked-substring policy.
pub fn username_blocked(name: &str, entries: &[String]) -> bool {
    let normalized = name.to_ascii_lowercase().replace('_', "");
    entries.iter().any(|entry| {
        let rule = entry.trim().to_ascii_lowercase();
        if rule.is_empty() {
            return false;
        }
        if let Some(inner) = rule.strip_prefix('*').and_then(|r| r.strip_suffix('*')) {
            inner.len() >= 3 && normalized.contains(inner)
        } else {
            normalized == rule.replace('_', "")
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_responses_preserve_nullable_fields_and_exclude_credentials() {
        let mut record = IdentityRecord {
            auth_version: 7,
            id: 42,
            username: "ExamplePlayer".into(),
            password_hash: "synthetic-hash-not-a-credential".into(),
            email: None,
            role: "member".into(),
            status: "active".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            last_login: None,
            uuid: "01234567-89ab-4def-8123-456789abcdef".into(),
            skin_hash: None,
            skin_model: "classic".into(),
            cape_id: None,
            status_reason: None,
        };
        let wire = serde_json::to_value(&record).unwrap();
        assert_eq!(wire.as_object().unwrap().len(), 12);
        assert!(wire.get("password_hash").is_none());
        assert!(wire.get("auth_version").is_none());
        for field in ["email", "last_login", "skin_hash", "cape_id", "status_reason"] {
            assert!(wire.get(field).unwrap().is_null());
        }
        assert_eq!(wire["uuid"], record.uuid);
        assert!(!record.is_admin());
        record.role = "Admin".into();
        assert!(!record.is_admin());
        record.role = "admin".into();
        assert!(record.is_admin());
    }

    #[test]
    fn supplied_name_policy_preserves_exact_substring_and_normalization_rules() {
        let rules = vec!["Reserved_Name".into(), " *BLOCKED* ".into(), "*xy*".into(), " ".into()];
        assert!(username_blocked("RESERVEDNAME", &rules));
        assert!(username_blocked("blocked_player", &rules));
        assert!(!username_blocked("ReservedNameExtra", &rules));
        assert!(!username_blocked("xyPlayer", &rules));
        assert!(!username_blocked("ExamplePlayer", &[]));
    }
}
