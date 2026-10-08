//! Game server integration API: tokens, login checks, stats sync.

mod common;
use common::*;

async fn admin_and_player(t: &TestApp) -> (String, i64) {
    let admin = t.login("admin", "supersecret").await;
    let (s, v) = t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username": "Steve", "password": "password123"}))).await;
    assert_eq!(s, StatusCode::OK, "{v}");
    (admin, v["id"].as_i64().unwrap())
}

async fn create_server(t: &TestApp, admin: &str, body: Value) -> (i64, String) {
    let (s, v) = t.call("POST", "/api/admin/servers", Some(admin), Some(body)).await;
    assert_eq!(s, StatusCode::OK, "{v}");
    let token = v["token"].as_str().unwrap().to_string();
    assert!(token.starts_with("sn_"));
    assert_eq!(v["server"]["token_hint"], token[token.len() - 4..]);
    assert!(v["server"].get("token_hash").is_none());
    (v["server"]["id"].as_i64().unwrap(), token)
}

async fn steve(t: &TestApp) -> String {
    t.uuid("Steve").await
}

#[tokio::test]
async fn tokens_authenticate_servers() {
    let t = setup().await;
    let (admin, _) = admin_and_player(&t).await;
    let (id, token) = create_server(&t, &admin, json!({"name": "Survival"})).await;

    let (s, _) = t.call("POST", "/api/server/v1/hello", None, Some(json!({}))).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    let (s, _) = t.call("POST", "/api/server/v1/hello", Some("sn_nope"), Some(json!({}))).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);

    let (s, v) = t
        .call(
            "POST",
            "/api/server/v1/hello",
            Some(&token),
            Some(json!({"software": "Paper", "mc_version": "1.21.1", "plugin_version": "0.1.0", "online_mode": true, "max_players": 50})),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{v}");
    assert_eq!(v["server_id"], id);
    assert_eq!(v["yggdrasil_url"], "https://panel.test/api/yggdrasil");

    let (_, list) = t.call("GET", "/api/admin/servers", Some(&admin), None).await;
    assert_eq!(list[0]["software"], "Paper");
    assert_eq!(list[0]["online"], true);
    assert_eq!(list[0]["max_players"], 50);

    // Regenerating invalidates the old token.
    let (s, v) = t.call("POST", &format!("/api/admin/servers/{id}/token"), Some(&admin), None).await;
    assert_eq!(s, StatusCode::OK);
    let fresh = v["token"].as_str().unwrap();
    let (s, _) = t.call("POST", "/api/server/v1/hello", Some(&token), Some(json!({}))).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    let (s, _) = t.call("POST", "/api/server/v1/hello", Some(fresh), Some(json!({}))).await;
    assert_eq!(s, StatusCode::OK);

    // Players can't manage servers.
    let player = t.login("Steve", "password123").await;
    let (s, _) = t.call("GET", "/api/admin/servers", Some(&player), None).await;
    assert_eq!(s, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn login_rules() {
    let t = setup().await;
    let (admin, steve_id) = admin_and_player(&t).await;
    let login = |token: String, uuid: String, name: &'static str, ip: Option<&'static str>| {
        let t = &t;
        async move { t.call("POST", "/api/server/v1/login", Some(&token), Some(json!({"uuid": uuid, "name": name, "ip": ip}))).await.1 }
    };

    // "all": unknown players are fine, accounts are identified.
    let (_, open) = create_server(&t, &admin, json!({"name": "Open"})).await;
    let v = login(open.clone(), "00000000-0000-0000-0000-000000000001".into(), "Stranger", None).await;
    assert_eq!(v["allowed"], true, "{v}");
    let v = login(open.clone(), steve(&t).await.replace('-', ""), "Steve", None).await;
    assert_eq!(v["allowed"], true);
    assert_eq!(v["account"]["username"], "Steve");

    // "members": unknown players are refused.
    let (_, members) = create_server(&t, &admin, json!({"name": "Members", "access": "members"})).await;
    let v = login(members.clone(), "00000000-0000-0000-0000-000000000001".into(), "Stranger", None).await;
    assert_eq!(v["allowed"], false);
    assert!(v["message"].as_str().unwrap().contains("account"));

    // "groups": only listed groups (admins always pass).
    let (s, v) = t.call("POST", "/api/admin/servers", Some(&admin), Some(json!({"name": "Staff", "access": "groups"}))).await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "{v}");
    let (_, staff) = create_server(&t, &admin, json!({"name": "Staff", "access": "groups", "allowed_groups": ["builders"]})).await;
    let v = login(staff.clone(), steve(&t).await, "Steve", None).await;
    assert_eq!(v["allowed"], false);
    let admin_uuid = t.uuid("admin").await;
    let v = login(staff.clone(), admin_uuid, "admin", None).await;
    assert_eq!(v["allowed"], true, "{v}");
    t.call("POST", "/api/admin/groups", Some(&admin), Some(json!({"name": "builders"}))).await;
    let (s, v) = t.call("PATCH", &format!("/api/admin/users/{steve_id}"), Some(&admin), Some(json!({"groups": ["builders"]}))).await;
    assert_eq!(s, StatusCode::OK, "{v}");
    let v = login(staff.clone(), steve(&t).await, "Steve", None).await;
    assert_eq!(v["allowed"], true, "{v}");

    // Require launcher: a launch from the same IP is needed.
    let (_, strict) = create_server(&t, &admin, json!({"name": "Strict", "require_launcher": true})).await;
    let v = login(strict.clone(), steve(&t).await, "Steve", Some("203.0.113.9")).await;
    assert_eq!(v["allowed"], false);
    assert!(v["message"].as_str().unwrap().contains("launcher"));
    let player = t.login("Steve", "password123").await;
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/launcher/events")
        .header("authorization", format!("Bearer {player}"))
        .header("content-type", "application/json")
        .header("x-forwarded-for", "203.0.113.9")
        .body(Body::from(json!({"kind": "launch", "instance_id": "survival"}).to_string()))
        .unwrap();
    let (s, v) = t.send(req).await;
    assert_eq!(s, StatusCode::OK, "{v}");
    let v = login(strict.clone(), steve(&t).await, "Steve", Some("203.0.113.9")).await;
    assert_eq!(v["allowed"], true, "{v}");
    let v = login(strict.clone(), steve(&t).await, "Steve", Some("198.51.100.1")).await;
    assert_eq!(v["allowed"], false);

    // Disabled accounts see the reason.
    t.call(
        "PATCH",
        &format!("/api/admin/users/{steve_id}"),
        Some(&admin),
        Some(json!({"status": "disabled", "status_reason": "Griefing"})),
    )
    .await;
    let v = login(open, steve(&t).await, "Steve", None).await;
    assert_eq!(v["allowed"], false);
    assert!(v["message"].as_str().unwrap().contains("Griefing"));
}

#[tokio::test]
async fn sync_tracks_players_stats_and_kicks() {
    let t = setup().await;
    let (admin, steve_id) = admin_and_player(&t).await;
    let (id, token) = create_server(&t, &admin, json!({"name": "Survival"})).await;
    t.call("POST", "/api/server/v1/hello", Some(&token), Some(json!({"max_players": 20}))).await;

    let sync = json!({
        "tps": 19.8,
        "online": [{"uuid": steve(&t).await.replace('-', ""), "name": "Steve"}],
        "stats": [{"uuid": steve(&t).await, "name": "Steve", "playtime_secs": 30, "joins": 1, "blocks_broken": 12, "deaths": 1}],
        "events": [
            {"uuid": steve(&t).await, "name": "Steve", "kind": "join"},
            {"uuid": steve(&t).await, "name": "Steve", "kind": "death", "detail": "Steve fell from a high place"}
        ]
    });
    let (s, v) = t.call("POST", "/api/server/v1/sync", Some(&token), Some(sync)).await;
    assert_eq!(s, StatusCode::OK, "{v}");
    assert_eq!(v["kick"], json!([]));
    // Deltas add up.
    let (s, _) = t
        .call(
            "POST",
            "/api/server/v1/sync",
            Some(&token),
            Some(json!({"tps": 20.0, "online": [{"uuid": steve(&t).await, "name": "Steve"}], "stats": [{"uuid": steve(&t).await, "name": "Steve", "playtime_secs": 30, "blocks_broken": 3}]})),
        )
        .await;
    assert_eq!(s, StatusCode::OK);

    let (s, d) = t.call("GET", &format!("/api/admin/servers/{id}"), Some(&admin), None).await;
    assert_eq!(s, StatusCode::OK, "{d}");
    assert_eq!(d["server"]["players"], 1);
    assert_eq!(d["server"]["tps"], 20.0);
    assert_eq!(d["online_players"][0]["name"], "Steve");
    let row = &d["leaderboard"][0];
    assert_eq!(row["playtime_secs"], 60);
    assert_eq!(row["blocks_broken"], 15);
    assert_eq!(row["joins"], 1);
    assert_eq!(d["events"][0]["kind"], "death");
    assert_eq!(d["events"][1]["kind"], "join");
    assert_eq!(d["totals"]["players"], 1);

    let (_, dash) = t.call("GET", "/api/admin/stats", Some(&admin), None).await;
    assert_eq!(dash["live"]["online_now"], 1);

    let (_, users) = t.call("GET", "/api/admin/users", Some(&admin), None).await;
    let s_view = users.as_array().unwrap().iter().find(|u| u["username"] == "Steve").unwrap();
    assert_eq!(s_view["playtime_secs"], 60);
    let (s, act) = t.call("GET", &format!("/api/admin/users/{steve_id}/activity"), Some(&admin), None).await;
    assert_eq!(s, StatusCode::OK, "{act}");
    assert_eq!(act["servers"][0]["server_name"], "Survival");
    assert_eq!(act["online_on"][0]["name"], "Survival");

    // Disabling an online player asks the server to kick them.
    t.call(
        "PATCH",
        &format!("/api/admin/users/{steve_id}"),
        Some(&admin),
        Some(json!({"status": "disabled", "status_reason": "Cheating"})),
    )
    .await;
    let (_, v) =
        t.call("POST", "/api/server/v1/sync", Some(&token), Some(json!({"online": [{"uuid": steve(&t).await, "name": "Steve"}]}))).await;
    assert_eq!(v["kick"][0]["uuid"], steve(&t).await);
    assert!(v["kick"][0]["message"].as_str().unwrap().contains("Cheating"));

    // Leaving empties the online list; deleting the server removes its data.
    t.call("POST", "/api/server/v1/sync", Some(&token), Some(json!({"online": []}))).await;
    let (_, d) = t.call("GET", &format!("/api/admin/servers/{id}"), Some(&admin), None).await;
    assert_eq!(d["online_players"], json!([]));
    let (s, _) = t.call("DELETE", &format!("/api/admin/servers/{id}"), Some(&admin), None).await;
    assert_eq!(s, StatusCode::OK);
    let (s, _) = t.call("POST", "/api/server/v1/sync", Some(&token), Some(json!({}))).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn unknown_uuid_cannot_claim_a_members_name_or_skip_launcher_check() {
    let t = setup().await;
    let (admin, _) = admin_and_player(&t).await;
    let (_, token) = create_server(&t, &admin, json!({"name":"Private", "access":"members"})).await;
    let (_, verdict) =
        t.call("POST", "/api/server/v1/login", Some(&token), Some(json!({"uuid":uuid::Uuid::new_v4(),"name":"Steve"}))).await;
    assert_eq!(verdict["allowed"], false);
    let (_, strict) = create_server(&t, &admin, json!({"name":"Launcher", "require_launcher":true})).await;
    for body in [json!({"uuid":steve(&t).await,"name":"Steve"}), json!({"uuid":uuid::Uuid::new_v4(),"name":"Stranger","ip":"203.0.113.9"})]
    {
        let (_, verdict) = t.call("POST", "/api/server/v1/login", Some(&strict), Some(body)).await;
        assert_eq!(verdict["allowed"], false);
    }
}

#[tokio::test]
async fn retry_does_not_duplicate_stats_or_events() {
    let t = setup().await;
    let (admin, _) = admin_and_player(&t).await;
    let (id, token) = create_server(&t, &admin, json!({"name":"Retry"})).await;
    let body = json!({"batch_id":uuid::Uuid::new_v4(), "stats":[{"uuid":steve(&t).await,"name":"Steve","playtime_secs":30}],"events":[{"kind":"join","name":"Steve","uuid":steve(&t).await}]});
    for _ in 0..2 {
        assert_eq!(t.call("POST", "/api/server/v1/sync", Some(&token), Some(body.clone())).await.0, StatusCode::OK);
    }
    let (_, detail) = t.call("GET", &format!("/api/admin/servers/{id}"), Some(&admin), None).await;
    assert_eq!(detail["leaderboard"][0]["playtime_secs"], 30);
    assert_eq!(detail["events"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn untrusted_forwarded_header_cannot_forge_a_launcher_ip() {
    let t = setup().await;
    let (admin, _) = admin_and_player(&t).await;
    let token = t.login("Steve", "password123").await;
    let mut request = Request::post("/api/v1/launcher/events")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .header("x-forwarded-for", "203.0.113.9")
        .body(Body::from(json!({"kind":"launch","instance_id":"survival"}).to_string()))
        .unwrap();
    request.extensions_mut().insert(axum::extract::ConnectInfo("198.51.100.1:12345".parse::<std::net::SocketAddr>().unwrap()));
    assert_eq!(t.send(request).await.0, StatusCode::OK);
    let (_, strict) = create_server(&t, &admin, json!({"name":"Strict", "require_launcher":true})).await;
    let (_, verdict) = t
        .call("POST", "/api/server/v1/login", Some(&strict), Some(json!({"uuid":steve(&t).await,"name":"Steve","ip":"203.0.113.9"})))
        .await;
    assert_eq!(verdict["allowed"], false);
}
