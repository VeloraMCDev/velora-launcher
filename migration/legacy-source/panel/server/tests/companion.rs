mod common;
use common::*;

#[tokio::test]
async fn guild_reads_use_the_servers_instance_and_the_players_actual_role() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    for name in ["Alex", "Steve", "Zed"] {
        t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username":name,"password":"password123"}))).await;
    }
    let alex = t.uuid("Alex").await;
    let steve = t.uuid("Steve").await;
    let zed = t.uuid("Zed").await;
    let player = t.login("Alex", "password123").await;
    let (_, srv) = t.call("POST", "/api/admin/servers", Some(&admin), Some(json!({"name":"SMP","instance_id":"smp"}))).await;
    let sid = srv["server"]["id"].as_i64().unwrap();
    let token = srv["token"].as_str().unwrap();
    let (_, other) = t.call("POST", "/api/admin/servers", Some(&admin), Some(json!({"name":"Other","instance_id":"other"}))).await;
    let other_sid = other["server"]["id"].as_i64().unwrap();
    let (_, guild) = t.call("POST", "/api/v1/guilds", Some(&player), Some(json!({"instance_id":"smp","name":"Iron","tag":"IRON"}))).await;
    let gid = guild["id"].as_str().unwrap();
    let (_, other_guild) =
        t.call("POST", "/api/v1/guilds", Some(&player), Some(json!({"instance_id":"other","name":"Void","tag":"VOID"}))).await;
    let other_gid = other_guild["id"].as_str().unwrap();
    sqlx::query("INSERT INTO guild_members(guild_id,uuid,name,role,joined_at) VALUES(?,?,'Steve','member','x')")
        .bind(gid)
        .bind(&steve)
        .execute(&t.db)
        .await
        .unwrap();
    for (guild_id, server_id, balance) in [(gid, sid, 250.0), (other_gid, other_sid, 900.0)] {
        sqlx::query("INSERT INTO guild_wallets(guild_id,server_id,balance,updated_at) VALUES(?,?,?,'x')")
            .bind(guild_id)
            .bind(server_id)
            .bind(balance)
            .execute(&t.db)
            .await
            .unwrap();
    }
    let (s, _) = t
        .call(
            "POST",
            "/api/server/v1/guilds/manage",
            Some(token),
            Some(json!({"uuid":zed,"name":"Zed","action":"join","target":"IRON","text":"Hello"})),
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    let (s, bank) = t
        .call(
            "POST",
            "/api/server/v1/companion",
            Some(token),
            Some(json!({"uuid":alex,"operation":"guild_bank","args":{"server_id":other_sid,"guild_id":other_gid}})),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{bank}");
    assert_eq!(bank["balance"], 250.0);
    let (s, requests) =
        t.call("POST", "/api/server/v1/companion", Some(token), Some(json!({"uuid":alex,"operation":"guild_requests","args":{}}))).await;
    assert_eq!(s, StatusCode::OK, "{requests}");
    assert_eq!(requests["requests"][0]["name"], "Zed");
    let (s, _) = t
        .call(
            "POST",
            "/api/server/v1/companion",
            Some(token),
            Some(json!({"uuid":steve,"operation":"guild_requests","args":{"uuid":alex,"role":"leader"}})),
        )
        .await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    let (s, _) = t
        .call("POST", "/api/server/v1/companion", Some(token), Some(json!({"uuid":zed,"operation":"guild_bank","args":{"guild_id":gid}})))
        .await;
    assert_eq!(s, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn bridge_requires_server_auth_active_accounts_and_known_operations() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username":"Alex","password":"password123"}))).await;
    let alex = t.uuid("Alex").await;
    let player = t.login("Alex", "password123").await;
    let (_, srv) = t.call("POST", "/api/admin/servers", Some(&admin), Some(json!({"name":"SMP","instance_id":"smp"}))).await;
    let token = srv["token"].as_str().unwrap();
    let body = json!({"uuid":alex,"operation":"quests","args":{}});
    let (s, _) = t.call("POST", "/api/server/v1/companion", Some(&player), Some(body.clone())).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    let (s, q) = t.call("POST", "/api/server/v1/companion", Some(token), Some(body.clone())).await;
    assert_eq!(s, StatusCode::OK, "{q}");
    assert!(!q.as_array().unwrap().is_empty());
    let (s, _) = t
        .call(
            "POST",
            "/api/server/v1/companion",
            Some(token),
            Some(json!({"uuid":alex,"operation":"economy/adjust","args":{"delta":99999}})),
        )
        .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let (s, _) = t
        .call(
            "POST",
            "/api/server/v1/companion",
            Some(token),
            Some(json!({"uuid":alex,"operation":"quests","args":{"padding":"x".repeat(4096)}})),
        )
        .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    sqlx::query("UPDATE users SET status='banned' WHERE uuid=?").bind(&alex).execute(&t.db).await.unwrap();
    let (s, _) = t.call("POST", "/api/server/v1/companion", Some(token), Some(body)).await;
    assert_eq!(s, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn duplicate_financial_actions_return_the_same_receipt_without_charging_twice() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username":"Alex","password":"password123"}))).await;
    let alex = t.uuid("Alex").await;
    let (_, srv) = t.call("POST", "/api/admin/servers", Some(&admin), Some(json!({"name":"SMP","instance_id":"smp"}))).await;
    let token = srv["token"].as_str().unwrap();
    t.call(
        "POST",
        "/api/server/v1/economy/adjust",
        Some(token),
        Some(json!({"uuid":alex,"username":"Alex","delta":10000,"operation_id":"seed","description":"seed"})),
    )
    .await;
    let id = uuid::Uuid::new_v4().to_string();
    let body = json!({"uuid":alex,"request_id":id,"operation":"casino_slots","args":{"bet":10}});
    let (s, first) = t.call("POST", "/api/server/v1/companion", Some(token), Some(body.clone())).await;
    assert_eq!(s, StatusCode::OK, "{first}");
    let (s, replayed) = t.call("POST", "/api/server/v1/companion", Some(token), Some(body)).await;
    assert_eq!(s, StatusCode::OK, "{replayed}");
    assert_eq!(first, replayed);
    let rounds: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM casino_rounds WHERE uuid=?").bind(&alex).fetch_one(&t.db).await.unwrap();
    assert_eq!(rounds, 1);
    let (s, _) = t
        .call(
            "POST",
            "/api/server/v1/companion",
            Some(token),
            Some(json!({"uuid":alex,"request_id":id,"operation":"casino_slots","args":{"bet":20}})),
        )
        .await;
    assert_eq!(s, StatusCode::CONFLICT);
    let (s, _) = t
        .call("POST", "/api/server/v1/companion", Some(token), Some(json!({"uuid":alex,"operation":"casino_slots","args":{"bet":20}})))
        .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn notification_actions_cannot_mark_another_players_inbox_read() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    for name in ["Alex", "Steve"] {
        t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username":name,"password":"password123"}))).await;
    }
    let alex = t.uuid("Alex").await;
    let steve = t.uuid("Steve").await;
    sqlx::query("INSERT INTO user_notifications(uuid,kind,title,body,created_at) VALUES(?,'test','Private','Keep private','x')")
        .bind(&steve)
        .execute(&t.db)
        .await
        .unwrap();
    let id: i64 = sqlx::query_scalar("SELECT id FROM user_notifications WHERE uuid=?").bind(&steve).fetch_one(&t.db).await.unwrap();
    let (_, srv) = t.call("POST", "/api/admin/servers", Some(&admin), Some(json!({"name":"SMP","instance_id":"smp"}))).await;
    let token = srv["token"].as_str().unwrap();
    let (s, v) =
        t.call("POST", "/api/server/v1/companion", Some(token), Some(json!({"uuid":alex,"operation":"notifications","args":{}}))).await;
    assert_eq!(s, StatusCode::OK);
    assert!(v["items"].as_array().unwrap().is_empty());
    let (s, v) = t
        .call(
            "POST",
            "/api/server/v1/companion",
            Some(token),
            Some(json!({"uuid":alex,"request_id":uuid::Uuid::new_v4().to_string(),"operation":"notifications_read","args":{"ids":[id]}})),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{v}");
    let read: Option<String> =
        sqlx::query_scalar("SELECT read_at FROM user_notifications WHERE id=?").bind(id).fetch_one(&t.db).await.unwrap();
    assert!(read.is_none());
}

#[tokio::test]
async fn the_orders_and_contracts_boards_are_readable_through_the_bridge() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username":"Alex","password":"password123"}))).await;
    let alex = t.uuid("Alex").await;
    let (_, srv) = t.call("POST", "/api/admin/servers", Some(&admin), Some(json!({"name":"SMP","instance_id":"smp"}))).await;
    let token = srv["token"].as_str().unwrap();
    let (s, orders) = t.call("POST", "/api/server/v1/companion", Some(token), Some(json!({"uuid":alex,"operation":"orders","args":{}}))).await;
    assert_eq!(s, StatusCode::OK, "{orders}");
    assert!(orders["orders"].is_array() && orders["rules"].is_object(), "{orders}");
    let (s, contracts) = t.call("POST", "/api/server/v1/companion", Some(token), Some(json!({"uuid":alex,"operation":"contracts","args":{}}))).await;
    assert_eq!(s, StatusCode::OK, "{contracts}");
    assert!(contracts["contracts"].is_array() && contracts["stats"].is_object(), "{contracts}");
}
