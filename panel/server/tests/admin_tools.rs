//! The admin tools added in the platform-experience work: companion layouts, email templates, reward queue controls and
//! cosmetic templates (including their auto-grants).

mod common;
use common::*;

#[tokio::test]
async fn companion_layouts_are_managed() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    let base = "/api/admin/companion/layouts";

    let (s, a) = t.call("POST", base, Some(&admin), Some(json!({"name": "Survival", "widgets": [{"id": 1, "type": "health"}]}))).await;
    assert_eq!(s, StatusCode::OK, "{a}");
    assert_eq!(a["is_default"], true, "the first layout becomes the default");
    assert_eq!(a["widgets"][0]["type"], "health");
    let (_, b) = t.call("POST", base, Some(&admin), Some(json!({"name": "PvP"}))).await;
    assert_eq!(b["is_default"], false);
    assert_eq!(t.call("POST", base, Some(&admin), Some(json!({"name": "  "}))).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(t.call("POST", base, Some(&admin), Some(json!({"name": "x", "widgets": "nope"}))).await.0, StatusCode::BAD_REQUEST);

    let (id_a, id_b) = (a["id"].as_i64().unwrap(), b["id"].as_i64().unwrap());
    let (s, u) = t.call("PUT", &format!("{base}/{id_b}"), Some(&admin), Some(json!({"id": id_b, "name": "Arena", "widgets": [], "is_default": true}))).await;
    assert_eq!((s, u["name"].as_str()), (StatusCode::OK, Some("Arena")));
    assert_eq!(u["is_default"], false, "saving never changes the default");

    assert_eq!(t.call("POST", &format!("{base}/{id_b}/set-default"), Some(&admin), Some(json!({}))).await.0, StatusCode::OK);
    let (_, list) = t.call("GET", base, Some(&admin), None).await;
    assert_eq!(list["layouts"][0]["name"], "Arena");
    assert_eq!(list["layouts"].as_array().unwrap().iter().filter(|l| l["is_default"] == true).count(), 1);

    // Deleting the default hands it to the next layout; unknown ids are 404.
    assert_eq!(t.call("DELETE", &format!("{base}/{id_b}"), Some(&admin), None).await.0, StatusCode::OK);
    let (_, g) = t.call("GET", &format!("{base}/{id_a}"), Some(&admin), None).await;
    assert_eq!(g["layout"]["is_default"], true);
    assert_eq!(t.call("GET", &format!("{base}/{id_b}"), Some(&admin), None).await.0, StatusCode::NOT_FOUND);
    assert_eq!(t.call("POST", &format!("{base}/999/set-default"), Some(&admin), Some(json!({}))).await.0, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn email_templates_are_managed() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    let base = "/api/admin/email-templates";

    let (_, list) = t.call("GET", base, Some(&admin), None).await;
    assert!(!list["templates"].as_array().unwrap().is_empty(), "the built-in templates are listed");

    let (s, c) = t.call("POST", base, Some(&admin), Some(json!({"id": "promo", "name": "Promo", "subject": "Hi {player}", "body": "Hello"}))).await;
    assert_eq!((s, c["id"].as_str()), (StatusCode::OK, Some("promo")), "{c}");
    assert_eq!(t.call("POST", base, Some(&admin), Some(json!({"id": "promo", "name": "x", "subject": "x", "body": "x"}))).await.0, StatusCode::CONFLICT);
    assert_eq!(t.call("POST", base, Some(&admin), Some(json!({"id": "bad id!", "name": "x", "subject": "x", "body": "x"}))).await.0, StatusCode::BAD_REQUEST);

    let (s, _) = t.call("PUT", &format!("{base}/promo"), Some(&admin), Some(json!({"id": "promo", "name": "Promo 2", "subject": "S", "body": "B"}))).await;
    assert_eq!(s, StatusCode::OK);
    let (_, g) = t.call("GET", &format!("{base}/promo"), Some(&admin), None).await;
    assert_eq!(g["template"]["name"], "Promo 2");

    assert_eq!(t.call("DELETE", &format!("{base}/promo"), Some(&admin), None).await.0, StatusCode::OK);
    assert_eq!(t.call("DELETE", &format!("{base}/promo"), Some(&admin), None).await.0, StatusCode::NOT_FOUND);
    // Without a mail server the test send reports why instead of pretending it worked.
    let (s, _) = t.call("POST", &format!("{base}/test"), Some(&admin), Some(json!({"template_id": "welcome", "recipient": "nope"}))).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn reward_queue_can_retry_delete_and_clear() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    for (n, delivered, error, attempts) in [(1, None, Some("boom"), 8), (2, Some("2026-01-01T00:00:00Z"), None, 1)] {
        sqlx::query("INSERT INTO reward_deliveries (uuid, kind, payload, source, created_at, delivered_at, attempts, error, sent_at) VALUES (?, 'money', '{}', 'test', '2026-01-01T00:00:00Z', ?, ?, ?, '2026-01-01T00:00:00Z')")
            .bind(format!("00000000-0000-0000-0000-00000000000{n}"))
            .bind(delivered)
            .bind(attempts)
            .bind(error)
            .execute(&t.db)
            .await
            .unwrap();
    }
    let (_, log) = t.call("GET", "/api/admin/reward-deliveries", Some(&admin), None).await;
    let failed = log.as_array().unwrap().iter().find(|d| d["error"] == "boom").unwrap()["id"].as_i64().unwrap();

    assert_eq!(t.call("POST", &format!("/api/admin/reward-deliveries/{failed}/retry"), Some(&admin), Some(json!({}))).await.0, StatusCode::OK);
    let (attempts, error, sent): (i64, Option<String>, Option<String>) =
        sqlx::query_as("SELECT attempts, error, sent_at FROM reward_deliveries WHERE id = ?").bind(failed).fetch_one(&t.db).await.unwrap();
    assert_eq!((attempts, error, sent), (0, None, None), "a retried delivery is eligible for the next poll");

    let (s, c) = t.call("POST", "/api/admin/reward-deliveries/clear-delivered", Some(&admin), Some(json!({}))).await;
    assert_eq!((s, c["deleted"].as_i64()), (StatusCode::OK, Some(1)));
    assert_eq!(t.call("DELETE", &format!("/api/admin/reward-deliveries/{failed}"), Some(&admin), None).await.0, StatusCode::OK);
    let (_, log) = t.call("GET", "/api/admin/reward-deliveries", Some(&admin), None).await;
    assert!(log.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn cosmetic_templates_validate_and_auto_grant() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username": "Alex", "password": "password123"}))).await;
    let uuid: String = sqlx::query_scalar("SELECT uuid FROM users WHERE username = 'Alex'").fetch_one(&t.db).await.unwrap();
    let base = "/api/admin/cosmetics/templates";
    let new = |key: &str, level: Option<i64>| json!({"key": key, "type": "title", "label": "Veteran", "description": "", "metadata": {"prefix": "Vet"}, "auto_grant": level.is_some(), "grant_on_level": level});

    assert_eq!(t.call("POST", base, Some(&admin), Some(new("Bad Key", None))).await.0, StatusCode::BAD_REQUEST);
    let mut wrong_type = new("t1", None);
    wrong_type["type"] = json!("sword");
    assert_eq!(t.call("POST", base, Some(&admin), Some(wrong_type)).await.0, StatusCode::BAD_REQUEST);
    let (s, made) = t.call("POST", base, Some(&admin), Some(new("vet.title", None))).await;
    assert_eq!(s, StatusCode::OK, "{made}");
    assert_eq!(t.call("POST", base, Some(&admin), Some(new("vet.title", None))).await.0, StatusCode::CONFLICT);

    // Editing works (PUT) and the list is a plain array.
    let id = made["id"].as_i64().unwrap();
    let (s, _) = t.call("PUT", &format!("{base}/{id}"), Some(&admin), Some(json!({"label": "Elder", "description": "x", "metadata": {}, "auto_grant": false}))).await;
    assert_eq!(s, StatusCode::OK);
    let (_, list) = t.call("GET", base, Some(&admin), None).await;
    assert_eq!(list[0]["label"], "Elder");
    assert_eq!(t.call("GET", &format!("{base}/999"), Some(&admin), None).await.0, StatusCode::NOT_FOUND);

    // Auto-grant: existing players who already qualify get it on save, later level-ups get it from the trigger.
    sqlx::query("INSERT INTO user_levels (uuid, global_level, updated_at) VALUES (?, 7, '2026-01-01T00:00:00Z') ON CONFLICT(uuid) DO UPDATE SET global_level = 7").bind(&uuid).execute(&t.db).await.unwrap();
    assert_eq!(t.call("POST", base, Some(&admin), Some(new("lvl5", Some(5)))).await.0, StatusCode::OK);
    assert_eq!(t.call("POST", base, Some(&admin), Some(new("lvl10", Some(10)))).await.0, StatusCode::OK);
    let has = |key: &'static str| {
        let (db, uuid) = (t.db.clone(), uuid.clone());
        async move { sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM player_unlocks WHERE uuid = ? AND unlock_key = ?)").bind(uuid).bind(key).fetch_one(&db).await.unwrap() }
    };
    assert!(has("lvl5").await && !has("lvl10").await);
    sqlx::query("UPDATE user_levels SET global_level = 10 WHERE uuid = ?").bind(&uuid).execute(&t.db).await.unwrap();
    assert!(has("lvl10").await, "reaching the level unlocks it");

    assert_eq!(t.call("DELETE", &format!("{base}/{id}"), Some(&admin), None).await.0, StatusCode::OK);
}

#[tokio::test]
async fn modelled_cosmetics_get_a_model_number_ride_in_the_pack_and_reach_the_game_server() {
    use std::io::Read;
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username": "Alex", "password": "password123"}))).await;
    let alex = t.login("Alex", "password123").await;
    let uuid: String = sqlx::query_scalar("SELECT uuid FROM users WHERE username = 'Alex'").fetch_one(&t.db).await.unwrap();
    let (_, srv) = t.call("POST", "/api/admin/servers", Some(&admin), Some(json!({"name": "SMP", "instance_id": "smp"}))).await;
    let key = srv["token"].as_str().unwrap().to_string();
    let base = "/api/admin/cosmetics/templates";
    let wings = |key: &str| json!({"key": key, "type": "cosmetic", "label": key, "description": "", "metadata": {"slot": "back", "scale": 1.5, "texture": "scopenet:item/wings"}});

    let (s, a) = t.call("POST", base, Some(&admin), Some(wings("wings_a"))).await;
    assert_eq!(s, StatusCode::OK, "{a}");
    let (_, b) = t.call("POST", base, Some(&admin), Some(wings("wings_b"))).await;
    let (na, nb) = (a["metadata"]["look"]["custom_model_data"].as_i64().unwrap(), b["metadata"]["look"]["custom_model_data"].as_i64().unwrap());
    assert_ne!(na, nb, "every modelled cosmetic gets its own number");
    assert_eq!(a["metadata"]["look"]["item"], "minecraft:stick");

    // Editing keeps the number; clearing the model drops the look.
    let id = a["id"].as_i64().unwrap();
    let (_, kept) = t.call("PUT", &format!("{base}/{id}"), Some(&admin), Some(json!({"label": "A", "description": "", "metadata": a["metadata"].clone()}))).await;
    assert!(kept["ok"] == true);
    let (_, list) = t.call("GET", base, Some(&admin), None).await;
    let stored = list.as_array().unwrap().iter().find(|c| c["key"] == "wings_a").unwrap();
    assert_eq!(stored["metadata"]["look"]["custom_model_data"], na);

    // The resource pack carries both models.
    t.call("PUT", "/api/admin/resource-pack", Some(&admin), Some(json!({"enabled": true, "required": false, "pack_format": 15}))).await;
    let (_, bytes) = t.fetch("/api/v1/resource-pack.zip").await;
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut text = String::new();
    zip.by_name("assets/minecraft/models/item/stick.json").unwrap().read_to_string(&mut text).unwrap();
    let overrides = serde_json::from_str::<Value>(&text).unwrap()["overrides"].clone();
    assert_eq!(overrides.as_array().unwrap().len(), 2, "{overrides}");
    assert!(zip.by_name(&format!("assets/scopenet/models/item/cosmetic_{id}.json")).is_ok());

    // An equipped cosmetic reaches the game server with its look; later template edits show without a re-grant.
    t.call("POST", "/api/admin/collections/grant", Some(&admin), Some(json!({"uuid": uuid, "unlock_key": "wings_a", "unlock_type": "cosmetic", "metadata": {}}))).await;
    assert_eq!(t.call("POST", "/api/v1/collections/equip", Some(&alex), Some(json!({"unlock_key": "wings_a", "equipped": true}))).await.0, StatusCode::OK);
    let (s, info) = t.call("POST", "/api/server/v1/player/info", Some(&key), Some(json!({"uuid": uuid}))).await;
    assert_eq!(s, StatusCode::OK, "{info}");
    let worn = &info["cosmetics"]["equipped"][0];
    assert_eq!((worn["key"].as_str(), worn["metadata"]["look"]["custom_model_data"].as_i64(), worn["metadata"]["slot"].as_str()), (Some("wings_a"), Some(na), Some("back")));
}

#[tokio::test]
async fn game_servers_list_and_equip_a_players_cosmetics() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username": "Alex", "password": "password123"}))).await;
    let uuid: String = sqlx::query_scalar("SELECT uuid FROM users WHERE username = 'Alex'").fetch_one(&t.db).await.unwrap();
    let (_, srv) = t.call("POST", "/api/admin/servers", Some(&admin), Some(json!({"name": "SMP", "instance_id": "smp"}))).await;
    let key = srv["token"].as_str().unwrap().to_string();
    for (k, kind, label) in [("halo", "cosmetic", "Golden Halo"), ("crown", "cosmetic", "Crown"), ("vet", "title", "Veteran")] {
        t.call("POST", "/api/admin/cosmetics/templates", Some(&admin), Some(json!({"key": k, "type": kind, "label": label, "description": "", "metadata": {}}))).await;
        t.call("POST", "/api/admin/collections/grant", Some(&admin), Some(json!({"uuid": uuid, "unlock_key": k, "unlock_type": kind, "metadata": {}}))).await;
    }
    let call = |path: &str, body: Value| {
        let (t, key, path) = (&t, key.clone(), path.to_string());
        async move { t.call("POST", &format!("/api/server/v1/{path}"), Some(&key), Some(body)).await }
    };
    let (s, list) = call("cosmetics/list", json!({"uuid": uuid})).await;
    assert_eq!(s, StatusCode::OK, "{list}");
    assert_eq!(list["cosmetics"].as_array().unwrap().len(), 3);
    assert!(list["cosmetics"].as_array().unwrap().iter().any(|c| c["label"] == "Golden Halo" && c["equipped"] == false));

    // Equipping one cosmetic puts away the other; a title is separate.
    assert_eq!(call("cosmetics/equip", json!({"uuid": uuid, "key": "halo", "equipped": true})).await.0, StatusCode::OK);
    assert_eq!(call("cosmetics/equip", json!({"uuid": uuid, "key": "crown", "equipped": true})).await.0, StatusCode::OK);
    assert_eq!(call("cosmetics/equip", json!({"uuid": uuid, "key": "vet", "equipped": true})).await.0, StatusCode::OK);
    let (_, list) = call("cosmetics/list", json!({"uuid": uuid})).await;
    let worn: Vec<&str> = list["cosmetics"].as_array().unwrap().iter().filter(|c| c["equipped"] == true).map(|c| c["key"].as_str().unwrap()).collect();
    assert_eq!(worn.len(), 2);
    assert!(worn.contains(&"crown") && worn.contains(&"vet"), "{worn:?}");
    assert_eq!(call("cosmetics/equip", json!({"uuid": uuid, "key": "crown", "equipped": false})).await.0, StatusCode::OK);
    assert_eq!(call("cosmetics/equip", json!({"uuid": uuid, "key": "nope", "equipped": true})).await.0, StatusCode::NOT_FOUND);
    assert_eq!(call("cosmetics/list", json!({"uuid": "bad"})).await.0, StatusCode::BAD_REQUEST);
}
