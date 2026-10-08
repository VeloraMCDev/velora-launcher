//! Buy orders and contracts end to end: escrow, picking an order up, filling it from in game, vault delivery and payment;
//! randomly generated contracts, kill progress from the game server's sync, and resource contracts that need real items.

mod common;
use common::*;

struct World {
    t: TestApp,
    admin: String,
    sid: i64,
    server: String,
    tok: std::collections::HashMap<&'static str, String>,
    uuid: std::collections::HashMap<&'static str, String>,
}

async fn world() -> World {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    let mut tok = std::collections::HashMap::new();
    let mut uuid = std::collections::HashMap::new();
    for n in ["Alex", "Steve", "Mia"] {
        t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username": n, "password": "password123"}))).await;
        tok.insert(n, t.login(n, "password123").await);
        uuid.insert(n, t.uuid(n).await);
    }
    let (_, srv) = t.call("POST", "/api/admin/servers", Some(&admin), Some(json!({"name": "SMP", "instance_id": "smp"}))).await;
    let sid = srv["server"]["id"].as_i64().unwrap();
    let server = srv["token"].as_str().unwrap().to_string();
    for (i, n) in ["Alex", "Steve", "Mia"].iter().enumerate() {
        t.call(
            "POST",
            "/api/server/v1/economy/adjust",
            Some(&server),
            Some(json!({"uuid": uuid[n], "username": n, "delta": 1000.0, "operation_id": format!("seed{i}"), "description": "seed"})),
        )
        .await;
        sqlx::query("UPDATE server_economy SET balance = 1000").execute(&t.db).await.unwrap();
        sqlx::query("INSERT INTO player_stats (server_id, uuid, name, first_seen, last_seen) VALUES (?, ?, ?, 'x', 'x')")
            .bind(sid)
            .bind(&uuid[n])
            .bind(n)
            .execute(&t.db)
            .await
            .unwrap();
    }
    World { t, admin, sid, server, tok, uuid }
}

impl World {
    async fn player(&self, who: &str, method: &str, path: &str, body: Option<Value>) -> (StatusCode, Value) {
        self.t.call(method, &format!("/api/v1/board/{}/{path}", self.sid), Some(&self.tok[who]), body).await
    }
    async fn game(&self, path: &str, body: Value) -> (StatusCode, Value) {
        self.t.call("POST", &format!("/api/server/v1/economy/{path}"), Some(&self.server), Some(body)).await
    }
    async fn balance(&self, who: &str) -> f64 {
        sqlx::query_scalar("SELECT balance FROM server_economy WHERE uuid = ?").bind(&self.uuid[who]).fetch_one(&self.t.db).await.unwrap()
    }
    fn op(&self) -> String {
        uuid::Uuid::new_v4().to_string()
    }
}

