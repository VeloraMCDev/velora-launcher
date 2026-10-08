//! Discord slash commands: signed requests only, a handful of useful answers, and one-click registration.

mod common;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, Request};
use axum::routing::put;
use axum::Json;
use common::*;
use ring::rand::SystemRandom;
use ring::signature::{Ed25519KeyPair, KeyPair};
use std::sync::{Arc, Mutex};

type Seen = Arc<Mutex<Vec<(String, String, Value)>>>;

async fn fake_discord() -> (String, Seen) {
    let seen: Seen = Default::default();
    let app = axum::Router::new()
        .route(
            "/applications/{app}/guilds/{guild}/commands",
            put(|State(seen): State<Seen>, Path((app, guild)): Path<(String, String)>, h: HeaderMap, Json(body): Json<Value>| async move {
                let auth = h.get("authorization").and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
                seen.lock().unwrap().push((format!("PUT /applications/{app}/guilds/{guild}/commands"), auth, body.clone()));
                Json(body)
            }),
        )
        .with_state(seen.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (url, seen)
}

struct Signer(Ed25519KeyPair);

impl Signer {
    fn new() -> Self {
        let pkcs8 = Ed25519KeyPair::generate_pkcs8(&SystemRandom::new()).unwrap();
        Signer(Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).unwrap())
    }
    fn public_hex(&self) -> String {
        hex::encode(self.0.public_key().as_ref())
    }
    fn request(&self, body: &Value, tamper: bool) -> Request<Body> {
        let bytes = body.to_string();
        let ts = "1700000000";
        let sig = self.0.sign(format!("{ts}{bytes}").as_bytes());
        let body = if tamper { bytes.replace("status", "xtatus") } else { bytes };
        Request::builder()
            .method("POST")
            .uri("/api/v1/discord/interactions")
            .header("content-type", "application/json")
            .header("x-signature-ed25519", hex::encode(sig.as_ref()))
            .header("x-signature-timestamp", ts)
            .body(Body::from(body))
            .unwrap()
    }
}

fn command(name: &str, options: Value) -> Value {
    json!({"type": 2, "data": {"name": name, "options": options}})
}

