mod common;
use velora_panel::{auth, bootstrap_admin, build_state_with_keys, config::Config, db};

fn config(path: &std::path::Path) -> Config {
    Config {
        bind: "127.0.0.1:0".into(),
        data_dir: path.into(),
        web_dir: path.join("web"),
        icons_dir: path.join("icons"),
        admin_username: "admin".into(),
        admin_password: Some("synthetic-admin-password".into()),
        jwt_secret: None,
        curseforge_api_key: None,
        max_upload_mb: 64,
        public_url: None,
        trusted_proxies: vec![],
    }
}

#[tokio::test]
async fn restart_preserves_persisted_jwt_sessions_admin_uuid_and_password() {
    let folder = tempfile::tempdir().unwrap();
    let cfg = config(folder.path());
    let pool = db::connect(folder.path()).await.unwrap();
    let first = build_state_with_keys(cfg.clone(), pool, common::test_keys()).await.unwrap();
    bootstrap_admin(&first).await.unwrap();
    let original = auth::find_user_by_name(&first, "admin").await.unwrap().unwrap();
    let token = first.keys.issue(&original).unwrap();
    let bytes = std::fs::read(folder.path().join("jwt.secret")).unwrap();
    assert_eq!(auth::authenticate(&first, &token).await.unwrap().uuid, original.uuid);
    first.platform_db.close().await;
    drop(first);
    let mut changed = cfg;
    changed.admin_username = "ChangedAdmin".into();
    changed.admin_password = Some("changed-synthetic-password".into());
    let pool = db::connect(folder.path()).await.unwrap();
    let restarted = build_state_with_keys(changed, pool, common::test_keys()).await.unwrap();
    bootstrap_admin(&restarted).await.unwrap();
    let current = auth::authenticate(&restarted, &token).await.unwrap();
    assert_eq!(current.id, original.id);
    assert_eq!(current.uuid, original.uuid);
    assert_eq!(current.password_hash, original.password_hash);
    assert_eq!(std::fs::read(folder.path().join("jwt.secret")).unwrap(), bytes);
    assert!(auth::find_user_by_name(&restarted, "ChangedAdmin").await.unwrap().is_none());
}

#[tokio::test]
async fn damaged_persisted_jwt_prevents_startup_without_rotation_and_explicit_legacy_config_is_preserved() {
    let folder = tempfile::tempdir().unwrap();
    let mut cfg = config(folder.path());
    let path = folder.path().join("jwt.secret");
    std::fs::write(&path, b"synthetic-short").unwrap();
    let pool = db::connect_memory().await.unwrap();
    assert!(build_state_with_keys(cfg.clone(), pool.clone(), common::test_keys()).await.is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"synthetic-short");
    assert_eq!(sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users").fetch_one(&pool).await.unwrap(), 0);
    cfg.jwt_secret = Some("synthetic-legacy".into());
    let state = build_state_with_keys(cfg, pool, common::test_keys()).await.unwrap();
    bootstrap_admin(&state).await.unwrap();
    let admin = auth::find_user_by_name(&state, "admin").await.unwrap().unwrap();
    let token = state.keys.issue(&admin).unwrap();
    assert_eq!(auth::authenticate(&state, &token).await.unwrap().id, admin.id);
    assert_eq!(std::fs::read(path).unwrap(), b"synthetic-short");
}
