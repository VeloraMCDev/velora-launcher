mod common;
use common::*;

#[tokio::test]
async fn health_reports_service_identity_and_matching_schema() {
    let app = setup().await;
    let (status, body) = app.call("GET", "/health", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
    assert_eq!(body["service"], "velora-panel");
    assert_eq!(body["schema"], body["expected_schema"]);
    assert!(body["version"].as_str().is_some_and(|v| !v.is_empty()));
    assert!(body["commit"].as_str().is_some());
    // No secrets or paths leak through the health document.
    let text = body.to_string();
    assert!(!text.contains("secret") && !text.contains("password") && !text.contains("/data"));
}

#[tokio::test]
async fn health_reports_unavailable_when_the_store_is_older_than_the_code() {
    let app = setup().await;
    sqlx::raw_sql("PRAGMA user_version = 1").execute(&app.db).await.unwrap();
    let (status, body) = app.call("GET", "/health", None, None).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["status"], "schema_mismatch");
}

#[tokio::test]
async fn legacy_healthz_is_unchanged() {
    let app = setup().await;
    let response = app.router.clone().oneshot(Request::builder().uri("/healthz").body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}
