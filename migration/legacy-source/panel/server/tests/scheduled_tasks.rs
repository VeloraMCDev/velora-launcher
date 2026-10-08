//! The scheduled-tasks page and the purge of players whose account was removed long ago.

mod common;
use common::*;

#[tokio::test]
async fn purge_task_cleans_leftovers_of_deleted_accounts_and_keeps_everyone_else() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username": "Ghost", "password": "password123"}))).await;
    t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username": "Alive", "password": "password123"}))).await;
    let (ghost, alive) = (t.uuid("Ghost").await, t.uuid("Alive").await);
    let (_, srv) = t.call("POST", "/api/admin/servers", Some(&admin), Some(json!({"name": "SMP", "instance_id": "smp"}))).await;
    let sid = srv["server"]["id"].as_i64().unwrap();
    for (u, n) in [(&ghost, "Ghost"), (&alive, "Alive")] {
        sqlx::query("INSERT INTO player_stats (server_id, uuid, name, first_seen, last_seen) VALUES (?, ?, ?, '2026-01-01', '2026-01-02')")
            .bind(sid).bind(u).bind(n).execute(&t.db).await.unwrap();
        sqlx::query("INSERT INTO user_levels (uuid, global_xp, global_level, updated_at) VALUES (?, 500, 3, '2026-01-01')")
            .bind(u).execute(&t.db).await.unwrap();
    }
    // The account vanishes the old way: the row is gone, nothing else was cleaned up.
    sqlx::query("DELETE FROM users WHERE uuid = ?").bind(&ghost).execute(&t.db).await.unwrap();

    let (code, list) = t.call("GET", "/api/admin/tasks", Some(&admin), None).await;
    assert_eq!(code, StatusCode::OK);
    assert!(list["tasks"].as_array().unwrap().iter().any(|x| x["id"] == "purge_deleted_players"));

    // Dry run reports but deletes nothing.
    t.call("PUT", "/api/admin/tasks/purge_deleted_players", Some(&admin), Some(json!({"settings": {"dry_run": true}}))).await;
    let (_, r) = t.call("POST", "/api/admin/tasks/purge_deleted_players/run", Some(&admin), None).await;
    assert!(r["message"].as_str().unwrap().contains("would clean 1"), "{r}");
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM player_stats WHERE uuid = ?").bind(&ghost).fetch_one(&t.db).await.unwrap();
    assert_eq!(n, 1);

    t.call("PUT", "/api/admin/tasks/purge_deleted_players", Some(&admin), Some(json!({"settings": {"dry_run": false}}))).await;
    let (_, r) = t.call("POST", "/api/admin/tasks/purge_deleted_players/run", Some(&admin), None).await;
    assert!(r["message"].as_str().unwrap().contains("cleaned 1"), "{r}");
    for table in ["player_stats", "user_levels"] {
        let gone: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table} WHERE uuid = ?")).bind(&ghost).fetch_one(&t.db).await.unwrap();
        let kept: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table} WHERE uuid = ?")).bind(&alive).fetch_one(&t.db).await.unwrap();
        assert_eq!((gone, kept), (0, 1), "{table}");
    }

    let (_, list) = t.call("GET", "/api/admin/tasks", Some(&admin), None).await;
    let task = list["tasks"].as_array().unwrap().iter().find(|x| x["id"] == "purge_deleted_players").unwrap();
    assert_eq!(task["runs"], 2);
    assert_eq!(task["history"].as_array().unwrap().len(), 2);

    // Built-in loops can't be switched off from here, and bad intervals are refused.
    assert_eq!(t.call("PUT", "/api/admin/tasks/reward_queue", Some(&admin), Some(json!({"enabled": false}))).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(t.call("PUT", "/api/admin/tasks/refresh_titles", Some(&admin), Some(json!({"interval_secs": 1}))).await.0, StatusCode::BAD_REQUEST);
    let alice = t.login("Alive", "password123").await;
    assert_eq!(t.call("GET", "/api/admin/tasks", Some(&alice), None).await.0, StatusCode::FORBIDDEN);
}
