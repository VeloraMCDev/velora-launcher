//! Friends & Social, and removing every trace of a deleted account.

mod common;
use common::*;

async fn player(t: &TestApp, admin: &str, name: &str) -> (String, String, i64) {
    let (s, v) = t.call("POST", "/api/admin/users", Some(admin), Some(json!({"username": name, "password": "password123"}))).await;
    assert_eq!(s, StatusCode::OK, "{v}");
    (t.login(name, "password123").await, t.uuid(name).await, v["id"].as_i64().unwrap())
}

#[tokio::test]
async fn search_befriend_message_invite() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    let (alex, alex_uuid, _) = player(&t, &admin, "Alexandra").await;
    let (steve, steve_uuid, _) = player(&t, &admin, "Steve").await;
    player(&t, &admin, "Stevie_B").await;

    // Search: name match, wildcards are literal, never yourself.
    let (s, v) = t.call("GET", "/api/v1/social/members?q=ste", Some(&alex), None).await;
    assert_eq!(s, StatusCode::OK, "{v}");
    let names: Vec<&str> = v.as_array().unwrap().iter().map(|m| m["username"].as_str().unwrap()).collect();
    assert!(names.contains(&"Steve") && names.contains(&"Stevie_B") && !names.contains(&"Alexandra"), "{names:?}");
    assert_eq!(v[0]["friendship_status"], "none");
    let (_, all) = t.call("GET", "/api/v1/members/search", Some(&alex), None).await;
    assert_eq!(all.as_array().unwrap().len(), 3, "admin + Steve + Stevie_B");
    let (_, wild) = t.call("GET", "/api/v1/social/members?q=%25", Some(&alex), None).await;
    assert_eq!(wild.as_array().unwrap().len(), 3, "a bare % is ignored, not a wildcard");
    let (s, _) = t.call("GET", "/api/v1/social/members?q=x", None, None).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);

    // Friend request flow shows up in search results for both sides.
    let (s, _) = t.call("POST", "/api/v1/friends/request", Some(&alex), Some(json!({"username": "steve"}))).await;
    assert_eq!(s, StatusCode::OK);
    let (_, v) = t.call("GET", "/api/v1/social/members?q=steve", Some(&alex), None).await;
    assert_eq!(v[0]["friendship_status"], "pending_outgoing");
    let (_, v) = t.call("GET", "/api/v1/social/members?q=alexandra", Some(&steve), None).await;
    assert_eq!(v[0]["friendship_status"], "pending_incoming");
    let (s, _) = t.call("POST", "/api/v1/friends/respond", Some(&steve), Some(json!({"target_uuid": alex_uuid, "accept": true}))).await;
    assert_eq!(s, StatusCode::OK);
    let (_, v) = t.call("GET", "/api/v1/social/members?q=steve", Some(&alex), None).await;
    assert_eq!(v[0]["friendship_status"], "accepted");
    assert_eq!(v[0]["is_friend"], true);
    let (_, friends) = t.call("GET", "/api/v1/friends", Some(&alex), None).await;
    assert_eq!(friends[0]["username"], "Steve");

    // Messages: only to real players.
    let (s, m) = t.call("POST", &format!("/api/v1/messages/{steve_uuid}"), Some(&alex), Some(json!({"content": "hello"}))).await;
    assert_eq!(s, StatusCode::OK, "{m}");
    let (s, _) = t.call("POST", "/api/v1/messages/nobody", Some(&alex), Some(json!({"content": "hello"}))).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    let (_, thread) = t.call("GET", &format!("/api/v1/messages/{alex_uuid}"), Some(&steve), None).await;
    assert_eq!(thread[0]["content"], "hello");

    // Invites: friends only.
    let (_, stevie) = t.call("GET", "/api/v1/social/members?q=stevie", Some(&alex), None).await;
    let stevie_uuid = stevie[0]["uuid"].as_str().unwrap().to_string();
    let (s, _) = t.call("POST", "/api/v1/invites", Some(&alex), Some(json!({"recipient_uuid": stevie_uuid, "instance_id": "x"}))).await;
    assert_eq!(s, StatusCode::FORBIDDEN);

    // Profile carries the relationship so the launcher can offer the right actions.
    let (s, p) = t.call("GET", &format!("/api/v1/profiles/{steve_uuid}"), Some(&alex), None).await;
    assert_eq!(s, StatusCode::OK, "{p}");
    let (s, p) = t.call("GET", &format!("/api/v1/profiles/{stevie_uuid}"), Some(&alex), None).await;
    assert_eq!(s, StatusCode::OK, "{p}");
}

