//! Admin claims: named, described regions owned by the server, delivered in the claim index and closed to guilds.

mod common;
use common::*;

#[tokio::test]
async fn admin_claims_are_managed_protected_and_delivered_to_the_game() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username": "Alex", "password": "password123"}))).await;
    let alex = t.login("Alex", "password123").await;
    let (_, srv) = t.call("POST", "/api/admin/servers", Some(&admin), Some(json!({"name": "SMP", "instance_id": "smp"}))).await;
    let sid = srv["server"]["id"].as_i64().unwrap();
    let token = srv["token"].as_str().unwrap().to_string();
    let base = format!("/api/admin/servers/{sid}/admin-claims");

    // Create from the panel, with a description and colour.
    let (s, c) = t.call("POST", &base, Some(&admin), Some(json!({"name": "Spawn", "description": "Welcome! No building here.", "color": "#22c55e"}))).await;
    assert_eq!(s, StatusCode::OK, "{c}");
    let id = c["id"].as_str().unwrap().to_string();
    assert_eq!(t.call("POST", &base, Some(&admin), Some(json!({"name": "spawn"}))).await.0, StatusCode::BAD_REQUEST, "names are unique, ignoring case");
    assert_eq!(t.call("POST", &base, Some(&alex), Some(json!({"name": "Nope"}))).await.0, StatusCode::FORBIDDEN);

    // Block coordinates -16..31 are chunks -1..1: a 3x3 area.
    let (_, a) = t.call("POST", &format!("/api/admin/admin-claims/{id}/area"), Some(&admin), Some(json!({"x1": -16, "z1": -16, "x2": 31, "z2": 31}))).await;
    assert_eq!(a["added"], 9);
    let (_, list) = t.call("GET", &base, Some(&admin), None).await;
    assert_eq!(list["claims"][0]["chunks"], 9);
    assert_eq!(list["claims"][0]["bounds"]["min_x"], -1);

    // The game gets it as a guild-shaped entry flagged admin, with its description and colour.
    let (st, idx) = t.call("POST", "/api/server/v1/guilds/claim-index", Some(&token), Some(json!({}))).await;
    assert_eq!(st, StatusCode::OK, "{idx}");
    assert_eq!(idx["claims"].as_array().unwrap().len(), 9);
    assert_eq!(idx["guilds"][0]["admin"], true);
    assert_eq!(idx["guilds"][0]["name"], "Spawn");
    assert_eq!(idx["guilds"][0]["description"], "Welcome! No building here.");
    assert_eq!(idx["guilds"][0]["color"], "#22c55e");

    // Guilds can't claim inside it, but can right next to it.
    let (_, g) = t.call("POST", "/api/v1/guilds", Some(&alex), Some(json!({"instance_id": "smp", "name": "Iron", "tag": "IRON"}))).await;
    let gid = g["id"].as_str().unwrap_or_else(|| panic!("{g}"));
    let claim = |x: i32| {
        let (t, alex) = (&t, alex.clone());
        let path = format!("/api/v1/guilds/{gid}/claim");
        async move { t.call("POST", &path, Some(&alex), Some(json!({"server_id": sid, "dimension": "minecraft:overworld", "chunk_x": x, "chunk_z": 0}))).await.0 }
    };
    assert_eq!(claim(0).await, StatusCode::BAD_REQUEST);
    assert_eq!(claim(2).await, StatusCode::OK);
    // And an admin area skips chunks a guild already owns.
    let (_, a) = t.call("POST", &format!("/api/admin/admin-claims/{id}/area"), Some(&admin), Some(json!({"x1": 1, "z1": 0, "x2": 3, "z2": 0, "chunks": true}))).await;
    assert_eq!((a["added"].as_i64().unwrap(), a["skipped"].as_i64().unwrap()), (1, 2), "chunk 1 was already ours, 2 is a guild's, 3 is new");

    // In-game commands: describe, rename, create standing here, remove here, delete.
    let game = |body: Value| {
        let (t, token) = (&t, token.clone());
        async move { t.call("POST", "/api/server/v1/admin-claims", Some(&token), Some(body)).await }
    };
    assert_eq!(game(json!({"action": "describe", "name": "Spawn", "value": "Come on in"})).await.0, StatusCode::OK);
    assert_eq!(game(json!({"action": "rename", "name": "Spawn", "value": "Hub"})).await.0, StatusCode::OK);
    let (s, made) = game(json!({"action": "create", "name": "Arena", "value": "PvP", "chunk_x": 10, "chunk_z": 10, "radius": 1})).await;
    assert_eq!((s, made["added"].as_i64()), (StatusCode::OK, Some(9)));
    let (_, r) = game(json!({"action": "remove", "chunk_x": 10, "chunk_z": 10})).await;
    assert_eq!(r["removed"], 1);
    assert_eq!(game(json!({"action": "color", "name": "Hub", "value": "red"})).await.0, StatusCode::BAD_REQUEST);
    let (_, l) = game(json!({"action": "list"})).await;
    assert_eq!(l["claims"].as_array().unwrap().len(), 2);
    assert_eq!(game(json!({"action": "delete", "name": "Arena"})).await.0, StatusCode::OK);
    let (_, idx) = t.call("POST", "/api/server/v1/guilds/claim-index", Some(&token), Some(json!({}))).await;
    let names: Vec<_> = idx["guilds"].as_array().unwrap().iter().map(|g| g["name"].as_str().unwrap().to_string()).collect();
    assert!(names.contains(&"Hub".to_string()) && !names.contains(&"Arena".to_string()), "{names:?}");
}


