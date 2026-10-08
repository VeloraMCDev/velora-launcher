//! The chat layout edited in the admin panel reaches game servers with their sync.

mod common;
use common::*;

#[tokio::test]
async fn chat_layout_is_validated_saved_and_synced_to_servers() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    let (_, srv) = t.call("POST", "/api/admin/servers", Some(&admin), Some(json!({"name": "SMP", "instance_id": "smp"}))).await;
    let server = srv["token"].as_str().unwrap().to_string();

    let (_, d) = t.call("GET", "/api/admin/chat", Some(&admin), None).await;
    assert!(d["settings"]["format"].as_str().unwrap().contains("{prefix}"), "LuckPerms prefixes are in the default layout");
    assert!(d["settings"]["format"].as_str().unwrap().contains("{suffix}"));

    let mut s = d["settings"].clone();
    s["format"] = json!("{prefix}{name}{suffix} » {message}");
    assert_eq!(t.call("PUT", "/api/admin/chat", Some(&admin), Some(s.clone())).await.0, StatusCode::OK);

    for (bad, why) in [
        ("{name}: hi", "needs a message"),
        ("{message}", "needs a name"),
        ("{name} {nope} {message}", "typo"),
        ("{name} {message", "unclosed"),
        ("{name} 100% {message}", "percent breaks Bukkit formats"),
    ] {
        s["format"] = json!(bad);
        assert_eq!(t.call("PUT", "/api/admin/chat", Some(&admin), Some(s.clone())).await.0, StatusCode::BAD_REQUEST, "{why}");
    }

    let (_, sync) = t.call("POST", "/api/server/v1/sync", Some(&server), Some(json!({"online": []}))).await;
    assert_eq!(sync["chat"]["format"], "{prefix}{name}{suffix} » {message}");
    assert_eq!(sync["chat"]["enabled"], true);
}