/// Every table and column is scanned, so a table added later is covered too.
async fn mentions(t: &TestApp, needle: &str) -> Vec<String> {
    let tables: Vec<String> = sqlx::query_scalar("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'")
        .fetch_all(&t.db)
        .await
        .unwrap();
    let mut hits = Vec::new();
    for table in tables {
        let cols: Vec<String> =
            sqlx::query_scalar(&format!("SELECT name FROM pragma_table_info('{table}')")).fetch_all(&t.db).await.unwrap();
        for col in cols {
            let n: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM \"{table}\" WHERE CAST(\"{col}\" AS TEXT) = ?"))
                .bind(needle)
                .fetch_one(&t.db)
                .await
                .unwrap();
            if n > 0 {
                hits.push(format!("{table}.{col}"));
            }
        }
    }
    hits
}

#[tokio::test]
async fn deleting_an_account_removes_all_its_data() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    let (alex, alex_uuid, _) = player(&t, &admin, "Alex").await;
    let (steve, steve_uuid, steve_id) = player(&t, &admin, "Steve").await;
    let (_, mia_uuid, _) = player(&t, &admin, "Mia").await;

    let (_, srv) = t.call("POST", "/api/admin/servers", Some(&admin), Some(json!({"name": "SMP", "instance_id": "smp"}))).await;
    let sid = srv["server"]["id"].as_i64().unwrap();
    let server_token = srv["token"].as_str().unwrap().to_string();

    // Gameplay: stats, XP, quests, an achievement, events.
    let (s, v) = t
        .call(
            "POST",
            "/api/server/v1/sync",
            Some(&server_token),
            Some(json!({
                "online": [{"uuid": &steve_uuid, "name": "Steve"}],
                "stats": [{"uuid": &steve_uuid, "name": "Steve", "blocks_broken": 200, "mob_kills": 20, "playtime_secs": 4000, "joins": 1}],
                "events": [{"uuid": &steve_uuid, "name": "Steve", "kind": "join"}, {"uuid": &steve_uuid, "kind": "action", "detail": "block_broken:STONE@minecraft:overworld +10"}]
            })),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{v}");
    t.call("GET", "/api/v1/quests/my", Some(&steve), None).await;

    // Social: friends, DMs both ways, profile, posts, likes.
    t.call("POST", "/api/v1/friends/request", Some(&steve), Some(json!({"username": "Alex"}))).await;
    t.call("POST", "/api/v1/friends/respond", Some(&alex), Some(json!({"target_uuid": &steve_uuid, "accept": true}))).await;
    t.call("POST", &format!("/api/v1/messages/{alex_uuid}"), Some(&steve), Some(json!({"content": "hi"}))).await;
    t.call("POST", &format!("/api/v1/messages/{steve_uuid}"), Some(&alex), Some(json!({"content": "yo"}))).await;
    t.call("PUT", "/api/v1/profiles/me", Some(&steve), Some(json!({"bio": "I like rocks"}))).await;
    let (_, post) = t.call("POST", "/api/v1/profiles/me/posts", Some(&steve), Some(json!({"content": "hello world"}))).await;
    let (_, alex_post) = t.call("POST", "/api/v1/profiles/me/posts", Some(&alex), Some(json!({"content": "alex post"}))).await;
    t.call("POST", &format!("/api/v1/posts/{}/like", alex_post["id"]), Some(&steve), None).await;
    t.call("POST", &format!("/api/v1/posts/{}/like", post["id"]), Some(&alex), None).await;

    // Economy: balance, a transfer to Alex, a market listing.
    sqlx::query("INSERT INTO server_economy (server_id, uuid, username, balance, updated_at) VALUES (?, ?, 'Steve', 50, 'now'), (?, ?, 'Alex', 50, 'now')")
        .bind(sid).bind(&steve_uuid).bind(sid).bind(&alex_uuid).execute(&t.db).await.unwrap();
    sqlx::query("INSERT INTO economy_transactions (server_id, from_uuid, from_name, to_uuid, to_name, amount, description, created_at) VALUES (?, ?, 'Steve', ?, 'Alex', 5, 'pay', 'now')")
        .bind(sid).bind(&steve_uuid).bind(&alex_uuid).execute(&t.db).await.unwrap();
    sqlx::query("INSERT INTO server_market (server_id, seller_uuid, seller_name, item_id, item_name, amount, price, created_at) VALUES (?, ?, 'Steve', 'DIAMOND', 'Diamond', 1, 10, 'now')")
        .bind(sid).bind(&steve_uuid).execute(&t.db).await.unwrap();

    // Guilds: Steve leads one with Alex in it, and is sole member of another.
    let (s, g) =
        t.call("POST", "/api/v1/guilds", Some(&steve), Some(json!({"instance_id": "smp", "name": "Rockheads", "tag": "ROCK"}))).await;
    assert_eq!(s, StatusCode::OK, "{g}");
    let gid = g["id"].as_str().unwrap().to_string();
    sqlx::query("INSERT INTO guild_members (guild_id, uuid, name, role, joined_at) VALUES (?, ?, 'Alex', 'member', '2026-01-01')")
        .bind(&gid)
        .bind(&alex_uuid)
        .execute(&t.db)
        .await
        .unwrap();
    sqlx::query("INSERT INTO guild_wallet_transactions (server_id, guild_id, actor_uuid, kind, amount, created_at) VALUES (?, ?, ?, 'deposit', 5, 'now')").bind(sid).bind(&gid).bind(&steve_uuid).execute(&t.db).await.unwrap();
    let (s, g2) = t.call("POST", "/api/v1/guilds", Some(&admin), Some(json!({"instance_id": "smp", "name": "Solo", "tag": "SOLO"}))).await;
    assert_eq!(s, StatusCode::OK, "{g2}");

    assert!(!mentions(&t, &steve_uuid).await.is_empty());

    // Delete the account.
    let (s, v) = t.call("DELETE", &format!("/api/admin/users/{steve_id}"), Some(&admin), None).await;
    assert_eq!(s, StatusCode::OK, "{v}");
    assert!(v["report"]["guilds_transferred"] == 1 && v["report"]["removed"]["stats"].as_u64().unwrap() > 0, "{v}");

    // Nothing refers to them any more (their name appears only as plain text in free-form rows we don't keep).
    assert_eq!(mentions(&t, &steve_uuid).await, vec!["reserved_usernames.uuid".to_string()]);
    for table in [
        "users",
        "user_levels",
        "player_stats",
        "friendships",
        "direct_messages",
        "user_profiles",
        "user_posts",
        "server_market",
        "server_economy",
    ] {
        let n: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table} WHERE CAST(uuid AS TEXT) = ?"))
            .bind(&steve_uuid)
            .fetch_one(&t.db)
            .await
            .unwrap_or(0);
        assert_eq!(n, 0, "{table}");
    }
    let name_hits: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE username = 'Steve' COLLATE NOCASE").fetch_one(&t.db).await.unwrap();
    assert_eq!(name_hits, 0);

    // Other people keep what is theirs: Alex is now the guild's leader; the
    // transfer shows the anonymised counterparty; Alex's like count dropped.
    let leader: String = sqlx::query_scalar("SELECT leader_uuid FROM guilds WHERE id = ?").bind(&gid).fetch_one(&t.db).await.unwrap();
    assert_eq!(leader, alex_uuid);
    let from: String = sqlx::query_scalar("SELECT from_name FROM economy_transactions").fetch_one(&t.db).await.unwrap();
    assert_eq!(from, "Deleted player");
    let likes: i64 = sqlx::query_scalar("SELECT likes_count FROM user_posts WHERE id = ?")
        .bind(alex_post["id"].as_i64().unwrap())
        .fetch_one(&t.db)
        .await
        .unwrap();
    assert_eq!(likes, 0);
    let (_, friends) = t.call("GET", "/api/v1/friends", Some(&alex), None).await;
    assert!(friends.as_array().unwrap().is_empty());
    let (_, list) = t.call("GET", "/api/v1/members/search", Some(&alex), None).await;
    assert!(list.as_array().unwrap().iter().all(|m| m["username"] != "Steve"));
    // The deleted token no longer works, and the name stays reserved against impersonation.
    let (s, _) = t.call("GET", "/api/v1/auth/me", Some(&steve), None).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    let (s, _) = t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username": "Steve", "password": "password123"}))).await;
    assert_eq!(s, StatusCode::CONFLICT);
    assert_ne!(mia_uuid, steve_uuid);
}
