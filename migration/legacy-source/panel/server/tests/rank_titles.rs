mod common;
use common::*;

#[tokio::test]
async fn renaming_a_rank_title_reaches_players() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    let (_, user) = t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username":"Steve","password":"password123"}))).await;
    assert!(user.get("id").is_some() || user.get("user").is_some(), "{user}");
    let steve = t.login("Steve", "password123").await;
    let uuid = t.uuid("Steve").await;
    let (_, server) = t.call("POST", "/api/admin/servers", Some(&admin), Some(json!({"name":"SMP","instance_id":"smp"}))).await;
    let sid = server["server"]["id"].as_i64().unwrap();
    sqlx::query("INSERT INTO server_levels (server_id, uuid, server_xp, server_level, updated_at) VALUES (?, ?, 100000, 10, 'now') ON CONFLICT(server_id, uuid) DO UPDATE SET server_xp = 100000")
        .bind(sid).bind(&uuid).execute(&t.db).await.unwrap();
    sqlx::query("INSERT INTO user_levels (uuid, global_xp, global_level, updated_at) VALUES (?, 100000, 10, 'now') ON CONFLICT(uuid) DO UPDATE SET global_xp = 100000").bind(&uuid).execute(&t.db).await.unwrap();

    let (s, made) = t
        .call(
            "POST",
            "/api/admin/rewards",
            Some(&admin),
            Some(json!({"level_type":"server","server_id":sid,"level_req":2,"reward_type":"title","reward_name":"Squire"})),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{made}");
    let id = made["id"].as_i64().unwrap();
    let rank = |v: &Value| v["server_levels"][0]["rank_name"].clone();
    let (_, levels) = t.call("GET", &format!("/api/v1/levels/player/{uuid}"), Some(&steve), None).await;
    assert_eq!(rank(&levels), "Squire", "{levels}");

    let (s, r) = t
        .call(
            "PUT",
            &format!("/api/admin/rewards/{id}"),
            Some(&admin),
            Some(json!({"level_type":"server","server_id":sid,"level_req":2,"reward_type":"title","reward_name":"Knight"})),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{r}");
    let (_, levels) = t.call("GET", &format!("/api/v1/levels/player/{uuid}"), Some(&steve), None).await;
    assert_eq!(rank(&levels), "Knight", "a renamed rank shows up straight away");

    assert_eq!(t.call("DELETE", &format!("/api/admin/rewards/{id}"), Some(&admin), None).await.0, StatusCode::OK);
    let (_, levels) = t.call("GET", &format!("/api/v1/levels/player/{uuid}"), Some(&steve), None).await;
    assert!(rank(&levels).is_null(), "{levels}");
}

#[tokio::test]
async fn renaming_a_global_title_reaches_the_profile_and_reward_list() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username":"Steve","password":"password123"}))).await;
    let steve = t.login("Steve", "password123").await;
    let uuid = t.uuid("Steve").await;
    sqlx::query("INSERT INTO user_levels (uuid, global_xp, global_level, updated_at) VALUES (?, 100000, 10, 'now') ON CONFLICT(uuid) DO UPDATE SET global_xp = 100000").bind(&uuid).execute(&t.db).await.unwrap();
    let (_, made) = t
        .call(
            "POST",
            "/api/admin/levels/rewards",
            Some(&admin),
            Some(json!({"level": 100, "reward_type": "title", "reward_value": "Explorer"})),
        )
        .await;
    let id = made["id"].as_i64().unwrap();
    let (_, me) = t.call("GET", "/api/v1/levels/me", Some(&steve), None).await;
    assert_eq!(me["title"], "Explorer", "{me}");
    t.call(
        "PUT",
        &format!("/api/admin/levels/rewards/{id}"),
        Some(&admin),
        Some(json!({"level": 100, "reward_type": "title", "reward_value": "Pathfinder"})),
    )
    .await;
    let (_, me) = t.call("GET", "/api/v1/levels/me", Some(&steve), None).await;
    assert_eq!(me["title"], "Pathfinder", "{me}");
    let rewards = me["rewards"].to_string();
    assert!(rewards.contains("Pathfinder") && !rewards.contains("Explorer"), "{rewards}");
    let (_, list) = t.call("GET", "/api/v1/levels/rewards", Some(&steve), None).await;
    assert!(list.to_string().contains("Pathfinder") && !list.to_string().contains("Explorer"), "{list}");
}

#[tokio::test]
async fn syncing_titles_repairs_stale_copies_and_reports_the_count() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    for n in ["Steve", "Alex"] {
        t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username": n, "password": "password123"}))).await;
    }
    let (steve_uuid, alex_uuid) = (t.uuid("Steve").await, t.uuid("Alex").await);
    for uuid in [&steve_uuid, &alex_uuid] {
        sqlx::query("INSERT INTO user_levels (uuid, global_xp, global_level, updated_at) VALUES (?, 100000, 10, 'now')")
            .bind(uuid)
            .execute(&t.db)
            .await
            .unwrap();
    }
    t.call(
        "POST",
        "/api/admin/levels/rewards",
        Some(&admin),
        Some(json!({"level": 5, "reward_type": "title", "reward_value": "Explorer"})),
    )
    .await;
    // Whatever title these players hold now is the correct one. Make one copy stale, as if a reward had been renamed by an older version.
    let correct: String =
        sqlx::query_scalar("SELECT title FROM user_levels WHERE uuid = ?").bind(&alex_uuid).fetch_one(&t.db).await.unwrap();
    sqlx::query("UPDATE user_levels SET title = 'Old Name' WHERE uuid = ?").bind(&steve_uuid).execute(&t.db).await.unwrap();

    let (s, r) = t.call("POST", "/api/admin/levels/sync-titles", Some(&admin), None).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    assert_eq!((r["checked"].as_u64(), r["changed"].as_u64()), (Some(2), Some(1)), "only the stale copy changes: {r}");
    let title: String =
        sqlx::query_scalar("SELECT title FROM user_levels WHERE uuid = ?").bind(&steve_uuid).fetch_one(&t.db).await.unwrap();
    assert_eq!(title, correct);
    let (_, again) = t.call("POST", "/api/admin/levels/sync-titles", Some(&admin), None).await;
    assert_eq!(again["changed"], 0, "running it again changes nothing");
    let steve = t.login("Steve", "password123").await;
    assert_eq!(t.call("POST", "/api/admin/levels/sync-titles", Some(&steve), None).await.0, StatusCode::FORBIDDEN, "admins only");
}
