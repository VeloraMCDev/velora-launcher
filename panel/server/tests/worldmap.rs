//! The Velora Map: tiles in from a game server, tiles and overlay out to signed-in viewers.

mod common;
use common::*;

const PNG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];

fn record(zoom: u8, x: i32, y: i32, payload: &[u8]) -> Vec<u8> {
    let mut out = vec![zoom];
    out.extend(x.to_be_bytes());
    out.extend(y.to_be_bytes());
    let mut png = PNG.to_vec();
    png.extend_from_slice(payload);
    out.extend((png.len() as u32).to_be_bytes());
    out.extend(png);
    out
}

struct World {
    t: TestApp,
    admin: String,
    alex: String,
    server: String,
    sid: i64,
}

async fn world() -> World {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username": "Alex", "password": "password123"}))).await;
    let alex = t.login("Alex", "password123").await;
    let (_, srv) = t.call("POST", "/api/admin/servers", Some(&admin), Some(json!({"name": "SMP", "instance_id": "smp"}))).await;
    World { t, admin, alex, server: srv["token"].as_str().unwrap().to_string(), sid: srv["server"]["id"].as_i64().unwrap() }
}

impl World {
    async fn upload(&self, dim: &str, body: Vec<u8>) -> (StatusCode, Value) {
        self.t.call_raw("POST", &format!("/api/server/v1/map/tiles?dim={dim}"), Some(&self.server), body).await
    }
    async fn tile(&self, token: &str, dim: &str, z: u8, x: i32, y: i32) -> (StatusCode, Vec<u8>) {
        self.t.fetch(&format!("/api/map/{}/{dim}/{z}/{x}/{y}.png?t={token}", self.sid)).await
    }
}

