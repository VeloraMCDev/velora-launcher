//! Legacy API v1 JWT encoding and validation, without database or HTTP state.
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

const TOKEN_DAYS: i64 = 30;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    #[serde(default)]
    pub version: i64,
    pub sub: i64,
    pub name: String,
    pub role: String,
    pub exp: i64,
}

/// Identity data supplied by the authority; never constructed from unverified claims.
pub struct Subject<'a> {
    pub id: i64,
    pub name: &'a str,
    pub role: &'a str,
    pub auth_version: i64,
}

pub struct Keys {
    enc: EncodingKey,
    dec: DecodingKey,
}

impl Keys {
    pub fn new(secret: &[u8]) -> Self {
        Self { enc: EncodingKey::from_secret(secret), dec: DecodingKey::from_secret(secret) }
    }

    pub fn issue(&self, subject: Subject<'_>) -> Result<String, jsonwebtoken::errors::Error> {
        let claims = Claims {
            version: subject.auth_version,
            sub: subject.id,
            name: subject.name.to_owned(),
            role: subject.role.to_owned(),
            exp: (chrono::Utc::now() + chrono::Duration::days(TOKEN_DAYS)).timestamp(),
        };
        encode(&Header::default(), &claims, &self.enc)
    }

    /// Signature/expiry validation alone does not establish current account access.
    /// The authority must still check live status and auth_version before admission.
    pub fn verify(&self, token: &str) -> Option<Claims> {
        decode::<Claims>(token, &self.dec, &Validation::default()).ok().map(|d| d.claims)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issued_tokens_keep_legacy_claim_fields_algorithm_and_lifetime() {
        let keys = Keys::new(b"synthetic-test-secret");
        let before = chrono::Utc::now().timestamp();
        let token = keys.issue(Subject { id: 42, name: "ExamplePlayer", role: "member", auth_version: 3 }).unwrap();
        let after = chrono::Utc::now().timestamp();
        assert_eq!(jsonwebtoken::decode_header(&token).unwrap().alg, jsonwebtoken::Algorithm::HS256);
        let claims = keys.verify(&token).unwrap();
        assert_eq!((claims.sub, claims.name.as_str(), claims.role.as_str(), claims.version), (42, "ExamplePlayer", "member", 3));
        assert!((before + 30 * 86400..=after + 30 * 86400).contains(&claims.exp));
        let fields = serde_json::to_value(claims).unwrap();
        assert_eq!(fields.as_object().unwrap().len(), 5);
        assert!(Keys::new(b"different-synthetic-secret").verify(&token).is_none());
    }

    #[test]
    fn legacy_tokens_without_version_still_decode_but_expired_tokens_do_not() {
        let legacy = serde_json::json!({ "sub": 7, "name": "LegacyPlayer", "role": "member", "exp": 4102444800_i64 });
        let token = encode(&Header::default(), &legacy, &EncodingKey::from_secret(b"synthetic-test-secret")).unwrap();
        let keys = Keys::new(b"synthetic-test-secret");
        let claims = keys.verify(&token).unwrap();
        assert_eq!((claims.sub, claims.version), (7, 0));
        let expired = serde_json::json!({ "sub": 7, "name": "LegacyPlayer", "role": "member", "exp": 1 });
        let token = encode(&Header::default(), &expired, &EncodingKey::from_secret(b"synthetic-test-secret")).unwrap();
        assert!(keys.verify(&token).is_none());
    }
}
