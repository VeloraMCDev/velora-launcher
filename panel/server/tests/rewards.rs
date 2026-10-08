//! Bundles of rewards beyond XP: set by an admin, applied when earned, and delivered to the game server.

mod common;
use common::*;

#[tokio::test]
async fn an_achievement_can_pay_money_chunks_items_and_permissions() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username": "Steve", "password": "password123"}))).await;
    let steve_token = t.login("Steve", "password123").await;
    let steve = t.uuid("Steve").await;
    let (_, srv) = t.call("POST", "/api/admin/servers", Some(&admin), Some(json!({"name": "SMP", "instance_id": "smp"}))).await;
    let sid = srv["server"]["id"].as_i64().unwrap();
    let key = srv["token"].as_str().unwrap().to_string();
    let game = |path: &str, body: Value| {
        let (t, key, path) = (&t, key.clone(), path.to_string());
        async move { t.call("POST", &format!("/api/server/v1/{path}"), Some(&key), Some(body)).await }
    };
    let (s, ach) = t.call("POST", "/api/admin/achievements", Some(&admin), Some(json!({
        "id": "ach_test", "title": "Tester", "description": "x", "category": "general", "icon_frame": "task", "icon_item": "minecraft:stone",
        "icon_bg": "#000000", "icon_border": "#ffffff", "xp_reward": 10}))).await;
    assert_eq!(s, StatusCode::OK, "{ach}");

    // Bad bundles are refused; good ones are normalised.
    for a in [json!({"type": "money", "amount": -5}), json!({"type": "item", "item": "bad item!", "amount": 1}), json!({"type": "nope"})] {
        assert_eq!(
            t.call("PUT", "/api/admin/reward-bundles/achievement/ach_test", Some(&admin), Some(json!({"actions": [a]}))).await.0,
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(
        t.call("PUT", "/api/admin/reward-bundles/banana/x", Some(&admin), Some(json!({"actions": []}))).await.0,
        StatusCode::BAD_REQUEST
    );
    let (s, saved) = t
        .call(
            "PUT",
            "/api/admin/reward-bundles/achievement/ach_test",
            Some(&admin),
            Some(json!({"actions": [
        {"type": "money", "amount": 250},
        {"type": "claim_chunks", "amount": 8},
        {"type": "item", "item": "diamond", "amount": 3},
        {"type": "permission", "node": "essentials.fly", "minutes": 10080},
        {"type": "group", "group": "vip"},
        {"type": "message", "text": "Well done!"}]})),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{saved}");
    assert_eq!(saved["actions"][2]["item"], "minecraft:diamond");

    // Players see short summaries (never command text).
    let (_, sums) = t.call("GET", "/api/v1/reward-bundles", Some(&steve_token), None).await;
    assert_eq!(sums["achievement:ach_test"][0], "$250.00");
    assert_eq!(sums["achievement:ach_test"][2], "Diamond x3");
    assert_eq!(sums["achievement:ach_test"][3], "Permission: essentials.fly for 7 days");

    // Earning it applies the panel-side rewards straight away…
    sqlx::query("INSERT INTO server_online (server_id, uuid, name, joined_at) VALUES (?, ?, 'Steve', 'now')")
        .bind(sid)
        .bind(&steve)
        .execute(&t.db)
        .await
        .unwrap();
    sqlx::query("INSERT INTO user_achievements (user_uuid, achievement_id, unlocked_at) VALUES (?, 'ach_test', 'now')")
        .bind(&steve)
        .execute(&t.db)
        .await
        .unwrap();
    let (s, polled) = game("rewards/poll", json!({})).await;
    assert_eq!(s, StatusCode::OK, "{polled}");
    let kinds: Vec<String> = polled["deliveries"].as_array().unwrap().iter().map(|d| d["kind"].as_str().unwrap().to_string()).collect();
    assert_eq!(kinds, ["item", "permission", "group", "message"], "{polled}");
    assert_eq!(polled["deliveries"][0]["payload"]["item"], "minecraft:diamond");
    assert_eq!(polled["deliveries"][0]["name"], "Steve");
    let balance: f64 = sqlx::query_scalar("SELECT balance FROM server_economy WHERE server_id = ? AND uuid = ?")
        .bind(sid)
        .bind(&steve)
        .fetch_one(&t.db)
        .await
        .unwrap();
    assert_eq!(balance, 1250.0, "the starting 1000 plus 250");
    let chunks: i64 =
        sqlx::query_scalar("SELECT claim_chunks FROM player_bonuses WHERE uuid = ?").bind(&steve).fetch_one(&t.db).await.unwrap();
    assert_eq!(chunks, 8);

    // …and a delivery is handed out once until acknowledged.
    assert!(game("rewards/poll", json!({})).await.1["deliveries"].as_array().unwrap().is_empty());
    let ids: Vec<i64> = polled["deliveries"].as_array().unwrap().iter().map(|d| d["id"].as_i64().unwrap()).collect();
    let (s, _) = game("rewards/ack", json!({"done": [ids[0], ids[1]], "failed": [{"id": ids[2], "error": "no such group"}]})).await;
    assert_eq!(s, StatusCode::OK);
    let (_, log) = t.call("GET", "/api/admin/reward-deliveries", Some(&admin), None).await;
    let log = log.as_array().unwrap();
    assert_eq!(log.len(), 4);
    assert!(log.iter().any(|d| d["error"] == "no such group"));
    assert_eq!(log.iter().filter(|d| !d["delivered_at"].is_null()).count(), 2);

    // The extra chunks lift the player's guild's claim limit.
    let (_, g) =
        t.call("POST", "/api/v1/guilds", Some(&steve_token), Some(json!({"instance_id": "smp", "name": "Iron", "tag": "IRON"}))).await;
    let (_, detail) = t.call("GET", &format!("/api/v1/guilds/{}", g["id"].as_str().unwrap()), Some(&steve_token), None).await;
    assert_eq!(detail["max_claims"].as_i64(), Some(16 + 8), "{detail}");
}

#[tokio::test]
async fn deliveries_wait_for_the_player_to_be_online() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username": "Alex", "password": "password123"}))).await;
    let alex = t.uuid("Alex").await;
    let (_, srv) = t.call("POST", "/api/admin/servers", Some(&admin), Some(json!({"name": "SMP", "instance_id": "smp"}))).await;
    let sid = srv["server"]["id"].as_i64().unwrap();
    let key = srv["token"].as_str().unwrap().to_string();
    sqlx::query("INSERT INTO reward_deliveries (uuid, kind, payload, source, created_at) VALUES (?, 'item', '{\"type\":\"item\",\"item\":\"minecraft:apple\",\"amount\":1}', 'x', 'now')").bind(&alex).execute(&t.db).await.unwrap();
    let poll = || async { t.call("POST", "/api/server/v1/rewards/poll", Some(&key), Some(json!({}))).await.1 };
    assert!(poll().await["deliveries"].as_array().unwrap().is_empty(), "offline players get nothing yet");
    sqlx::query("INSERT INTO server_online (server_id, uuid, name, joined_at) VALUES (?, ?, 'Alex', 'now')")
        .bind(sid)
        .bind(&alex)
        .execute(&t.db)
        .await
        .unwrap();
    assert_eq!(poll().await["deliveries"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn a_rank_milestone_can_grant_more_than_its_title() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username": "Alex", "password": "password123"}))).await;
    // This test is about the milestone's own reward, so the default money for levels and milestones is off.
    let (_, cur) = t.call("GET", "/api/admin/progression", Some(&admin), None).await;
    let mut off = cur["settings"].clone();
    off["rules"]["auto_money"]["enabled"] = json!(false);
    assert_eq!(t.call("PUT", "/api/admin/progression", Some(&admin), Some(off)).await.0, StatusCode::OK);
    let alex = t.uuid("Alex").await;
    let (_, srv) = t.call("POST", "/api/admin/servers", Some(&admin), Some(json!({"name": "SMP", "instance_id": "smp"}))).await;
    let sid = srv["server"]["id"].as_i64().unwrap();
    let key = srv["token"].as_str().unwrap().to_string();
    let (_, made) = t
        .call(
            "POST",
            "/api/admin/rewards",
            Some(&admin),
            Some(json!({"level_type": "global", "level_req": 3, "reward_type": "title", "reward_name": "Veteran"})),
        )
        .await;
    let id = made["id"].as_i64().unwrap();
    let (s, _) = t
        .call(
            "PUT",
            &format!("/api/admin/reward-bundles/level_reward/{id}"),
            Some(&admin),
            Some(json!({"actions": [
        {"type": "money", "amount": 500}, {"type": "command", "command": "/give {player} cake 1"}]})),
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    sqlx::query("INSERT INTO server_online (server_id, uuid, name, joined_at) VALUES (?, ?, 'Alex', 'now')")
        .bind(sid)
        .bind(&alex)
        .execute(&t.db)
        .await
        .unwrap();
    let (s, r) = t
        .call(
            "POST",
            "/api/server/v1/player/xp",
            Some(&key),
            Some(json!({"operation_id": "lv", "uuid": alex, "scope": "global", "amount": 100000})),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{r}");
    let (_, polled) = t.call("POST", "/api/server/v1/rewards/poll", Some(&key), Some(json!({}))).await;
    assert_eq!(polled["deliveries"][0]["kind"], "command", "{polled}");
    assert_eq!(polled["deliveries"][0]["payload"]["command"], "give {player} cake 1");
    let balance: f64 = sqlx::query_scalar("SELECT balance FROM server_economy WHERE server_id = ? AND uuid = ?")
        .bind(sid)
        .bind(&alex)
        .fetch_one(&t.db)
        .await
        .unwrap();
    assert_eq!(balance, 1500.0);
    // Earning the same milestone twice pays once.
    t.call(
        "POST",
        "/api/server/v1/player/xp",
        Some(&key),
        Some(json!({"operation_id": "lv2", "uuid": alex, "scope": "global", "amount": 100})),
    )
    .await;
    assert!(t.call("POST", "/api/server/v1/rewards/poll", Some(&key), Some(json!({}))).await.1["deliveries"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn admin_rules_change_balances_claims_and_membership() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    for n in ["Leader", "Joiner", "Extra"] {
        t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username": n, "password": "password123"}))).await;
    }
    let leader = t.login("Leader", "password123").await;
    let (_, srv) = t.call("POST", "/api/admin/servers", Some(&admin), Some(json!({"name": "SMP", "instance_id": "smp"}))).await;
    let key = srv["token"].as_str().unwrap().to_string();
    let (_, settings) = t.call("GET", "/api/admin/progression", Some(&admin), None).await;
    let mut s = settings["settings"].clone();
    s["rules"] = json!({"starting_balance": 250.0, "guild_base_claims": 10, "guild_claims_per_member": 3, "guild_claims_per_level": 0, "guild_max_members": 2, "market_max_listings": 1});
    let (st, body) = t.call("PUT", "/api/admin/progression", Some(&admin), Some(s)).await;
    assert_eq!(st, StatusCode::OK, "{body}");

    // New players start with the configured balance.
    let uuid = t.uuid("Leader").await;
    let (_, bal) = t.call("POST", "/api/server/v1/economy/balance", Some(&key), Some(json!({"uuid": uuid, "username": "Leader"}))).await;
    assert_eq!(bal["balance"].as_f64(), Some(250.0), "{bal}");

    // A guild starts with the configured claims, grows with members, and stops at the member limit.
    let (_, g) = t.call("POST", "/api/v1/guilds", Some(&leader), Some(json!({"instance_id": "smp", "name": "Iron", "tag": "IRON"}))).await;
    let id = g["id"].as_str().unwrap().to_string();
    let max = || async { t.call("GET", &format!("/api/v1/guilds/{id}"), Some(&leader), None).await.1["max_claims"].as_i64() };
    assert_eq!(max().await, Some(10));
    let add = |name: &str| {
        let (t, leader, id, name) = (&t, leader.clone(), id.clone(), name.to_string());
        async move { t.call("POST", &format!("/api/v1/guilds/{id}/members"), Some(&leader), Some(json!({"username": name}))).await }
    };
    let (st, body) = add("Joiner").await;
    assert_eq!(st, StatusCode::OK, "{body}");
    assert_eq!(max().await, Some(13), "+3 per member beyond the leader");
    let (st, body) = add("Extra").await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "the guild is full at 2 members: {body}");
}

#[tokio::test]
async fn quests_achievements_and_levels_pay_default_money_scaled_by_difficulty_in_fives() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    // The seeded quests and achievements already have an automatic money reward, every one a flat number ending in 5 or 0.
    let rows: Vec<(String, String, String)> = sqlx::query_as("SELECT source_type, source_id, actions FROM reward_bundles").fetch_all(&t.db).await.unwrap();
    assert!(rows.len() > 100, "every seeded quest and achievement got one: {}", rows.len());
    for (kind, id, actions) in &rows {
        let a: Value = serde_json::from_str(actions).unwrap();
        let money = a.as_array().unwrap().iter().find(|x| x["type"] == "money").unwrap_or_else(|| panic!("{kind}:{id} has no money"));
        let amount = money["amount"].as_f64().unwrap();
        assert!(amount >= 5.0 && amount % 5.0 == 0.0, "{kind}:{id} pays {amount}");
        assert_eq!(money["auto"], true);
    }
    // Harder (more XP) pays more, and weeklies pay more per XP than dailies.
    let pay = |id: &'static str| { let t = &t; async move { let a: String = sqlx::query_scalar("SELECT actions FROM reward_bundles WHERE source_id = ?").bind(id).fetch_one(&t.db).await.unwrap(); serde_json::from_str::<Value>(&a).unwrap()[0]["amount"].as_f64().unwrap() } };
    sqlx::query("INSERT INTO quests (id, title, description, period, category, target_stat, target_count, xp_reward, icon, enabled, created_at) VALUES ('easy','e','e','daily','x','blocks_broken',10,100,'star',1,'x'), ('hard','h','h','daily','x','blocks_broken',10,400,'star',1,'x'), ('wk','w','w','weekly','x','blocks_broken',10,400,'star',1,'x')").execute(&t.db).await.unwrap();
    let (s, r) = t.call("POST", "/api/admin/progression/auto-money/apply", Some(&admin), None).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    assert_eq!(pay("easy").await, 35.0);
    assert_eq!(pay("hard").await, 140.0);
    assert_eq!(pay("wk").await, 200.0);
    // An amount the admin set is never touched, even when the rates change; the automatic ones follow the rates.
    t.call("PUT", "/api/admin/reward-bundles/quest/easy", Some(&admin), Some(json!({"actions": [{"type": "money", "amount": 999}]}))).await;
    let (_, settings) = t.call("GET", "/api/admin/progression", Some(&admin), None).await;
    let mut next = settings["settings"].clone();
    next["rules"]["auto_money"]["scale"] = json!(2.0);
    let (s, saved) = t.call("PUT", "/api/admin/progression", Some(&admin), Some(next)).await;
    assert_eq!(s, StatusCode::OK, "{saved}");
    assert_eq!(pay("easy").await, 999.0);
    assert_eq!(pay("hard").await, 280.0);
    assert_eq!(saved["money_preview"]["levels"][0]["level"], 2);
}

#[tokio::test]
async fn the_bulk_editor_lists_and_saves_many_rows_at_once() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    let (s, list) = t.call("GET", "/api/admin/bulk/quest", Some(&admin), None).await;
    assert_eq!(s, StatusCode::OK, "{list}");
    let rows = list["rows"].as_array().unwrap();
    assert!(rows.len() > 5 && rows.iter().all(|r| r["auto"] == true && r["money"].as_f64().unwrap() % 5.0 == 0.0));
    let (a, b) = (rows[0]["id"].as_str().unwrap().to_string(), rows[1]["id"].as_str().unwrap().to_string());
    let (s, r) = t.call("POST", "/api/admin/bulk/quest", Some(&admin), Some(json!({"patches": [
        {"id": a, "xp": 500, "money": 125},
        {"id": b, "xp": 20, "enabled": false},
    ]}))).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    assert_eq!(r["changed"], 2);
    let (_, list) = t.call("GET", "/api/admin/bulk/quest", Some(&admin), None).await;
    let row = |id: &str| list["rows"].as_array().unwrap().iter().find(|r| r["id"] == id).unwrap().clone();
    assert_eq!((row(&a)["xp"].as_i64(), row(&a)["money"].as_f64(), row(&a)["auto"].as_bool()), (Some(500), Some(125.0), Some(false)), "your own amount sticks");
    assert_eq!((row(&b)["xp"].as_i64(), row(&b)["enabled"].as_bool(), row(&b)["auto"].as_bool()), (Some(20), Some(false), Some(true)), "automatic money follows the new XP");
    assert_eq!(row(&b)["money"].as_f64().unwrap() % 5.0, 0.0);
    // Going back to automatic puts the default back.
    t.call("POST", "/api/admin/bulk/quest", Some(&admin), Some(json!({"patches": [{"id": a, "reset_money": true}]}))).await;
    let (_, list) = t.call("GET", "/api/admin/bulk/quest", Some(&admin), None).await;
    let r = list["rows"].as_array().unwrap().iter().find(|r| r["id"] == a.as_str()).unwrap();
    assert_eq!(r["auto"], true);
    assert_ne!(r["money"].as_f64(), Some(125.0));
    // Bad input and unknown kinds are refused.
    assert_eq!(t.call("POST", "/api/admin/bulk/quest", Some(&admin), Some(json!({"patches": [{"id": a, "xp": -1}]}))).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(t.call("GET", "/api/admin/bulk/nope", Some(&admin), None).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(t.call("GET", "/api/admin/bulk/achievement", Some(&admin), None).await.0, StatusCode::OK);
    assert_eq!(t.call("GET", "/api/admin/bulk/level_reward", Some(&admin), None).await.0, StatusCode::OK);
}

#[tokio::test]
async fn a_reward_can_unlock_a_cosmetic() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username": "Steve", "password": "password123"}))).await;
    let steve = t.uuid("Steve").await;
    let (_, srv) = t.call("POST", "/api/admin/servers", Some(&admin), Some(json!({"name": "SMP", "instance_id": "smp"}))).await;
    let key = srv["token"].as_str().unwrap().to_string();
    t.call("POST", "/api/admin/cosmetics/templates", Some(&admin), Some(json!({
        "key": "angel_wings", "type": "cosmetic", "label": "Angel Wings", "description": "",
        "metadata": {"slot": "back", "model": "scopenet:item/wings"}}))).await;
    t.call("POST", "/api/admin/achievements", Some(&admin), Some(json!({
        "id": "ach_wings", "title": "Winged", "description": "x", "category": "general", "icon_frame": "task", "icon_item": "minecraft:stone",
        "icon_bg": "#000000", "icon_border": "#ffffff", "xp_reward": 10}))).await;

    // A bundle must name a cosmetic.
    let put = |actions: Value| t.call("PUT", "/api/admin/reward-bundles/achievement/ach_wings", Some(&admin), Some(json!({"actions": actions})));
    assert_eq!(put(json!([{"type": "unlock", "key": ""}])).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(put(json!([{"type": "unlock", "key": "angel_wings"}])).await.0, StatusCode::OK);

    sqlx::query("INSERT INTO user_achievements (user_uuid, achievement_id, unlocked_at) VALUES (?, 'ach_wings', 'now')").bind(&steve).execute(&t.db).await.unwrap();
    assert_eq!(t.call("POST", "/api/server/v1/rewards/poll", Some(&key), Some(json!({}))).await.0, StatusCode::OK);
    let (kind, meta): (String, String) =
        sqlx::query_as("SELECT unlock_type, metadata FROM player_unlocks WHERE uuid = ? AND unlock_key = 'angel_wings'").bind(&steve).fetch_one(&t.db).await.unwrap();
    assert_eq!(kind, "cosmetic");
    assert!(meta.contains("scopenet:item/wings"), "the template's model travels with the unlock: {meta}");
}

#[tokio::test]
async fn quest_chains_exist_before_they_have_quests() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    let base = "/api/admin/quest-chains";
    assert_eq!(t.call("POST", base, Some(&admin), Some(json!({"name": "  "}))).await.0, StatusCode::BAD_REQUEST);
    let (s, made) = t.call("POST", base, Some(&admin), Some(json!({"name": "Beginner's Journey", "description": "Start here"}))).await;
    assert_eq!(s, StatusCode::OK, "{made}");
    assert_eq!(made["id"], "beginner_s_journey");
    assert_eq!(t.call("POST", base, Some(&admin), Some(json!({"name": "Beginner's Journey"}))).await.0, StatusCode::CONFLICT);

    // It is listed while empty, and a quest using it moves in.
    let (_, list) = t.call("GET", base, Some(&admin), None).await;
    assert_eq!(list[0]["name"], "Beginner's Journey");
    sqlx::query("UPDATE quests SET chain_id = 'beginner_s_journey', chain_step = 0 WHERE id = (SELECT id FROM quests LIMIT 1)").execute(&t.db).await.unwrap();
    assert_eq!(t.call("PUT", &format!("{base}/beginner_s_journey"), Some(&admin), Some(json!({"name": "Journey", "description": ""}))).await.0, StatusCode::OK);
    let (_, list) = t.call("GET", base, Some(&admin), None).await;
    assert_eq!((list.as_array().unwrap().len(), list[0]["name"].as_str()), (1, Some("Journey")));

    // Deleting keeps the quests but ungroups them.
    assert_eq!(t.call("DELETE", &format!("{base}/beginner_s_journey"), Some(&admin), None).await.0, StatusCode::OK);
    let left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM quests WHERE chain_id = 'beginner_s_journey'").fetch_one(&t.db).await.unwrap();
    assert_eq!(left, 0);
    assert!(t.call("GET", base, Some(&admin), None).await.1.as_array().unwrap().is_empty());
}
