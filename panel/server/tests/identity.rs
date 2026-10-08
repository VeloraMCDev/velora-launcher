mod common;
use common::*;

#[tokio::test]
async fn launcher_admission_keeps_event_partial_commits_and_rename_audit_rollback() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    let (status, user) =
        t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username":"Original", "password":"password123"}))).await;
    assert_eq!(status, StatusCode::OK, "{user}");
    let id = user["id"].as_i64().unwrap();
    let token = t.login("Original", "password123").await;
    let send_launch = |kind: &str| {
        Request::post("/api/v1/launcher/events")
            .header("authorization", format!("Bearer {token}"))
            .header("x-forwarded-for", "203.0.113.8")
            .header("content-type", "application/json")
            .body(Body::from(json!({"kind":kind,"instance_id":"x".repeat(70),"username":"Spoofed","uuid":"spoofed"}).to_string()))
            .unwrap()
    };
    let (status, _) = t.send(send_launch("unknown")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM launcher_sessions").fetch_one(&t.db).await.unwrap(), 0);
    let (status, value) = t.send(send_launch("launch")).await;
    assert_eq!(status, StatusCode::OK, "{value}");
    let session: (i64, String, String) =
        sqlx::query_as("SELECT user_id,ip,username FROM launcher_sessions").fetch_one(&t.db).await.unwrap();
    assert_eq!(session, (id, "203.0.113.8".into(), "Original".into()));
    let event: (String, String, String, String) =
        sqlx::query_as("SELECT username,uuid,instance_id,source FROM events WHERE kind='launch'").fetch_one(&t.db).await.unwrap();
    assert_eq!(event, ("Original".into(), user["uuid"].as_str().unwrap().into(), "x".repeat(64), "launcher".into()));

    sqlx::raw_sql("INSERT INTO launcher_sessions(user_id,ip,created_at) VALUES(NULL,'198.51.100.1','2000-01-01T00:00:00Z');
        CREATE TRIGGER fail_launch_audit BEFORE INSERT ON events WHEN NEW.kind='launch' BEGIN SELECT RAISE(ABORT,'synthetic audit failure'); END;")
        .execute(&t.db).await.unwrap();
    assert_eq!(t.send(send_launch("launch")).await.0, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM launcher_sessions WHERE user_id=?").bind(id).fetch_one(&t.db).await.unwrap(),
        2
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM launcher_sessions WHERE user_id IS NULL").fetch_one(&t.db).await.unwrap(),
        0
    );
    assert_eq!(sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM events WHERE kind='launch'").fetch_one(&t.db).await.unwrap(), 1);
    sqlx::raw_sql("DROP TRIGGER fail_launch_audit;
        CREATE TRIGGER fail_rename_audit BEFORE INSERT ON events WHEN NEW.kind='username_change' BEGIN SELECT RAISE(ABORT,'synthetic audit failure'); END;")
        .execute(&t.db).await.unwrap();
    let rename = json!({"username":"Renamed", "password":"password123"});
    assert_eq!(t.call("PUT", "/api/v1/account/username", Some(&token), Some(rename.clone())).await.0, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(t.call("GET", "/api/v1/auth/me", Some(&token), None).await.0, StatusCode::OK);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM launcher_sessions WHERE user_id=?").bind(id).fetch_one(&t.db).await.unwrap(),
        2
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT username FROM users WHERE id=?").bind(id).fetch_one(&t.db).await.unwrap(),
        "Original"
    );
    sqlx::query("DROP TRIGGER fail_rename_audit").execute(&t.db).await.unwrap();
    let (status, renamed) = t.call("PUT", "/api/v1/account/username", Some(&token), Some(rename)).await;
    assert_eq!(status, StatusCode::OK, "{renamed}");
    assert_eq!(renamed["user"]["uuid"], user["uuid"]);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM launcher_sessions WHERE user_id=?").bind(id).fetch_one(&t.db).await.unwrap(),
        0
    );
    assert_eq!(t.call("GET", "/api/v1/auth/me", Some(&token), None).await.0, StatusCode::UNAUTHORIZED);
}

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
    sqlx::query("UPDATE game_servers SET last_seen=? WHERE id=?")
        .bind(scopenet_panel::db::now())
        .bind(server_id)
        .execute(&t.db)
        .await
        .unwrap();
    sqlx::query("INSERT INTO server_online (server_id, uuid, name, joined_at) VALUES (?, ?, 'Steve', 'x')")
        .bind(server_id)
        .bind(&uuid)
        .execute(&t.db)
        .await
        .unwrap();
    let (status, busy) =
        t.call("PUT", "/api/v1/account/username", Some(old_token), Some(json!({"username":"Alex", "password":"password123"}))).await;
    assert_eq!(status, StatusCode::CONFLICT, "{busy}");
    assert_eq!(busy["error"], "disconnect from your servers before changing your username");
    assert_eq!(t.call("GET", "/api/v1/auth/me", Some(old_token), None).await.0, StatusCode::OK);
    // A stale presence row must not prevent renaming after the server stops reporting.
    sqlx::query("UPDATE game_servers SET last_seen='2000-01-01T00:00:00Z' WHERE id=?").bind(server_id).execute(&t.db).await.unwrap();
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

#[tokio::test]
async fn failed_rename_audit_rolls_back_identity_and_session_revocation() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    let (status, _) = t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username":"Before", "password":"password123"}))).await;
    assert_eq!(status, StatusCode::OK);
    let (_, auth) = t.call("POST", "/api/v1/auth/login", None, Some(json!({"username":"Before", "password":"password123"}))).await;
    let token = auth["token"].as_str().unwrap();
    let game_token = auth["yggdrasil"]["access_token"].as_str().unwrap();
    let original_uuid = t.uuid("Before").await;
    sqlx::raw_sql("CREATE TRIGGER synthetic_audit_failure BEFORE INSERT ON events WHEN NEW.kind='username_change' BEGIN SELECT RAISE(ABORT,'synthetic audit outage'); END;")
        .execute(&t.db).await.unwrap();
    let (status, _) =
        t.call("PUT", "/api/v1/account/username", Some(token), Some(json!({"username":"After", "password":"password123"}))).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(t.uuid("Before").await, original_uuid);
    assert_eq!(t.call("GET", "/api/v1/auth/me", Some(token), None).await.0, StatusCode::OK);
    assert_eq!(
        t.call("POST", "/api/yggdrasil/authserver/validate", None, Some(json!({"accessToken":game_token}))).await.0,
        StatusCode::NO_CONTENT
    );
    let renamed_count: i64 = sqlx::query_scalar("SELECT count(*) FROM users WHERE username='After'").fetch_one(&t.db).await.unwrap();
    assert_eq!(renamed_count, 0);
    sqlx::query("DROP TRIGGER synthetic_audit_failure").execute(&t.db).await.unwrap();
    let (status, renamed) =
        t.call("PUT", "/api/v1/account/username", Some(token), Some(json!({"username":"After", "password":"password123"}))).await;
    assert_eq!(status, StatusCode::OK, "{renamed}");
    assert_eq!(renamed["user"]["uuid"], original_uuid);
    assert_eq!(t.call("GET", "/api/v1/auth/me", Some(token), None).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(
        t.call("POST", "/api/yggdrasil/authserver/validate", None, Some(json!({"accessToken":game_token}))).await.0,
        StatusCode::FORBIDDEN
    );
}
