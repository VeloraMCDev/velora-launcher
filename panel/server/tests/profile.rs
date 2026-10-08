//! Player profiles: presence, guild, rank, wealth, activity, mutual friends and cosmetics.

mod common;
use common::*;

#[tokio::test]
async fn profile_shows_the_whole_player() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    for n in ["Alex", "Steve", "Mia"] {
        t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username": n, "password": "password123"}))).await;
    }
    let (alex, steve) = (t.login("Alex", "password123").await, t.login("Steve", "password123").await);
    let (au, su, mu) = (t.uuid("Alex").await, t.uuid("Steve").await, t.uuid("Mia").await);
    let (_, srv) = t.call("POST", "/api/admin/servers", Some(&admin), Some(json!({"name": "SMP", "instance_id": "smp"}))).await;
    let sid = srv["server"]["id"].as_i64().unwrap();

    // Alex and Steve both know Mia, but not each other.
    for (a, b) in [(&au, &mu), (&su, &mu)] {
        sqlx::query("INSERT INTO friendships (user_uuid, friend_uuid, status, action_uuid, created_at, updated_at) VALUES (?, ?, 'accepted', ?, 'x', 'x')").bind(a).bind(b).bind(a).execute(&t.db).await.unwrap();
    }
    t.call("POST", "/api/v1/guilds", Some(&steve), Some(json!({"instance_id": "smp", "name": "Iron", "tag": "IRON"}))).await;
    sqlx::query("INSERT INTO server_economy (server_id, uuid, username, balance, updated_at) VALUES (?, ?, 'Steve', 4321.5, 'x')")
        .bind(sid)
        .bind(&su)
        .execute(&t.db)
        .await
        .unwrap();
    sqlx::query("INSERT INTO player_ranks (server_id, uuid, primary_group, display, prefix, groups, permissions, weight, updated_at) VALUES (?, ?, 'vip', 'VIP', '[VIP]', '[\"vip\"]', '[]', 10, 'x')").bind(sid).bind(&su).execute(&t.db).await.unwrap();
    sqlx::query("INSERT INTO player_stats (server_id, uuid, name, playtime_secs, first_seen, last_seen) VALUES (?, ?, 'Steve', 7200, 'x', '2026-01-01T00:00:00Z')").bind(sid).bind(&su).execute(&t.db).await.unwrap();
    sqlx::query("INSERT INTO server_events (server_id, uuid, name, kind, detail, created_at) VALUES (?, ?, 'Steve', 'join', NULL, '2026-01-01T00:00:00Z'), (?, ?, 'Steve', 'chat', 'secret', '2026-01-01T00:00:01Z')").bind(sid).bind(&su).bind(sid).bind(&su).execute(&t.db).await.unwrap();

    let (s, p) = t.call("GET", &format!("/api/v1/profiles/{su}"), Some(&alex), None).await;
    assert_eq!(s, StatusCode::OK, "{p}");
    assert_eq!(p["relationship"], "none");
    assert_eq!(p["mutual_friends"][0]["username"], "Mia");
    assert_eq!((p["guild"]["tag"].as_str(), p["guild"]["role"].as_str()), (Some("IRON"), Some("leader")));
    assert_eq!((p["rank"]["display"].as_str(), p["rank"]["server_name"].as_str()), (Some("VIP"), Some("SMP")));
    assert_eq!(p["favorite_server"], "SMP");
    assert_eq!(p["wealth"], 4321.5);
    let activity = p["recent_activity"].as_array().unwrap();
    assert!(activity.iter().any(|a| a["text"] == "Joined SMP"));
    assert!(activity.iter().all(|a| !a["text"].as_str().unwrap().contains("secret")), "chat must not leak");

    // Viewing yourself, and cosmetics.
    let (_, me) = t.call("GET", &format!("/api/v1/profiles/{au}"), Some(&alex), None).await;
    assert_eq!(me["relationship"], "self");
    let (s, _) = t.call("PUT", "/api/v1/profiles/me", Some(&alex), Some(json!({"bio": "hi", "accent_color": "#22D3EE"}))).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(t.call("GET", &format!("/api/v1/profiles/{au}"), Some(&alex), None).await.1["accent_color"], "#22d3ee");
    let (s, _) =
        t.call("PUT", "/api/v1/profiles/me", Some(&alex), Some(json!({"bio": "hi", "accent_color": "red; background:url(x)"}))).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
}
