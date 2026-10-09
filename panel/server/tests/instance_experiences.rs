mod common;
use common::{json, Body, Request, ServiceExt, StatusCode, Value};
use scopenet_panel::{app, auth, bootstrap_admin, build_state_with_keys, config::Config, db, state::AppState};

struct Platform {
    state: AppState,
    router: axum::Router,
    token: String,
    _dir: tempfile::TempDir,
}
impl Platform {
    async fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let cfg = Config {
            bind: "127.0.0.1:0".into(),
            data_dir: dir.path().into(),
            web_dir: dir.path().join("web"),
            icons_dir: dir.path().join("icons"),
            admin_username: "admin".into(),
            admin_password: Some("test-admin-pass".into()),
            jwt_secret: Some("test-instance-signing-secret-with-32-bytes".into()),
            curseforge_api_key: None,
            max_upload_mb: 64,
            public_url: None,
            trusted_proxies: vec![],
        };
        let pool = db::connect(dir.path()).await.unwrap();
        let state = build_state_with_keys(cfg, pool, common::test_keys()).await.unwrap();
        bootstrap_admin(&state).await.unwrap();
        let admin = auth::find_user_by_name(&state, "admin").await.unwrap().unwrap();
        let token = state.keys.issue(&admin).unwrap();
        Self { router: app(state.clone()), state, token, _dir: dir }
    }
    async fn call(&self, method: &str, path: &str, instance: Option<&str>, token: &str, body: Option<Value>) -> (StatusCode, Value) {
        let mut req = Request::builder().method(method).uri(path).header("Authorization", format!("Bearer {token}"));
        if let Some(id) = instance {
            req = req.header("X-SCOPENET-Instance", id);
        }
        let body = if let Some(body) = body {
            req = req.header("Content-Type", "application/json");
            Body::from(body.to_string())
        } else {
            Body::empty()
        };
        let response = self.router.clone().oneshot(req.body(body).unwrap()).await.unwrap();
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), 16 * 1024 * 1024).await.unwrap();
        (status, serde_json::from_slice(&body).unwrap_or_else(|_| json!(String::from_utf8_lossy(&body))))
    }
    async fn instance(&self, name: &str) -> String {
        let (status, body) = self
            .call("POST", "/api/admin/instances", None, &self.token, Some(json!({"name":name,"mc_version":"1.21.1","loader":"vanilla"})))
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        body["id"].as_str().unwrap().into()
    }
}