#[tokio::test]
async fn tiles_flow_from_the_game_server_to_signed_in_viewers() {
    let w = world().await;
    let (s, cfg) = w.t.call("GET", "/api/server/v1/map/config", Some(&w.server), None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!((cfg["enabled"].as_bool(), cfg["epoch"].as_u64(), cfg["tile_size"].as_u64()), (Some(true), Some(0), Some(256)));

    // Nothing drawn yet.
    let (_, info) = w.t.call("GET", &format!("/api/v1/servers/{}/map", w.sid), Some(&w.alex), None).await;
    assert_eq!((info["ready"].as_bool(), info["token"].is_null()), (Some(false), true));
    assert!(info["message"].as_str().unwrap().contains("Waiting"));

    let body = [record(0, -1, 2, b"aa"), record(0, 0, 0, b"bb"), record(1, 0, 0, b"cc")].concat();
    let (s, r) = w.upload("minecraft:overworld", body).await;
    assert_eq!((s, r["stored"].as_u64()), (StatusCode::OK, Some(3)));
    assert_eq!(w.upload("minecraft:the_nether", record(0, 5, 5, b"n")).await.0, StatusCode::OK);

    let (_, info) = w.t.call("GET", &format!("/api/v1/servers/{}/map", w.sid), Some(&w.alex), None).await;
    assert_eq!(info["ready"], true);
    let dims = info["dimensions"].as_array().unwrap();
    assert_eq!((dims[0]["slug"].as_str(), dims[1]["label"].as_str()), (Some("overworld"), Some("The Nether")));
    assert_eq!(dims[0]["bounds"]["min_y"], 0);
    assert_eq!(dims[0]["bounds"]["max_x"], 0);
    let token = info["token"].as_str().unwrap().to_string();
    assert_eq!(info["tile_base"], format!("/api/map/{}", w.sid));

    // The token opens tiles for plain <img> tags.
    let (s, bytes) = w.tile(&token, "overworld", 0, -1, 2).await;
    assert_eq!(s, StatusCode::OK);
    assert!(bytes.ends_with(b"aa") && bytes.starts_with(&PNG));
    assert_eq!(w.tile(&token, "overworld", 0, 7, 7).await.0, StatusCode::NOT_FOUND);
    assert_eq!(w.tile(&token, "overworld", 9, 0, 0).await.0, StatusCode::NOT_FOUND);
    assert_eq!(w.tile("wrong", "overworld", 0, -1, 2).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(w.tile(&token, "..%2Fetc", 0, 0, 0).await.0, StatusCode::NOT_FOUND);
    // Tokens belong to one server.
    let (_, other) = w.t.call("POST", "/api/admin/servers", Some(&w.admin), Some(json!({"name": "Other", "instance_id": "o"}))).await;
    let other_id = other["server"]["id"].as_i64().unwrap();
    assert_eq!(w.t.fetch(&format!("/api/map/{other_id}/overworld/0/0/0.png?t={token}")).await.0, StatusCode::UNAUTHORIZED);
    // Info needs an account.
    assert_eq!(w.t.call("GET", &format!("/api/v1/servers/{}/map", w.sid), None, None).await.0, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn bad_uploads_are_refused() {
    let w = world().await;
    assert_eq!(w.upload("minecraft:overworld", b"not tiles at all".to_vec()).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(w.upload("../x", record(0, 0, 0, b"a")).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(w.upload("minecraft:overworld", record(12, 0, 0, b"a")).await.0, StatusCode::BAD_REQUEST);
    let (s, _) = w.t.call_raw("POST", "/api/server/v1/map/tiles?dim=minecraft:overworld", None, record(0, 0, 0, b"a")).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn players_and_overlay_are_served_to_viewers() {
    let w = world().await;
    let (s, _) = w.t.call("POST", "/api/server/v1/map/players", Some(&w.server), Some(json!({"players": [
        {"uuid": "11111111-2222-3333-4444-555555555555", "name": "Steve", "dimension": "minecraft:overworld", "x": 10.5, "y": 70.0, "z": -3.0, "yaw": 90.0}]}))).await;
    assert_eq!(s, StatusCode::OK);
    let overlay = json!({"claims": [{"guild_id": "g1", "name": "Iron", "tag": "IRON", "color": 123}], "pins": [{"id": "spawn-0", "kind": "spawn", "label": "Spawn"}]});
    assert_eq!(w.t.call("POST", "/api/server/v1/map/overlay", Some(&w.server), Some(overlay)).await.0, StatusCode::OK);
    assert_eq!(
        w.t.call("POST", "/api/server/v1/map/overlay", Some(&w.server), Some(json!({"claims": "nope"}))).await.0,
        StatusCode::BAD_REQUEST
    );

    let (s, o) = w.t.call("GET", &format!("/api/v1/servers/{}/map/overlay", w.sid), Some(&w.alex), None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(o["players"][0]["name"], "Steve");
    assert_eq!(o["claims"][0]["tag"], "IRON");
    assert_eq!(o["pins"][0]["kind"], "spawn");
    assert!(o["updated"].is_string());
    let (_, info) = w.t.call("GET", &format!("/api/v1/servers/{}/map", w.sid), Some(&w.alex), None).await;
    assert_eq!(info["players"], 1);
}

#[tokio::test]
async fn map_diagnostics_are_admin_only_and_reset_with_tiles() {
    let w = world().await;
    let payload = json!({"stage":"waiting for readable chunks","error":"No readable full chunks","region_files":1});
    assert_eq!(w.t.call("POST", "/api/server/v1/map/diagnostics", Some(&w.server), Some(payload)).await.0, StatusCode::OK);
    assert_eq!(w.t.call("POST", "/api/server/v1/map/diagnostics", None, Some(json!({}))).await.0, StatusCode::UNAUTHORIZED);
    let (_, status) = w.t.call("GET", &format!("/api/admin/servers/{}/map", w.sid), Some(&w.admin), None).await;
    assert_eq!(status["diagnostics"]["stage"], "waiting for readable chunks");
    assert!(status["diagnostics"]["age_seconds"].is_number());
    w.t.call("DELETE", &format!("/api/admin/servers/{}/map", w.sid), Some(&w.admin), None).await;
    let (_, status) = w.t.call("GET", &format!("/api/admin/servers/{}/map", w.sid), Some(&w.admin), None).await;
    assert!(status["diagnostics"].is_null());
}

#[tokio::test]
async fn the_map_can_be_switched_off_and_reset() {
    let w = world().await;
    w.upload("minecraft:overworld", record(0, 0, 0, b"a")).await;
    let (s, r) = w.t.call("DELETE", &format!("/api/admin/servers/{}/map", w.sid), Some(&w.admin), None).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    let (_, cfg) = w.t.call("GET", "/api/server/v1/map/config", Some(&w.server), None).await;
    assert_eq!(cfg["epoch"], 1, "game servers see a new epoch and redraw");
    let (_, info) = w.t.call("GET", &format!("/api/v1/servers/{}/map", w.sid), Some(&w.alex), None).await;
    assert_eq!(info["ready"], false);
    assert_eq!(
        w.t.call("GET", &format!("/api/admin/servers/{}/map", w.sid), Some(&w.alex), None).await.0,
        StatusCode::FORBIDDEN,
        "admin only"
    );

    let (s, _) =
        w.t.call(
            "PUT",
            &format!("/api/admin/servers/{}", w.sid),
            Some(&w.admin),
            Some(json!({"name": "SMP", "instance_id": "smp", "map_enabled": false})),
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(w.upload("minecraft:overworld", record(0, 0, 0, b"a")).await.0, StatusCode::FORBIDDEN);
    let (_, cfg) = w.t.call("GET", "/api/server/v1/map/config", Some(&w.server), None).await;
    assert_eq!(cfg["enabled"], false);
    assert_eq!(w.t.call("GET", &format!("/api/v1/servers/{}/map/overlay", w.sid), Some(&w.alex), None).await.0, StatusCode::FORBIDDEN);
}
