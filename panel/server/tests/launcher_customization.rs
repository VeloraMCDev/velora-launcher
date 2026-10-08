mod common;
use common::*;

#[tokio::test]
async fn png_rank_titles_follow_the_current_reward_and_keep_text() {
    let t = setup().await;
    // Keep this fixture independent of the starter title milestones seeded by setup.
    sqlx::query("DELETE FROM level_rewards WHERE reward_type='title'").execute(&t.db).await.unwrap();
    let admin = t.login("admin", "supersecret").await;
    t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username":"Steve","password":"password123"}))).await;
    let uuid = t.uuid("Steve").await;
    sqlx::query("INSERT INTO user_levels(uuid, global_xp, global_level, updated_at) VALUES (?, 100000, 10, 'now') ON CONFLICT(uuid) DO UPDATE SET global_xp=100000")
        .bind(&uuid).execute(&t.db).await.unwrap();
    let payload = json!({"level":2,"reward_type":"title","reward_value":"Explorer","reward_data":{"title_image":"/uploads/explorer.png"}});
    let (s, made) = t.call("POST", "/api/admin/levels/rewards", Some(&admin), Some(payload.clone())).await;
    assert_eq!(s, StatusCode::OK, "{made}");
    let path = format!("/api/v1/levels/player/{uuid}");
    let (_, levels) = t.call("GET", &path, None, None).await;
    assert_eq!(levels["title"], "Explorer");
    assert_eq!(levels["title_image"], "/uploads/explorer.png");
    let (_, server) = t.call("POST", "/api/admin/servers", Some(&admin), Some(json!({"name":"SMP","instance_id":"smp"}))).await;
    let sid = server["server"]["id"].as_i64().unwrap();
    sqlx::query("INSERT INTO server_levels(server_id,uuid,server_xp,server_level,updated_at) VALUES (?, ?, 100000, 10, 'now')")
        .bind(sid)
        .bind(&uuid)
        .execute(&t.db)
        .await
        .unwrap();
    let (s, response) = t.call("POST", "/api/admin/rewards", Some(&admin), Some(json!({"level_type":"server","server_id":sid,"level_req":2,"reward_type":"title","reward_name":"Squire","reward_data":{"title_image":"/uploads/squire.png"}}))).await;
    assert_eq!(s, StatusCode::OK, "{response}");
    let (_, levels) = t.call("GET", &path, None, None).await;
    assert_eq!(levels["server_levels"][0]["rank_name"], "Squire");
    assert_eq!(levels["server_levels"][0]["title_image"], "/uploads/squire.png");
    let id = made["id"].as_i64().unwrap();
    let mut changed = payload;
    changed["reward_data"]["title_image"] = json!("https://example.com/knight.png?v=2");
    let (s, _) = t.call("PUT", &format!("/api/admin/levels/rewards/{id}"), Some(&admin), Some(changed)).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(t.call("GET", &path, None, None).await.1["title_image"], "https://example.com/knight.png?v=2");
    t.call("POST", "/api/admin/levels/rewards", Some(&admin), Some(json!({"level":3,"reward_type":"title","reward_value":"Knight"}))).await;
    let (_, levels) = t.call("GET", &path, None, None).await;
    assert_eq!(levels["title"], "Knight");
    assert!(levels["title_image"].is_null());
    for image in [json!("javascript:alert(1).png"), json!("https://example.com/image.svg"), json!(42)] {
        let (s, response) = t
            .call(
                "POST",
                "/api/admin/levels/rewards",
                Some(&admin),
                Some(json!({"level":2,"reward_type":"title","reward_value":"Invalid","reward_data":{"title_image":image}})),
            )
            .await;
        assert_eq!(s, StatusCode::BAD_REQUEST, "{response}");
    }
}

#[tokio::test]
async fn custom_achievement_and_quest_icons_round_trip() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    let (s, response) = t
        .call(
            "POST",
            "/api/admin/achievements",
            Some(&admin),
            Some(json!({
                "id":"png_achievement","title":"Builder","description":"Build something","category":"building",
                "icon_frame":"goal","icon_item":"/uploads/builder.png","icon_bg":"#123456","icon_border":"#abcdef",
                "requirement_type":"stat","requirement_key":"blocks_placed","requirement_value":10,"xp_reward":25,"secret":false
            })),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{response}");
    let (_, list) = t.call("GET", "/api/admin/achievements", Some(&admin), None).await;
    let ach = list.as_array().unwrap().iter().find(|v| v["id"] == "png_achievement").unwrap();
    assert_eq!(ach["icon_item"], "/uploads/builder.png");
    assert_eq!(ach["icon_bg"], "#123456");
    let (s, response) = t
        .call(
            "POST",
            "/api/admin/quests",
            Some(&admin),
            Some(json!({
                "id":"png_quest","title":"Miner","description":"Mine blocks","category":"mining","quest_type":"daily",
                "stat_type":"blocks_broken","target_count":10,"xp_reward":25,"icon":"/uploads/miner.png","active":true
            })),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{response}");
    let (_, list) = t.call("GET", "/api/admin/quests", Some(&admin), None).await;
    let quest = list.as_array().unwrap().iter().find(|v| v["id"] == "png_quest").unwrap();
    assert_eq!(quest["icon"], "/uploads/miner.png");
}

#[tokio::test]
async fn message_preview_preserves_unread_status() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    for name in ["Alex", "Steve"] {
        t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username":name,"password":"password123"}))).await;
    }
    let alex = t.login("Alex", "password123").await;
    let steve = t.login("Steve", "password123").await;
    let alex_uuid = t.uuid("Alex").await;
    let steve_uuid = t.uuid("Steve").await;
    let (s, message) = t.call("POST", &format!("/api/v1/messages/{steve_uuid}"), Some(&alex), Some(json!({"content":"Hello!"}))).await;
    assert_eq!(s, StatusCode::OK, "{message}");
    let path = format!("/api/v1/messages/{alex_uuid}");
    let (s, preview) = t.call("GET", &format!("{path}?mark_read=false"), Some(&steve), None).await;
    assert_eq!(s, StatusCode::OK, "{preview}");
    let unread: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM direct_messages WHERE recipient_uuid=? AND is_read=0")
        .bind(&steve_uuid)
        .fetch_one(&t.db)
        .await
        .unwrap();
    assert_eq!(unread, 1);
    t.call("GET", &path, Some(&steve), None).await;
    let unread: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM direct_messages WHERE recipient_uuid=? AND is_read=0")
        .bind(&steve_uuid)
        .fetch_one(&t.db)
        .await
        .unwrap();
    assert_eq!(unread, 0);
}
