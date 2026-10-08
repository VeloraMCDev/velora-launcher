use serde::{Deserialize, Serialize};
use velora_panel_settings::*;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct SuppliedAuth {
    value: String,
}
impl Default for SuppliedAuth {
    fn default() -> Self {
        Self { value: "host-default".into() }
    }
}
#[tokio::test]
async fn kv_scope_fallbacks_upserts_and_failures_match_legacy() {
    let one = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    let other = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    for pool in [&one, &other] {
        sqlx::query("CREATE TABLE kv(key TEXT PRIMARY KEY,value TEXT NOT NULL)").execute(pool).await.unwrap();
    }
    assert_eq!(get::<Settings<SuppliedAuth>>(&one, "missing").await.unwrap().auth.value, "host-default");
    sqlx::query("INSERT INTO kv VALUES('settings','malformed')").execute(&one).await.unwrap();
    assert_eq!(get::<Settings<SuppliedAuth>>(&one, "settings").await.unwrap().auth.value, "host-default");
    let mut settings = Settings::<SuppliedAuth>::default();
    settings.auth.value = "caller-identity".into();
    settings.curseforge_api_key = Some("synthetic-key".into());
    set(&one, "settings", &settings).await.unwrap();
    set(&one, "settings", &settings).await.unwrap();
    assert_eq!(get::<Settings<SuppliedAuth>>(&one, "settings").await.unwrap().auth, settings.auth);
    assert_eq!(get::<Settings<SuppliedAuth>>(&other, "settings").await.unwrap().auth.value, "host-default");
    assert_eq!(sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM kv").fetch_one(&one).await.unwrap(), 1);
    one.close().await;
    assert!(get::<Settings<SuppliedAuth>>(&one, "settings").await.is_err());
    assert!(matches!(set(&one, "settings", &settings).await, Err(WriteError::Database(_))));
}
#[test]
fn normalization_retains_secret_replacement_policy_and_auth_fields() {
    for (request, expected) in [(None, Some("saved")), (Some("  "), Some("saved")), (Some(" - "), None), (Some(" fresh "), Some("fresh"))] {
        let settings = Settings::<SuppliedAuth> {
            curseforge_api_key: request.map(str::to_string),
            public_url: Some(" https://example.invalid/// ".into()),
            username_blocklist: vec!["  Exact_Name  ".into(), " *WORD* ".into(), "  ".into()],
            ..Default::default()
        };
        let out = normalize(settings, Some("saved".into())).unwrap();
        assert_eq!(out.curseforge_api_key.as_deref(), expected);
        assert_eq!(out.public_url.as_deref(), Some("https://example.invalid"));
        assert_eq!(out.username_blocklist, ["exact_name", "*word*"]);
        assert_eq!(out.auth.value, "host-default");
    }
}
#[test]
fn validation_keeps_error_precedence_limits_and_original_url_policy() {
    for rule in ["ab", "word*", "*word", "a*b", "ééé", "with space"] {
        let s = Settings::<SuppliedAuth> { username_blocklist: vec![rule.into()], public_url: Some("bad".into()), ..Default::default() };
        assert!(normalize(s, None).unwrap_err().0.starts_with("blacklist entries"));
    }
    let s = Settings::<SuppliedAuth> { username_blocklist: vec!["valid".into(); 201], ..Default::default() };
    assert!(normalize(s, None).is_err());
    let s = Settings::<SuppliedAuth> { username_blocklist: vec![], public_url: Some("ftp://example.invalid".into()), ..Default::default() };
    assert_eq!(normalize(s, None).unwrap_err().0, "the public URL must start with https:// (or http://)");
    let s = Settings::<SuppliedAuth> { username_blocklist: vec![], public_url: Some("https://".into()), ..Default::default() };
    assert!(normalize(s, None).is_err()); // Prefix validation follows slash normalization, exactly as in the host.
}
#[test]
fn branding_and_settings_wire_shape_preserve_unknown_and_default_fields() {
    assert!(validate_branding_name("  ").is_err());
    assert!(validate_branding_name(" supplied ").is_ok());
    let settings: Settings<SuppliedAuth> = serde_json::from_value(serde_json::json!({"unknown":"retained-by-caller"})).unwrap();
    assert_eq!(settings.auth.value, "host-default");
    let value = serde_json::to_value(settings).unwrap();
    assert_eq!(value.as_object().unwrap().len(), 5);
    assert!(value.get("curseforge_api_key").unwrap().is_null());
}
#[tokio::test]
async fn serialization_failure_keeps_saved_records_and_error_precedence() {
    struct Broken;
    impl Serialize for Broken {
        fn serialize<S: serde::Serializer>(&self, _serializer: S) -> Result<S::Ok, S::Error> {
            Err(serde::ser::Error::custom("synthetic failure"))
        }
    }
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::query("CREATE TABLE kv(key TEXT PRIMARY KEY,value TEXT NOT NULL)").execute(&pool).await.unwrap();
    set(&pool, "saved", &"original").await.unwrap();
    assert!(matches!(set(&pool, "saved", &Broken).await, Err(WriteError::Json(_))));
    assert_eq!(get::<String>(&pool, "saved").await.unwrap(), "original");
    pool.close().await;
    assert!(matches!(set(&pool, "saved", &Broken).await, Err(WriteError::Json(_))));
}
