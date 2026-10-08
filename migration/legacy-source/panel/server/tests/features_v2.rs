//! Integration tests for v2 features:
//! - Global & Server Leveling
//! - Daily & Weekly Quests
//! - Guilds & Grief Protection Land Claiming
//! - Friends, DMs, Posts & Player Profiles
//! - Server Feature Toggles (config bypass)

mod common;
use common::*;

async fn setup_users(t: &TestApp) -> (String, String, String, String) {
    let admin = t.login("admin", "supersecret").await;
    let (s, v1) = t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username": "Alex", "password": "password123"}))).await;
    assert_eq!(s, StatusCode::OK, "{v1}");
    let (s, v2) = t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username": "Steve", "password": "password123"}))).await;
    assert_eq!(s, StatusCode::OK, "{v2}");

    let alex_token = t.login("Alex", "password123").await;
    let steve_token = t.login("Steve", "password123").await;
    let alex_uuid = t.uuid("Alex").await;
    let steve_uuid = t.uuid("Steve").await;

    (alex_token, steve_token, alex_uuid, steve_uuid)
}

async fn create_test_server(t: &TestApp, admin: &str) -> (i64, String) {
    let (s, v) = t.call("POST", "/api/admin/servers", Some(admin), Some(json!({"name": "Survival Hub"}))).await;
    assert_eq!(s, StatusCode::OK, "{v}");
    let server_id = v["server"]["id"].as_i64().unwrap();
    let token = v["token"].as_str().unwrap().to_string();
    (server_id, token)
}

