//! Plugin integrations (LuckPerms ranks and group mapping) and the events
//! handed back to game servers with their sync.

mod common;
use common::*;

struct Env {
    t: TestApp,
    admin: String,
    server: String,
    sid: i64,
}

async fn env() -> Env {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    let (_, srv) = t.call("POST", "/api/admin/servers", Some(&admin), Some(json!({"name": "SMP", "instance_id": "smp"}))).await;
    Env { sid: srv["server"]["id"].as_i64().unwrap(), server: srv["token"].as_str().unwrap().to_string(), t, admin }
}

impl Env {
    async fn player(&self, name: &str) -> (String, String) {
        self.t.call("POST", "/api/admin/users", Some(&self.admin), Some(json!({"username": name, "password": "password123"}))).await;
        (self.t.login(name, "password123").await, self.t.uuid(name).await)
    }
    async fn report(&self, name: &str, data: Value) -> (StatusCode, Value) {
        self.t
            .call(
                "POST",
                "/api/server/v1/integrations/report",
                Some(&self.server),
                Some(json!({"name": name, "version": "5.4.1", "data": data})),
            )
            .await
    }
    async fn sync(&self, online: &[(&str, &str)]) -> Value {
        let online: Vec<Value> = online.iter().map(|(u, n)| json!({"uuid": u, "name": n})).collect();
        let (s, v) = self.t.call("POST", "/api/server/v1/sync", Some(&self.server), Some(json!({"online": online}))).await;
        assert_eq!(s, StatusCode::OK, "{v}");
        v
    }
    async fn groups_of(&self, name: &str) -> Vec<String> {
        sqlx::query_scalar("SELECT g.name FROM groups g JOIN user_groups ug ON ug.group_id = g.id JOIN users u ON u.id = ug.user_id WHERE u.username = ? ORDER BY g.name")
            .bind(name).fetch_all(&self.t.db).await.unwrap()
    }
}

