//! Guild land rules: leaders choose what happens on their claims, admins decide which rules are theirs to choose, and the game
//! server receives the result in the claim index.

mod common;
use common::*;

#[tokio::test]
async fn guilds_choose_their_land_rules_within_the_admin_policy() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    for name in ["Alex", "Bob", "Eve"] {
        t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username": name, "password": "password123"}))).await;
    }
    let (alex, bob, eve) = (t.login("Alex", "password123").await, t.login("Bob", "password123").await, t.login("Eve", "password123").await);
    let (_, srv) = t.call("POST", "/api/admin/servers", Some(&admin), Some(json!({"name": "SMP", "instance_id": "smp"}))).await;
    let (sid, token) = (srv["server"]["id"].as_i64().unwrap(), srv["token"].as_str().unwrap().to_string());
    let (_, g) = t.call("POST", "/api/v1/guilds", Some(&alex), Some(json!({"instance_id": "smp", "name": "Iron", "tag": "IRON"}))).await;
    let gid = g["id"].as_str().unwrap().to_string();
    let bob_uuid = t.uuid("Bob").await;
    sqlx::query("INSERT INTO guild_members (guild_id, uuid, name, role, joined_at) VALUES (?, ?, 'Bob', 'member', '2026-01-01T00:00:00Z')").bind(&gid).bind(&bob_uuid).execute(&t.db).await.unwrap();
    let url = format!("/api/v1/guilds/{gid}/claim-flags");

    // Defaults: land is protected from visitors, ordinary otherwise.
    let (s, v) = t.call("GET", &url, Some(&alex), None).await;
    assert_eq!(s, StatusCode::OK, "{v}");
    assert_eq!((v["flags"]["build"].as_bool(), v["flags"]["entry"].as_bool(), v["flags"]["pvp"].as_bool()), (Some(false), Some(true), Some(true)));
    assert_eq!(v["can_edit"], true);
    assert!(v["catalog"].as_array().unwrap().iter().all(|f| f["editable"] == true));

    // Members can look but not change; outsiders can't even look.
    let (s, v) = t.call("GET", &url, Some(&bob), None).await;
    assert_eq!((s, v["can_edit"].as_bool()), (StatusCode::OK, Some(false)));
    assert_eq!(t.call("PUT", &url, Some(&bob), Some(json!({"flags": {"pvp": false}}))).await.0, StatusCode::FORBIDDEN);
    assert_eq!(t.call("GET", &url, Some(&eve), None).await.0, StatusCode::FORBIDDEN);
    assert_eq!(t.call("PUT", &url, Some(&eve), Some(json!({"flags": {"pvp": false}}))).await.0, StatusCode::FORBIDDEN);

    // The leader chooses; bad input is refused.
    let (s, v) = t.call("PUT", &url, Some(&alex), Some(json!({"flags": {"pvp": false, "build": true}}))).await;
    assert_eq!(s, StatusCode::OK, "{v}");
    assert_eq!((v["flags"]["pvp"].as_bool(), v["flags"]["build"].as_bool()), (Some(false), Some(true)));
    assert_eq!(t.call("PUT", &url, Some(&alex), Some(json!({"flags": {"fly": true}}))).await.0, StatusCode::BAD_REQUEST, "unknown rule");
    assert_eq!(t.call("PUT", &url, Some(&alex), Some(json!({"flags": {"pvp": "yes"}}))).await.0, StatusCode::BAD_REQUEST);

    // The game server gets them with the claim index.
    t.call("POST", &format!("/api/v1/guilds/{gid}/claim"), Some(&alex), Some(json!({"server_id": sid, "dimension": "minecraft:overworld", "chunk_x": 0, "chunk_z": 0}))).await;
    let (_, idx) = t.call("POST", "/api/server/v1/guilds/claim-index", Some(&token), Some(json!({}))).await;
    let guild = idx["guilds"].as_array().unwrap().iter().find(|g| g["id"] == gid.as_str()).expect("the guild is in the index");
    assert_eq!((guild["flags"]["pvp"].as_bool(), guild["flags"]["build"].as_bool()), (Some(false), Some(true)), "{idx}");
    let revision = idx["revision"].as_str().unwrap().to_string();

    // In game: /guild flags lists the rules, /guild flags <rule> on|off changes one (leaders and officers only).
    let (alex_uuid, manage) = (t.uuid("Alex").await, "/api/server/v1/guilds/manage");
    let (s, v) = t.call("POST", manage, Some(&token), Some(json!({"uuid": alex_uuid, "name": "Alex", "action": "flags"}))).await;
    assert_eq!(s, StatusCode::OK, "{v}");
    assert_eq!(v["can_edit"], true);
    let pvp = v["rules"].as_array().unwrap().iter().find(|r| r["id"] == "pvp").unwrap();
    assert_eq!(pvp["value"], false, "the earlier change shows");
    let flag = |who: &str, rule: &str, text: &str| json!({"uuid": who, "name": "x", "action": "flag", "target": rule, "text": text});
    let (s, v) = t.call("POST", manage, Some(&token), Some(flag(&alex_uuid, "pvp", "on"))).await;
    assert_eq!(s, StatusCode::OK, "{v}");
    assert_eq!(t.call("POST", manage, Some(&token), Some(flag(&alex_uuid, "pvp", "maybe"))).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(t.call("POST", manage, Some(&token), Some(flag(&alex_uuid, "nonsense", "on"))).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(t.call("POST", manage, Some(&token), Some(flag(&bob_uuid, "pvp", "off"))).await.0, StatusCode::FORBIDDEN, "members can only look");
    let (_, v) = t.call("POST", manage, Some(&token), Some(json!({"uuid": bob_uuid, "name": "Bob", "action": "flags"}))).await;
    assert_eq!(v["can_edit"], false);
    let (_, v) = t.call("GET", &url, Some(&alex), None).await;
    assert_eq!(v["flags"]["pvp"], true, "the in-game change is the same data the panel shows");
    t.call("POST", manage, Some(&token), Some(flag(&alex_uuid, "pvp", "off"))).await;

    // Changing a rule makes game servers reload the index.
    t.call("PUT", &url, Some(&alex), Some(json!({"flags": {"mob_spawning": false}}))).await;
    let (_, idx) = t.call("POST", "/api/server/v1/guilds/claim-index", Some(&token), Some(json!({"revision": revision}))).await;
    assert_eq!(idx["unchanged"], false, "a new revision");

    // Admins decide which rules guilds control. Locked rules fall back to the default, in the API and in the index.
    assert_eq!(t.call("PUT", "/api/admin/guild-flag-policy", Some(&alex), Some(json!({"editable": ["pvp"]}))).await.0, StatusCode::FORBIDDEN);
    assert_eq!(t.call("PUT", "/api/admin/guild-flag-policy", Some(&admin), Some(json!({"editable": ["warp"]}))).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(t.call("PUT", "/api/admin/guild-flag-policy", Some(&admin), Some(json!({"editable": ["pvp"]}))).await.0, StatusCode::OK);
    let (_, v) = t.call("GET", &url, Some(&alex), None).await;
    assert_eq!(v["flags"]["build"], false, "build was true but is now locked to its default");
    assert_eq!(v["flags"]["pvp"], false, "pvp is still the guild's choice");
    assert_eq!(t.call("PUT", &url, Some(&alex), Some(json!({"flags": {"build": true}}))).await.0, StatusCode::BAD_REQUEST);
    let (_, idx) = t.call("POST", "/api/server/v1/guilds/claim-index", Some(&token), Some(json!({}))).await;
    let guild = idx["guilds"].as_array().unwrap().iter().find(|g| g["id"] == gid.as_str()).unwrap();
    assert_eq!(guild["flags"]["build"], false);
    let (_, p) = t.call("GET", "/api/admin/guild-flag-policy", Some(&admin), None).await;
    assert_eq!(p["catalog"].as_array().unwrap().iter().filter(|f| f["editable"] == true).count(), 1);
}