#[tokio::test]
async fn leveling_and_quests_progression() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    let (alex_token, _, alex_uuid, _) = setup_users(&t).await;
    let (_server_id, server_token) = create_test_server(&t, &admin).await;
    // Hand every quest to every player so progress can be checked per quest.
    let (s, v) =
        t.call("PUT", "/api/admin/progression", Some(&admin), Some(json!({"daily_quest_limit": 0, "weekly_quest_limit": 0}))).await;
    assert_eq!(s, StatusCode::OK, "{v}");

    // 1. Initial level check
    let (s, lvl) = t.call("GET", "/api/v1/leveling/me", Some(&alex_token), None).await;
    assert_eq!(s, StatusCode::OK, "{lvl}");
    assert_eq!(lvl["global_level"], 1);
    assert_eq!(lvl["global_xp"], 0);

    // 2. Fetch seeded quests
    let (s, quests) = t.call("GET", "/api/v1/quests/me", Some(&alex_token), None).await;
    assert_eq!(s, StatusCode::OK, "{quests}");
    let quest_list = quests.as_array().unwrap();
    assert!(!quest_list.is_empty(), "Expected seeded quests to be populated");

    // 3. Game Server sync with leveling ENABLED
    let (s, sync_res) = t
        .call(
            "POST",
            "/api/server/v1/sync",
            Some(&server_token),
            Some(json!({
                "features": {
                    "leveling_enabled": true,
                    "global_xp_multiplier": 1.0,
                    "server_xp_multiplier": 1.5,
                    "quests_enabled": true,
                    "achievements_enabled": true,
                    "guilds_enabled": true,
                    "land_claiming_enabled": true
                },
                "players": [
                    {"uuid": &alex_uuid, "username": "Alex", "ip": "127.0.0.1", "playtime_secs": 60}
                ],
                "stats": [
                    {
                        "uuid": &alex_uuid,
                        "name": "Alex",
                        "blocks_broken": 20,
                        "blocks_placed": 10,
                        "mob_kills": 5,
                        "player_kills": 0,
                        "deaths": 0,
                        "playtime_secs": 60
                    }
                ],
                "events": [{"uuid": &alex_uuid, "kind": "action", "detail": "block_broken:DIAMOND_ORE@minecraft:overworld +4"}]
            })),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{sync_res}");

    // 4. Verify Alex gained XP from the server actions
    let (s, lvl_after) = t.call("GET", "/api/v1/leveling/me", Some(&alex_token), None).await;
    assert_eq!(s, StatusCode::OK, "{lvl_after}");
    let current_xp = lvl_after["global_xp"].as_i64().unwrap();
    assert!(current_xp > 0, "Alex should have earned XP");
    let (s, quests_after) = t.call("GET", "/api/v1/quests/me", Some(&alex_token), None).await;
    assert_eq!(s, StatusCode::OK);
    let progress =
        |id: &str| quests_after.as_array().unwrap().iter().find(|q| q["quest"]["id"] == id).unwrap()["progress"].as_i64().unwrap();
    assert_eq!(progress("q_d_mine_diamond"), 4);
    assert_eq!(progress("q_d_mine_coal"), 0);

    // 5. Game Server sync with leveling DISABLED (server host turned it off in config)
    let (s, sync_off) = t
        .call(
            "POST",
            "/api/server/v1/sync",
            Some(&server_token),
            Some(json!({
                "features": {
                    "leveling_enabled": false,
                    "quests_enabled": false
                },
                "players": [
                    {"uuid": &alex_uuid, "username": "Alex", "ip": "127.0.0.1", "playtime_secs": 120}
                ],
                "stats": [
                    {
                        "uuid": &alex_uuid,
                        "name": "Alex",
                        "blocks_broken": 50,
                        "blocks_placed": 50,
                        "mob_kills": 20,
                        "player_kills": 0,
                        "deaths": 0,
                        "playtime_secs": 60
                    }
                ],
                "events": []
            })),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{sync_off}");

    // Verify XP did NOT increase when leveling is disabled
    let (s, lvl_no_change) = t.call("GET", "/api/v1/leveling/me", Some(&alex_token), None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(lvl_no_change["global_xp"].as_i64().unwrap(), current_xp);
}

#[tokio::test]
async fn guilds_and_land_claims() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    let (alex_token, _, alex_uuid, steve_uuid) = setup_users(&t).await;
    let (server_id, server_token) = create_test_server(&t, &admin).await;
    let (status, updated) = t
        .call(
            "PUT",
            &format!("/api/admin/servers/{server_id}"),
            Some(&admin),
            Some(json!({"name": "Survival Hub", "instance_id": "smp-pack"})),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{updated}");

    // 1. Alex founds a guild
    let (s, guild) = t
        .call(
            "POST",
            "/api/v1/guilds",
            Some(&alex_token),
            Some(json!({
                "instance_id": "smp-pack",
                "name": "Iron Fortress",
                "tag": "IRON",
                "description": "Defenders of the realm"
            })),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{guild}");
    let guild_id = guild["id"].as_str().unwrap().to_string();
    assert_eq!(guild["tag"], "IRON");

    // 2. Alex claims chunk (3, 7) in the Overworld
    let (s, claim) = t
        .call(
            "POST",
            &format!("/api/v1/guilds/{guild_id}/claim"),
            Some(&alex_token),
            Some(json!({
                "server_id": server_id,
                "instance_id": "smp-pack",
                "dimension": "minecraft:overworld",
                "chunk_x": 3,
                "chunk_z": 7
            })),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{claim}");
    let claim_id = claim["id"].as_i64().unwrap();

    // 3. Server checks chunk permissions for in-game protection
    // Alex (guild leader) attempts to build -> allowed
    let (s, check_alex) = t
        .call(
            "POST",
            "/api/server/v1/guilds/check-chunk",
            Some(&server_token),
            Some(json!({
                "dimension": "minecraft:overworld",
                "chunk_x": 3,
                "chunk_z": 7,
                "uuid": &alex_uuid
            })),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{check_alex}");
    assert_eq!(check_alex["allowed"], true);

    // Steve (non-member) attempts to build -> DENIED with grief protection reason
    let (s, check_steve) = t
        .call(
            "POST",
            "/api/server/v1/guilds/check-chunk",
            Some(&server_token),
            Some(json!({
                "dimension": "minecraft:overworld",
                "chunk_x": 3,
                "chunk_z": 7,
                "uuid": &steve_uuid
            })),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{check_steve}");
    assert_eq!(check_steve["allowed"], false);
    assert_eq!(check_steve["claimed"], true);

    // Unclaimed wilderness chunk (100, 100) -> allowed for all players
    let (s, check_wild) = t
        .call(
            "POST",
            "/api/server/v1/guilds/check-chunk",
            Some(&server_token),
            Some(json!({
                "dimension": "minecraft:overworld",
                "chunk_x": 100,
                "chunk_z": 100,
                "uuid": &steve_uuid
            })),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{check_wild}");
    assert_eq!(check_wild["allowed"], true);

    // 4. Alex unclaims the chunk
    let (s, _) = t.call("DELETE", &format!("/api/v1/guilds/claims/{claim_id}"), Some(&alex_token), None).await;
    assert_eq!(s, StatusCode::OK);

    // After unclaiming, Steve is now allowed to build at (3, 7)
    let (s, check_unclaimed) = t
        .call(
            "POST",
            "/api/server/v1/guilds/check-chunk",
            Some(&server_token),
            Some(json!({
                "dimension": "minecraft:overworld",
                "chunk_x": 3,
                "chunk_z": 7,
                "uuid": &steve_uuid
            })),
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(check_unclaimed["allowed"], true);

    // The in-game command uses the server unclaim route rather than DELETE.
    let (s, _) = t
        .call(
            "POST",
            "/api/server/v1/guilds/claim",
            Some(&server_token),
            Some(json!({"uuid": &alex_uuid, "dimension": "minecraft:overworld", "chunk_x": 3, "chunk_z": 7})),
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    let (s, response) = t
        .call(
            "POST",
            "/api/server/v1/guilds/unclaim",
            Some(&server_token),
            Some(json!({"uuid": &alex_uuid, "dimension": "minecraft:overworld", "chunk_x": 3, "chunk_z": 7})),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{response}");
}

#[tokio::test]
async fn social_friends_dms_and_profiles() {
    let t = setup().await;
    let (alex_token, steve_token, alex_uuid, steve_uuid) = setup_users(&t).await;

    // 1. Alex sends friend request to Steve
    let (s, _) = t.call("POST", "/api/v1/social/friends/request", Some(&alex_token), Some(json!({"friend_username": "Steve"}))).await;
    assert_eq!(s, StatusCode::OK);

    // 2. Steve accepts friend request
    let (s, _) = t
        .call("POST", "/api/v1/social/friends/respond", Some(&steve_token), Some(json!({"friend_uuid": &alex_uuid, "accept": true})))
        .await;
    assert_eq!(s, StatusCode::OK);

    // Verify both see each other in their friends list
    let (s, alex_friends) = t.call("GET", "/api/v1/social/friends", Some(&alex_token), None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(alex_friends[0]["uuid"], steve_uuid);
    assert_eq!(alex_friends[0]["status"], "accepted");

    // 3. Direct Messaging
    let (s, msg) = t
        .call(
            "POST",
            &format!("/api/v1/messages/{steve_uuid}"),
            Some(&alex_token),
            Some(json!({
                "content": "Hey Steve! Ready for the wither fight?"
            })),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{msg}");

    // Steve reads messages from Alex
    let (s, msgs) = t.call("GET", &format!("/api/v1/messages/{alex_uuid}"), Some(&steve_token), None).await;
    assert_eq!(s, StatusCode::OK, "{msgs}");
    assert_eq!(msgs[0]["content"], "Hey Steve! Ready for the wither fight?");

    // 4. User Profile & Posts
    let (s, _) = t
        .call(
            "PUT",
            "/api/v1/social/profile/me",
            Some(&alex_token),
            Some(json!({
                "bio": "Redstone expert and dragon hunter."
            })),
        )
        .await;
    assert_eq!(s, StatusCode::OK);

    // Alex posts a social update
    let (s, post) = t
        .call(
            "POST",
            "/api/v1/social/posts",
            Some(&alex_token),
            Some(json!({
                "content": "Defeated the Ender Dragon on hardcore mode!",
                "image_url": "https://example.com/screenshot.png"
            })),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{post}");
    let post_id = post["id"].as_i64().unwrap();

    // Steve likes Alex's post
    let (s, like_res) = t.call("POST", &format!("/api/v1/social/posts/{post_id}/like"), Some(&steve_token), None).await;
    assert_eq!(s, StatusCode::OK, "{like_res}");
    assert_eq!(like_res["ok"], true);
    let (s, repeat_like) = t.call("POST", &format!("/api/v1/social/posts/{post_id}/like"), Some(&steve_token), None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(repeat_like["liked"], false);

    // Steve views Alex's full profile
    let (s, profile) = t.call("GET", &format!("/api/v1/social/profile/{alex_uuid}"), Some(&steve_token), None).await;
    assert_eq!(s, StatusCode::OK, "{profile}");
    assert_eq!(profile["bio"], "Redstone expert and dragon hunter.");
    assert_eq!(profile["posts"][0]["likes_count"], 1);
    assert_eq!(profile["posts"][0]["liked_by_me"], true);
    let (s, author_profile) = t.call("GET", &format!("/api/v1/social/profile/{alex_uuid}"), Some(&alex_token), None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(author_profile["posts"][0]["liked_by_me"], false);
}
