//! Renaming and disbanding guilds: leader only, names stay unique, and disbanding pays the treasury out.

mod common;
use common::*;

struct F {
    t: TestApp,
    admin: String,
    alex: String,
    steve: String,
    mia: String,
    gid: String,
    sid: i64,
    server: String,
    alex_uuid: String,
}

async fn fixture() -> F {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    for n in ["Alex", "Steve", "Mia"] {
        t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username": n, "password": "password123"}))).await;
    }
    let (alex, steve, mia) =
        (t.login("Alex", "password123").await, t.login("Steve", "password123").await, t.login("Mia", "password123").await);
    let (alex_uuid, steve_uuid) = (t.uuid("Alex").await, t.uuid("Steve").await);
    let (_, srv) = t.call("POST", "/api/admin/servers", Some(&admin), Some(json!({"name": "SMP", "instance_id": "smp"}))).await;
    let sid = srv["server"]["id"].as_i64().unwrap();
    let server = srv["token"].as_str().unwrap().to_string();
    let (_, g) = t.call("POST", "/api/v1/guilds", Some(&alex), Some(json!({"instance_id": "smp", "name": "Iron", "tag": "IRON"}))).await;
    let gid = g["id"].as_str().unwrap().to_string();
    sqlx::query("INSERT INTO guild_members (guild_id, uuid, name, role, joined_at) VALUES (?, ?, 'Steve', 'member', '2026-01-01')")
        .bind(&gid)
        .bind(&steve_uuid)
        .execute(&t.db)
        .await
        .unwrap();
    t.call("POST", "/api/v1/guilds", Some(&mia), Some(json!({"instance_id": "smp", "name": "Void", "tag": "VOID"}))).await;
    F { t, admin, alex, steve, mia, gid, sid, server, alex_uuid }
}

#[tokio::test]
async fn only_the_leader_renames_and_names_stay_unique() {
    let f = fixture().await;
    let path = format!("/api/v1/guilds/{}/name", f.gid);
    assert_eq!(f.t.call("PUT", &path, Some(&f.steve), Some(json!({"name": "Steel"}))).await.0, StatusCode::FORBIDDEN, "members can't");
    assert_eq!(
        f.t.call("PUT", &path, Some(&f.mia), Some(json!({"name": "Steel"}))).await.0,
        StatusCode::FORBIDDEN,
        "other guilds' leaders can't"
    );
    let (s, r) = f.t.call("PUT", &path, Some(&f.alex), Some(json!({"name": "  Steel Fortress ", "tag": "stl"}))).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    assert_eq!((r["name"].as_str(), r["tag"].as_str()), (Some("Steel Fortress"), Some("STL")));
    let (_, mine) = f.t.call("GET", "/api/v1/guilds/my?instance_id=smp", Some(&f.alex), None).await;
    assert_eq!(mine["name"], "Steel Fortress", "{mine}");

    for bad in [
        json!({"name": "ab"}),
        json!({"name": "x".repeat(33)}),
        json!({"name": "Fine", "tag": "no spaces"}),
        json!({"name": "Fine", "tag": "A"}),
    ] {
        assert_eq!(f.t.call("PUT", &path, Some(&f.alex), Some(bad)).await.0, StatusCode::BAD_REQUEST);
    }
    let (s, r) = f.t.call("PUT", &path, Some(&f.alex), Some(json!({"name": "void"}))).await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "taken by another guild, ignoring case: {r}");
    assert_eq!(
        f.t.call("PUT", &path, Some(&f.alex), Some(json!({"name": "steel fortress"}))).await.0,
        StatusCode::OK,
        "its own name in other letters is fine"
    );

    let (s, r) = f.t.call("PUT", &format!("/api/admin/guilds/{}", f.gid), Some(&f.admin), Some(json!({"name": "Admin Pick"}))).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    assert_eq!(
        f.t.call("PUT", &format!("/api/admin/guilds/{}", f.gid), Some(&f.steve), Some(json!({"name": "Nope"}))).await.0,
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
async fn disbanding_removes_everything_and_pays_the_treasury_to_the_leader() {
    let f = fixture().await;
    sqlx::query("INSERT INTO guild_wallets (server_id, guild_id, balance, updated_at) VALUES (?, ?, 300.5, 'now')")
        .bind(f.sid)
        .bind(&f.gid)
        .execute(&f.t.db)
        .await
        .unwrap();
    sqlx::query("INSERT INTO server_economy (server_id, uuid, username, balance, updated_at) VALUES (?, ?, 'Alex', 50, 'now')")
        .bind(f.sid)
        .bind(&f.alex_uuid)
        .execute(&f.t.db)
        .await
        .unwrap();
    sqlx::query("INSERT INTO guild_claims (guild_id, server_id, dimension, chunk_x, chunk_z, claimed_by_uuid, claimed_at) VALUES (?, ?, 'minecraft:overworld', 1, 2, ?, 'now')")
        .bind(&f.gid).bind(f.sid).bind(&f.alex_uuid).execute(&f.t.db).await.unwrap();

    let path = format!("/api/v1/guilds/{}", f.gid);
    assert_eq!(f.t.call("DELETE", &path, Some(&f.steve), None).await.0, StatusCode::FORBIDDEN, "members can't disband");
    assert_eq!(f.t.call("DELETE", &path, Some(&f.mia), None).await.0, StatusCode::FORBIDDEN);
    let count = |table: &'static str| {
        let (db, gid) = (f.t.db.clone(), f.gid.clone());
        async move {
            sqlx::query_scalar::<_, i64>(&format!("SELECT COUNT(*) FROM {table} WHERE guild_id = ?"))
                .bind(gid)
                .fetch_one(&db)
                .await
                .unwrap()
        }
    };
    assert_eq!(count("guild_members").await, 2, "nothing changed yet");

    let (s, r) = f.t.call("DELETE", &path, Some(&f.alex), None).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    assert_eq!(r["refunded"].as_f64(), Some(300.5));
    for table in ["guild_members", "guild_claims", "guild_wallets"] {
        assert_eq!(count(table).await, 0, "{table}");
    }
    let guilds: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM guilds WHERE id = ?").bind(&f.gid).fetch_one(&f.t.db).await.unwrap();
    assert_eq!(guilds, 0);
    let balance: f64 = sqlx::query_scalar("SELECT balance FROM server_economy WHERE server_id = ? AND uuid = ?")
        .bind(f.sid)
        .bind(&f.alex_uuid)
        .fetch_one(&f.t.db)
        .await
        .unwrap();
    assert_eq!(balance, 350.5, "the treasury went to the leader");
    // Members are free to join or found another guild.
    let (s, _) =
        f.t.call("POST", "/api/v1/guilds", Some(&f.steve), Some(json!({"instance_id": "smp", "name": "Fresh", "tag": "NEW"}))).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(f.t.call("DELETE", &path, Some(&f.alex), None).await.0, StatusCode::FORBIDDEN, "already gone");
}

