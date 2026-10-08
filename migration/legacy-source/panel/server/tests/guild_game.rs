//! Guild management from inside the game: kicks, roles, join requests, announcements, leadership — with the launcher's rules.

mod common;
use common::*;

struct F {
    t: TestApp,
    server: String,
    tok: std::collections::HashMap<&'static str, String>,
    uuid: std::collections::HashMap<&'static str, String>,
    gid: String,
}

async fn fixture() -> F {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    let mut tok = std::collections::HashMap::new();
    let mut uuid = std::collections::HashMap::new();
    for n in ["Alex", "Steve", "Mia", "Zed"] {
        t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username": n, "password": "password123"}))).await;
        tok.insert(n, t.login(n, "password123").await);
        uuid.insert(n, t.uuid(n).await);
    }
    let (_, srv) = t.call("POST", "/api/admin/servers", Some(&admin), Some(json!({"name": "SMP", "instance_id": "smp"}))).await;
    let server = srv["token"].as_str().unwrap().to_string();
    let (_, g) = t.call("POST", "/api/v1/guilds", Some(&tok["Alex"]), Some(json!({"instance_id": "smp", "name": "Iron", "tag": "IRON"}))).await;
    let gid = g["id"].as_str().unwrap().to_string();
    for (n, role) in [("Steve", "officer"), ("Mia", "member")] {
        sqlx::query("INSERT INTO guild_members (guild_id, uuid, name, role, joined_at) VALUES (?, ?, ?, ?, '2026-01-01')")
            .bind(&gid).bind(&uuid[n]).bind(n).bind(role).execute(&t.db).await.unwrap();
    }
    F { t, server, tok, uuid, gid }
}

impl F {
    async fn act(&self, who: &str, action: &str, extra: Value) -> (StatusCode, Value) {
        let mut body = json!({"uuid": self.uuid[who], "name": who, "action": action});
        for (k, v) in extra.as_object().unwrap() {
            body[k] = v.clone();
        }
        self.t.call("POST", "/api/server/v1/guilds/manage", Some(&self.server), Some(body)).await
    }
    async fn role(&self, who: &str) -> Option<String> {
        sqlx::query_scalar("SELECT role FROM guild_members WHERE uuid = ?").bind(&self.uuid[who]).fetch_optional(&self.t.db).await.unwrap()
    }
    async fn bell(&self, who: &str) -> Vec<String> {
        let (_, n) = self.t.call("GET", "/api/v1/notifications", Some(&self.tok[who]), None).await;
        n["items"].as_array().unwrap().iter().map(|i| i["kind"].as_str().unwrap().to_string()).collect()
    }
}

#[tokio::test]
async fn kicking_follows_rank_and_tells_the_player() {
    let f = fixture().await;
    assert_eq!(f.act("Mia", "kick", json!({"target": "Steve"})).await.0, StatusCode::FORBIDDEN, "plain members can't kick");
    assert_eq!(f.act("Steve", "kick", json!({"target": "Alex"})).await.0, StatusCode::FORBIDDEN, "nobody removes the leader");
    assert_eq!(f.act("Alex", "kick", json!({"target": "Alex"})).await.0, StatusCode::BAD_REQUEST);
    let (s, r) = f.act("Steve", "kick", json!({"target": "Mia"})).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    assert_eq!(f.role("Mia").await, None);
    assert!(f.bell("Mia").await.contains(&"guild_kicked".to_string()));

    // Officers can't remove other officers; the leader can.
    sqlx::query("INSERT INTO guild_members (guild_id, uuid, name, role, joined_at) VALUES (?, ?, 'Zed', 'officer', '2026-01-01')").bind(&f.gid).bind(&f.uuid["Zed"]).execute(&f.t.db).await.unwrap();
    assert_eq!(f.act("Steve", "kick", json!({"target": "Zed"})).await.0, StatusCode::FORBIDDEN);
    assert_eq!(f.act("Alex", "kick", json!({"target": "Zed"})).await.0, StatusCode::OK);

    // The launcher's own route enforces the same rules now.
    let path = format!("/api/v1/guilds/{}/members/{}", f.gid, f.uuid["Alex"]);
    assert_eq!(f.t.call("DELETE", &path, Some(&f.tok["Steve"]), None).await.0, StatusCode::FORBIDDEN, "launcher can't remove the leader either");
}

