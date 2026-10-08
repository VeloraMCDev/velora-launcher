mod common;
use common::*;
use std::io::{Cursor, Read};

#[tokio::test]
async fn png_rank_titles_become_font_glyphs_in_the_pack_and_in_player_info() {
    let t = setup_with(|_, dir| {
        std::fs::create_dir_all(dir.join("uploads")).unwrap();
        std::fs::write(dir.join("uploads/veteran.png"), tiny_png()).unwrap();
        std::fs::write(dir.join("uploads/elder.png"), tiny_png()).unwrap();
    })
    .await;
    sqlx::query("DELETE FROM level_rewards WHERE reward_type='title'").execute(&t.db).await.unwrap();
    let admin = t.login("admin", "supersecret").await;
    let (_, server) = t.call("POST", "/api/admin/servers", Some(&admin), Some(json!({"name":"SMP","instance_id":"smp"}))).await;
    let key = server["token"].as_str().unwrap();
    t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username":"Steve","password":"password123"}))).await;
    let steve = t.uuid("Steve").await;
    sqlx::query("INSERT INTO user_levels(uuid, global_xp, global_level, updated_at) VALUES (?, 3200, 10, 'now') ON CONFLICT(uuid) DO UPDATE SET global_xp=3200")
        .bind(&steve).execute(&t.db).await.unwrap();

    let (_, vet) = t.call("POST", "/api/admin/levels/rewards", Some(&admin), Some(json!({"level":2,"reward_type":"title","reward_value":"Veteran","reward_data":{"title_image":"/uploads/veteran.png"}}))).await;
    let (_, elder) = t.call("POST", "/api/admin/levels/rewards", Some(&admin), Some(json!({"level":50,"reward_type":"title","reward_value":"Elder","reward_data":{"title_image":"/uploads/elder.png"}}))).await;
    // Remote images can't go into a pack, so no glyph for them.
    t.call("POST", "/api/admin/levels/rewards", Some(&admin), Some(json!({"level":5,"reward_type":"title","reward_value":"Remote","reward_data":{"title_image":"https://example.com/x.png"}}))).await;
    let (vet, elder) = (vet["id"].as_i64().unwrap(), elder["id"].as_i64().unwrap());

    // Level 10 → the highest title reached is Remote (level 5), which has no uploaded PNG.
    let (_, info) = t.call("POST", "/api/server/v1/player/info", Some(key), Some(json!({"uuid": steve}))).await;
    assert_eq!((info["global"]["title"].as_str(), info["global"]["title_glyph"].is_null()), (Some("Remote"), true));

    sqlx::query("DELETE FROM level_rewards WHERE reward_name='Remote'").execute(&t.db).await.unwrap();
    let (_, info) = t.call("POST", "/api/server/v1/player/info", Some(key), Some(json!({"uuid": steve}))).await;
    let glyph = info["global"]["title_glyph"].as_str().unwrap().to_string();
    assert_eq!(glyph.chars().count(), 1);
    assert!(('\u{F700}'..='\u{F8FF}').contains(&glyph.chars().next().unwrap()), "{glyph:?}");

    t.call("PUT", "/api/admin/resource-pack", Some(&admin), Some(json!({"enabled":true,"required":true,"pack_format":15}))).await;
    let (_, bytes) = t.fetch("/api/v1/resource-pack.zip").await;
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut font = String::new();
    zip.by_name("assets/minecraft/font/default.json").unwrap().read_to_string(&mut font).unwrap();
    let font: Value = serde_json::from_str(&font).unwrap();
    let providers = font["providers"].as_array().unwrap();
    assert_eq!(providers.len(), 2);
    let mine = providers.iter().find(|p| p["file"] == format!("scopenet:rank/{vet}.png")).unwrap();
    assert_eq!((mine["type"].as_str(), mine["chars"][0].as_str()), (Some("bitmap"), Some(glyph.as_str())));
    assert!(mine["height"].as_u64().unwrap() >= mine["ascent"].as_u64().unwrap());
    assert!(zip.by_name(&format!("assets/scopenet/textures/rank/{vet}.png")).is_ok());
    assert!(zip.by_name(&format!("assets/scopenet/textures/rank/{elder}.png")).is_ok());
    let other = providers.iter().find(|p| p["file"] == format!("scopenet:rank/{elder}.png")).unwrap();
    assert_ne!(other["chars"][0], mine["chars"][0], "every rank gets its own character");

    // Characters stay put when other rewards change.
    t.call("DELETE", &format!("/api/admin/levels/rewards/{vet}"), Some(&admin), None).await;
    let (_, bytes) = t.fetch("/api/v1/resource-pack.zip").await;
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut font = String::new();
    zip.by_name("assets/minecraft/font/default.json").unwrap().read_to_string(&mut font).unwrap();
    let font: Value = serde_json::from_str(&font).unwrap();
    assert_eq!(font["providers"][0]["chars"][0], other["chars"][0]);

    // The chat layout accepts the placeholder in both spellings.
    let (s, saved) = t.call("PUT", "/api/admin/chat", Some(&admin), Some(json!({"format":"%rank_title%{name}&7: &f{message}"}))).await;
    assert_eq!(s, StatusCode::OK, "{saved}");
    assert_eq!(saved["settings"]["format"], "{rank_title}{name}&7: &f{message}");
}
