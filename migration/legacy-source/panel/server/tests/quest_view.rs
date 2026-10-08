//! Looking at a player's daily and weekly quests from the game (and, through the same code, from Discord).

mod common;
use common::*;

#[tokio::test]
async fn anyone_can_view_a_players_quests_by_name_or_uuid() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username": "Alex", "password": "password123"}))).await;
    let alex_token = t.login("Alex", "password123").await;
    let alex = t.uuid("Alex").await;
    let (_, srv) = t.call("POST", "/api/admin/servers", Some(&admin), Some(json!({"name": "SMP", "instance_id": "smp"}))).await;
    let server = srv["token"].as_str().unwrap().to_string();

    let (s, q) = t.call("POST", "/api/server/v1/quests/player", Some(&server), Some(json!({"player": "alex"}))).await;
    assert_eq!(s, StatusCode::OK, "{q}");
    assert_eq!(q["player"], "Alex");
    assert!(!q["daily"].as_array().unwrap().is_empty(), "daily quests are listed");
    assert!(!q["weekly"].as_array().unwrap().is_empty(), "weekly quests are listed");
    let first = &q["daily"][0];
    assert_eq!(first["progress"], 0);
    assert!(first["target"].as_i64().unwrap() > 0);
    assert!(q["daily_resets"].as_str().unwrap().contains('T'));

    // It matches what the launcher shows that player.
    let (_, mine) = t.call("GET", "/api/v1/quests/my", Some(&alex_token), None).await;
    let launcher_daily: Vec<&str> = mine.as_array().unwrap().iter().filter(|x| x["quest"]["period"] != "weekly").map(|x| x["quest"]["id"].as_str().unwrap()).collect();
    let game_daily: Vec<&str> = q["daily"].as_array().unwrap().iter().map(|x| x["id"].as_str().unwrap()).collect();
    assert_eq!(launcher_daily, game_daily);

    // By UUID, and an unknown name is a clean error.
    let (s, _) = t.call("POST", "/api/server/v1/quests/player", Some(&server), Some(json!({"uuid": alex}))).await;
    assert_eq!(s, StatusCode::OK);
    let (s, e) = t.call("POST", "/api/server/v1/quests/player", Some(&server), Some(json!({"player": "Nobody"}))).await;
    assert_eq!(s, StatusCode::NOT_FOUND, "{e}");
}
