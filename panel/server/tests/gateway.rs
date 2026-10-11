mod common;
use common::*;

#[tokio::test]
async fn gateway_auth_persistence_conflicts_and_validation() {
    // Dedicated test process and one test: no concurrent environment mutation.
    let credential = "synthetic-gateway-test-credential-only-000000";
    unsafe {
        std::env::set_var("VELORA_GATEWAY_TOKEN", credential);
    }
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    assert_eq!(t.call("GET", "/api/admin/gateway", None, None).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(t.call("GET", "/api/gateway/v1/routes", Some(&admin), None).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(t.call("GET", "/api/admin/gateway", Some(credential), None).await.0, StatusCode::UNAUTHORIZED);
    let routes = json!({"revision":0,"routes":[{"id":"smp","experience":"SMP","hostname":"SMP.velora.scopedd.lol.","address":"100.64.0.2","port":25565,"enabled":true}]});
    let (code, saved) = t.call("PUT", "/api/admin/gateway", Some(&admin), Some(routes.clone())).await;
    assert_eq!(code, StatusCode::OK);
    assert_eq!(saved["revision"], 1);
    assert_eq!(saved["routes"][0]["hostname"], "smp.velora.scopedd.lol");
    assert_eq!(t.call("PUT", "/api/admin/gateway", Some(&admin), Some(routes)).await.0, StatusCode::CONFLICT);
    let (_, pulled) = t.call("GET", "/api/gateway/v1/routes", Some(credential), None).await;
    assert_eq!(pulled, saved);
    let mut bad = saved.clone();
    bad["routes"][0]["address"] = json!("8.8.8.8");
    assert_eq!(t.call("PUT", "/api/admin/gateway", Some(&admin), Some(bad)).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(t.call("GET", "/api/gateway/v1/routes", Some(credential), None).await.1, saved);
    let report = json!({"revision":1,"connections":2,"routes":[{"id":"smp","reachable":true}],"error":null});
    assert_eq!(t.call("POST", "/api/gateway/v1/status", None, Some(report.clone())).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(t.call("POST", "/api/gateway/v1/status", Some(credential), Some(report)).await.0, StatusCode::OK);
    let (_, overview) = t.call("GET", "/api/admin/gateway", Some(&admin), None).await;
    assert_eq!(overview["status"]["report"]["revision"], 1);
    let persisted: String = sqlx::query_scalar("SELECT value FROM kv WHERE key='gateway.routes'").fetch_one(&t.db).await.unwrap();
    assert_eq!(serde_json::from_str::<Value>(&persisted).unwrap(), saved);
    let empty = json!({"revision":1,"routes":[]});
    assert_eq!(t.call("PUT", "/api/admin/gateway", Some(&admin), Some(empty)).await.1["revision"], 2);
    unsafe {
        std::env::remove_var("VELORA_GATEWAY_TOKEN");
    }
}
