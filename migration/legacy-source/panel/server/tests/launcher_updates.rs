mod common;
use common::*;

fn installer(marker: u8) -> Vec<u8> {
    let mut bytes = vec![marker; 256];
    bytes[..2].copy_from_slice(b"MZ");
    bytes[60..64].copy_from_slice(&64u32.to_le_bytes());
    bytes[64..68].copy_from_slice(b"PE\0\0");
    bytes
}
async fn upload(t: &TestApp, token: &str, version: &str, bytes: &[u8]) -> (StatusCode, Value) {
    let (mime, body) = multipart(&[("version", version), ("notes", "New features")], ("Setup.exe", bytes));
    t.send(
        Request::builder()
            .method("POST")
            .uri("/api/admin/launcher/update")
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", mime)
            .body(Body::from(body))
            .unwrap(),
    )
    .await
}

#[tokio::test]
async fn publishes_public_feed_and_keeps_old_downloads_immutable() {
    let t = setup().await;
    let (status, empty) = t.call("GET", "/api/v1/launcher/update", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(empty.is_null());
    let admin = t.login("admin", "supersecret").await;
    let first_bytes = installer(1);
    let (status, first) = upload(&t, &admin, "v0.10.0", &first_bytes).await;
    assert_eq!(status, StatusCode::OK, "{first}");
    assert_eq!(first["version"], "0.10.0");
    assert_eq!(first["size"], first_bytes.len());
    use sha2::{Digest, Sha256};
    assert_eq!(first["sha256"], hex::encode(Sha256::digest(&first_bytes)));
    let old_url = first["url"].as_str().unwrap();
    assert_eq!(t.fetch(old_url).await, (StatusCode::OK, first_bytes.clone()));
    let (status, _) = upload(&t, &admin, "0.11.0", &installer(2)).await;
    assert_eq!(status, StatusCode::OK);
    let (_, feed) = t.call("GET", "/api/v1/launcher/update", None, None).await;
    assert_eq!(feed["version"], "0.11.0");
    assert_ne!(feed["url"], old_url);
    assert_eq!(t.fetch(old_url).await, (StatusCode::OK, first_bytes));
    let (_, landing) = t.call("GET", "/api/admin/landing", Some(&admin), None).await;
    assert_eq!(landing["hosted_downloads"][0]["file_url"], feed["url"]);
    let (status, _) = t.call("GET", "/api/v1/launcher/updates/../setup.exe", None, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn only_admins_can_publish_valid_bounded_installers() {
    let t = setup_with(|cfg, _| cfg.max_upload_mb = 1).await;
    let admin = t.login("admin", "supersecret").await;
    t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username":"Alex","password":"password123"}))).await;
    let player = t.login("Alex", "password123").await;
    assert_eq!(upload(&t, &player, "0.10.0", &installer(1)).await.0, StatusCode::FORBIDDEN);
    assert_eq!(t.call("GET", "/api/admin/launcher/update", None, None).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(upload(&t, &admin, "latest", &installer(1)).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(upload(&t, &admin, "0.10.0", b"MZnot-an-exe").await.0, StatusCode::BAD_REQUEST);
    let oversized = vec![0; 1024 * 1024 + 1];
    assert!(upload(&t, &admin, "0.10.0", &oversized).await.0.is_client_error());
    assert!(t.call("GET", "/api/v1/launcher/update", None, None).await.1.is_null());
}
