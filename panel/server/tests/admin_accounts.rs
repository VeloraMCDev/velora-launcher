//! Real administrator routes exercise authority mutations and the host projection.
mod common;
use common::*;

#[tokio::test]
async fn administrator_accounts_preserve_group_wire_projection_partial_updates_and_self_protection() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    assert_eq!(t.call("POST", "/api/admin/groups", Some(&admin), Some(json!({"name":" Founders "}))).await.0, StatusCode::OK);
    let (status, created) = t
        .call(
            "POST",
            "/api/admin/users",
            Some(&admin),
            Some(json!({"username":"Awaiting", "password":"password123", "status":"pending", "groups":["Founders","Missing","Founders"]})),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{created}");
    let id = created["id"].as_i64().unwrap();
    let uuid = created["uuid"].clone();
    assert_eq!(created["groups"], json!(["Founders"]));
    assert!(created.get("password_hash").is_none());
    assert!(created.get("auth_version").is_none());
    assert_eq!(created["playtime_secs"], 0);
    let (status, listed) = t.call("GET", "/api/admin/users", Some(&admin), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(listed[0]["id"], id);
    let (_, groups) = t.call("GET", "/api/admin/groups", Some(&admin), None).await;
    assert_eq!(groups[0]["members"], 1);
    assert_eq!(groups[0]["color"], "#7c5cff");
    assert_eq!(
        t.call(
            "PATCH",
            &format!("/api/admin/users/{id}"),
            Some(&admin),
            Some(json!({"email":" changed@example.invalid ", "role":"invalid"}))
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    let stored_email: String = sqlx::query_scalar("SELECT email FROM users WHERE id=?").bind(id).fetch_one(&t.db).await.unwrap();
    assert_eq!(stored_email, "changed@example.invalid", "legacy earlier field changes survive a later validation error");
    let (status, approved) = t
        .call("PATCH", &format!("/api/admin/users/{id}"), Some(&admin), Some(json!({"username":"Ignored", "status":"active", "groups":[]})))
        .await;
    assert_eq!(status, StatusCode::OK, "{approved}");
    assert_eq!(approved["uuid"], uuid);
    assert_eq!(approved["username"], "Awaiting");
    assert_eq!(approved["groups"], json!([]));
    let player = t.login("Awaiting", "password123").await;
    assert_eq!(t.call("GET", "/api/admin/users", Some(&player), None).await.0, StatusCode::FORBIDDEN);
    let (_, me) = t.call("GET", "/api/v1/auth/me", Some(&admin), None).await;
    let admin_id = me["id"].as_i64().unwrap();
    for body in [json!({"role":"player"}), json!({"status":"disabled"})] {
        assert_eq!(t.call("PATCH", &format!("/api/admin/users/{admin_id}"), Some(&admin), Some(body)).await.0, StatusCode::BAD_REQUEST);
    }
    assert_eq!(t.call("DELETE", &format!("/api/admin/users/{admin_id}"), Some(&admin), None).await.0, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn administrator_password_revocation_is_atomic_and_uuid_is_stable() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    let (_, user) = t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username":"Example", "password":"password123"}))).await;
    let id = user["id"].as_i64().unwrap();
    let (_, session) = t.call("POST", "/api/v1/auth/login", None, Some(json!({"username":"Example", "password":"password123"}))).await;
    let web = session["token"].as_str().unwrap();
    let game = session["yggdrasil"]["access_token"].as_str().unwrap();
    sqlx::query("INSERT INTO ygg_sessions (server_id, user_id, created_at) VALUES ('synthetic-server', ?, '2999-01-01T00:00:00Z')")
        .bind(id)
        .execute(&t.db)
        .await
        .unwrap();
    sqlx::raw_sql("CREATE TRIGGER synthetic_admin_revoke_failure BEFORE DELETE ON ygg_sessions BEGIN SELECT RAISE(ABORT, 'synthetic revocation failure'); END;").execute(&t.db).await.unwrap();
    assert_eq!(
        t.call("PATCH", &format!("/api/admin/users/{id}"), Some(&admin), Some(json!({"password":"replacement123"}))).await.0,
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(t.call("GET", "/api/v1/auth/me", Some(web), None).await.0, StatusCode::OK);
    assert_eq!(
        t.call("POST", "/api/yggdrasil/authserver/validate", None, Some(json!({"accessToken":game}))).await.0,
        StatusCode::NO_CONTENT
    );
    sqlx::raw_sql("DROP TRIGGER synthetic_admin_revoke_failure;").execute(&t.db).await.unwrap();
    let (status, replaced) =
        t.call("PATCH", &format!("/api/admin/users/{id}"), Some(&admin), Some(json!({"password":"replacement123"}))).await;
    assert_eq!(status, StatusCode::OK, "{replaced}");
    assert_eq!(replaced["uuid"], user["uuid"]);
    assert_eq!(t.call("GET", "/api/v1/auth/me", Some(web), None).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(
        t.call("POST", "/api/yggdrasil/authserver/validate", None, Some(json!({"accessToken":game}))).await.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        t.call("POST", "/api/v1/auth/login", None, Some(json!({"username":"Example", "password":"password123"}))).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        t.call("POST", "/api/v1/auth/login", None, Some(json!({"username":"Example", "password":"replacement123"}))).await.0,
        StatusCode::OK
    );
}