#[tokio::test]
async fn roles_and_leadership_are_the_leaders_to_hand_out() {
    let f = fixture().await;
    assert_eq!(f.act("Steve", "promote", json!({"target": "Mia"})).await.0, StatusCode::FORBIDDEN);
    assert_eq!(f.act("Alex", "promote", json!({"target": "Mia"})).await.0, StatusCode::OK);
    assert_eq!(f.role("Mia").await.as_deref(), Some("officer"));
    assert_eq!(f.act("Alex", "demote", json!({"target": "Mia"})).await.0, StatusCode::OK);
    assert_eq!(f.role("Mia").await.as_deref(), Some("member"));

    // Custom roles made in the launcher can be given in game.
    f.t.call("POST", &format!("/api/v1/guilds/{}/roles", f.gid), Some(&f.tok["Alex"]), Some(json!({"name": "Scout", "can_claim": true}))).await;
    assert_eq!(f.act("Alex", "role", json!({"target": "Mia", "text": "scout"})).await.0, StatusCode::OK);
    assert_eq!(f.role("Mia").await.as_deref(), Some("Scout"));
    assert_eq!(f.act("Alex", "role", json!({"target": "Mia", "text": "king"})).await.0, StatusCode::BAD_REQUEST);
    let (_, roles) = f.act("Mia", "roles", json!({})).await;
    assert_eq!(roles["roles"][0]["name"], "Scout");

    // Handing over the guild.
    assert_eq!(f.act("Steve", "transfer", json!({"target": "Mia"})).await.0, StatusCode::FORBIDDEN);
    let (s, r) = f.act("Alex", "transfer", json!({"target": "Steve"})).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    assert_eq!((f.role("Steve").await.as_deref(), f.role("Alex").await.as_deref()), (Some("leader"), Some("officer")));
    let leader: String = sqlx::query_scalar("SELECT leader_uuid FROM guilds WHERE id = ?").bind(&f.gid).fetch_one(&f.t.db).await.unwrap();
    assert_eq!(leader, f.uuid["Steve"]);
    assert!(f.bell("Steve").await.contains(&"guild_leader".to_string()));

    // And the launcher can do it too.
    let path = format!("/api/v1/guilds/{}/leader", f.gid);
    assert_eq!(f.t.call("PUT", &path, Some(&f.tok["Steve"]), Some(json!({"uuid": f.uuid["Alex"]}))).await.0, StatusCode::OK);
    assert_eq!(f.role("Alex").await.as_deref(), Some("leader"));
}

#[tokio::test]
async fn join_requests_details_and_announcements() {
    let f = fixture().await;
    let (_, list) = f.act("Zed", "list", json!({})).await;
    assert_eq!(list["guilds"][0]["name"], "Iron");
    assert_eq!(list["guilds"][0]["members"], 3);
    let (_, info) = f.act("Zed", "info", json!({"target": "iron"})).await;
    assert_eq!(info["leader"], "Alex");
    assert_eq!(info["your_role"], Value::Null);
    assert_eq!(f.act("Zed", "info", json!({})).await.0, StatusCode::BAD_REQUEST, "no own guild");

    let (s, r) = f.act("Zed", "join", json!({"target": "IRON", "text": "Let me in!"})).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    assert!(f.bell("Alex").await.contains(&"guild_request".to_string()), "leaders hear about requests");
    assert_eq!(f.act("Mia", "requests", json!({})).await.0, StatusCode::FORBIDDEN, "plain members cannot review requests");
    let (_, reqs) = f.act("Steve", "requests", json!({})).await;
    assert_eq!(reqs["requests"][0]["name"], "Zed");
    assert_eq!(reqs["requests"][0]["message"], "Let me in!");
    // Launcher sees the same request.
    let (_, via_launcher) = f.t.call("GET", &format!("/api/v1/guilds/{}/requests", f.gid), Some(&f.tok["Alex"]), None).await;
    assert_eq!(via_launcher.as_array().unwrap().len(), 1);

    assert_eq!(f.act("Zed", "accept", json!({"target": "Zed"})).await.0, StatusCode::BAD_REQUEST, "not in a guild");
    let (s, r) = f.act("Steve", "accept", json!({"target": "zed"})).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    assert_eq!(f.role("Zed").await.as_deref(), Some("member"));
    assert!(f.bell("Zed").await.contains(&"guild_joined".to_string()));

    // Message of the day, description and the announcement board.
    assert_eq!(f.act("Mia", "motd", json!({"text": "hi"})).await.0, StatusCode::FORBIDDEN);
    assert_eq!(f.act("Steve", "motd", json!({"text": "Raid at dawn"})).await.0, StatusCode::OK);
    assert_eq!(f.act("Steve", "desc", json!({"text": "x".repeat(501)})).await.0, StatusCode::BAD_REQUEST);
    let (_, info) = f.act("Mia", "info", json!({})).await;
    assert_eq!(info["motd"], "Raid at dawn");
    assert_eq!(info["your_role"], "member");
    assert_eq!(f.act("Steve", "post", json!({"title": "Meeting", "text": "Saturday 8pm"})).await.0, StatusCode::OK);
    assert!(f.bell("Mia").await.contains(&"guild_post".to_string()));
    let (_, posts) = f.act("Mia", "posts", json!({})).await;
    assert_eq!(posts["posts"][0]["title"], "Meeting");
    let (_, board) = f.t.call("GET", &format!("/api/v1/guilds/{}/posts", f.gid), Some(&f.tok["Mia"]), None).await;
    assert_eq!(board[0]["title"], "Meeting", "the launcher shows the same board");
}
