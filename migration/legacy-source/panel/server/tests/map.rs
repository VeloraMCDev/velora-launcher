//! The map: the address shown in launchers, actions on clicked players, and guild invitations.

mod common;
use common::*;

struct World {
    t: TestApp,
    admin: String,
    server: String,
    sid: i64,
    alex: String,
    steve: String,
    alex_uuid: String,
    steve_uuid: String,
    gid: String,
}

async fn world() -> World {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    for n in ["Alex", "Steve", "Mia"] {
        t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username": n, "password": "password123"}))).await;
    }
    let (alex, steve) = (t.login("Alex", "password123").await, t.login("Steve", "password123").await);
    let (alex_uuid, steve_uuid) = (t.uuid("Alex").await, t.uuid("Steve").await);
    let (s, srv) = t.call("POST", "/api/admin/servers", Some(&admin), Some(json!({"name": "SMP", "instance_id": "smp"}))).await;
    assert_eq!(s, StatusCode::OK, "{srv}");
    let sid = srv["server"]["id"].as_i64().unwrap();
    let server = srv["token"].as_str().unwrap().to_string();
    let (_, g) = t.call("POST", "/api/v1/guilds", Some(&alex), Some(json!({"instance_id": "smp", "name": "Iron", "tag": "IRON"}))).await;
    let gid = g["id"].as_str().unwrap().to_string();
    World { t, admin, server, sid, alex, steve, alex_uuid, steve_uuid, gid }
}

impl World {
    async fn online(&self, uuid: &str, name: &str) {
        sqlx::query("INSERT OR REPLACE INTO server_online (server_id, uuid, name, joined_at) VALUES (?, ?, ?, '2026-01-01T00:00:00Z')")
            .bind(self.sid)
            .bind(uuid)
            .bind(name)
            .execute(&self.t.db)
            .await
            .unwrap();
    }
    async fn poll(&self) -> Value {
        self.t.call("POST", "/api/server/v1/actions/poll", Some(&self.server), Some(json!({}))).await.1
    }
}

