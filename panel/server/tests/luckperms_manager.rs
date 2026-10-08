//! The LuckPerms manager: instructions queued by admins, collected and acknowledged by the game server, milestones and moves.

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
    async fn report(&self, data: Value) -> Value {
        let (s, v) = self.t.call("POST", "/api/server/v1/integrations/report", Some(&self.server), Some(json!({"name": "luckperms", "version": "5.4", "data": data}))).await;
        assert_eq!(s, StatusCode::OK, "{v}");
        v["config"].clone()
    }
    async fn admin(&self, method: &str, path: &str, body: Option<Value>) -> (StatusCode, Value) {
        self.t.call(method, path, Some(&self.admin), body).await
    }
}

#[tokio::test]
async fn instructions_are_queued_collected_and_acknowledged() {
    let e = env().await;
    // Nothing reaches a server that has never reported LuckPerms.
    let (s, _) = e.admin("POST", "/api/admin/luckperms/groups", Some(json!({"name": "vip"}))).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);

    // The plugin's first report makes the server manageable and publishes a snapshot.
    let snapshot = json!([{"name": "default", "weight": 0, "prefix": "", "suffix": "", "parents": [], "permissions": [], "members": 3}]);
    e.report(json!({"groups": snapshot, "players": [], "can_manage": true})).await;
    let (_, o) = e.admin("GET", "/api/admin/luckperms", None).await;
    assert_eq!(o["connected"], true);
    assert_eq!(o["can_manage"], true);
    assert_eq!(o["groups"][0]["name"], "default");
    assert_eq!(o["server_id"].as_i64(), Some(e.sid));

    // Create a group with everything; it becomes one batch instruction.
    let (s, q) = e.admin("POST", "/api/admin/luckperms/groups", Some(json!({
        "name": "VIP", "display": "VIP", "weight": 50, "prefix": "&6[VIP] ", "parents": ["default"],
        "permissions": [{"permission": "scopenet.casino.use"}, {"permission": "essentials.fly", "value": false, "world": "world_nether"}]
    }))).await;
    assert_eq!(s, StatusCode::OK, "{q}");
    assert_eq!(e.admin("POST", "/api/admin/luckperms/groups", Some(json!({"name": "bad name"}))).await.0, StatusCode::BAD_REQUEST);

    // A light poll collects it (and only once until it is acknowledged).
    let cfg = e.report(json!({"light": true})).await;
    let cmds = cfg["commands"].as_array().unwrap();
    assert_eq!(cmds.len(), 1);
    let op = &cmds[0]["op"];
    assert_eq!(op["kind"], "batch");
    assert_eq!(op["ops"][0], json!({"kind": "group_create", "group": "vip"}));
    assert_eq!(op["ops"][1]["prefix"], "&6[VIP] ");
    assert_eq!(op["ops"][1]["weight"], 50);
    assert_eq!(op["ops"].as_array().unwrap().len(), 5, "create, update, parent, two permissions");
    assert_eq!(op["ops"][4]["world"], "world_nether");
    assert!(e.report(json!({"light": true})).await["commands"].as_array().unwrap().is_empty());

    // Acknowledge it; the log shows it as done.
    let id = cmds[0]["id"].as_i64().unwrap();
    e.report(json!({"light": true, "command_results": [{"id": id, "ok": true}]})).await;
    let (_, o) = e.admin("GET", "/api/admin/luckperms", None).await;
    assert_eq!(o["commands"][0]["status"], "done");

    // Other edits: update, permission set/unset, parent, delete.
    assert_eq!(e.admin("PUT", "/api/admin/luckperms/groups/vip", Some(json!({"suffix": " &7*"}))).await.0, StatusCode::OK);
    assert_eq!(e.admin("PUT", "/api/admin/luckperms/groups/vip", Some(json!({}))).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(e.admin("POST", "/api/admin/luckperms/groups/vip/permissions", Some(json!({"permission": "scopenet.kit.vip"}))).await.0, StatusCode::OK);
    assert_eq!(e.admin("DELETE", "/api/admin/luckperms/groups/vip/permissions", Some(json!({"permission": "scopenet.kit.vip"}))).await.0, StatusCode::OK);
    assert_eq!(e.admin("POST", "/api/admin/luckperms/groups/vip/permissions", Some(json!({"permission": "no spaces"}))).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(e.admin("POST", "/api/admin/luckperms/groups/vip/parents", Some(json!({"parent": "vip"}))).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(e.admin("DELETE", "/api/admin/luckperms/groups/default", None).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(e.admin("DELETE", "/api/admin/luckperms/groups/vip", None).await.0, StatusCode::OK);
    let cfg = e.report(json!({"light": true})).await;
    let kinds: Vec<&str> = cfg["commands"].as_array().unwrap().iter().map(|c| c["op"]["kind"].as_str().unwrap()).collect();
    assert_eq!(kinds, ["group_update", "perm_set", "perm_unset", "group_delete"]);

    // A failure is shown with its reason and can be retried.
    let first = cfg["commands"][0]["id"].as_i64().unwrap();
    e.report(json!({"light": true, "command_results": [{"id": first, "ok": false, "error": "no such group"}]})).await;
    let (_, o) = e.admin("GET", "/api/admin/luckperms", None).await;
    let failed = o["commands"].as_array().unwrap().iter().find(|c| c["id"] == first).unwrap();
    assert_eq!((failed["status"].as_str(), failed["error"].as_str()), (Some("failed"), Some("no such group")));
    assert_eq!(e.admin("POST", &format!("/api/admin/luckperms/commands/{first}/retry"), Some(json!({}))).await.0, StatusCode::OK);
    assert_eq!(e.report(json!({"light": true})).await["commands"][0]["id"].as_i64(), Some(first));
}

#[tokio::test]
async fn players_are_moved_and_levels_hand_out_groups() {
    let e = env().await;
    e.report(json!({"groups": [], "players": [], "can_manage": true})).await;
    e.t.call("POST", "/api/admin/users", Some(&e.admin), Some(json!({"username": "Alex", "password": "password123"}))).await;
    let uuid = e.t.uuid("Alex").await;

    // Search lists panel players.
    let (_, l) = e.admin("GET", "/api/admin/luckperms/players?q=ale", None).await;
    assert_eq!(l["players"][0]["name"], "Alex");

    // Move: remove one group and add another, queued for the game.
    let (s, _) = e.admin("POST", &format!("/api/admin/luckperms/players/{uuid}/groups"), Some(json!({"add": ["vip"], "remove": ["default"]}))).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(e.admin("POST", &format!("/api/admin/luckperms/players/{uuid}/groups"), Some(json!({}))).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(e.admin("POST", "/api/admin/luckperms/players/not-a-uuid/groups", Some(json!({"add": ["vip"]}))).await.0, StatusCode::BAD_REQUEST);
    let cfg = e.report(json!({"light": true})).await;
    assert_eq!(cfg["commands"][0]["op"], json!({"kind": "user_groups", "uuid": uuid, "add": ["vip"], "remove": ["default"]}));

    // Level milestones: validated, replaceable, and used when the player reports in.
    assert_eq!(e.admin("PUT", "/api/admin/luckperms/level-links", Some(json!({"links": [{"level": 5, "group": "a"}, {"level": 5, "group": "b"}]}))).await.0, StatusCode::BAD_REQUEST);
    let (s, r) = e.admin("PUT", "/api/admin/luckperms/level-links", Some(json!({"links": [{"level": 1, "group": "newbie"}, {"level": 10, "group": "Veteran"}]}))).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    assert_eq!(r["level_links"][1], json!({"level": 10, "group": "veteran", "source": "manager"}));
    sqlx::query("UPDATE user_levels SET global_xp = 100000 WHERE uuid = ?").bind(&uuid).execute(&e.t.db).await.unwrap();
    sqlx::query("INSERT OR IGNORE INTO user_levels (uuid, global_xp, updated_at) VALUES (?, 100000, '2026-01-01T00:00:00Z')").bind(&uuid).execute(&e.t.db).await.unwrap();
    let cfg = e.report(json!({"groups": [], "can_manage": true, "players": [{"uuid": uuid, "primary": "newbie", "display": "", "prefix": "", "suffix": "", "weight": 0, "groups": ["newbie"], "permissions": []}]})).await;
    let assign = &cfg["level_assign"][0];
    assert_eq!(assign["add"], json!(["veteran"]));
    assert_eq!(assign["remove"], json!(["newbie"]), "only the highest milestone's group is kept");

    // Deleting a group also drops its milestone.
    assert_eq!(e.admin("DELETE", "/api/admin/luckperms/groups/veteran", None).await.0, StatusCode::OK);
    let (_, o) = e.admin("GET", "/api/admin/luckperms", None).await;
    assert_eq!(o["level_links"].as_array().unwrap().len(), 1);
}