#[tokio::test]
async fn admin_claim_flags_default_to_a_protected_region_and_reach_the_game() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    let (_, srv) = t.call("POST", "/api/admin/servers", Some(&admin), Some(json!({"name": "SMP", "instance_id": "smp"}))).await;
    let sid = srv["server"]["id"].as_i64().unwrap();
    let token = srv["token"].as_str().unwrap().to_string();
    let base = format!("/api/admin/servers/{sid}/admin-claims");
    let (_, c) = t.call("POST", &base, Some(&admin), Some(json!({"name": "Spawn"}))).await;
    let id = c["id"].as_str().unwrap().to_string();
    t.call("POST", &format!("/api/admin/admin-claims/{id}/area"), Some(&admin), Some(json!({"x1": 0, "z1": 0, "x2": 15, "z2": 15}))).await;

    // A new claim behaves like claims always have: no building, no griefing, everything else ordinary.
    let (_, list) = t.call("GET", &base, Some(&admin), None).await;
    let flags = &list["claims"][0]["flags"];
    assert_eq!((flags["build"].as_bool(), flags["explosions"].as_bool(), flags["pvp"].as_bool(), flags["fly"].as_bool(), flags["mob_spawning"].as_bool()), (Some(false), Some(false), Some(true), Some(true), Some(true)));
    let catalog = list["flag_catalog"].as_array().unwrap();
    assert!(catalog.len() >= 14 && catalog.iter().all(|f| f["label"].is_string() && f["help"].is_string() && f["group"].is_string()));

    // Change a few; the rest keep their value.
    let (s, r) = t.call("PATCH", &format!("/api/admin/admin-claims/{id}"), Some(&admin), Some(json!({"flags": {"pvp": false, "mob_spawning": false, "fly": false}}))).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    t.call("PATCH", &format!("/api/admin/admin-claims/{id}"), Some(&admin), Some(json!({"flags": {"interact": true}}))).await;
    let (_, list) = t.call("GET", &base, Some(&admin), None).await;
    let f = &list["claims"][0]["flags"];
    assert_eq!((f["pvp"].as_bool(), f["mob_spawning"].as_bool(), f["fly"].as_bool(), f["interact"].as_bool(), f["build"].as_bool(), f["hunger"].as_bool()), (Some(false), Some(false), Some(false), Some(true), Some(false), Some(true)));

    // Unknown or non-boolean flags are refused and change nothing.
    assert_eq!(t.call("PATCH", &format!("/api/admin/admin-claims/{id}"), Some(&admin), Some(json!({"flags": {"teleport_everyone": true}}))).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(t.call("PATCH", &format!("/api/admin/admin-claims/{id}"), Some(&admin), Some(json!({"flags": {"pvp": "yes"}}))).await.0, StatusCode::BAD_REQUEST);

    // The game server gets the resolved flags with the claim (and a changed flag changes the index revision).
    let (_, idx) = t.call("POST", "/api/server/v1/guilds/claim-index", Some(&token), Some(json!({}))).await;
    assert_eq!(idx["guilds"][0]["flags"]["pvp"], false);
    assert_eq!(idx["guilds"][0]["flags"]["interact"], true);
    assert_eq!(idx["guilds"][0]["flags"]["build"], false);
    let rev = idx["revision"].as_str().unwrap().to_string();
    t.call("PATCH", &format!("/api/admin/admin-claims/{id}"), Some(&admin), Some(json!({"flags": {"explosions": true}}))).await;
    let (_, again) = t.call("POST", "/api/server/v1/guilds/claim-index", Some(&token), Some(json!({"revision": rev}))).await;
    assert_eq!(again["unchanged"], false, "flag changes reach the game");
    assert_eq!(again["guilds"][0]["flags"]["explosions"], true);
}
