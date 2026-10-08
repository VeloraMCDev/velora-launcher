//! The bot posts and edits messages in channels chosen by ID, and explains Discord's refusals. A fake Discord stands in for the real one.

mod common;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::{get, patch, post};
use axum::Json;
use common::*;
use std::sync::{Arc, Mutex};

type Seen = Arc<Mutex<Vec<(String, String, Value)>>>; // (METHOD path, authorization header, body)

async fn fake_discord() -> (String, Seen) {
    let seen: Seen = Default::default();
    let app = axum::Router::new()
        .route(
            "/channels/{channel}/messages",
            post(|State(seen): State<Seen>, Path(channel): Path<String>, h: HeaderMap, Json(body): Json<Value>| async move {
                let auth = h.get("authorization").and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
                seen.lock().unwrap().push((format!("POST /channels/{channel}/messages"), auth, body));
                match channel.as_str() {
                    "222222222222222222" => (StatusCode::FORBIDDEN, Json(json!({"message": "Missing Access"}))),
                    "333333333333333333" => (StatusCode::NOT_FOUND, Json(json!({"message": "Unknown Channel"}))),
                    _ => (StatusCode::OK, Json(json!({"id": "555555555555555555"}))),
                }
            }),
        )
        .route(
            "/channels/{channel}/messages/{message}",
            patch(|State(seen): State<Seen>, Path((channel, message)): Path<(String, String)>, h: HeaderMap, Json(body): Json<Value>| async move {
                let auth = h.get("authorization").and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
                seen.lock().unwrap().push((format!("PATCH /channels/{channel}/messages/{message}"), auth, body));
                if message == "555555555555555555" {
                    (StatusCode::OK, Json(json!({"id": message})))
                } else {
                    (StatusCode::NOT_FOUND, Json(json!({"message": "Unknown Message"})))
                }
            }),
        )
        .route(
            "/guilds/{guild}/channels",
            get(|| async {
                Json(json!([
                    {"id": "900", "name": "Community", "type": 4, "position": 0},
                    {"id": "100000000000000001", "name": "announcements", "type": 5, "position": 1, "parent_id": "900"},
                    {"id": "100000000000000002", "name": "general", "type": 0, "position": 0, "parent_id": "900"},
                    {"id": "100000000000000003", "name": "voice", "type": 2, "position": 2, "parent_id": "900"},
                    {"id": "100000000000000004", "name": "lonely", "type": 0, "position": 5}
                ]))
            }),
        )
        .with_state(seen.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (url, seen)
}

#[tokio::test]
async fn the_bot_posts_to_a_channel_id_and_explains_refusals() {
    let (discord, seen) = fake_discord().await;
    std::env::set_var("SCOPENET_DISCORD_API", &discord);
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    let conn = |token: &str| {
        json!({"discord_client_id": "123456789012345678", "discord_client_secret": "", "discord_bot_token": token, "discord_guild_id": "999999999999999999",
        "resend_api_key": "", "sender_email": "", "sender_name": ""})
    };
    let discord_settings = |channel: &str| json!({"role_sync": "off", "invite_url": "", "channel_id": channel, "notify_achievements": true, "notify_guilds": true, "notify_members": false});

    // Nothing set up: the test button says what to do.
    let (s, r) = t.call("POST", "/api/admin/discord/test", Some(&admin), None).await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "{r}");
    assert!(r["error"].as_str().unwrap().contains("channel"), "{r}");

    // A channel ID is checked before it is saved; a channel needs the bot token.
    let (s, _) = t.call("PUT", "/api/admin/discord", Some(&admin), Some(discord_settings("not-a-number"))).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let (s, r) = t.call("PUT", "/api/admin/discord", Some(&admin), Some(discord_settings("111111111111111111"))).await;
    assert_eq!((s, r["channel_id"].as_str()), (StatusCode::OK, Some("111111111111111111")), "{r}");
    let (s, r) = t.call("POST", "/api/admin/discord/test", Some(&admin), None).await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "no bot token yet: {r}");
    assert!(r["error"].as_str().unwrap().contains("bot token"), "{r}");

    // With a token, the test message goes out as the bot, to that channel, without webhook-only fields.
    assert_eq!(t.call("PUT", "/api/admin/connections", Some(&admin), Some(conn("bot-secret"))).await.0, StatusCode::OK);
    let (s, r) = t.call("POST", "/api/admin/discord/test", Some(&admin), None).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    {
        let seen = seen.lock().unwrap();
        let (route, auth, body) = seen.last().unwrap();
        assert_eq!((route.as_str(), auth.as_str()), ("POST /channels/111111111111111111/messages", "Bot bot-secret"));
        assert!(body.get("username").is_none(), "{body}");
        assert_eq!(body["embeds"][0]["title"], "Velora is connected");
    }

    // The settings page learns the bot is set up and gets an invite link.
    let (_, view) = t.call("GET", "/api/admin/discord", Some(&admin), None).await;
    assert_eq!(
        (view["channel_id"].as_str(), view["bot_ready"].as_bool(), view["bot_token_set"].as_bool()),
        (Some("111111111111111111"), Some(true), Some(true))
    );
    let invite = view["bot_invite_url"].as_str().unwrap();
    assert!(invite.contains("client_id=123456789012345678") && invite.contains("scope=bot") && invite.contains("permissions="), "{invite}");

    // Refusals are explained in plain words.
    for (channel, needle) in [("222222222222222222", "isn't allowed"), ("333333333333333333", "Check the channel ID")] {
        t.call("PUT", "/api/admin/discord", Some(&admin), Some(discord_settings(channel))).await;
        let (s, r) = t.call("POST", "/api/admin/discord/test", Some(&admin), None).await;
        assert_eq!(s, StatusCode::BAD_REQUEST, "{r}");
        assert!(r["error"].as_str().unwrap().contains(needle), "{channel}: {r}");
    }

    // The channel picker lists text and announcement channels in server order, with their category.
    let (s, list) = t.call("GET", "/api/admin/discord/channels", Some(&admin), None).await;
    assert_eq!(s, StatusCode::OK, "{list}");
    let names: Vec<&str> = list.as_array().unwrap().iter().map(|c| c["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["lonely", "general", "announcements"], "uncategorised first, voice channels left out: {list}");
    assert_eq!(list[2]["category"], "Community");

    // Live boards: posted once, then edited in place; a deleted message is replaced.
    t.call("PUT", "/api/admin/discord", Some(&admin), Some(discord_settings("111111111111111111"))).await;
    let (_, studio) = t.call("GET", "/api/admin/discord/studio", Some(&admin), None).await;
    let mut live = studio["live"].clone();
    live["status"]["channel_id"] = json!("444444444444444444");
    live["status"]["style"]["enabled"] = json!(true);
    assert_eq!(t.call("PUT", "/api/admin/discord/studio/live", Some(&admin), Some(json!({"live": live}))).await.0, StatusCode::OK);
    seen.lock().unwrap().clear();
    assert_eq!(t.call("POST", "/api/admin/discord/studio/send/status", Some(&admin), None).await.0, StatusCode::OK);
    assert_eq!(t.call("POST", "/api/admin/discord/studio/send/status", Some(&admin), None).await.0, StatusCode::OK);
    let routes: Vec<String> = seen.lock().unwrap().iter().map(|(r, _, _)| r.clone()).collect();
    assert_eq!(
        routes,
        ["POST /channels/444444444444444444/messages", "PATCH /channels/444444444444444444/messages/555555555555555555"],
        "its own channel, then an edit"
    );
    let (_, studio) = t.call("GET", "/api/admin/discord/studio", Some(&admin), None).await;
    assert_eq!(studio["live"]["status"]["channel_id"], "444444444444444444");
    assert_eq!(studio["live"]["status"]["posted"], true);
}
