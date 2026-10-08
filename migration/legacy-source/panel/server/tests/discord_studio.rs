mod common;
use common::*;

#[tokio::test]
async fn discord_messages_are_customised_and_previewed_with_real_data() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    let (s, all) = t.call("GET", "/api/admin/discord/studio", Some(&admin), None).await;
    assert_eq!(s, StatusCode::OK, "{all}");
    assert_eq!(all["templates"].as_object().unwrap().len(), 3);
    assert_eq!(all["live"].as_object().unwrap().len(), 4);
    assert!(all["placeholders"]["achievement"].as_array().unwrap().iter().any(|p| p == "achievement_description"));

    // Edit the achievement message; bad image addresses are refused.
    let mut tpl = all["templates"].clone();
    tpl["achievement"]["title"] = json!("🎉 {player} did it!");
    tpl["achievement"]["image"] = json!("javascript:evil");
    let (s, _) = t.call("PUT", "/api/admin/discord/studio/templates", Some(&admin), Some(json!({"templates": tpl}))).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    tpl["achievement"]["image"] = json!("https://cdn.example.com/banner.png");
    let (s, _) = t.call("PUT", "/api/admin/discord/studio/templates", Some(&admin), Some(json!({"templates": tpl.clone()}))).await;
    assert_eq!(s, StatusCode::OK);
    let (s, p) = t
        .call("POST", "/api/admin/discord/studio/preview", Some(&admin), Some(json!({"kind": "achievement", "style": tpl["achievement"]})))
        .await;
    assert_eq!(s, StatusCode::OK, "{p}");
    let e = &p["embeds"][0];
    assert_eq!(e["title"], "🎉 admin did it!");
    assert_eq!(e["image"]["url"], "https://cdn.example.com/banner.png");
    assert!(e["description"].as_str().unwrap().contains("Deep Diver"));
    assert_eq!(e["fields"][0]["name"], "Reward");

    // Live embeds show real data: a server, a guild and a rich player.
    t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username": "Steve", "password": "password123"}))).await;
    let steve_token = t.login("Steve", "password123").await;
    let (_, srv) = t.call("POST", "/api/admin/servers", Some(&admin), Some(json!({"name": "Survival SMP", "instance_id": "smp"}))).await;
    let sid = srv["server"]["id"].as_i64().unwrap();
    t.call("POST", "/api/v1/guilds", Some(&steve_token), Some(json!({"instance_id": "smp", "name": "Iron Wolves", "tag": "IRON"}))).await;
    sqlx::query("INSERT INTO server_economy (server_id, uuid, username, balance, updated_at) VALUES (?, ?, 'Steve', 123456, 'now')")
        .bind(sid)
        .bind(t.uuid("Steve").await)
        .execute(&t.db)
        .await
        .unwrap();
    for (kind, needle) in [("status", "Survival SMP"), ("guilds", "Iron Wolves"), ("baltop", "$123,456"), ("leaderboard", "Leaderboard")] {
        let cfg = all["live"][kind].clone();
        let (s, p) = t.call("POST", "/api/admin/discord/studio/preview", Some(&admin), Some(json!({"kind": kind, "live": cfg}))).await;
        assert_eq!(s, StatusCode::OK, "{kind}: {p}");
        assert!(p.to_string().contains(needle), "{kind} should mention {needle}: {p}");
    }

    // Interval and row limits are clamped; the webhook is write-only.
    let mut live = all["live"].clone();
    live["status"]["interval_secs"] = json!(5);
    live["status"]["limit"] = json!(500);
    live["status"]["webhook_url"] = json!("https://evil.example.com/hook");
    assert_eq!(
        t.call("PUT", "/api/admin/discord/studio/live", Some(&admin), Some(json!({"live": live.clone()}))).await.0,
        StatusCode::BAD_REQUEST
    );
    live["status"]["webhook_url"] = json!("https://discord.com/api/webhooks/1/abc");
    assert_eq!(t.call("PUT", "/api/admin/discord/studio/live", Some(&admin), Some(json!({"live": live}))).await.0, StatusCode::OK);
    let (_, again) = t.call("GET", "/api/admin/discord/studio", Some(&admin), None).await;
    assert_eq!(again["live"]["status"]["interval_secs"], 60);
    assert_eq!(again["live"]["status"]["limit"], 25);
    assert_eq!(again["live"]["status"]["webhook_set"], true);
    assert_eq!(again["live"]["status"]["webhook_url"], "");
}
