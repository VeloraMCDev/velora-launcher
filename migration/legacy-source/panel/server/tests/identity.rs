mod common;
use common::*;

#[tokio::test]
async fn renaming_preserves_uuid_stats_and_reserves_names() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    let (_, created) = t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username":"Steve", "password":"password123"}))).await;
    let id = created["id"].as_i64().unwrap();
    let uuid = t.uuid("Steve").await;
    let (_, auth) = t.call("POST", "/api/v1/auth/login", None, Some(json!({"username":"Steve", "password":"password123"}))).await;
    let old_token = auth["token"].as_str().unwrap();
    let old_game_token = auth["yggdrasil"]["access_token"].as_str().unwrap();
    let (s, _) = t.call("PUT", "/api/v1/account/username", Some(old_token), Some(json!({"username":"Alex", "password":"wrong"}))).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    let (_, server) = t.call("POST", "/api/admin/servers", Some(&admin), Some(json!({"name":"Survival"}))).await;
    let server_id = server["server"]["id"].as_i64().unwrap();
    t.call(
        "POST",
        "/api/server/v1/sync",
        server["token"].as_str(),
        Some(json!({"stats":[{"uuid":uuid,"name":"Steve","playtime_secs":123}]})),
    )
    .await;
    let (s, renamed) = t
        .call(
            "PUT",
            "/api/v1/account/username",
            Some(old_token),
            Some(json!({"username":"Alex", "password":"password123", "uuid":"spoofed"})),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{renamed}");
    assert_eq!(renamed["user"]["uuid"], uuid);
    assert_eq!(renamed["user"]["id"], id);
    assert_eq!(renamed["user"]["username"], "Alex");
    let (s, _) = t.call("GET", "/api/v1/auth/me", Some(old_token), None).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    let (s, _) = t.call("POST", "/api/yggdrasil/authserver/validate", None, Some(json!({"accessToken":old_game_token}))).await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    let (s, _) = t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username":"steve", "password":"password123"}))).await;
    assert_eq!(s, StatusCode::CONFLICT);
    let (_, details) = t.call("GET", &format!("/api/admin/servers/{server_id}"), Some(&admin), None).await;
    assert_eq!(details["leaderboard"][0]["uuid"], uuid);
    assert_eq!(details["leaderboard"][0]["playtime_secs"], 123);
    assert!(sqlx::query("UPDATE users SET uuid=? WHERE id=?")
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(id)
        .execute(&t.db)
        .await
        .is_err());
    let fresh = renamed["token"].as_str().unwrap();
    let (_, profile) = t.call("GET", "/api/v1/account/profile", Some(fresh), None).await;
    assert_eq!(profile["uuid"], uuid);
    // Deleting an account never releases its historical identity to a new user.
    t.call("DELETE", &format!("/api/admin/users/{id}"), Some(&admin), None).await;
    let (s, _) = t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username":"Alex", "password":"password123"}))).await;
    assert_eq!(s, StatusCode::CONFLICT);
}

#[tokio::test]
async fn activity_cannot_impersonate_another_account() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    let body = json!({"kind":"launcher_open","instance_id":"", "username":"SomeoneElse"});
    assert_eq!(t.call("POST", "/api/v1/launcher/events", None, Some(body.clone())).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(t.call("POST", "/api/v1/launcher/events", Some(&admin), Some(body)).await.0, StatusCode::OK);
    let (_, rows) = t.call("GET", "/api/admin/activity?source=launcher", Some(&admin), None).await;
    assert_eq!(rows[0]["name"], "admin");
    assert_eq!(rows[0]["uuid"], t.uuid("admin").await);
    assert_eq!(t.call("GET", "/api/admin/activity", None, None).await.0, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn new_accounts_are_random_and_old_uuids_stay_fixed() {
    let t = setup().await;
    let id = t.uuid("admin").await;
    assert_eq!(uuid::Uuid::parse_str(&id).unwrap().get_version_num(), 4);
    assert_ne!(id, scopenet_shared::offline_uuid("admin"));
    assert!(sqlx::query("UPDATE users SET uuid='' WHERE username='admin'").execute(&t.db).await.is_err());
}