#[tokio::test]
async fn a_buy_order_is_escrowed_picked_up_filled_and_paid_once() {
    let w = world().await;
    // Posting an order takes the money straight away.
    let (s, r) = w.player("Alex", "POST", "orders", Some(json!({"item_id": "minecraft:cobblestone", "amount": 130, "total": 200.0}))).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    let id = r["id"].as_i64().unwrap();
    assert_eq!(w.balance("Alex").await, 800.0, "held in escrow");
    assert_eq!(
        w.player("Alex", "POST", "orders", Some(json!({"item_id": "DIAMOND", "amount": 0, "total": 10.0}))).await.0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        w.player("Mia", "POST", "orders", Some(json!({"item_id": "DIAMOND", "amount": 1, "total": 99999999.0}))).await.0,
        StatusCode::BAD_REQUEST
    );
    let broke = w.player("Mia", "POST", "orders", Some(json!({"item_id": "DIAMOND", "amount": 64, "total": 5000.0}))).await;
    assert_eq!(broke.0, StatusCode::BAD_REQUEST, "can't escrow what you don't have: {}", broke.1);
    assert_eq!(w.balance("Mia").await, 1000.0);

    // Everyone sees it on the board.
    let (_, b) = w.player("Steve", "GET", "orders", None).await;
    let o = &b["orders"][0];
    assert_eq!(
        (o["id"].as_i64(), o["item_id"].as_str(), o["status"].as_str(), o["mine"].as_bool()),
        (Some(id), Some("COBBLESTONE"), Some("open"), Some(false)),
        "{b}"
    );
    assert_eq!(o["each"], 1.54);

    // You cannot pick up or fill your own order; picking it up reserves it.
    assert_eq!(w.player("Alex", "POST", &format!("orders/{id}/claim"), Some(json!({}))).await.0, StatusCode::BAD_REQUEST);
    let (s, _) = w.player("Steve", "POST", &format!("orders/{id}/claim"), Some(json!({}))).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(w.player("Mia", "POST", &format!("orders/{id}/claim"), Some(json!({}))).await.0, StatusCode::CONFLICT, "already taken");
    let (_, b) = w.player("Mia", "GET", "orders", None).await;
    assert_eq!((b["orders"][0]["status"].as_str(), b["orders"][0]["claimer_name"].as_str()), (Some("claimed"), Some("Steve")));
    let fill = |who: &str, item: &str, n: i64, op: &str| json!({"operation_id": op, "uuid": w.uuid[who], "name": who, "order_id": id, "item_id": item, "amount": n});
    assert_eq!(w.game("orders/fill", fill("Mia", "COBBLESTONE", 130, &w.op())).await.0, StatusCode::CONFLICT, "reserved for Steve");

    // Wrong item, too few: refused, nothing moves.
    let (s, r) = w.game("orders/fill", fill("Steve", "STONE", 130, &w.op())).await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "{r}");
    let (s, r) = w.game("orders/fill", fill("Steve", "COBBLESTONE", 129, &w.op())).await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "{r}");
    assert_eq!(w.balance("Steve").await, 1000.0);

    // The real thing: more than needed, so the surplus comes back.
    let op = w.op();
    let (s, r) = w.game("orders/fill", fill("Steve", "cobblestone", 140, &op)).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    assert_eq!(r["paid"], 200.0);
    assert_eq!(r["items"][0]["amount"], 10, "surplus returned to the filler");
    assert_eq!(w.balance("Steve").await, 1200.0, "escrow released to the filler");
    assert_eq!(w.balance("Alex").await, 800.0, "the buyer already paid");
    // 130 items reach the buyer's vault in stacks of 64.
    let stacks: Vec<(String, i64, i64)> = sqlx::query_as("SELECT item_id, amount, to_vault FROM market_mailbox WHERE uuid = ? ORDER BY id")
        .bind(&w.uuid["Alex"])
        .fetch_all(&w.t.db)
        .await
        .unwrap();
    assert_eq!(stacks, vec![("COBBLESTONE".into(), 64, 1), ("COBBLESTONE".into(), 64, 1), ("COBBLESTONE".into(), 2, 1)]);
    // Replaying the same request does nothing more; a second fill is refused.
    let (_, again) = w.game("orders/fill", fill("Steve", "COBBLESTONE", 140, &op)).await;
    assert_eq!(again["paid"], 200.0);
    assert_eq!(w.balance("Steve").await, 1200.0);
    assert_eq!(w.game("orders/fill", fill("Mia", "COBBLESTONE", 130, &w.op())).await.0, StatusCode::CONFLICT, "already filled");
    let n: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM market_mailbox WHERE uuid = ?").bind(&w.uuid["Alex"]).fetch_one(&w.t.db).await.unwrap();
    assert_eq!(n, 3);
    // The vault pull the game server polls sees them.
    let (_, pull) = w.game("market/vault", json!({"limit": 10})).await;
    assert_eq!(pull["deliveries"].as_array().unwrap().len(), 3);
    let (_, hist) = w.player("Alex", "GET", "orders", None).await;
    assert_eq!(hist["history"][0]["resolved"], "filled");
    let (_, notes) = w.t.call("GET", "/api/v1/notifications", Some(&w.tok["Alex"]), None).await;
    assert!(notes.to_string().contains("buy order was filled"), "{notes}");
}

