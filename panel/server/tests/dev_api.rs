//! What plugins can read and do through the server API (player info, XP,
//! quest objectives, friends, guilds).

mod common;
use common::*;

#[tokio::test]
async fn plugins_read_players_and_change_progress() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    let mut tokens = vec![];
    for n in ["Steve", "Alex"] {
        t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username": n, "password": "password123"}))).await;
        tokens.push(t.login(n, "password123").await);
    }
    let (steve, alex) = (t.uuid("Steve").await, t.uuid("Alex").await);
    let (_, srv) = t.call("POST", "/api/admin/servers", Some(&admin), Some(json!({"name": "SMP", "instance_id": "smp"}))).await;
    let sid = srv["server"]["id"].as_i64().unwrap();
    let key = srv["token"].as_str().unwrap().to_string();
    let call = |path: &str, body: Value| {
        let (t, key, path) = (&t, key.clone(), path.to_string());
        async move { t.call("POST", &format!("/api/server/v1/{path}"), Some(&key), Some(body)).await }
    };
    t.call("PUT", "/api/admin/progression", Some(&admin), Some(json!({"daily_quest_limit": 0, "weekly_quest_limit": 0}))).await;

    // Unknown players are reported, not errors.
    let (s, v) = call("player/info", json!({"uuid": "00000000-0000-0000-0000-00000000dead"})).await;
    assert_eq!((s, v["exists"].clone()), (StatusCode::OK, json!(false)));
    assert_eq!(call("player/info", json!({"uuid": "nope"})).await.0, StatusCode::BAD_REQUEST);

    // Info: levels, balance, guild, quests.
    sqlx::query("INSERT INTO server_economy (server_id, uuid, username, balance, updated_at) VALUES (?, ?, 'Steve', 321.5, 'now')")
        .bind(sid)
        .bind(&steve)
        .execute(&t.db)
        .await
        .unwrap();
    let (_, g) =
        t.call("POST", "/api/v1/guilds", Some(&tokens[0]), Some(json!({"instance_id": "smp", "name": "Iron", "tag": "IRON"}))).await;
    t.call("GET", "/api/v1/quests/my", Some(&tokens[0]), None).await;
    let (s, info) = call("player/info", json!({"uuid": steve})).await;
    assert_eq!(s, StatusCode::OK, "{info}");
    assert_eq!(
        (info["name"].as_str(), info["balance"].as_f64(), info["guild"]["tag"].as_str(), info["guild"]["role"].as_str()),
        (Some("Steve"), Some(321.5), Some("IRON"), Some("leader"))
    );
    // Founding a guild unlocks an achievement worth XP, so the player starts above zero.
    let base = info["global"]["xp"].as_i64().unwrap();
    assert!(base > 0);
    assert!(info["quests"]["daily"]["total"].as_i64().unwrap() > 5);
    assert_eq!(info["quests"]["daily"]["completed"], 0);

    // addXP: global and server scoped, idempotent, negative takes away.
    let xp = |op: &str, scope: &str, amount: i64| json!({"operation_id": op, "uuid": steve, "scope": scope, "amount": amount, "reason": "event"});
    let (s, r) = call("player/xp", xp("x1", "global", 1200)).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    assert_eq!(r["xp"].as_i64(), Some(base + 1200));
    assert!(r["level"].as_i64().unwrap() >= 5);
    assert_eq!(call("player/xp", xp("x1", "global", 1200)).await.1, r, "a retry changes nothing");
    let (_, r) = call("player/xp", xp("x2", "server", 400)).await;
    assert_eq!(r["level"], 2);
    let (_, r) = call("player/xp", xp("x3", "global", -200)).await;
    assert_eq!(r["xp"].as_i64(), Some(base + 1000));
    assert_eq!(call("player/xp", xp("x4", "global", 0)).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(
        call("player/xp", json!({"operation_id": "x5", "uuid": "00000000-0000-0000-0000-00000000dead", "amount": 5})).await.0,
        StatusCode::NOT_FOUND
    );
    let (_, info) = call("player/info", json!({"uuid": steve})).await;
    assert_eq!(
        (info["global"]["xp"].as_i64(), info["server"]["xp"].as_i64(), info["server"]["level"].as_i64()),
        (Some(base + 1000), Some(400), Some(2))
    );

    // Quest objectives by quest id (complete it), and by action.
    let (_, quests) = t.call("GET", "/api/v1/quests/my", Some(&tokens[0]), None).await;
    let quest =
        quests.as_array().unwrap().iter().find(|q| q["quest"]["period"] == "daily").unwrap()["quest"]["id"].as_str().unwrap().to_string();
    let (s, r) = call("player/quest-objective", json!({"operation_id": "q1", "uuid": steve, "objective": quest, "amount": 0})).await;
    assert_eq!((s, r["advanced"].clone()), (StatusCode::OK, json!(true)), "{r}");
    let (_, quests) = t.call("GET", "/api/v1/quests/my", Some(&tokens[0]), None).await;
    let done = quests.as_array().unwrap().iter().find(|q| q["quest"]["id"] == quest.as_str()).unwrap();
    assert_eq!(done["completed"], true);
    let (_, r) = call(
        "player/quest-objective",
        json!({"operation_id": "q2", "uuid": steve, "objective": "block_broken:DIAMOND_ORE@minecraft:overworld", "amount": 3}),
    )
    .await;
    assert_eq!(r["advanced"], true);
    let (_, r) =
        call("player/quest-objective", json!({"operation_id": "q3", "uuid": steve, "objective": "nothing:at_all", "amount": 3})).await;
    assert_eq!(r["advanced"], false);
    let (_, info) = call("player/info", json!({"uuid": steve})).await;
    assert_eq!(info["quests"]["daily"]["completed"], 1);

    // Friends and guilds.
    t.call("POST", "/api/v1/friends/request", Some(&tokens[0]), Some(json!({"username": "Alex"}))).await;
    t.call("POST", "/api/v1/friends/respond", Some(&tokens[1]), Some(json!({"target_uuid": steve, "accept": true}))).await;
    let (_, f) = call("player/friends", json!({"uuid": steve})).await;
    assert_eq!((f["friends"][0]["name"].as_str(), f["friends"][0]["uuid"].as_str()), (Some("Alex"), Some(alex.as_str())));
    let (gs, gi) = call("guild/info", json!({"guild": "iron"})).await;
    assert_eq!(gs, StatusCode::OK, "{gi}");
    assert_eq!((gi["exists"].clone(), gi["name"].as_str(), gi["members"][0]["name"].as_str()), (json!(true), Some("Iron"), Some("Steve")));
    let (_, by_member) = call("guild/info", json!({"member": steve})).await;
    assert_eq!(by_member["id"], g["id"]);
    assert_eq!(call("guild/info", json!({"guild": "nobody"})).await.1["exists"], false);
    assert_eq!(t.call("POST", "/api/server/v1/player/info", None, Some(json!({"uuid": steve}))).await.0, StatusCode::UNAUTHORIZED);
}
