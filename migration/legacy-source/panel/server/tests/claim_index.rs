//! The claim index game servers poll instead of asking about every block.

mod common;
use common::*;

#[tokio::test]
async fn claim_index_follows_claims_and_membership() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    let mut tokens = Vec::new();
    for n in ["Alex", "Steve"] {
        t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username": n, "password": "password123"}))).await;
        tokens.push(t.login(n, "password123").await);
    }
    let alex_uuid = t.uuid("Alex").await;
    let steve_uuid = t.uuid("Steve").await;
    let (_, srv) = t.call("POST", "/api/admin/servers", Some(&admin), Some(json!({"name": "SMP", "instance_id": "smp"}))).await;
    let sid = srv["server"]["id"].as_i64().unwrap();
    let token = srv["token"].as_str().unwrap().to_string();
    let index = |rev: Option<String>| {
        let (t, token) = (&t, token.clone());
        async move { t.call("POST", "/api/server/v1/guilds/claim-index", Some(&token), Some(json!({ "revision": rev }))).await }
    };

    let (s, empty) = index(None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(empty["unchanged"], false);
    assert!(empty["claims"].as_array().unwrap().is_empty());
    let rev0 = empty["revision"].as_str().unwrap().to_string();
    let (_, same) = index(Some(rev0.clone())).await;
    assert_eq!(same["unchanged"], true);
    assert!(same.get("claims").is_none());

    // A guild, then a claim: the revision moves and the index lists the chunk.
    let (_, g) =
        t.call("POST", "/api/v1/guilds", Some(&tokens[0]), Some(json!({"instance_id": "smp", "name": "Iron", "tag": "IRON"}))).await;
    let gid = g["id"].as_str().unwrap().to_string();
    let (_, r1) = index(Some(rev0.clone())).await;
    assert_eq!(r1["unchanged"], false, "guild creation bumps the revision");
    let claim = |x: i32| {
        let (t, tok, gid) = (&t, tokens[0].clone(), gid.clone());
        async move {
            t.call(
                "POST",
                &format!("/api/v1/guilds/{gid}/claim"),
                Some(&tok),
                Some(json!({"server_id": sid, "dimension": "minecraft:overworld", "chunk_x": x, "chunk_z": -4})),
            )
            .await
        }
    };
    let (s, c1) = claim(3).await;
    assert_eq!(s, StatusCode::OK, "{c1}");
    claim(4).await;
    let (_, idx) = index(Some(r1["revision"].as_str().unwrap().to_string())).await;
    assert_eq!(idx["unchanged"], false);
    assert_eq!(idx["claims"].as_array().unwrap().len(), 2);
    assert_eq!(idx["claims"][0], json!(["minecraft:overworld", 3, -4, 0]));
    assert_eq!(idx["guilds"][0]["tag"], "IRON");
    assert_eq!(idx["members"][&gid], json!([alex_uuid]));
    let rev2 = idx["revision"].as_str().unwrap().to_string();
    assert_eq!(index(Some(rev2.clone())).await.1["unchanged"], true);

    // Membership changes bump it too.
    sqlx::query("INSERT INTO guild_members (guild_id, uuid, name, role, joined_at) VALUES (?, ?, 'Steve', 'member', 'now')")
        .bind(&gid)
        .bind(&steve_uuid)
        .execute(&t.db)
        .await
        .unwrap();
    let (_, idx) = index(Some(rev2.clone())).await;
    assert_eq!(idx["unchanged"], false);
    assert_eq!(idx["members"][&gid].as_array().unwrap().len(), 2);
    let rev3 = idx["revision"].as_str().unwrap().to_string();

    // Unclaim, and deleting the guild (cascades the claims away).
    let cid = c1["id"].as_i64().unwrap();
    t.call("DELETE", &format!("/api/v1/guilds/claims/{cid}"), Some(&tokens[0]), None).await;
    let (_, idx) = index(Some(rev3)).await;
    assert_eq!(idx["claims"].as_array().unwrap().len(), 1);
    let rev4 = idx["revision"].as_str().unwrap().to_string();
    sqlx::query("DELETE FROM guilds WHERE id = ?").bind(&gid).execute(&t.db).await.unwrap();
    let (_, idx) = index(Some(rev4)).await;
    assert_eq!(idx["unchanged"], false, "cascaded deletes still fire the revision triggers");
    assert!(idx["claims"].as_array().unwrap().is_empty());

    // Unauthenticated callers get nothing.
    let (s, _) = t.call("POST", "/api/server/v1/guilds/claim-index", None, Some(json!({}))).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
}