#[tokio::test]
async fn tpa_and_messages_are_queued_once_for_the_game_server() {
    let w = world().await;
    let path = format!("/api/v1/servers/{}/map/actions", w.sid);
    let tpa = json!({"action": "tpa", "target_uuid": w.steve_uuid});

    // Nobody is online yet.
    assert_eq!(w.t.call("POST", &path, Some(&w.alex), Some(tpa.clone())).await.0, StatusCode::BAD_REQUEST);
    w.online(&w.steve_uuid, "Steve").await;
    let (s, r) = w.t.call("POST", &path, Some(&w.alex), Some(tpa.clone())).await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "the sender has to be in game for a teleport: {r}");
    w.online(&w.alex_uuid, "Alex").await;
    assert_eq!(w.t.call("POST", &path, Some(&w.alex), Some(tpa)).await.0, StatusCode::OK);

    let (s, r) =
        w.t.call(
            "POST",
            &path,
            Some(&w.alex),
            Some(json!({"action": "message", "target_uuid": w.steve_uuid, "text": "  hi\n\tthere\u{7} friend  "})),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{r}");

    let polled = w.poll().await;
    let actions = polled["actions"].as_array().unwrap();
    assert_eq!(actions.len(), 2);
    assert_eq!((actions[0]["kind"].as_str(), actions[0]["from_name"].as_str()), (Some("tpa"), Some("Alex")));
    assert_eq!(actions[1]["text"], "hi there friend", "control characters and stray spaces are removed");
    assert_eq!(w.poll().await["actions"].as_array().unwrap().len(), 0, "each action is handed out once");

    // Bad input.
    let bad = |body: Value| w.t.call("POST", &path, Some(&w.alex), Some(body));
    assert_eq!(bad(json!({"action": "message", "target_uuid": w.steve_uuid, "text": "   "})).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(bad(json!({"action": "kill", "target_uuid": w.steve_uuid})).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(bad(json!({"action": "tpa", "target_uuid": w.alex_uuid})).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(
        w.t.call("POST", &path, None, Some(json!({"action": "tpa", "target_uuid": w.steve_uuid}))).await.0,
        StatusCode::UNAUTHORIZED
    );

    // A burst is slowed down.
    let mut last = StatusCode::OK;
    for _ in 0..10 {
        last = bad(json!({"action": "message", "target_uuid": w.steve_uuid, "text": "spam"})).await.0;
    }
    assert_eq!(last, StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn guild_invites_need_consent() {
    let w = world().await;
    // A member can't invite; the leader can.
    let (s, _) = w.t.call("POST", &format!("/api/v1/guilds/{}/invites", w.gid), Some(&w.steve), Some(json!({"uuid": w.alex_uuid}))).await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    w.online(&w.steve_uuid, "Steve").await;
    let (s, r) = w.t.call("POST", &format!("/api/v1/guilds/{}/invites", w.gid), Some(&w.alex), Some(json!({"uuid": w.steve_uuid}))).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    let in_guild = |uuid: &str| {
        let (db, gid, uuid) = (w.t.db.clone(), w.gid.clone(), uuid.to_string());
        async move {
            sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM guild_members WHERE guild_id = ? AND uuid = ?)")
                .bind(gid)
                .bind(uuid)
                .fetch_one(&db)
                .await
                .unwrap()
        }
    };
    assert!(!in_guild(&w.steve_uuid).await, "nobody is added without saying yes");

    // The invited player is told in game, and sees it in the launcher.
    let told = w.poll().await;
    assert!(told["actions"][0]["text"].as_str().unwrap().contains("[IRON] Iron"));
    assert_eq!(told["actions"][0]["kind"], "notify");
    let (_, mine) = w.t.call("GET", "/api/v1/guilds/invites", Some(&w.steve), None).await;
    assert_eq!(mine[0]["guild_tag"], "IRON");
    let id = mine[0]["id"].as_i64().unwrap();

    // Someone else can't answer it; Steve can.
    let mia = w.t.login("Mia", "password123").await;
    assert_eq!(w.t.call("POST", &format!("/api/v1/guilds/invites/{id}/accept"), Some(&mia), None).await.0, StatusCode::NOT_FOUND);
    let (s, r) = w.t.call("POST", &format!("/api/v1/guilds/invites/{id}/accept"), Some(&w.steve), None).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    assert!(in_guild(&w.steve_uuid).await);
    assert_eq!(w.t.call("GET", "/api/v1/guilds/invites", Some(&w.steve), None).await.1.as_array().unwrap().len(), 0);

    // Already in a guild here: no second invitation.
    let (s, _) = w.t.call("POST", &format!("/api/v1/guilds/{}/invites", w.gid), Some(&w.alex), Some(json!({"uuid": w.steve_uuid}))).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn invites_work_in_game_too() {
    let w = world().await;
    let server = |path: &'static str, body: Value| {
        let (t, token) = (&w.t, w.server.clone());
        async move { t.call("POST", &format!("/api/server/v1/{path}"), Some(&token), Some(body)).await }
    };
    let (s, r) = server("guilds/invite/send", json!({"uuid": w.alex_uuid, "target": "steve"})).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    assert_eq!(
        server("guilds/invite/send", json!({"uuid": w.steve_uuid, "target": "alex"})).await.0,
        StatusCode::FORBIDDEN,
        "not a leader"
    );
    assert_eq!(server("guilds/invite/send", json!({"uuid": w.alex_uuid, "target": "nobody"})).await.0, StatusCode::NOT_FOUND);

    let (_, list) = server("guilds/invite/list", json!({"uuid": w.steve_uuid})).await;
    assert_eq!(list["invites"][0]["guild_tag"], "IRON");
    let (s, r) = server("guilds/invite/respond", json!({"uuid": w.steve_uuid, "tag": "iron", "accept": false})).await;
    assert_eq!((s, r["accepted"].clone()), (StatusCode::OK, json!(false)));
    assert_eq!(
        server("guilds/invite/respond", json!({"uuid": w.steve_uuid, "accept": true})).await.0,
        StatusCode::NOT_FOUND,
        "a declined invitation is gone"
    );

    server("guilds/invite/send", json!({"uuid": w.alex_uuid, "target": "Steve"})).await;
    let (s, r) = server("guilds/invite/respond", json!({"uuid": w.steve_uuid, "accept": true})).await;
    assert_eq!((s, r["accepted"].clone()), (StatusCode::OK, json!(true)));
}