#[tokio::test]
async fn orders_can_be_cancelled_expire_and_be_released() {
    let w = world().await;
    let (_, r) = w
        .game(
            "orders/create",
            json!({"operation_id": w.op(), "uuid": w.uuid["Alex"], "name": "Alex", "item_id": "DIAMOND", "amount": 8, "total": 400.0}),
        )
        .await;
    let id = r["id"].as_i64().unwrap();
    assert_eq!(w.balance("Alex").await, 600.0);
    // Only the buyer can cancel; the money comes back.
    assert_eq!(w.player("Steve", "POST", &format!("orders/{id}/cancel"), Some(json!({}))).await.0, StatusCode::FORBIDDEN);
    let (s, r) = w.player("Alex", "POST", &format!("orders/{id}/cancel"), Some(json!({}))).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    assert_eq!(w.balance("Alex").await, 1000.0);
    assert_eq!(w.player("Alex", "POST", &format!("orders/{id}/cancel"), Some(json!({}))).await.0, StatusCode::CONFLICT, "already finished");

    // A limit on how many are open at once.
    for i in 0..5 {
        let (s, r) = w.player("Alex", "POST", "orders", Some(json!({"item_id": "COAL", "amount": 10, "total": 10.0 + i as f64}))).await;
        assert_eq!(s, StatusCode::OK, "{r}");
    }
    assert_eq!(
        w.player("Alex", "POST", "orders", Some(json!({"item_id": "COAL", "amount": 10, "total": 10.0}))).await.0,
        StatusCode::CONFLICT
    );

    // A reservation can be given up, and lapses by itself.
    let (_, b) = w.player("Steve", "GET", "orders", None).await;
    let oid = b["orders"][0]["id"].as_i64().unwrap();
    w.player("Steve", "POST", &format!("orders/{oid}/claim"), Some(json!({}))).await;
    assert_eq!(
        w.player("Mia", "POST", &format!("orders/{oid}/release"), Some(json!({}))).await.0,
        StatusCode::CONFLICT,
        "not yours to release"
    );
    assert_eq!(w.player("Steve", "POST", &format!("orders/{oid}/release"), Some(json!({}))).await.0, StatusCode::OK);
    assert_eq!(w.player("Mia", "POST", &format!("orders/{oid}/claim"), Some(json!({}))).await.0, StatusCode::OK);
    sqlx::query("UPDATE buy_orders SET claim_until = '2000-01-01T00:00:00Z' WHERE id = ?").bind(oid).execute(&w.t.db).await.unwrap();
    let (_, b) = w.player("Steve", "GET", "orders", None).await;
    assert_eq!(b["orders"][0]["status"], "open", "a lapsed reservation no longer counts");
    assert_eq!(w.player("Steve", "POST", &format!("orders/{oid}/claim"), Some(json!({}))).await.0, StatusCode::OK);

    // Unfilled orders expire and refund their escrow.
    let before = w.balance("Alex").await;
    sqlx::query("UPDATE buy_orders SET expires_at = '2000-01-01T00:00:00Z' WHERE buyer_uuid = ?")
        .bind(&w.uuid["Alex"])
        .execute(&w.t.db)
        .await
        .unwrap();
    let (s, r) = w.t.call("POST", "/api/admin/tasks/settle_orders/run", Some(&w.admin), None).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    assert_eq!(w.balance("Alex").await, 1000.0, "all five refunded (was {before})");
    let (_, b) = w.player("Alex", "GET", "orders", None).await;
    assert_eq!(b["orders"].as_array().unwrap().len(), 0);

    // Admin settings apply, and admins can remove an order.
    let (_, c) = w.t.call("GET", "/api/admin/economy/orders", Some(&w.admin), None).await;
    let mut cfg = c["config"].clone();
    cfg["fee_percent"] = json!(10.0);
    cfg["max_open_per_player"] = json!(500);
    let (s, saved) = w.t.call("PUT", "/api/admin/economy/orders", Some(&w.admin), Some(cfg)).await;
    assert_eq!(s, StatusCode::OK, "{saved}");
    assert_eq!(saved["config"]["max_open_per_player"], 100, "clamped");
    let (_, r) = w.player("Alex", "POST", "orders", Some(json!({"item_id": "IRON_INGOT", "amount": 10, "total": 100.0}))).await;
    let oid = r["id"].as_i64().unwrap();
    let (s, r) = w
        .game(
            "orders/fill",
            json!({"operation_id": w.op(), "uuid": w.uuid["Mia"], "name": "Mia", "order_id": oid, "item_id": "IRON_INGOT", "amount": 10}),
        )
        .await;
    assert_eq!((s, r["paid"].as_f64()), (StatusCode::OK, Some(90.0)), "the house keeps its 10%: {r}");
    let (_, r) = w.player("Alex", "POST", "orders", Some(json!({"item_id": "GOLD_INGOT", "amount": 10, "total": 100.0}))).await;
    let oid = r["id"].as_i64().unwrap();
    assert_eq!(w.t.call("POST", &format!("/api/admin/economy/orders/{oid}/cancel"), Some(&w.admin), None).await.0, StatusCode::OK);
    assert_eq!(w.balance("Alex").await, 900.0);
}

