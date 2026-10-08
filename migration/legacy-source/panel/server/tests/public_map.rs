mod common;
use common::*;

fn tile(zoom: u8, x: i32, y: i32) -> Vec<u8> {
    let png = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 1, 2, 3, 4];
    let mut v = vec![zoom];
    v.extend(x.to_be_bytes());
    v.extend(y.to_be_bytes());
    v.extend((png.len() as u32).to_be_bytes());
    v.extend(png);
    v
}

#[tokio::test]
async fn the_landing_page_decides_what_the_public_sees_of_the_map() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    let (_, srv) = t.call("POST", "/api/admin/servers", Some(&admin), Some(json!({"name": "SMP", "instance_id": "smp"}))).await;
    let key = srv["token"].as_str().unwrap().to_string();
    assert_eq!(t.call_raw("POST", "/api/server/v1/map/tiles?dim=minecraft:overworld", Some(&key), tile(0, 0, 0)).await.0, StatusCode::OK);
    t.call("POST", "/api/server/v1/map/players", Some(&key), Some(json!({"players": [{"uuid": "11111111-1111-1111-1111-111111111111", "name": "Steve", "dimension": "minecraft:overworld", "x": 1.0, "y": 64.0, "z": 2.0, "yaw": 0.0}]}))).await;
    t.call("POST", "/api/server/v1/map/overlay", Some(&key), Some(json!({"claims": [{"id": "c"}], "pins": [{"id": "p"}]}))).await;

    // The default landing page has a map section: guild land and places are public, player positions never.
    let (s, info) = t.call("GET", "/api/v1/public/map/0", None, None).await;
    assert_eq!(s, StatusCode::OK, "{info}");
    assert_eq!((info["layers"]["players"].as_bool(), info["layers"]["claims"].as_bool()), (Some(false), Some(true)));
    let (_, o) = t.call("GET", "/api/v1/public/map/0/overlay", None, None).await;
    assert_eq!(o["players"].as_array().unwrap().len(), 0, "{o}");
    // Take the section away and nothing is public, and it stays away (it is only ever added once).
    t.call("PUT", "/api/admin/landing", Some(&admin), Some(json!({"blocks": []}))).await;
    assert_eq!(t.call("GET", "/api/v1/public/map/0", None, None).await.0, StatusCode::NOT_FOUND);
    let (_, landing) = t.call("GET", "/api/v1/landing", None, None).await;
    assert!(landing["blocks"].as_array().unwrap().is_empty(), "{landing}");

    let landing = |opts: Value| json!({"blocks": [{"id": "m1", "type": "map", "enabled": true, "options": opts}]});
    assert_eq!(t.call("PUT", "/api/admin/landing", Some(&admin), Some(landing(json!({"show_players": false})))).await.0, StatusCode::OK);
    let (s, info) = t.call("GET", "/api/v1/public/map/0", None, None).await;
    assert_eq!(s, StatusCode::OK, "{info}");
    assert_eq!(info["ready"], true);
    assert_eq!(info["layers"]["players"], false);
    assert_eq!(info["layers"]["claims"], true);
    // The tile key from the public info loads tiles without signing in.
    let token = info["token"].as_str().unwrap();
    let (s, png) = t.fetch(&format!("/api/map/{}/overworld/0/0/0.png?t={token}", info["server_id"])).await;
    assert_eq!((s, png.len()), (StatusCode::OK, 12));

    // Players stay hidden unless the admin turns them on.
    let (_, o) = t.call("GET", "/api/v1/public/map/0/overlay", None, None).await;
    assert_eq!(o["players"].as_array().unwrap().len(), 0);
    assert_eq!(o["claims"].as_array().unwrap().len(), 1);
    t.call("PUT", "/api/admin/landing", Some(&admin), Some(landing(json!({"show_players": true, "show_pins": false})))).await;
    let (_, o) = t.call("GET", "/api/v1/public/map/0/overlay", None, None).await;
    assert_eq!(o["players"][0]["name"], "Steve");
    assert_eq!(o["pins"].as_array().unwrap().len(), 0);

    // Another server's map isn't exposed, and turning the section off closes it.
    assert_eq!(t.call("GET", "/api/v1/public/map/999", None, None).await.0, StatusCode::NOT_FOUND);
    t.call(
        "PUT",
        "/api/admin/landing",
        Some(&admin),
        Some(json!({"blocks": [{"id": "m1", "type": "map", "enabled": false, "options": {}}]})),
    )
    .await;
    assert_eq!(t.call("GET", "/api/v1/public/map/0", None, None).await.0, StatusCode::NOT_FOUND);
}
