//! Admin progression controls: quest limits/rotation, level curve, manual XP.
//! One test on purpose: the level curve is process-wide state.

mod common;
use common::*;

async fn player(t: &TestApp, admin: &str, name: &str) -> (String, String) {
    let (s, v) = t.call("POST", "/api/admin/users", Some(admin), Some(json!({"username": name, "password": "password123"}))).await;
    assert_eq!(s, StatusCode::OK, "{v}");
    (t.login(name, "password123").await, t.uuid(name).await)
}

fn ids(v: &Value, period: &str) -> Vec<String> {
    let mut out: Vec<String> = v
        .as_array()
        .unwrap()
        .iter()
        .filter(|q| q["quest"]["period"] == period)
        .map(|q| q["quest"]["id"].as_str().unwrap().to_string())
        .collect();
    out.sort();
    out
}

#[tokio::test]
async fn admins_control_quest_limits_levels_and_xp() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    let (alex, _) = player(&t, &admin, "Alex").await;
    let (steve, steve_uuid) = player(&t, &admin, "Steve").await;

    // Defaults: 5 daily + 3 weekly quests per player out of the seeded pool.
    let (s, cfg) = t.call("GET", "/api/admin/progression", Some(&admin), None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(cfg["settings"]["daily_quest_limit"], 5);
    let weekly_pool = cfg["quest_pool"]["weekly"]["enabled"].as_u64().unwrap() as usize;
    assert!(cfg["quest_pool"]["daily"]["enabled"].as_u64().unwrap() > 20 && weekly_pool > 20);
    let (s, _) = t.call("GET", "/api/admin/progression", Some(&steve), None).await;
    assert_eq!(s, StatusCode::FORBIDDEN);

    let (_, mine) = t.call("GET", "/api/v1/quests/my", Some(&steve), None).await;
    assert_eq!(ids(&mine, "daily").len(), 5);
    assert_eq!(ids(&mine, "weekly").len(), 3);
    // Stable on repeat, different between players.
    let (_, again) = t.call("GET", "/api/v1/quests/my", Some(&steve), None).await;
    assert_eq!(ids(&mine, "daily"), ids(&again, "daily"));
    let (_, theirs) = t.call("GET", "/api/v1/quests/my", Some(&alex), None).await;
    assert_ne!(ids(&mine, "daily"), ids(&theirs, "daily"));

    // A quest that wasn't handed out can't be claimed.
    let given = ids(&mine, "daily");
    let other = ids(&theirs, "daily").into_iter().find(|id| !given.contains(id)).unwrap();
    let (s, e) = t.call("POST", &format!("/api/v1/quests/{other}/claim"), Some(&steve), None).await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "{e}");

    // Lower the limit: the first picks stay. Raise it: they are kept and topped up.
    let mut settings = cfg["settings"].clone();
    settings["daily_quest_limit"] = json!(2);
    let (s, v) = t.call("PUT", "/api/admin/progression", Some(&admin), Some(settings.clone())).await;
    assert_eq!(s, StatusCode::OK, "{v}");
    let (_, fewer) = t.call("GET", "/api/v1/quests/my", Some(&steve), None).await;
    let fewer_ids = ids(&fewer, "daily");
    assert_eq!(fewer_ids.len(), 2);
    assert!(fewer_ids.iter().all(|id| given.contains(id)));
    settings["daily_quest_limit"] = json!(7);
    t.call("PUT", "/api/admin/progression", Some(&admin), Some(settings.clone())).await;
    let (_, more) = t.call("GET", "/api/v1/quests/my", Some(&steve), None).await;
    assert_eq!(ids(&more, "daily").len(), 7);
    assert!(fewer_ids.iter().all(|id| ids(&more, "daily").contains(id)));

    // 0 hands out everything; "shared" gives all players the same set.
    settings["weekly_quest_limit"] = json!(0);
    settings["daily_quest_limit"] = json!(4);
    settings["quest_rotation"] = json!("shared");
    t.call("PUT", "/api/admin/progression", Some(&admin), Some(settings.clone())).await;
    let (_, all_weekly) = t.call("GET", "/api/v1/quests/my", Some(&steve), None).await;
    assert_eq!(ids(&all_weekly, "weekly").len(), weekly_pool);
    // Players who already drew today's set keep it; new players share one set.
    let (mia, _) = player(&t, &admin, "Mia").await;
    let (zoe, _) = player(&t, &admin, "Zoe").await;
    let (_, a) = t.call("GET", "/api/v1/quests/my", Some(&mia), None).await;
    let (_, b) = t.call("GET", "/api/v1/quests/my", Some(&zoe), None).await;
    assert_eq!(ids(&a, "daily").len(), 4);
    assert_eq!(ids(&a, "daily"), ids(&b, "daily"), "shared rotation is the same for everyone");

    // Pinned quests are picked first.
    let pin = ids(&theirs, "daily").into_iter().find(|id| !ids(&a, "daily").contains(id)).unwrap();
    let (s, q) = t.call("GET", "/api/admin/quests", Some(&admin), None).await;
    assert_eq!(s, StatusCode::OK);
    let found = q.as_array().unwrap().iter().find(|x| x["id"] == pin.as_str()).unwrap();
    let quest = json!({
        "title": found["title"], "description": found["description"], "period": found["period"], "category": found["category"],
        "target_stat": found["target_stat"], "target_count": found["target_count"], "xp_reward": found["xp_reward"], "pinned": true
    });
    let (s, e) = t.call("PUT", &format!("/api/admin/quests/{pin}"), Some(&admin), Some(quest)).await;
    assert_eq!(s, StatusCode::OK, "{e}");
    let (_, pinned) = t.call("GET", "/api/v1/quests/my", Some(&admin), None).await;
    assert!(ids(&pinned, "daily").contains(&pin));

    // Validation.
    settings["daily_quest_limit"] = json!(500);
    let (s, _) = t.call("PUT", "/api/admin/progression", Some(&admin), Some(settings.clone())).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);

    // Manual XP.
    let adjust = |body: Value| {
        let uri = format!("/api/admin/progression/players/{steve_uuid}/adjust");
        let (t, admin) = (&t, admin.clone());
        async move { t.call("POST", &uri, Some(&admin), Some(body)).await }
    };
    let (s, v) = adjust(json!({"scope": "global", "mode": "add", "amount": 1000, "reason": "event prize"})).await;
    assert_eq!(s, StatusCode::OK, "{v}");
    assert_eq!(v["global"]["xp"], 1000);
    assert_eq!(v["global"]["level"], 4); // 100 * 4^1.5 = 800 <= 1000 < 100 * 5^1.5 = 1118
    let (_, v) = adjust(json!({"scope": "global", "mode": "set_level", "level": 10, "amount": 10})).await;
    assert_eq!(v["global"]["xp"], 3162);
    assert_eq!(v["global"]["level"], 10);
    let (_, v) = adjust(json!({"scope": "global", "mode": "remove", "amount": 5000})).await;
    assert_eq!(v["global"]["xp"], 0);
    let (s, _) = adjust(json!({"scope": "global", "mode": "set_level", "amount": 0})).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);

    let (_, servers) = t.call("POST", "/api/admin/servers", Some(&admin), Some(json!({"name": "Survival"}))).await;
    let sid = servers["server"]["id"].as_i64().unwrap();
    let (s, v) = adjust(json!({"scope": "server", "server_id": sid, "mode": "set_xp", "amount": 300})).await;
    assert_eq!(s, StatusCode::OK, "{v}");
    assert_eq!(v["servers"][0]["xp"], 300);
    assert_eq!(v["servers"][0]["level"], 2);
    let (_, me) = t.call("GET", "/api/v1/levels/me", Some(&steve), None).await;
    assert_eq!(me["server_levels"][0]["server_xp"], 300);

    let (_, list) = t.call("GET", "/api/admin/progression/players?q=ste", Some(&admin), None).await;
    assert_eq!(list["players"][0]["name"], "Steve");
    let (_, log) = t.call("GET", "/api/admin/activity?source=panel", Some(&admin), None).await;
    assert!(log.as_array().unwrap().iter().any(|e| e["kind"] == "xp_adjust" && e["detail"].as_str().unwrap().contains("event prize")));

    // A steeper curve recalculates stored levels: 1000 XP is level 4 at base
    // 100 but only level 2 at base 500 (500 * 2^1.5 = 1414 > 1000 -> level 1).
    adjust(json!({"scope": "global", "mode": "set_xp", "amount": 1000})).await;
    settings["daily_quest_limit"] = json!(5);
    settings["level_base"] = json!(500.0);
    let (s, v) = t.call("PUT", "/api/admin/progression", Some(&admin), Some(settings.clone())).await;
    assert_eq!(s, StatusCode::OK, "{v}");
    assert!(v["recalculated"].as_u64().unwrap() >= 1);
    let (_, me) = t.call("GET", "/api/v1/levels/me", Some(&steve), None).await;
    assert_eq!(me["global_level"], 1);
    // And a cap.
    settings["level_base"] = json!(100.0);
    settings["max_level"] = json!(3);
    t.call("PUT", "/api/admin/progression", Some(&admin), Some(settings.clone())).await;
    let (_, me) = t.call("GET", "/api/v1/levels/me", Some(&steve), None).await;
    assert_eq!(me["global_level"], 3);
    assert_eq!(me["progress_pct"], 100.0);
}