#[tokio::test]
async fn profile_posts_use_shared_platform_storage_from_persistent_instances() {
    let p = Platform::new().await;
    let first = p.instance("First social experience").await;
    let second = p.instance("Second social experience").await;
    let (status, body) = p.call("POST", "/api/admin/users", None, &p.token,
        Some(json!({"username":"SocialReader","password":"fixture-player-pass"}))).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, login) = p.call("POST", "/api/v1/auth/login", None, "",
        Some(json!({"username":"SocialReader","password":"fixture-player-pass"}))).await;
    assert_eq!(status, StatusCode::OK, "{login}");
    let reader = login["token"].as_str().unwrap();
    let owner = auth::find_user_by_name(&p.state, "admin").await.unwrap().unwrap();

    let (status, _) = p.call("POST", "/api/v1/profiles/me/posts", Some(&first), &p.token,
        Some(json!({"content":"   "}))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let mut ids = Vec::new();
    for instance in [&first, &second] {
        let (status, post) = p.call("POST", "/api/v1/profiles/me/posts", Some(instance), &p.token,
            Some(json!({"content":"  Shared profile post  "}))).await;
        assert_eq!(status, StatusCode::OK, "{post}");
        assert_eq!(post["content"], "Shared profile post");
        assert_eq!(post["user_uuid"], owner.uuid);
        ids.push(post["id"].as_i64().unwrap());
    }
    assert_ne!(ids[0], ids[1], "Post IDs belong to the shared platform store");
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM user_posts").fetch_one(&p.state.platform_db).await.unwrap();
    assert_eq!(count, 2);
    for instance in [&first, &second] {
        let scoped = p.state.experiences.state(&p.state, instance).await.unwrap();
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM main.user_posts").fetch_one(&scoped.db).await.unwrap();
        assert_eq!(count, 0, "No duplicate profile history in an experience database");
        let (status, profile) = p.call("GET", &format!("/api/v1/profiles/{}", owner.uuid), Some(instance), reader, None).await;
        assert_eq!(status, StatusCode::OK, "{profile}");
        assert_eq!(profile["posts"].as_array().unwrap().len(), 2);
    }
    let like = format!("/api/v1/posts/{}/like", ids[0]);
    for (instance, expected) in [(&second, true), (&first, false)] {
        let (status, body) = p.call("POST", &like, Some(instance), reader, None).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["liked"], expected);
    }
    let count: i64 = sqlx::query_scalar("SELECT likes_count FROM user_posts WHERE id=?").bind(ids[0])
        .fetch_one(&p.state.platform_db).await.unwrap();
    assert_eq!(count, 1);
    let delete = format!("/api/v1/profiles/me/posts/{}", ids[0]);
    let (status, body) = p.call("DELETE", &delete, Some(&second), reader, None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM user_posts WHERE id=?)").bind(ids[0])
        .fetch_one(&p.state.platform_db).await.unwrap();
    assert!(exists, "Another account cannot delete the author's post");
    let (status, body) = p.call("DELETE", &delete, Some(&second), &p.token, None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let likes: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM user_post_likes WHERE post_id=?").bind(ids[0])
        .fetch_one(&p.state.platform_db).await.unwrap();
    assert_eq!(likes, 0, "Deletion cascades shared likes");
    p.state.experiences.retire(&first).await;
    let (status, profile) = p.call("GET", &format!("/api/v1/profiles/{}", owner.uuid), Some(&first), reader, None).await;
    assert_eq!(status, StatusCode::OK, "{profile}");
    assert_eq!(profile["posts"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn velora_and_legacy_instance_headers_select_the_same_store_and_conflicts_fail_closed() {
    let p = Platform::new().await;
    let a = p.instance("First operator instance").await;
    let b = p.instance("Second operator instance").await;
    for (modern, legacy, query, expected) in [
        (Some(a.as_str()), None, None, StatusCode::OK),
        (None, Some(a.as_str()), None, StatusCode::OK),
        (Some(a.as_str()), Some(a.as_str()), None, StatusCode::OK),
        (Some(a.as_str()), Some(b.as_str()), None, StatusCode::BAD_REQUEST),
        (Some(a.as_str()), None, Some(b.as_str()), StatusCode::BAD_REQUEST),
        (Some(a.as_str()), Some(a.as_str()), Some(a.as_str()), StatusCode::OK),
    ] {
        let uri = query.map(|id| format!("/api/admin/progression?instance={id}")).unwrap_or_else(|| "/api/admin/progression".into());
        let mut request = Request::builder().uri(uri).header("authorization", format!("Bearer {}", p.token));
        if let Some(id) = modern { request = request.header("X-Velora-Instance", id); }
        if let Some(id) = legacy { request = request.header("X-SCOPENET-Instance", id); }
        let response = p.router.clone().oneshot(request.body(Body::empty()).unwrap()).await.unwrap();
        assert_eq!(response.status(), expected, "modern={modern:?} legacy={legacy:?} query={query:?}");
    }
}

#[tokio::test]
async fn instance_settings_history_and_capabilities_are_isolated_but_identity_is_shared() {
    let p = Platform::new().await;
    let smp = p.instance("SCOPENET SMP").await;
    let frontiers = p.instance("SCOPENET Frontiers").await;
    let (status, _) = p.call("GET", "/api/admin/progression", None, &p.token, None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "ambiguous scope must never expose a global gameplay store");
    let (status, body) = p.call("PUT", "/api/admin/progression", Some(&smp), &p.token, Some(json!({"level_base":300.0}))).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (_, a) = p.call("GET", "/api/admin/progression", Some(&smp), &p.token, None).await;
    let (_, b) = p.call("GET", "/api/admin/progression", Some(&frontiers), &p.token, None).await;
    assert_eq!(a["settings"]["level_base"], 300.0);
    assert_eq!(b["settings"]["level_base"], 100.0);
    let a = p.state.experiences.state(&p.state, &smp).await.unwrap();
    let b = p.state.experiences.state(&p.state, &frontiers).await.unwrap();
    let user = auth::find_user_by_name(&a, "admin").await.unwrap().unwrap();
    assert_eq!(auth::find_user_by_name(&b, "admin").await.unwrap().unwrap().uuid, user.uuid);
    let copied_accounts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM main.users").fetch_one(&a.db).await.unwrap();
    assert_eq!(copied_accounts, 0, "credentials must not be copied into an experience");
    sqlx::query("INSERT INTO user_levels(uuid,global_xp,updated_at) VALUES(?,1234,?)")
        .bind(&user.uuid)
        .bind(db::now())
        .execute(&a.db)
        .await
        .unwrap();
    let (_, levels_a) = p.call("GET", "/api/v1/levels/me", Some(&smp), &p.token, None).await;
    let (_, levels_b) = p.call("GET", "/api/v1/levels/me", Some(&frontiers), &p.token, None).await;
    assert_eq!(levels_a["global_xp"], 1234);
    assert_eq!(levels_b["global_xp"], 0);
    let (status, body) = p
        .call(
            "PUT",
            &format!("/api/admin/instances/{frontiers}/experience"),
            None,
            &p.token,
            Some(json!({"kind":"frontiers","features":["maps","content","events","companion"]})),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, _) = p.call("GET", "/api/v1/casino/config/1", Some(&frontiers), &p.token, None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = p.call("GET", "/api/v1/levels/me", Some(&frontiers), &p.token, None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, me) = p.call("GET", "/api/v1/auth/me", Some(&frontiers), &p.token, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(me["username"], "admin");
}

#[tokio::test]
async fn server_credentials_select_their_experience_and_cannot_cross_it() {
    let p = Platform::new().await;
    let a = p.instance("Alpha").await;
    let b = p.instance("Beta").await;
    let (status, server) =
        p.call("POST", "/api/admin/servers", Some(&a), &p.token, Some(json!({"name":"Alpha server","instance_id":a}))).await;
    assert_eq!(status, StatusCode::OK, "{server}");
    let token = server["token"].as_str().unwrap();
    let (status, _) = p.call("POST", "/api/server/v1/hello", Some(&b), token, Some(json!({}))).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, body) = p.call("POST", "/api/server/v1/hello", None, token, Some(json!({"software":"paper"}))).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (_, servers) = p.call("GET", "/api/admin/servers", Some(&b), &p.token, None).await;
    assert_eq!(servers.as_array().unwrap().len(), 0);
    let (status, _) = p
        .call(
            "PUT",
            &format!("/api/admin/servers/{}", server["server"]["id"]),
            Some(&a),
            &p.token,
            Some(json!({"name":"Wrong owner","instance_id":b})),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn legacy_adoption_is_repeatable_and_preserves_the_source_history() {
    let p = Platform::new().await;
    let a = p.instance("Alpha").await;
    let b = p.instance("Beta").await;
    let user = auth::find_user_by_name(&p.state, "admin").await.unwrap().unwrap();
    for (id, balance) in [(&a, 2500.0), (&b, 7500.0)] {
        let sid: i64 = sqlx::query_scalar(
            "INSERT INTO game_servers(name,token_hash,token_hint,instance_id,created_at) VALUES(?,?,?,?,?) RETURNING id",
        )
        .bind(id)
        .bind(format!("token-{id}"))
        .bind("legacy")
        .bind(id)
        .bind(db::now())
        .fetch_one(&p.state.db)
        .await
        .unwrap();
        sqlx::query("INSERT INTO server_economy(server_id,uuid,username,balance,updated_at) VALUES(?,?,?,?,?)")
            .bind(sid)
            .bind(&user.uuid)
            .bind(&user.username)
            .bind(balance)
            .bind(db::now())
            .execute(&p.state.db)
            .await
            .unwrap();
    }
    sqlx::query("INSERT INTO user_levels(uuid,global_xp,updated_at) VALUES(?,4321,?)")
        .bind(&user.uuid)
        .bind(db::now())
        .execute(&p.state.db)
        .await
        .unwrap();
    let oldest: String =
        sqlx::query_scalar("SELECT id FROM instances ORDER BY created_at,id LIMIT 1").fetch_one(&p.state.db).await.unwrap();
    for id in [&a, &b] {
        let state = p.state.experiences.state(&p.state, id).await.unwrap();
        let balances: Vec<f64> =
            sqlx::query_scalar("SELECT balance FROM server_economy WHERE uuid=?").bind(&user.uuid).fetch_all(&state.db).await.unwrap();
        assert_eq!(balances, vec![if id == &a { 2500.0 } else { 7500.0 }]);
        let xp: Option<i64> =
            sqlx::query_scalar("SELECT global_xp FROM user_levels WHERE uuid=?").bind(&user.uuid).fetch_optional(&state.db).await.unwrap();
        assert_eq!(xp.unwrap_or(0), if id == &oldest { 4321 } else { 0 });
        assert!(Arc::ptr_eq(&state.worldmap, &p.state.experiences.state(&p.state, id).await.unwrap().worldmap));
    }
    let source: i64 =
        sqlx::query_scalar("SELECT global_xp FROM user_levels WHERE uuid=?").bind(&user.uuid).fetch_one(&p.state.db).await.unwrap();
    assert_eq!(source, 4321);
}
use std::sync::Arc;

#[tokio::test]
async fn module_snapshots_and_group_integrations_belong_to_the_instance() {
    let p = Platform::new().await;
    let a = p.instance("Alpha").await;
    let b = p.instance("Beta").await;
    for id in [&a, &b] {
        let (status, body) = p
            .call(
                "PUT",
                &format!("/api/admin/instances/{id}/experience"),
                None,
                &p.token,
                Some(json!({"modules":{"frontiers":{"settlements":[]}}})),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
    }
    let (_, server) = p.call("POST", "/api/admin/servers", Some(&a), &p.token, Some(json!({"name":"Alpha","instance_id":a}))).await;
    let token = server["token"].as_str().unwrap();
    let (status, body) = p
        .call(
            "PUT",
            "/api/server/v1/experience/modules/frontiers",
            None,
            token,
            Some(json!({"settlements":[{"name":"Oakvale","citizens":12}]})),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, _) = p.call("PUT", "/api/server/v1/experience/modules/unknown", None, token, Some(json!({}))).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (_, manifest) = p.call("GET", "/api/v1/launcher/manifest", None, &p.token, None).await;
    let instances = manifest["instances"].as_array().unwrap();
    assert_eq!(instances.iter().find(|i| i["id"] == a).unwrap()["experience"]["modules"]["frontiers"]["settlements"][0]["name"], "Oakvale");
    assert_eq!(instances.iter().find(|i| i["id"] == b).unwrap()["experience"]["modules"]["frontiers"]["settlements"], json!([]));
    let (_, group) = p.call("POST", "/api/admin/groups", None, &p.token, Some(json!({"name":"Builders","color":"#112233"}))).await;
    let (_, groups) = p.call("GET", "/api/admin/groups", None, &p.token, None).await;
    let gid = groups.as_array().unwrap().iter().find(|g| g["name"] == "Builders").expect(&group.to_string())["id"].as_i64().unwrap();
    let (status, body) = p
        .call("PUT", &format!("/api/admin/groups/{gid}/luckperms"), Some(&a), &p.token, Some(json!({"luckperms_group":"alpha-builder"})))
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, body) =
        p.call("PUT", &format!("/api/admin/groups/{gid}/discord-role"), Some(&a), &p.token, Some(json!({"discord_role":"123456"}))).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    for (id, lp, role) in [(&a, "alpha-builder", "123456"), (&b, "", "")] {
        let (_, groups) = p.call("GET", "/api/admin/instance-groups", Some(id), &p.token, None).await;
        let group = groups.as_array().unwrap().iter().find(|g| g["id"] == gid).unwrap();
        assert_eq!(group["luckperms_group"], lp);
        assert_eq!(group["discord_role"], role);
    }
    let (status, _) = p.call("GET", "/api/yggdrasil", None, &p.token, None).await;
    assert_eq!(status, StatusCode::OK, "game authentication remains global with multiple instances");
}

#[tokio::test]
async fn legacy_maps_and_servers_survive_adoption_and_deleted_storage_is_not_reused() {
    let p = Platform::new().await;
    let a = p.instance("Alpha").await;
    p.instance("Beta").await;
    sqlx::query(
        "INSERT INTO game_servers(name,token_hash,token_hint,instance_id,created_at) VALUES('Legacy','legacy-token','legacy','',?)",
    )
    .bind(db::now())
    .execute(&p.state.db)
    .await
    .unwrap();
    let sid: i64 = sqlx::query_scalar("SELECT id FROM game_servers WHERE token_hash='legacy-token'").fetch_one(&p.state.db).await.unwrap();
    let original = p.state.cfg.data_dir.join("map").join(sid.to_string()).join("overworld");
    std::fs::create_dir_all(&original).unwrap();
    std::fs::write(original.join("tile.png"), b"saved tile").unwrap();
    let scoped = p.state.experiences.state(&p.state, &a).await.unwrap();
    assert_eq!(std::fs::read(scoped.cfg.data_dir.join("map").join(sid.to_string()).join("overworld/tile.png")).unwrap(), b"saved tile");
    assert_eq!(std::fs::read(original.join("tile.png")).unwrap(), b"saved tile");
    let owner: String =
        sqlx::query_scalar("SELECT instance_id FROM game_servers WHERE id=?").bind(sid).fetch_one(&p.state.db).await.unwrap();
    assert_eq!(owner, a);
    scopenet_panel::store::kv_set(&scoped, "custom", &json!({"version":2})).await.unwrap();
    let mut restarted = p.state.clone();
    restarted.experiences = Arc::new(scopenet_panel::experience::ExperienceStores::default());
    let reopened = restarted.experiences.state(&restarted, &a).await.unwrap();
    let custom: Value = scopenet_panel::store::kv_get(&reopened, "custom").await.unwrap();
    assert_eq!(custom["version"], 2, "restarts do not replay legacy configuration");
    restarted.experiences.retire(&a).await;
    let (status, body) = p.call("DELETE", &format!("/api/admin/instances/{a}"), None, &p.token, None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(scoped.db.is_closed());
    let replacement = p.instance("Alpha").await;
    assert_ne!(replacement, a, "retained gameplay storage must not become another instance's data");
}

#[tokio::test]
async fn account_deletion_cleans_gameplay_from_every_experience() {
    let p = Platform::new().await;
    let a = p.instance("Alpha").await;
    let b = p.instance("Beta").await;
    auth::create_user(&p.state, "builder", "test-player-password", None, "player", "active").await.unwrap();
    let user = auth::find_user_by_name(&p.state, "builder").await.unwrap().unwrap();
    for id in [&a, &b] {
        let scoped = p.state.experiences.state(&p.state, id).await.unwrap();
        sqlx::query("INSERT INTO user_levels(uuid,global_xp,updated_at) VALUES(?,500,?)")
            .bind(&user.uuid)
            .bind(db::now())
            .execute(&scoped.db)
            .await
            .unwrap();
    }
    let (status, body) = p.call("DELETE", &format!("/api/admin/users/{}", user.id), None, &p.token, None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(auth::find_user_by_name(&p.state, "builder").await.unwrap().is_none());
    for id in [&a, &b] {
        let scoped = p.state.experiences.state(&p.state, id).await.unwrap();
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM user_levels WHERE uuid=?").bind(&user.uuid).fetch_one(&scoped.db).await.unwrap();
        assert_eq!(count, 0);
    }
}

#[tokio::test]
async fn immutable_resource_pack_downloads_keep_legacy_minecraft_urls() {
    let p = Platform::new().await;
    let a = p.instance("Alpha").await;
    p.instance("Beta").await;
    let scoped = p.state.experiences.state(&p.state, &a).await.unwrap();
    scopenet_panel::store::kv_set(&scoped, "resource_pack", &json!({"enabled":true,"required":false,"pack_format":84})).await.unwrap();
    let (_, server) = p.call("POST", "/api/admin/servers", Some(&a), &p.token, Some(json!({"name":"Alpha"}))).await;
    let token = server["token"].as_str().unwrap();
    let (status, pack) = p.call("POST", "/api/server/v1/resource-pack", None, token, None).await;
    assert_eq!(status, StatusCode::OK, "{pack}");
    let hash = pack["sha1"].as_str().unwrap();
    let response = p
        .router
        .clone()
        .oneshot(Request::builder().uri(format!("/api/v1/resource-pack.zip?revision={hash}")).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 16 * 1024 * 1024).await.unwrap();
    assert_eq!(&bytes[..2], b"PK");
}

#[tokio::test]
async fn private_experiences_use_live_platform_access_and_task_capabilities() {
    let p = Platform::new().await;
    let a = p.instance("Alpha").await;
    p.instance("Beta").await;
    sqlx::query("UPDATE instances SET visibility='members' WHERE id=?").bind(&a).execute(&p.state.db).await.unwrap();
    let response = p
        .router
        .clone()
        .oneshot(Request::builder().uri("/api/v1/servers/public").header("X-SCOPENET-Instance", &a).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    auth::create_user(&p.state, "builder", "test-player-password", None, "player", "active").await.unwrap();
    let user = auth::find_user_by_name(&p.state, "builder").await.unwrap().unwrap();
    let token = p.state.keys.issue(&user).unwrap();
    let (status, _) = p.call("GET", "/api/v1/servers/public", Some(&a), &token, None).await;
    assert_eq!(status, StatusCode::OK);
    sqlx::query("UPDATE users SET auth_version=auth_version+1 WHERE id=?").bind(user.id).execute(&p.state.db).await.unwrap();
    let (status, _) = p.call("GET", "/api/v1/servers/public", Some(&a), &token, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, body) = p
        .call(
            "PUT",
            &format!("/api/admin/instances/{a}/experience"),
            None,
            &p.token,
            Some(json!({"features":["maps","content","events","companion"]})),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, tasks) = p.call("GET", "/api/admin/tasks", Some(&a), &p.token, None).await;
    assert_eq!(status, StatusCode::OK, "{tasks}");
    assert!(tasks["tasks"].as_array().unwrap().iter().all(|t| t["category"] != "Economy" && t["category"] != "Progression"));
}

#[tokio::test]
async fn shared_player_reports_aggregate_live_instance_activity_without_copying_stats() {
    let p = Platform::new().await;
    let a = p.instance("Alpha").await;
    let b = p.instance("Beta").await;
    let user = auth::find_user_by_name(&p.state, "admin").await.unwrap().unwrap();
    for id in [&a, &b] {
        let (_, server) = p.call("POST", "/api/admin/servers", Some(id), &p.token, Some(json!({"name":id}))).await;
        let token = server["token"].as_str().unwrap();
        let (status, body) = p
            .call(
                "POST",
                "/api/server/v1/sync",
                None,
                token,
                Some(json!({"online":[{"uuid":user.uuid,"name":"admin"}],"stats":[{"uuid":user.uuid,"name":"admin","playtime_secs":30}]})),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
    }
    let (status, activity) = p.call("GET", &format!("/api/admin/users/{}/activity", user.id), None, &p.token, None).await;
    assert_eq!(status, StatusCode::OK, "{activity}");
    assert_eq!(activity["servers"].as_array().unwrap().len(), 2);
    assert_eq!(activity["online_on"].as_array().unwrap().len(), 2);
    let (_, users) = p.call("GET", "/api/admin/users", None, &p.token, None).await;
    assert_eq!(users.as_array().unwrap()[0]["playtime_secs"], 60);
    let copied: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM player_stats").fetch_one(&p.state.db).await.unwrap();
    assert_eq!(copied, 0, "global profiles use reporting views, not a second gameplay history");
}