#[tokio::test]
async fn the_game_can_rename_and_disband_too() {
    let f = fixture().await;
    let alex = f.alex_uuid.clone();
    let steve = f.t.uuid("Steve").await;
    let post = |path: &str, body: Value| {
        let (t, server) = (&f.t, f.server.clone());
        let path = format!("/api/server/v1/{path}");
        async move { t.call("POST", &path, Some(&server), Some(body)).await }
    };
    assert_eq!(post("guilds/rename", json!({"uuid": steve, "name": "Mutiny"})).await.0, StatusCode::FORBIDDEN);
    let (s, r) = post("guilds/rename", json!({"uuid": alex, "name": "Iron Guard", "tag": "IG"})).await;
    assert_eq!((s, r["tag"].as_str()), (StatusCode::OK, Some("IG")), "{r}");
    assert_eq!(post("guilds/disband", json!({"uuid": steve})).await.0, StatusCode::FORBIDDEN);
    assert_eq!(post("guilds/disband", json!({"uuid": alex})).await.0, StatusCode::OK);
    assert_eq!(post("guilds/disband", json!({"uuid": alex})).await.0, StatusCode::BAD_REQUEST, "no guild left");
}

#[tokio::test]
async fn members_get_bell_notifications_for_renames_and_disbands() {
    let f = fixture().await;
    let path = format!("/api/v1/guilds/{}/name", f.gid);
    assert_eq!(f.t.call("PUT", &path, Some(&f.alex), Some(json!({"name": "Steel"}))).await.0, StatusCode::OK);
    let (_, n) = f.t.call("GET", "/api/v1/notifications", Some(&f.steve), None).await;
    assert_eq!(n["unread"], 1);
    assert_eq!(n["items"][0]["kind"], "guild_renamed");
    assert!(n["items"][0]["body"].as_str().unwrap().contains("Steel"));
    let (_, other) = f.t.call("GET", "/api/v1/notifications", Some(&f.mia), None).await;
    assert_eq!(other["unread"], 0, "other guilds hear nothing");

    let id = n["items"][0]["id"].as_i64().unwrap();
    f.t.call("POST", "/api/v1/notifications/read", Some(&f.steve), Some(json!({"ids": [id]}))).await;
    let (_, n) = f.t.call("GET", "/api/v1/notifications", Some(&f.steve), None).await;
    assert_eq!(n["unread"], 0);

    let del = format!("/api/v1/guilds/{}", f.gid);
    f.t.call("DELETE", &del, Some(&f.alex), None).await;
    let (_, n) = f.t.call("GET", "/api/v1/notifications?unread=true", Some(&f.steve), None).await;
    assert_eq!(n["items"][0]["kind"], "guild_disbanded");
    f.t.call("DELETE", "/api/v1/notifications", Some(&f.steve), None).await;
    let (_, n) = f.t.call("GET", "/api/v1/notifications", Some(&f.steve), None).await;
    assert_eq!(n["items"].as_array().unwrap().len(), 0);
}
