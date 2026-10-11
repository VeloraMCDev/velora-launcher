mod common;
use common::*;

#[tokio::test]
async fn gateway_auth_persistence_conflicts_and_validation() {
    // Dedicated test process and one test: no concurrent environment mutation.
    let credential = "synthetic-gateway-test-credential-only-000000";
    unsafe {
        std::env::set_var("VELORA_GATEWAY_TOKEN", credential);
    }
    let t = setup_persistent().await;
    let admin = t.login("admin", "supersecret").await;
    let mut instances = Vec::new();
    for name in ["First gateway experience", "Second gateway experience"] {
        let (code, instance) = t.call("POST", "/api/admin/instances", Some(&admin),
            Some(json!({"name":name,"mc_version":"1.21.1","loader":"vanilla"}))).await;
        assert_eq!(code, StatusCode::OK, "{instance}");
        instances.push(instance["id"].as_str().unwrap().to_owned());
    }
    assert_eq!(t.call("GET", "/api/admin/gateway", None, None).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(t.call("GET", "/api/gateway/v1/routes", Some(&admin), None).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(t.call("GET", "/api/admin/gateway", Some(credential), None).await.0, StatusCode::UNAUTHORIZED);
    let (code, player) = t.call("POST", "/api/admin/users", Some(&admin),
        Some(json!({"username":"GatewayReader","password":"synthetic-player-test-pass"}))).await;
    assert_eq!(code, StatusCode::OK, "{player}");
    let player = t.login("GatewayReader", "synthetic-player-test-pass").await;
    assert_eq!(t.call("GET", "/api/admin/gateway", Some(&player), None).await.0, StatusCode::FORBIDDEN);
    assert_eq!(t.call("PUT", "/api/admin/gateway", Some(&player), Some(json!({"revision":0,"routes":[]}))).await.0, StatusCode::FORBIDDEN);
    let routes = json!({"revision":0,"routes":[{"id":"smp","experience":"SMP","hostname":"SMP.velora.scopedd.lol.","address":"100.64.0.2","port":25565,"enabled":true}]});
    let (code, saved) = t.call("PUT", "/api/admin/gateway", Some(&admin), Some(routes.clone())).await;
    assert_eq!(code, StatusCode::OK);
    assert_eq!(saved["revision"], 1);
    assert_eq!(saved["routes"][0]["hostname"], "smp.velora.scopedd.lol");
    for instance in &instances {
        let (code, overview) = t.call("GET", &format!("/api/admin/gateway?instance={instance}"), Some(&admin), None).await;
        assert_eq!(code, StatusCode::OK, "{overview}");
        assert_eq!(overview["snapshot"], saved, "Gateway routes are shared across all experiences");
    }
    assert_eq!(t.call("GET", "/api/admin/progression", Some(&admin), None).await.0, StatusCode::BAD_REQUEST);
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