#[tokio::test]
async fn luckperms_ranks_mapping_and_sync_modes() {
    let e = env().await;
    let (_, steve) = e.player("Steve").await;
    let (_, alex) = e.player("Alex").await;
    for g in ["VIP", "Staff"] {
        e.t.call("POST", "/api/admin/groups", Some(&e.admin), Some(json!({"name": g, "color": "#ff0000"}))).await;
    }
    let (_, groups) = e.t.call("GET", "/api/admin/groups", Some(&e.admin), None).await;
    let id = |n: &str| groups.as_array().unwrap().iter().find(|g| g["name"] == n).unwrap()["id"].as_i64().unwrap();
    let (s, _) =
        e.t.call("PUT", &format!("/api/admin/groups/{}/luckperms", id("VIP")), Some(&e.admin), Some(json!({"luckperms_group": "vip"})))
            .await;
    assert_eq!(s, StatusCode::OK);
    let (s, _) =
        e.t.call(
            "PUT",
            &format!("/api/admin/groups/{}/luckperms", id("Staff")),
            Some(&e.admin),
            Some(json!({"luckperms_group": "bad group!"})),
        )
        .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    e.t.call("PUT", &format!("/api/admin/groups/{}/luckperms", id("Staff")), Some(&e.admin), Some(json!({"luckperms_group": "Moderator"})))
        .await;

    let lp = |steve_groups: Value| {
        json!({
        "groups": [{"name": "vip", "weight": 10}, {"name": "moderator", "weight": 50}],
        "players": [
            {"uuid": steve, "primary": "VIP", "display": "VIP", "prefix": "§6[VIP] ", "weight": 10, "groups": steve_groups, "permissions": ["scopenet.command.*", "essentials.fly"]},
            {"uuid": alex, "primary": "default", "groups": ["default"]},
            {"uuid": "not-a-uuid", "primary": "x"}
        ]})
    };

    // Default mode is display-only: ranks are stored, no group changes.
    let (s, r) = e.report("luckperms", lp(json!(["default", "vip"]))).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    assert_eq!(r["config"]["mode"], "off");
    assert!(e.groups_of("Steve").await.is_empty());
    let (_, view) = e.t.call("GET", &format!("/api/admin/servers/{}/integrations", e.sid), Some(&e.admin), None).await;
    let ranks = view["ranks"].as_array().unwrap();
    assert_eq!(ranks.len(), 2);
    assert_eq!(ranks[0]["name"], "Steve");
    assert_eq!(ranks[0]["primary"], "vip");
    assert_eq!(ranks[0]["prefix"], "§6[VIP] ");
    assert_eq!(view["integrations"][0]["name"], "luckperms");
    assert_eq!(view["integrations"][0]["data"]["groups"][1]["name"], "moderator");

    // game_to_panel: mapped groups follow LuckPerms, including removal.
    let (s, _) =
        e.t.call("PUT", "/api/admin/integrations/settings", Some(&e.admin), Some(json!({"luckperms_sync": "game_to_panel"}))).await;
    assert_eq!(s, StatusCode::OK);
    e.report("luckperms", lp(json!(["default", "vip", "moderator"]))).await;
    assert_eq!(e.groups_of("Steve").await, ["Staff", "VIP"]);
    let (_, r) = e.report("luckperms", lp(json!(["default"]))).await;
    assert!(e.groups_of("Steve").await.is_empty(), "removed in LuckPerms, removed here");
    assert!(r["config"]["assign"].as_array().unwrap().is_empty());

    // panel_to_game: the panel's groups are pushed to the game as assignments.
    sqlx::query("INSERT INTO user_groups (user_id, group_id) SELECT u.id, ? FROM users u WHERE u.username = 'Alex'")
        .bind(id("VIP"))
        .execute(&e.t.db)
        .await
        .unwrap();
    e.t.call("PUT", "/api/admin/integrations/settings", Some(&e.admin), Some(json!({"luckperms_sync": "panel_to_game"}))).await;
    let (_, r) = e.report("luckperms", lp(json!(["default", "moderator"]))).await;
    let assign = r["config"]["assign"].as_array().unwrap();
    let for_alex = assign.iter().find(|a| a["uuid"] == alex).unwrap();
    assert_eq!(for_alex["add"], json!(["vip"]));
    let for_steve = assign.iter().find(|a| a["uuid"] == steve).unwrap();
    assert_eq!(for_steve["remove"], json!(["moderator"]), "LuckPerms has a mapped group the panel doesn't");

    let (s, _) = e.t.call("PUT", "/api/admin/integrations/settings", Some(&e.admin), Some(json!({"luckperms_sync": "sideways"}))).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let (s, _) = e.report("nonsense", json!({})).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let (s, _) = e.t.call("POST", "/api/server/v1/integrations/report", None, Some(json!({"name": "spark"}))).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn other_integrations_are_stored_per_server() {
    let e = env().await;
    e.report("spark", json!({"tps": {"m1": 19.98}, "mspt": {"mean": 12.5, "p95": 31.0}})).await;
    e.report(
        "worldguard",
        json!({"regions": [{"world": "world", "id": "spawn", "min": [-50, 0, -50], "max": [50, 255, 50], "owners": ["admin"]}]}),
    )
    .await;
    e.report("spark", json!({"tps": {"m1": 20.0}})).await;
    let (_, view) = e.t.call("GET", &format!("/api/admin/servers/{}/integrations", e.sid), Some(&e.admin), None).await;
    let names: Vec<&str> = view["integrations"].as_array().unwrap().iter().map(|i| i["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["spark", "worldguard"]);
    assert_eq!(view["integrations"][0]["data"]["tps"]["m1"], 20.0, "the latest report replaces the previous one");
    let big = json!({"pad": "x".repeat(1_100_000)});
    assert_eq!(e.report("worldguard", big).await.0, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn game_servers_receive_events_with_their_sync() {
    let e = env().await;
    let (_, steve) = e.player("Steve").await;
    let (_, alex) = e.player("Alex").await;
    assert!(e.sync(&[(&steve, "Steve")]).await["notifications"].as_array().unwrap().is_empty());

    // Level-ups: global and per-server.
    let adjust = |scope: &str, mode: &str, amount: i64| {
        let (t, admin, uuid, sid) = (&e.t, e.admin.clone(), steve.clone(), e.sid);
        let body = if scope == "global" {
            json!({"scope": "global", "mode": mode, "amount": amount})
        } else {
            json!({"scope": "server", "server_id": sid, "mode": mode, "amount": amount})
        };
        async move { t.call("POST", &format!("/api/admin/progression/players/{uuid}/adjust"), Some(&admin), Some(body)).await }
    };
    adjust("global", "set_xp", 10).await;
    adjust("global", "set_xp", 1200).await; // level 1 -> 5
    adjust("server", "set_xp", 10).await;
    adjust("server", "set_xp", 300).await; // level 1 -> 2
    let n = e.sync(&[(&steve, "Steve")]).await["notifications"].clone();
    let kinds: Vec<(&str, &str)> =
        n.as_array().unwrap().iter().map(|x| (x["kind"].as_str().unwrap(), x["data"]["scope"].as_str().unwrap_or(""))).collect();
    assert_eq!(kinds, [("level_up", "global"), ("level_up", "server")], "{n}");
    assert_eq!((n[0]["data"]["previous"].as_i64(), n[0]["data"]["level"].as_i64()), (Some(1), Some(5)));
    assert_eq!(n[0]["uuid"], steve);
    // Delivered once.
    assert!(e.sync(&[(&steve, "Steve")]).await["notifications"].as_array().unwrap().is_empty());
    // Dropping levels doesn't announce anything.
    adjust("global", "reset", 0).await;
    assert!(e.sync(&[(&steve, "Steve")]).await["notifications"].as_array().unwrap().is_empty());

    // Achievements and guild changes.
    let ach: String = sqlx::query_scalar("SELECT id FROM achievements LIMIT 1").fetch_one(&e.t.db).await.unwrap();
    sqlx::query("INSERT INTO user_achievements (user_uuid, achievement_id, unlocked_at) VALUES (?, ?, 'now')")
        .bind(&steve)
        .bind(&ach)
        .execute(&e.t.db)
        .await
        .unwrap();
    sqlx::query("INSERT INTO guilds (id, instance_id, name, tag, leader_uuid, created_at) VALUES ('g1', 'smp', 'Iron', 'IRON', ?, 'now')")
        .bind(&alex)
        .execute(&e.t.db)
        .await
        .unwrap();
    sqlx::query("INSERT INTO guild_members (guild_id, uuid, name, role, joined_at) VALUES ('g1', ?, 'Steve', 'member', 'now')")
        .bind(&steve)
        .execute(&e.t.db)
        .await
        .unwrap();
    sqlx::query("INSERT INTO guild_members (guild_id, uuid, name, role, joined_at) VALUES ('g1', ?, 'Alex', 'leader', 'now')")
        .bind(&alex)
        .execute(&e.t.db)
        .await
        .unwrap();
    sqlx::query("DELETE FROM guild_members WHERE uuid = ?").bind(&steve).execute(&e.t.db).await.unwrap();
    let n = e.sync(&[(&steve, "Steve")]).await["notifications"].clone();
    let kinds: Vec<&str> = n.as_array().unwrap().iter().map(|x| x["kind"].as_str().unwrap()).collect();
    assert_eq!(kinds, ["achievement", "guild_join", "guild_leave"], "Alex was offline, so only Steve's events: {n}");
    assert_eq!(n[0]["data"]["id"], ach);
    assert_eq!(n[1]["data"]["tag"], "IRON");
}