#[tokio::test]
async fn slash_commands_answer_only_signed_requests_and_register_in_one_click() {
    let (discord, seen) = fake_discord().await;
    std::env::set_var("SCOPENET_DISCORD_API", &discord);
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    for n in ["Alex", "Steve"] {
        t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username": n, "password": "password123"}))).await;
    }
    let alex = t.uuid("Alex").await;
    let (_, srv) = t.call("POST", "/api/admin/servers", Some(&admin), Some(json!({"name": "SMP", "instance_id": "smp"}))).await;
    let sid = srv["server"]["id"].as_i64().unwrap();
    sqlx::query("INSERT INTO player_stats (server_id, uuid, name, playtime_secs, player_kills, first_seen, last_seen) VALUES (?, ?, 'Alex', 7300, 4, '2026-01-01', '2026-01-02')")
        .bind(sid).bind(&alex).execute(&t.db).await.unwrap();
    let signer = Signer::new();

    // No key saved yet: nothing is trusted.
    let (s, _) = t.send(signer.request(&json!({"type": 1}), false)).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);

    let settings = |key: Value| json!({"role_sync": "off", "invite_url": "", "channel_id": "", "notify_achievements": true, "notify_guilds": true, "notify_members": false, "public_key": key});
    assert_eq!(t.call("PUT", "/api/admin/discord", Some(&admin), Some(settings(json!("nope")))).await.0, StatusCode::BAD_REQUEST, "keys are checked");
    let (s, v) = t.call("PUT", "/api/admin/discord", Some(&admin), Some(settings(json!(signer.public_hex())))).await;
    assert_eq!((s, v["public_key"].as_str()), (StatusCode::OK, Some(signer.public_hex().as_str())), "{v}");

    // Discord's endpoint check, and forged bodies.
    let (s, v) = t.send(signer.request(&json!({"type": 1}), false)).await;
    assert_eq!((s, v["type"].as_i64()), (StatusCode::OK, Some(1)));
    let (s, _) = t.send(signer.request(&command("status", json!([])), true)).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED, "a changed body fails the signature");
    let (s, _) = t.send(Signer::new().request(&command("status", json!([])), false)).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED, "someone else's key fails too");

    // /status
    let (s, v) = t.send(signer.request(&command("status", json!([])), false)).await;
    assert_eq!(s, StatusCode::OK);
    let embed = &v["data"]["embeds"][0];
    assert!(embed["title"].as_str().unwrap().contains("offline"), "no server has checked in: {v}");
    assert_eq!(embed["fields"][0]["name"].as_str().unwrap().trim_start_matches(|c: char| !c.is_alphanumeric()), "SMP");

    // /quests and /stats for a player, and a helpful miss.
    let (_, v) = t.send(signer.request(&command("quests", json!([{"name": "player", "value": "alex"}])), false)).await;
    let embed = &v["data"]["embeds"][0];
    assert!(embed["title"].as_str().unwrap().contains("Alex's quests"), "{v}");
    assert!(embed["fields"][0]["name"].as_str().unwrap().contains("Daily"));
    assert!(embed["fields"][0]["value"].as_str().unwrap().contains("XP"));
    let (_, v) = t.send(signer.request(&command("stats", json!([{"name": "player", "value": "Alex"}])), false)).await;
    let fields = v["data"]["embeds"][0]["fields"].as_array().unwrap();
    assert!(fields.iter().any(|f| f["name"] == "Playtime" && f["value"] == "2h 1m"), "{v}");
    let (_, v) = t.send(signer.request(&command("stats", json!([{"name": "player", "value": "Nobody"}])), false)).await;
    assert_eq!(v["data"]["flags"], 64, "errors are only shown to the person who asked");
    assert!(v["data"]["content"].as_str().unwrap().contains("Nobody"));

    // /leaderboard, /players, /guilds on an empty server don't blow up.
    let (_, v) = t.send(signer.request(&command("leaderboard", json!([{"name": "by", "value": "playtime"}])), false)).await;
    assert!(v["data"]["embeds"][0]["description"].as_str().unwrap().contains("Alex"));
    let (_, v) = t.send(signer.request(&command("players", json!([])), false)).await;
    assert!(v["data"]["embeds"][0]["title"].as_str().unwrap().contains("Nobody"));
    let (_, v) = t.send(signer.request(&command("guilds", json!([])), false)).await;
    assert_eq!(v["data"]["content"], "No guilds yet.");

    // Autocomplete suggests player names.
    let ac = json!({"type": 4, "data": {"name": "quests", "options": [{"name": "player", "value": "al", "focused": true}]}});
    let (_, v) = t.send(signer.request(&ac, false)).await;
    assert_eq!(v["type"], 8);
    assert_eq!(v["data"]["choices"][0]["value"], "Alex");

    // One-click registration needs the application ID, bot token and server ID, then sends every command to Discord.
    let (s, e) = t.call("POST", "/api/admin/discord/commands/register", Some(&admin), None).await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "{e}");
    assert!(e["error"].as_str().unwrap().contains("Settings"));
    let conn = json!({"discord_client_id": "123456789012345678", "discord_client_secret": "", "discord_bot_token": "bot-secret", "discord_guild_id": "999999999999999999",
        "resend_api_key": "", "sender_email": "", "sender_name": ""});
    t.call("PUT", "/api/admin/connections", Some(&admin), Some(conn)).await;
    let (s, r) = t.call("POST", "/api/admin/discord/commands/register", Some(&admin), None).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    assert_eq!(r["commands"], 7);
    let seen = seen.lock().unwrap();
    let (route, auth, body) = seen.last().unwrap();
    assert_eq!((route.as_str(), auth.as_str()), ("PUT /applications/123456789012345678/guilds/999999999999999999/commands", "Bot bot-secret"));
    let names: Vec<&str> = body.as_array().unwrap().iter().map(|c| c["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["status", "players", "quests", "stats", "leaderboard", "guild", "guilds"]);
}
