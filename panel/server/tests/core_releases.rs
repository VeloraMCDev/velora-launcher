mod common;
use common::*;

#[tokio::test]
async fn nothing_is_served_before_a_release_is_approved() {
    let t = setup().await;
    let (status, latest) = t.call("GET", "/api/v1/core/latest", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(latest.is_null(), "no release is advertised before the first approval");

    let digest = "ab".repeat(32);
    let (status, _) = t.call("GET", &format!("/api/v1/core/files/{digest}/velora-core-server-1.20.1-0.6.0.jar"), None, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "an unapproved jar is never served");
    for bad in ["short/x.jar", "../../etc/passwd", "zz/x.jar"] {
        let (status, _) = t.call("GET", &format!("/api/v1/core/files/{bad}"), None, None).await;
        assert!(status.is_client_error(), "{bad} must be refused, got {status}");
    }
}

#[tokio::test]
async fn only_admins_can_list_or_approve_releases() {
    let t = setup().await;
    assert_eq!(t.call("GET", "/api/admin/core/releases", None, None).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(t.call("POST", "/api/admin/core/releases/velora-core-v0.6.0/approve", None, None).await.0, StatusCode::UNAUTHORIZED);
    let admin = t.login("admin", "supersecret").await;
    t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username":"Alex","password":"password123"}))).await;
    let player = t.login("Alex", "password123").await;
    assert_eq!(t.call("GET", "/api/admin/core/releases", Some(&player), None).await.0, StatusCode::FORBIDDEN);
    assert_eq!(t.call("POST", "/api/admin/core/releases/velora-core-v0.6.0/approve", Some(&player), None).await.0, StatusCode::FORBIDDEN);
}