async fn kill(w: &World, who: &str, mob: &str, n: i64) {
    let (s, v) =
        w.t.call(
            "POST",
            "/api/server/v1/sync",
            Some(&w.server),
            Some(json!({
                "batch_id": w.op(), "online": [], "stats": [],
                "events": [{"uuid": w.uuid[who], "name": who, "kind": "action", "detail": format!("mob_kills:{mob} +{n}")}]
            })),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{v}");
}

#[tokio::test]
async fn contracts_are_generated_kills_count_and_pay_out() {
    let w = world().await;
    let (s, b) = w.player("Alex", "GET", "contracts", None).await;
    assert_eq!(s, StatusCode::OK, "{b}");
    let list = b["contracts"].as_array().unwrap();
    assert_eq!(list.len(), 4, "a full board");
    let mut targets: Vec<&str> = list.iter().map(|c| c["target"].as_str().unwrap()).collect();
    targets.sort();
    targets.dedup();
    assert_eq!(targets.len(), 4, "no repeats");
    assert!(list.iter().all(|c| c["reward"].as_f64().unwrap() > 0.0 && c["progress"] == 0));
    let (_, again) = w.player("Alex", "GET", "contracts", None).await;
    assert_eq!(again["contracts"], b["contracts"], "the board is stable between visits");
    let (_, other) = w.player("Steve", "GET", "contracts", None).await;
    assert_ne!(other["contracts"][0]["id"], b["contracts"][0]["id"], "everyone has their own");

    // Set up known contracts to test against.
    sqlx::query("DELETE FROM contracts WHERE uuid = ?").bind(&w.uuid["Alex"]).execute(&w.t.db).await.unwrap();
    for (kind, target, title, req, reward) in [
        ("kill", "SKELETON", "Kill 50 Skeletons", 50, 500.0),
        ("gather", "COBBLESTONE", "Submit 256x Cobblestone", 256, 300.0),
        ("gather", "DIAMOND", "Submit 8x Diamonds", 8, 400.0),
    ] {
        sqlx::query("INSERT INTO contracts (server_id, uuid, kind, target, title, required, reward, created_at, expires_at) VALUES (?, ?, ?, ?, ?, ?, ?, '2025-01-01T00:00:00Z', '2999-01-01T00:00:00Z')")
            .bind(w.sid).bind(&w.uuid["Alex"]).bind(kind).bind(target).bind(title).bind(req).bind(reward).execute(&w.t.db).await.unwrap();
    }
    // Kills only count for the right mob, and nothing else.
    kill(&w, "Alex", "ZOMBIE", 30).await;
    kill(&w, "Alex", "skeleton", 20).await;
    kill(&w, "Alex", "SKELETON@minecraft:overworld", 20).await;
    kill(&w, "Steve", "SKELETON", 99).await;
    let progress: i64 = sqlx::query_scalar("SELECT progress FROM contracts WHERE uuid = ? AND target = 'SKELETON'")
        .bind(&w.uuid["Alex"])
        .fetch_one(&w.t.db)
        .await
        .unwrap();
    assert_eq!(progress, 40);
    assert_eq!(w.balance("Alex").await, 1000.0);
    kill(&w, "Alex", "SKELETON", 25).await;
    assert_eq!(w.balance("Alex").await, 1500.0, "paid the moment it completes");
    let status: String = sqlx::query_scalar("SELECT status FROM contracts WHERE uuid = ? AND target = 'SKELETON'")
        .bind(&w.uuid["Alex"])
        .fetch_one(&w.t.db)
        .await
        .unwrap();
    assert_eq!(status, "completed");
    kill(&w, "Alex", "SKELETON", 25).await;
    assert_eq!(w.balance("Alex").await, 1500.0, "never paid twice");

    // Resource contracts only move when items are handed in.
    let sub = |item: &str, n: i64| json!({"operation_id": w.op(), "uuid": w.uuid["Alex"], "name": "Alex", "item_id": item, "amount": n});
    let (s, r) = w.game("contracts/submit", sub("DIRT", 64)).await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "no contract wants dirt: {r}");
    let (s, r) = w.game("contracts/submit", sub("COBBLESTONE", 200)).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    assert_eq!(r["items"].as_array().unwrap().len(), 0);
    let progress: i64 = sqlx::query_scalar("SELECT progress FROM contracts WHERE uuid = ? AND target = 'COBBLESTONE'")
        .bind(&w.uuid["Alex"])
        .fetch_one(&w.t.db)
        .await
        .unwrap();
    assert_eq!(progress, 200);
    let (_, r) = w.game("contracts/submit", sub("minecraft:cobblestone", 100)).await;
    assert_eq!(r["items"][0]["amount"], 44, "the extra comes back: {r}");
    assert_eq!(r["completed"], true);
    assert_eq!(w.balance("Alex").await, 1800.0);
    let op = w.op();
    let body = json!({"operation_id": op, "uuid": w.uuid["Alex"], "name": "Alex", "item_id": "DIAMOND", "amount": 8});
    let (_, first) = w.game("contracts/submit", body.clone()).await;
    let (_, replay) = w.game("contracts/submit", body).await;
    assert_eq!(first, replay);
    assert_eq!(w.balance("Alex").await, 2200.0, "a replayed submission pays once");
    // Stop refill while testing exhaustion: submitting normally generates a new board,
    // which can legitimately include another Diamond contract after this deletion.
    let (_, current) = w.t.call("GET", "/api/admin/economy/contracts", Some(&w.admin), None).await;
    let mut exhausted_config = current["config"].clone();
    let original_limit = exhausted_config["daily_limit"].clone();
    exhausted_config["daily_limit"] = json!(3);
    assert_eq!(w.t.call("PUT", "/api/admin/economy/contracts", Some(&w.admin), Some(exhausted_config.clone())).await.0, StatusCode::OK);
    sqlx::query("DELETE FROM contracts WHERE uuid = ? AND status = 'active' AND target = 'DIAMOND'")
        .bind(&w.uuid["Alex"])
        .execute(&w.t.db)
        .await
        .unwrap();
    assert_eq!(w.game("contracts/submit", sub("DIAMOND", 8)).await.0, StatusCode::BAD_REQUEST);
    exhausted_config["daily_limit"] = original_limit;
    assert_eq!(w.t.call("PUT", "/api/admin/economy/contracts", Some(&w.admin), Some(exhausted_config)).await.0, StatusCode::OK);

    // The board refills (new contracts replace finished ones, up to the daily limit).
    let (_, b) = w.player("Alex", "GET", "contracts", None).await;
    assert_eq!(b["contracts"].as_array().unwrap().len(), 4);
    assert_eq!(b["stats"]["done_today"], 3);
    assert_eq!(b["stats"]["completed"], 3);
    // Swapping one out is allowed a few times a day.
    let cid = b["contracts"][0]["id"].as_i64().unwrap();
    for i in 0..3 {
        let (_, bb) = w.player("Alex", "GET", "contracts", None).await;
        let id = bb["contracts"][0]["id"].as_i64().unwrap();
        let (s, r) = w.player("Alex", "POST", &format!("contracts/{id}/abandon"), Some(json!({}))).await;
        assert_eq!(s, StatusCode::OK, "swap {i}: {r}");
    }
    assert_eq!(
        w.player("Alex", "POST", &format!("contracts/{cid}/abandon"), Some(json!({}))).await.0,
        StatusCode::CONFLICT,
        "out of swaps for today"
    );

    // Daily limit and admin controls.
    let (_, c) = w.t.call("GET", "/api/admin/economy/contracts", Some(&w.admin), None).await;
    let mut cfg = c["config"].clone();
    cfg["daily_limit"] = json!(3);
    cfg["board_size"] = json!(6);
    w.t.call("PUT", "/api/admin/economy/contracts", Some(&w.admin), Some(cfg)).await;
    let (_, b) = w.player("Alex", "GET", "contracts", None).await;
    assert_eq!(b["stats"]["limit_reached"], true);
    sqlx::query("UPDATE contracts SET status = 'expired' WHERE uuid = ? AND status = 'active'")
        .bind(&w.uuid["Alex"])
        .execute(&w.t.db)
        .await
        .unwrap();
    let (_, b) = w.player("Alex", "GET", "contracts", None).await;
    assert_eq!(b["contracts"].as_array().unwrap().len(), 0, "no new contracts once today's limit is reached");
    // Expired contracts don't count.
    sqlx::query("INSERT INTO contracts (server_id, uuid, kind, target, title, required, reward, created_at, expires_at) VALUES (?, ?, 'kill', 'CREEPER', 'Kill 10 Creepers', 10, 100, '2000-01-01T00:00:00Z', '2000-01-02T00:00:00Z')").bind(w.sid).bind(&w.uuid["Mia"]).execute(&w.t.db).await.unwrap();
    kill(&w, "Mia", "CREEPER", 10).await;
    assert_eq!(w.balance("Mia").await, 1000.0);
}
