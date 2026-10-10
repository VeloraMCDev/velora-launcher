//! The casino end to end: instant games and Mines move real balances, the free spin is once a day, bounties are paid by the game
//! server's kill events, bets settle from stat changes, and admins configure and moderate all of it.

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
        let (s, _) = t.call("POST", "/api/server/v1/economy/adjust", Some(&server), Some(json!({"uuid": uuid[n], "username": n, "delta": 10000.0, "operation_id": format!("seed{i}"), "description": "seed"}))).await;
        assert_eq!(s, StatusCode::OK);
        sqlx::query("UPDATE server_economy SET balance = 10000").execute(&t.db).await.unwrap();
        sqlx::query("INSERT INTO player_stats (server_id, uuid, name, first_seen, last_seen) VALUES (?, ?, ?, 'x', 'x')").bind(sid).bind(&uuid[n]).bind(n).execute(&t.db).await.unwrap();
    }
    World { t, admin, sid, server, tok, uuid }
}

impl World {
    async fn post(&self, who: &str, path: &str, body: Value) -> (StatusCode, Value) {
        self.t.call("POST", &format!("/api/v1/casino/{}/{path}", self.sid), Some(&self.tok[who]), Some(body)).await
    }
    async fn get(&self, who: &str, path: &str) -> Value {
        let url = if path.is_empty() { format!("/api/v1/casino/{}", self.sid) } else { format!("/api/v1/casino/{}/{path}", self.sid) };
        let (s, v) = self.t.call("GET", &url, Some(&self.tok[who]), None).await;
        assert_eq!(s, StatusCode::OK, "{v}");
        v
    }
    async fn balance(&self, who: &str) -> f64 {
        self.get(who, "").await["balance"].as_f64().unwrap()
    }
    async fn settle(&self) {
        let (s, r) = self.t.call("POST", "/api/admin/tasks/settle_casino/run", Some(&self.admin), None).await;
        assert_eq!(s, StatusCode::OK, "{r}");
    }
    async fn kill(&self, killer: &str, victim: &str) {
        let batch = uuid::Uuid::new_v4().to_string();
        let (s, v) = self.t.call("POST", "/api/server/v1/sync", Some(&self.server), Some(json!({
            "batch_id": batch, "online": [], "stats": [],
            "events": [{"uuid": self.uuid[killer], "name": killer, "kind": "pvp_kill", "detail": self.uuid[victim]}]
        }))).await;
        assert_eq!(s, StatusCode::OK, "{v}");
    }
}

#[tokio::test]
async fn roulette_and_burst_receipts_prevent_duplicate_wagers() {
    let w = world().await;
    let wager = json!({"bet":100.0,"selection":"red","operation_id":uuid::Uuid::new_v4().to_string()});
    let (s, first) = w.post("Alex", "roulette", wager.clone()).await;
    assert_eq!(s, StatusCode::OK, "{first}");
    let (_, retry) = w.post("Alex", "roulette", wager).await;
    assert_eq!(first, retry, "retry must return the original spin");
    assert_eq!(w.get("Alex", "history").await["rounds_played"], 1);
    let before = w.balance("Alex").await;
    let start = json!({"bet":100.0,"operation_id":uuid::Uuid::new_v4().to_string()});
    let (s, round) = w.post("Alex", "burst/start", start.clone()).await;
    assert_eq!(s, StatusCode::OK, "{round}");
    assert_eq!(w.post("Alex", "burst/start", start).await.1, round);
    assert_eq!(w.balance("Alex").await, before - 100.0);
    let id = round["game"]["id"].as_i64().unwrap();
    assert_eq!(w.get("Alex", "").await["burst"]["id"], id);
    let cash = json!({"id":id,"expected_steps":0,"operation_id":uuid::Uuid::new_v4().to_string()});
    assert_eq!(w.post("Steve", "burst/cashout", cash.clone()).await.0, StatusCode::CONFLICT);
    let (s, receipt) = w.post("Alex", "burst/cashout", cash.clone()).await;
    assert_eq!(s, StatusCode::OK, "{receipt}");
    assert_eq!(receipt["payout"], 100.0, "cash out before risking a step returns the stake");
    assert_eq!(w.post("Alex", "burst/cashout", cash).await.1, receipt);
    assert_eq!(w.balance("Alex").await, before);
    assert_eq!(w.post("Alex", "burst/advance", json!({"id":id,"expected_steps":0,"operation_id":uuid::Uuid::new_v4().to_string()})).await.0, StatusCode::CONFLICT);
    assert_eq!(w.get("Alex", "history").await["rounds_played"], 2);
}

#[tokio::test]
async fn burst_steps_use_snapshotted_rules_and_reject_stale_actions() {
    let w = world().await;
    let (_, mut config) = w.t.call("GET", "/api/admin/casino", Some(&w.admin), None).await;
    config["config"]["burst"]["survival"] = json!(0.99);
    w.t.call("PUT", "/api/admin/casino", Some(&w.admin), Some(config["config"].clone())).await;
    let (_, start) = w.post("Alex", "burst/start", json!({"bet":100.0,"operation_id":uuid::Uuid::new_v4().to_string()})).await;
    let id = start["game"]["id"].as_i64().unwrap();
    // Deterministic fixture: the persisted round, rather than new settings, controls its odds.
    sqlx::query("UPDATE casino_burst SET survival=1 WHERE id=?").bind(id).execute(&w.t.db).await.unwrap();
    let next = json!({"id":id,"expected_steps":0,"operation_id":uuid::Uuid::new_v4().to_string()});
    let (s, step) = w.post("Alex", "burst/advance", next.clone()).await;
    assert_eq!(s, StatusCode::OK, "{step}");
    assert_eq!(step["game"]["steps"], 1);
    assert_eq!(w.post("Alex", "burst/advance", next).await.1, step);
    assert_eq!(w.post("Alex", "burst/advance", json!({"id":id,"expected_steps":0,"operation_id":uuid::Uuid::new_v4().to_string()})).await.0, StatusCode::CONFLICT);
    config["config"]["burst"]["enabled"] = json!(false);
    w.t.call("PUT", "/api/admin/casino", Some(&w.admin), Some(config["config"].clone())).await;
    assert_eq!(w.post("Alex", "burst/advance", json!({"id":id,"expected_steps":1,"operation_id":uuid::Uuid::new_v4().to_string()})).await.0, StatusCode::FORBIDDEN);
    assert_eq!(w.post("Alex", "burst/cashout", json!({"id":id,"expected_steps":1,"operation_id":uuid::Uuid::new_v4().to_string()})).await.0, StatusCode::OK);
}

#[tokio::test]
async fn instant_games_move_the_right_money() {
    let w = world().await;
    let state = w.get("Alex", "").await;
    assert_eq!(state["enabled"], true);
    assert_eq!(state["balance"], 10000.0);
    assert!(state["plinko_tables"]["high"]["16"].as_array().unwrap().len() == 17);
    assert!(state["rtp"]["slots"].as_f64().unwrap() < 1.0, "the house keeps an edge");

    let mut expected = 10000.0;
    for _ in 0..6 {
        let (s, r) = w.post("Alex", "slots", json!({"bet": 100.0})).await;
        assert_eq!(s, StatusCode::OK, "{r}");
        expected += r["profit"].as_f64().unwrap();
        assert_eq!(r["payout"].as_f64().unwrap(), (100.0 * r["multiplier"].as_f64().unwrap() * 100.0).round() / 100.0);
        assert_eq!(r["result"]["reels"].as_array().unwrap().len(), 3);
        assert!((r["balance"].as_f64().unwrap() - expected).abs() < 0.01, "{r}");
    }
    let (_, r) = w.post("Alex", "wheel", json!({"bet": 50.0})).await;
    expected += r["profit"].as_f64().unwrap();
    let (s, r) = w.post("Alex", "plinko", json!({"bet": 20.0, "rows": 12, "risk": "high"})).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    assert_eq!(r["result"]["path"].as_array().unwrap().len(), 12);
    expected += r["profit"].as_f64().unwrap();
    assert!((w.balance("Alex").await - expected).abs() < 0.01);

    // Limits and validation.
    assert_eq!(w.post("Alex", "slots", json!({"bet": 1.0})).await.0, StatusCode::BAD_REQUEST, "below the minimum");
    assert_eq!(w.post("Alex", "slots", json!({"bet": 99999999.0})).await.0, StatusCode::BAD_REQUEST, "above the maximum");
    assert_eq!(w.post("Alex", "plinko", json!({"bet": 20.0, "rows": 99, "risk": "high"})).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(w.post("Alex", "plinko", json!({"bet": 20.0, "rows": 10, "risk": "insane"})).await.0, StatusCode::BAD_REQUEST);
    let (_, poor) = w.t.call("POST", "/api/server/v1/economy/adjust", Some(&w.server), Some(json!({"uuid": w.uuid["Mia"], "username": "Mia", "delta": -9995.0, "operation_id": "drain", "description": "x"}))).await;
    assert_eq!(poor["balance"], 5.0);
    assert_eq!(w.post("Mia", "slots", json!({"bet": 10.0})).await.0, StatusCode::BAD_REQUEST, "cannot afford it");
    assert_eq!(w.balance("Mia").await, 5.0, "a refused bet costs nothing");

    // Rounds are on record and show in the ledger.
    let h = w.get("Alex", "history").await;
    assert_eq!(h["rounds_played"], 8);
    let (_, tx) = w.t.call("GET", "/api/v1/economy/transactions", Some(&w.tok["Alex"]), None).await;
    assert!(tx.as_array().unwrap().iter().any(|t| t["description"].as_str().unwrap().starts_with("Casino: Slots")));
}

#[tokio::test]
async fn mines_plays_out_and_pays_the_cash_out() {
    let w = world().await;
    assert_eq!(w.post("Alex", "mines/reveal", json!({"tile": 0})).await.0, StatusCode::NOT_FOUND);
    assert_eq!(w.post("Alex", "mines/start", json!({"bet": 100.0, "mines": 99})).await.0, StatusCode::BAD_REQUEST);
    let (s, r) = w.post("Alex", "mines/start", json!({"bet": 100.0, "mines": 3})).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    assert_eq!(r["balance"], 9900.0);
    assert!(r["game"].get("layout").is_none(), "mines stay hidden while the game runs");
    assert_eq!(w.post("Alex", "mines/start", json!({"bet": 100.0, "mines": 3})).await.0, StatusCode::CONFLICT, "one game at a time");
    assert_eq!(w.post("Alex", "mines/cashout", json!({})).await.0, StatusCode::BAD_REQUEST, "nothing turned over yet");
    assert!(w.get("Alex", "").await["mines"]["id"].is_i64(), "a running game can be resumed");

    let layout: Vec<u32> = serde_json::from_str(&sqlx::query_scalar::<_, String>("SELECT layout FROM casino_mines WHERE uuid = ?").bind(&w.uuid["Alex"]).fetch_one(&w.t.db).await.unwrap()).unwrap();
    let safe: Vec<u32> = (0..25).filter(|t| !layout.contains(t)).collect();
    for tile in &safe[..3] {
        let (s, r) = w.post("Alex", "mines/reveal", json!({"tile": tile})).await;
        assert_eq!(s, StatusCode::OK, "{r}");
        assert_eq!(r["game"]["status"], "active");
    }
    assert_eq!(w.post("Alex", "mines/reveal", json!({"tile": safe[0]})).await.0, StatusCode::BAD_REQUEST, "already turned over");
    let (s, r) = w.post("Alex", "mines/cashout", json!({})).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    let payout = r["payout"].as_f64().unwrap();
    assert!(payout > 100.0, "three safe tiles pay more than the bet: {r}");
    assert_eq!(r["game"]["layout"].as_array().unwrap().len(), 3, "the board is shown once it's over");
    assert!((r["balance"].as_f64().unwrap() - (9900.0 + payout)).abs() < 0.01);

    // Stepping on a mine loses the bet.
    w.post("Alex", "mines/start", json!({"bet": 200.0, "mines": 5})).await;
    let layout: Vec<u32> = serde_json::from_str(&sqlx::query_scalar::<_, String>("SELECT layout FROM casino_mines WHERE uuid = ? AND status = 'active'").bind(&w.uuid["Alex"]).fetch_one(&w.t.db).await.unwrap()).unwrap();
    let before = w.balance("Alex").await;
    let (_, r) = w.post("Alex", "mines/reveal", json!({"tile": layout[0]})).await;
    assert_eq!((r["game"]["status"].as_str(), r["payout"].as_f64()), (Some("lost"), Some(0.0)), "{r}");
    assert_eq!(w.balance("Alex").await, before);
    assert_eq!(w.post("Alex", "mines/cashout", json!({})).await.0, StatusCode::NOT_FOUND, "the game is over");
}

#[tokio::test]
async fn one_free_spin_a_day_and_admin_settings_apply() {
    let w = world().await;
    let (s, r) = w.post("Alex", "daily", json!({})).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    assert!(r["payout"].as_f64().unwrap() > 0.0);
    assert_eq!(r["left"], 0);
    assert!((w.balance("Alex").await - (10000.0 + r["payout"].as_f64().unwrap())).abs() < 0.01);
    assert_eq!(w.post("Alex", "daily", json!({})).await.0, StatusCode::CONFLICT, "only one a day");
    assert_eq!(w.post("Mia", "daily", json!({})).await.0, StatusCode::OK, "each player has their own");
    assert_eq!(w.get("Alex", "").await["free"]["left"], 0);

    // Players cannot configure; admins can, and it takes effect immediately.
    assert_eq!(w.t.call("GET", "/api/admin/casino", Some(&w.tok["Alex"]), None).await.0, StatusCode::FORBIDDEN);
    let (s, cfg) = w.t.call("GET", "/api/admin/casino", Some(&w.admin), None).await;
    assert_eq!(s, StatusCode::OK, "{cfg}");
    let mut c = cfg["config"].clone();
    c["daily"]["spins_per_day"] = json!(3);
    c["slots"]["enabled"] = json!(false);
    c["slots"]["symbols"] = json!([]);
    c["mines"]["size"] = json!(9);
    let (s, saved) = w.t.call("PUT", "/api/admin/casino", Some(&w.admin), Some(c)).await;
    assert_eq!(s, StatusCode::OK, "{saved}");
    assert!(saved["config"]["slots"]["symbols"].as_array().unwrap().len() >= 2, "an emptied reel set is repaired");
    assert_eq!(saved["config"]["mines"]["size"], 7, "clamped");
    assert!(saved["preview"]["slots"]["rtp"].as_f64().unwrap() > 0.5);
    assert_eq!(w.post("Alex", "daily", json!({})).await.0, StatusCode::OK, "more spins allowed now");
    assert_eq!(w.post("Alex", "slots", json!({"bet": 10.0})).await.0, StatusCode::FORBIDDEN, "slots are switched off");

    // A live preview of unsaved settings.
    let (s, p) = w.t.call("POST", "/api/admin/casino/preview", Some(&w.admin), Some(json!({"plinko": {"rtp": 0.9}}))).await;
    assert_eq!(s, StatusCode::OK);
    assert!((p["preview"]["plinko"]["low"]["10"]["rtp"].as_f64().unwrap() - 0.9).abs() < 0.01);

    // A whole casino switch and a daily loss limit.
    let mut c = saved["config"].clone();
    c["slots"]["enabled"] = json!(true);
    c["daily_loss_limit"] = json!(150.0);
    w.t.call("PUT", "/api/admin/casino", Some(&w.admin), Some(c.clone())).await;
    sqlx::query("INSERT INTO casino_rounds (server_id, uuid, name, game, bet, payout, created_at) VALUES (?, ?, 'Steve', 'wheel', 500, 0, ?)")
        .bind(w.sid).bind(&w.uuid["Steve"]).bind(velora_panel::db::now()).execute(&w.t.db).await.unwrap();
    let (s, r) = w.post("Steve", "wheel", json!({"bet": 100.0})).await;
    assert_eq!(s, StatusCode::FORBIDDEN, "{r}");
    assert!(r["error"].as_str().unwrap().contains("loss limit"), "{r}");
    assert_eq!(w.post("Mia", "wheel", json!({"bet": 100.0})).await.0, StatusCode::OK, "the limit is per player");
    c["enabled"] = json!(false);
    w.t.call("PUT", "/api/admin/casino", Some(&w.admin), Some(c)).await;
    assert_eq!(w.post("Alex", "wheel", json!({"bet": 10.0})).await.0, StatusCode::FORBIDDEN);
    assert_eq!(w.get("Alex", "").await["enabled"], false, "the lobby still loads so it can say it's closed");

    let (_, stats) = w.t.call("GET", "/api/admin/casino/stats", Some(&w.admin), None).await;
    assert!(stats["games"].as_array().unwrap().iter().any(|g| g["game"] == "daily"), "{stats}");
}

#[tokio::test]
async fn bounties_are_escrowed_claimed_by_kills_and_expire() {
    let w = world().await;
    let sid = w.sid;
    // Bad requests.
    assert_eq!(w.post("Alex", "bounties", json!({"target": "Alex", "amount": 500.0})).await.0, StatusCode::BAD_REQUEST, "not yourself");
    assert_eq!(w.post("Alex", "bounties", json!({"target": "Nobody", "amount": 500.0})).await.0, StatusCode::NOT_FOUND);
    assert_eq!(w.post("Alex", "bounties", json!({"target": "Steve", "amount": 5.0})).await.0, StatusCode::BAD_REQUEST, "below the minimum");

    let (s, r) = w.post("Alex", "bounties", json!({"target": "steve", "amount": 1000.0, "anonymous": true})).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    assert_eq!(r["reward"], 900.0, "10% tax");
    assert_eq!(w.balance("Alex").await, 9000.0, "the money is held");
    w.post("Mia", "bounties", json!({"target": "Steve", "amount": 100.0})).await;
    let board = w.get("Mia", "bounties").await;
    assert_eq!(board["board"][0]["name"], "Steve");
    assert_eq!(board["board"][0]["total"], 990.0);
    assert_eq!(board["board"][0]["by"], "Mia", "anonymous placers stay hidden");
    assert_eq!(w.get("Steve", "").await["bounty_on_me"], 990.0);
    let (_, bell) = w.t.call("GET", "/api/v1/notifications", Some(&w.tok["Steve"]), None).await;
    assert!(bell["items"].as_array().unwrap().iter().any(|i| i["kind"] == "bounty"));

    // Killing the target pays everyone else's bounty, but never your own.
    w.kill("Steve", "Steve").await;
    w.kill("Mia", "Steve").await;
    assert_eq!(w.balance("Mia").await, 10000.0 - 100.0 + 900.0, "Mia collects Alex's bounty but not her own");
    let board = w.get("Mia", "bounties").await;
    assert_eq!(board["board"].as_array().unwrap().len(), 1, "Mia's own bounty is still up");
    assert_eq!(board["recent"][0]["killer"], "Mia");

    // The same pair cannot farm a bounty within the cooldown.
    w.post("Alex", "bounties", json!({"target": "Steve", "amount": 200.0})).await;
    w.kill("Mia", "Steve").await;
    assert_eq!(w.balance("Mia").await, 10800.0, "cooldown applies");
    w.kill("Alex", "Steve").await;
    assert_eq!(w.balance("Alex").await, 10000.0 - 1000.0 - 200.0 + 90.0, "placers can't collect their own");

    // Cancelling returns the reward (not the tax); expiry does the same.
    let (_, mine) = (0, w.get("Alex", "bounties").await);
    let id = mine["mine"][0]["id"].as_i64().unwrap();
    let before = w.balance("Alex").await;
    assert_eq!(w.t.call("POST", &format!("/api/v1/casino/{sid}/bounties/{id}/cancel"), Some(&w.tok["Mia"]), None).await.0, StatusCode::NOT_FOUND, "only the placer");
    let (s, r) = w.t.call("POST", &format!("/api/v1/casino/{sid}/bounties/{id}/cancel"), Some(&w.tok["Alex"]), None).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    assert!((w.balance("Alex").await - before - r["refunded"].as_f64().unwrap()).abs() < 0.01);
    w.post("Alex", "bounties", json!({"target": "Mia", "amount": 400.0})).await;
    sqlx::query("UPDATE casino_bounties SET expires_at = '2000-01-01T00:00:00Z' WHERE status = 'active'").execute(&w.t.db).await.unwrap();
    let before = w.balance("Alex").await;
    w.settle().await;
    assert!(w.balance("Alex").await > before, "expired bounties are refunded");
    assert_eq!(w.get("Alex", "bounties").await["board"].as_array().unwrap().len(), 0);

    // Admin moderation lists everything.
    let (s, all) = w.t.call("GET", "/api/admin/casino/bounties", Some(&w.admin), None).await;
    assert_eq!(s, StatusCode::OK);
    assert!(all["bounties"].as_array().unwrap().len() >= 4);
}

#[tokio::test]
async fn bets_settle_from_stats_with_pool_payouts_and_refunds() {
    let w = world().await;
    let sid = w.sid;
    let bet = |who: &'static str, id: i64, side: &str, stake: f64| {
        let (w, side) = (&w, side.to_string());
        async move { w.post(who, &format!("markets/{id}/bet"), json!({"side": side, "stake": stake})).await }
    };
    assert_eq!(w.post("Mia", "markets", json!({"subject": "Steve", "metric": "nonsense", "threshold": 2, "window_minutes": 30})).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(w.post("Mia", "markets", json!({"subject": "Steve", "metric": "player_kills", "threshold": 2, "window_minutes": 7})).await.0, StatusCode::BAD_REQUEST, "window must be offered");
    let (s, m) = w.post("Mia", "markets", json!({"subject": "Steve", "metric": "player_kills", "threshold": 2, "window_minutes": 30})).await;
    assert_eq!(s, StatusCode::OK, "{m}");
    let id = m["id"].as_i64().unwrap();

    assert_eq!(bet("Steve", id, "yes", 100.0).await.0, StatusCode::FORBIDDEN, "no betting on yourself");
    assert_eq!(bet("Alex", id, "yes", 1.0).await.0, StatusCode::BAD_REQUEST, "stake too small");
    assert_eq!(bet("Alex", id, "yes", 100.0).await.0, StatusCode::OK);
    assert_eq!(bet("Alex", id, "no", 100.0).await.0, StatusCode::CONFLICT, "no hedging");
    assert_eq!(bet("Mia", id, "no", 300.0).await.0, StatusCode::OK);
    let list = w.get("Alex", "markets").await;
    let m = &list["open"][0];
    assert_eq!((m["yes_pool"].as_f64(), m["no_pool"].as_f64(), m["bettors"].as_i64()), (Some(100.0), Some(300.0), Some(2)));
    assert_eq!(m["odds_yes"], 3.8);
    assert_eq!(m["mine"][0]["side"], "yes");

    // Steve gets two kills; the bet pays Alex's side once the time is up.
    sqlx::query("UPDATE player_stats SET player_kills = player_kills + 2 WHERE uuid = ?").bind(&w.uuid["Steve"]).execute(&w.t.db).await.unwrap();
    w.settle().await;
    assert_eq!(w.get("Alex", "markets").await["open"].as_array().unwrap().len(), 1, "not over yet");
    sqlx::query("UPDATE casino_markets SET locks_at = '2000-01-01T00:00:00Z', ends_at = '2000-01-01T00:00:00Z'").execute(&w.t.db).await.unwrap();
    assert_eq!(bet("Alex", id, "yes", 100.0).await.0, StatusCode::CONFLICT, "betting closed");
    w.settle().await;
    assert_eq!(w.balance("Alex").await, 10000.0 - 100.0 + 380.0, "yes takes the pool less the 5% rake");
    assert_eq!(w.balance("Mia").await, 10000.0 - 300.0);
    let done = w.get("Alex", "markets").await;
    assert_eq!((done["done"][0]["status"].as_str(), done["done"][0]["outcome"].as_str()), (Some("settled"), Some("yes")));
    w.settle().await;
    assert_eq!(w.balance("Alex").await, 10000.0 - 100.0 + 380.0, "settling is never paid twice");

    // With nobody on the other side every stake comes back.
    let (_, m2) = w.post("Mia", "markets", json!({"subject": "Steve", "metric": "deaths", "threshold": 1, "window_minutes": 60})).await;
    let id2 = m2["id"].as_i64().unwrap();
    bet("Alex", id2, "no", 50.0).await;
    sqlx::query("UPDATE casino_markets SET locks_at = '2000-01-01T00:00:00Z', ends_at = '2000-01-01T00:00:00Z' WHERE id = ?").bind(id2).execute(&w.t.db).await.unwrap();
    let before = w.balance("Alex").await;
    w.settle().await;
    assert_eq!(w.balance("Alex").await, before + 50.0);

    // Cancelling an open bet refunds everyone; only its creator or an admin can.
    let (_, m3) = w.post("Mia", "markets", json!({"subject": "Alex", "metric": "mob_kills", "threshold": 5, "window_minutes": 360})).await;
    let id3 = m3["id"].as_i64().unwrap();
    bet("Steve", id3, "yes", 100.0).await;
    let steve = w.balance("Steve").await;
    assert_eq!(w.t.call("POST", &format!("/api/v1/casino/{sid}/markets/{id3}/cancel"), Some(&w.tok["Steve"]), None).await.0, StatusCode::FORBIDDEN);
    assert_eq!(w.t.call("POST", &format!("/api/admin/casino/markets/{id3}/void"), Some(&w.admin), None).await.0, StatusCode::OK);
    assert_eq!(w.balance("Steve").await, steve + 100.0);

    // Admins can restrict who opens bets.
    let (_, cfg) = w.t.call("GET", "/api/admin/casino", Some(&w.admin), None).await;
    let mut c = cfg["config"].clone();
    c["betting"]["creators"] = json!("admins");
    w.t.call("PUT", "/api/admin/casino", Some(&w.admin), Some(c)).await;
    assert_eq!(w.post("Mia", "markets", json!({"subject": "Steve", "metric": "deaths", "threshold": 1, "window_minutes": 60})).await.0, StatusCode::FORBIDDEN);
    assert_eq!(w.get("Mia", "markets").await["can_create"], false);
    let (_, players) = w.t.call("GET", &format!("/api/v1/casino/{sid}/players?q=ste"), Some(&w.tok["Mia"]), None).await;
    assert_eq!(players["players"][0]["name"], "Steve");
}

// ---- the newer games: dice, coin flip, crash, blackjack, double or nothing and chaos ----

impl World {
    async fn configure(&self, edit: impl FnOnce(&mut Value)) {
        let (_, cfg) = self.t.call("GET", "/api/admin/casino", Some(&self.admin), None).await;
        let mut c = cfg["config"].clone();
        edit(&mut c);
        let (s, r) = self.t.call("PUT", "/api/admin/casino", Some(&self.admin), Some(c)).await;
        assert_eq!(s, StatusCode::OK, "{r}");
    }
}

#[tokio::test]
async fn dice_and_coin_flip_pay_what_they_say() {
    let w = world().await;
    w.configure(|c| c["chaos"]["enabled"] = json!(false)).await;
    let state = w.get("Alex", "").await;
    assert!(state["config"]["dice"]["enabled"].as_bool().unwrap() && state["config"]["coinflip"]["payout"].as_f64().unwrap() < 2.0);

    let mut expected = 10000.0;
    let (mut won, mut lost) = (0, 0);
    for i in 0..40 {
        let (s, r) = w.post("Alex", "dice", json!({"bet": 100.0, "chance": 50.0, "mode": if i % 2 == 0 { "under" } else { "over" }})).await;
        assert_eq!(s, StatusCode::OK, "{r}");
        let roll = r["result"]["roll"].as_f64().unwrap();
        assert!((0.0..100.0).contains(&roll));
        let win = r["result"]["win"].as_bool().unwrap();
        let over = i % 2 == 1;
        assert_eq!(win, if over { roll >= 50.0 } else { roll < 50.0 }, "{r}");
        assert_eq!(r["payout"].as_f64().unwrap(), if win { 194.0 } else { 0.0 }, "50% pays 1.94x at a 3% edge: {r}");
        assert_eq!(r["double"].is_object(), win, "a win offers Double or Nothing and a loss does not");
        if win { won += 1 } else { lost += 1 }
        expected += r["profit"].as_f64().unwrap();
    }
    assert!(won > 0 && lost > 0, "forty coin tosses should not all land one way ({won}/{lost})");
    assert!((w.balance("Alex").await - expected).abs() < 0.01);

    let (s, r) = w.post("Alex", "dice", json!({"bet": 100.0, "chance": 10.0, "mode": "under"})).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    assert!(r["multiplier"].as_f64().unwrap() == 0.0 || (r["multiplier"].as_f64().unwrap() - 9.7).abs() < 0.01, "10% pays 9.7x: {r}");
    assert_eq!(w.post("Alex", "dice", json!({"bet": 100.0, "chance": 99.0})).await.0, StatusCode::BAD_REQUEST, "above the allowed chance");
    assert_eq!(w.post("Alex", "dice", json!({"bet": 100.0, "chance": 0.0})).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(w.post("Alex", "dice", json!({"bet": 100.0, "chance": null})).await.0, StatusCode::UNPROCESSABLE_ENTITY);

    let (before, mut heads) = (w.balance("Alex").await, 0);
    for _ in 0..30 {
        let (s, r) = w.post("Alex", "coinflip", json!({"bet": 50.0, "side": "heads"})).await;
        assert_eq!(s, StatusCode::OK, "{r}");
        let landed = r["result"]["landed"].as_str().unwrap();
        assert!(landed == "heads" || landed == "tails");
        assert_eq!(r["payout"].as_f64().unwrap(), if landed == "heads" { 98.0 } else { 0.0 });
        if landed == "heads" { heads += 1 }
    }
    assert!((1..30).contains(&heads), "the coin lands both ways");
    assert!(w.balance("Alex").await != before);
    assert_eq!(w.post("Alex", "coinflip", json!({"bet": 50.0, "side": "edge"})).await.0, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn chaos_twists_wins_and_can_be_switched_off() {
    let w = world().await;
    w.configure(|c| { c["chaos"]["surge_chance"] = json!(0.5); c["chaos"]["curse_chance"] = json!(0.5); c["dice"]["house_edge"] = json!(0.0); }).await;
    // Chaos is opt-in: without the player's switch there are no twists, however the admin set the odds.
    for _ in 0..20 {
        let (_, r) = w.post("Alex", "dice", json!({"bet": 10.0, "chance": 95.0, "mode": "under"})).await;
        assert!(r["result"]["twist"].is_null(), "no twists unless the player turns Chaos on: {r}");
    }
    let (mut surge, mut curse) = (0, 0);
    for _ in 0..40 {
        let (s, r) = w.post("Alex", "dice", json!({"bet": 100.0, "chance": 95.0, "mode": "under", "chaos": true})).await;
        assert_eq!(s, StatusCode::OK, "{r}");
        match r["result"]["twist"]["kind"].as_str() {
            Some("surge") => { surge += 1; assert!(r["multiplier"].as_f64().unwrap() >= 1.5 * 1.05 - 0.02); }
            Some("curse") => { curse += 1; assert!((r["multiplier"].as_f64().unwrap() - 0.5 * 100.0 / 95.0).abs() < 0.02, "{r}"); }
            _ => assert!(r["result"]["twist"].is_null()),
        }
    }
    assert!(surge > 0 && curse > 0, "with 50/50 odds both twists show up ({surge}/{curse})");
    assert!(w.get("Alex", "").await["rtp"]["chaos"].is_number());
    w.configure(|c| c["chaos"]["enabled"] = json!(false)).await;
    for _ in 0..10 {
        let (_, r) = w.post("Alex", "dice", json!({"bet": 100.0, "chance": 95.0, "chaos": true})).await;
        assert!(r["result"]["twist"].is_null(), "no twists when chaos is off: {r}");
    }
}

#[tokio::test]
async fn double_or_nothing_is_offered_after_a_win_and_paid_once() {
    let w = world().await;
    w.configure(|c| { c["chaos"]["enabled"] = json!(false); c["double"]["win_chance"] = json!(0.95); c["double"]["max_streak"] = json!(3); }).await;
    // Win something first.
    let mut offer = Value::Null;
    for _ in 0..60 {
        let (_, r) = w.post("Alex", "coinflip", json!({"bet": 100.0, "side": "heads"})).await;
        if r["double"].is_object() { offer = r["double"].clone(); break; }
        assert!(w.get("Alex", "").await["double"].is_null(), "no offer after a loss");
    }
    assert!(offer.is_object(), "a win offers a double");
    assert_eq!(offer["stake"], 196.0);
    assert_eq!(offer["payout"], 392.0);
    assert_eq!(w.get("Alex", "").await["double"]["id"], offer["id"], "the offer survives a reload");
    let id = offer["id"].as_i64().unwrap();

    assert_eq!(w.post("Steve", "double", json!({"id": id})).await.0, StatusCode::CONFLICT, "someone else's offer is not yours");
    let before = w.balance("Alex").await;
    let (s, r) = w.post("Alex", "double", json!({"id": id})).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    assert_eq!(r["streak"], 1);
    let after = w.balance("Alex").await;
    if r["won"].as_bool().unwrap() {
        assert!((after - (before + 196.0)).abs() < 0.01, "the winnings doubled: {r}");
        assert!(r["double"].is_object(), "a win can be doubled again");
    } else {
        assert!((after - (before - 196.0)).abs() < 0.01, "or the winnings are gone: {r}");
        assert!(r["double"].is_null());
    }
    assert_eq!(w.post("Alex", "double", json!({"id": id})).await.0, StatusCode::CONFLICT, "an offer is used once");
    assert_eq!(w.balance("Alex").await, after, "the refused second try cost nothing");

    // Streak limit: keep doubling until offers stop, never more than max_streak (3) in a row.
    let mut next = r["double"].clone();
    let mut taken = 1;
    while next.is_object() {
        let (s, r) = w.post("Alex", "double", json!({"id": next["id"]})).await;
        assert_eq!(s, StatusCode::OK, "{r}");
        taken += 1;
        next = r["double"].clone();
    }
    assert!(taken <= 3, "streak limit respected: {taken}");

    // An expired offer cannot be taken.
    sqlx::query("INSERT INTO casino_double (server_id, uuid, stake, streak, expires_at, created_at) VALUES (?, ?, 50, 0, '2000-01-01T00:00:00Z', '2000-01-01T00:00:00Z')").bind(w.sid).bind(&w.uuid["Alex"]).execute(&w.t.db).await.unwrap();
    let id: i64 = sqlx::query_scalar("SELECT MAX(id) FROM casino_double").fetch_one(&w.t.db).await.unwrap();
    assert_eq!(w.post("Alex", "double", json!({"id": id})).await.0, StatusCode::CONFLICT);

    // Admins can turn it off.
    w.configure(|c| c["double"]["enabled"] = json!(false)).await;
    for _ in 0..30 {
        let (_, r) = w.post("Alex", "coinflip", json!({"bet": 10.0, "side": "heads"})).await;
        assert!(r["double"].is_null(), "{r}");
    }
}

#[tokio::test]
async fn crash_busts_or_cashes_out_at_the_climbing_multiplier() {
    let w = world().await;
    w.configure(|c| c["chaos"]["enabled"] = json!(false)).await;
    assert_eq!(w.post("Alex", "crash/cashout", json!({})).await.0, StatusCode::NOT_FOUND);
    assert_eq!(w.post("Alex", "crash/start", json!({"bet": 100.0, "auto": 0.5})).await.0, StatusCode::BAD_REQUEST);
    let (s, r) = w.post("Alex", "crash/start", json!({"bet": 100.0})).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    assert_eq!(r["balance"], 9900.0);
    assert!(r["game"].get("crash_point").is_none(), "the crash point is secret while the round runs");
    // The crash point is random: a round can bust the instant it starts, so pin it before checking "one round at a time".
    sqlx::query("UPDATE casino_crash SET status = 'active', crash_point = 50.0").execute(&w.t.db).await.unwrap();
    assert_eq!(w.post("Alex", "crash/start", json!({"bet": 100.0})).await.0, StatusCode::CONFLICT, "one round at a time");
    assert!(w.get("Alex", "").await["crash"]["id"].is_i64(), "a running round can be resumed");

    // Make the round long enough to cash out in, then cash out ~5 seconds in (1.82x).
    sqlx::query("UPDATE casino_crash SET crash_point = 50.0, started_ms = started_ms - 5000").execute(&w.t.db).await.unwrap();
    let live = w.get("Alex", "crash/status").await;
    assert_eq!(live["game"]["status"], "active");
    let m = live["game"]["multiplier"].as_f64().unwrap();
    assert!((1.7..2.2).contains(&m), "{m}");
    let (s, r) = w.post("Alex", "crash/cashout", json!({})).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    let payout = r["payout"].as_f64().unwrap();
    assert!((170.0..220.0).contains(&payout), "{r}");
    assert_eq!(r["game"]["crash_point"], 50.0, "revealed once it is over");
    assert!((r["balance"].as_f64().unwrap() - (9900.0 + payout)).abs() < 0.01);
    assert!(r["double"].is_object(), "crash wins can be doubled");
    assert_eq!(w.post("Alex", "crash/cashout", json!({})).await.0, StatusCode::NOT_FOUND, "already cashed out");

    // A round that has already crashed loses, however late the cash-out arrives.
    w.post("Alex", "crash/start", json!({"bet": 100.0})).await;
    sqlx::query("UPDATE casino_crash SET status = 'active', crash_point = 1.2, started_ms = started_ms - 5000 WHERE id = (SELECT MAX(id) FROM casino_crash)").execute(&w.t.db).await.unwrap();
    let before = w.balance("Alex").await;
    let (s, r) = w.post("Alex", "crash/cashout", json!({})).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    assert_eq!((r["payout"].as_f64(), r["game"]["status"].as_str()), (Some(0.0), Some("busted")), "{r}");
    assert_eq!(w.balance("Alex").await, before);

    // The page polls: a crash is settled by the poll itself.
    w.post("Alex", "crash/start", json!({"bet": 100.0})).await;
    sqlx::query("UPDATE casino_crash SET crash_point = 1.5, started_ms = started_ms - 9000 WHERE status = 'active'").execute(&w.t.db).await.unwrap();
    let r = w.get("Alex", "crash/status").await;
    assert_eq!(r["game"]["status"], "busted", "{r}");
    assert_eq!(w.get("Alex", "crash/status").await["game"], Value::Null, "nothing left to poll");

    // Auto cash-out pays exactly the target when the round gets there first.
    w.post("Alex", "crash/start", json!({"bet": 100.0, "auto": 1.5})).await;
    sqlx::query("UPDATE casino_crash SET crash_point = 20.0, started_ms = started_ms - 8000 WHERE status = 'active'").execute(&w.t.db).await.unwrap();
    let r = w.get("Alex", "crash/status").await;
    assert_eq!((r["payout"].as_f64(), r["game"]["status"].as_str()), (Some(150.0), Some("cashed")), "{r}");
    // ...and loses if the round crashed before the target.
    w.post("Alex", "crash/start", json!({"bet": 100.0, "auto": 3.0})).await;
    sqlx::query("UPDATE casino_crash SET crash_point = 1.3, started_ms = started_ms - 20000 WHERE status = 'active'").execute(&w.t.db).await.unwrap();
    assert_eq!(w.get("Alex", "crash/status").await["game"]["status"], "busted");

    let h = w.get("Alex", "history").await;
    assert_eq!(h["rounds"].as_array().unwrap().iter().filter(|r| r["game"] == "crash").count(), 5, "every round is on record");
}

#[tokio::test]
async fn blackjack_deals_hits_stands_doubles_and_pays() {
    let w = world().await;
    w.configure(|c| c["chaos"]["enabled"] = json!(false)).await;
    // Card ids: ace 0, five 4, six 5, seven 6, eight 7, nine 8, ten 9, king 12.
    let set = |player: &'static str, dealer: &'static str| {
        let (db, uuid) = (w.t.db.clone(), w.uuid["Alex"].clone());
        async move { sqlx::query("UPDATE casino_blackjack SET player = ?, dealer = ? WHERE uuid = ? AND status = 'active'").bind(player).bind(dealer).bind(uuid).execute(&db).await.unwrap(); }
    };
    assert_eq!(w.post("Alex", "blackjack/hit", json!({})).await.0, StatusCode::NOT_FOUND);
    assert_eq!(w.post("Alex", "blackjack/start", json!({"bet": 1.0})).await.0, StatusCode::BAD_REQUEST);

    // Start a hand, then rig the cards (a natural would end it at once, so retry until it is live).
    let mut live = false;
    for _ in 0..30 {
        let (s, r) = w.post("Alex", "blackjack/start", json!({"bet": 100.0})).await;
        assert_eq!(s, StatusCode::OK, "{r}");
        if r["game"]["status"] == "active" {
            assert_eq!(r["game"]["dealer"].as_array().unwrap().len(), 1, "one dealer card stays hidden");
            assert_eq!(r["game"]["hidden"], 1);
            assert_eq!(w.post("Alex", "blackjack/start", json!({"bet": 100.0})).await.0, StatusCode::CONFLICT, "one hand at a time");
            live = true;
            break;
        }
    }
    assert!(live, "a live hand dealt");

    // 10 + 6 against a dealer's 10 + 9: hit a 5 for 21 and the hand ends by itself; the dealer stays on 19.
    set("[9, 5]", "[9, 8]").await;
    assert_eq!(w.get("Alex", "").await["blackjack"]["player_total"], 16);
    // Stand on 16: dealer's 19 beats it.
    let before = w.balance("Alex").await;
    let (s, r) = w.post("Alex", "blackjack/stand", json!({})).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    assert_eq!((r["game"]["outcome"].as_str(), r["payout"].as_f64()), (Some("lose"), Some(0.0)), "{r}");
    assert_eq!(r["game"]["dealer"].as_array().unwrap().len(), 2, "the hole card is shown");
    assert_eq!(w.balance("Alex").await, before);
    assert!(r["double"].is_null());
    assert_eq!(w.post("Alex", "blackjack/stand", json!({})).await.0, StatusCode::NOT_FOUND, "the hand is over");

    // A winning stand: 10 + 9 against 10 + 8 pays double and offers Double or Nothing.
    deal_live(&w).await;
    set("[9, 8]", "[9, 7]").await;
    let before = w.balance("Alex").await;
    let (_, r) = w.post("Alex", "blackjack/stand", json!({})).await;
    assert_eq!((r["game"]["outcome"].as_str(), r["payout"].as_f64()), (Some("win"), Some(200.0)), "{r}");
    assert!((w.balance("Alex").await - (before + 200.0)).abs() < 0.01);
    assert_eq!(r["double"]["stake"], 200.0);

    // Bust on a hit: 10 + 6 + king.
    deal_live(&w).await;
    set("[9, 5, 12]", "[9, 8]").await;
    let (_, r) = w.post("Alex", "blackjack/stand", json!({})).await;
    assert_eq!(r["payout"].as_f64(), Some(0.0));
    deal_live(&w).await;
    set("[9, 5]", "[9, 8]").await;
    let mut busted = false;
    for _ in 0..12 {
        let (_, r) = w.post("Alex", "blackjack/hit", json!({})).await;
        if r["game"]["status"] == "done" { busted = r["game"]["outcome"] == "bust" || r["payout"].as_f64().unwrap_or(0.0) > 0.0 || r["game"]["outcome"] == "lose"; break; }
    }
    assert!(busted, "hitting eventually ends the hand");

    // Double down: stake doubles, exactly one card, then the dealer plays.
    deal_live(&w).await;
    set("[4, 5]", "[9, 7]").await; // 5 + 6 = 11 against 10 + 8
    let before = w.balance("Alex").await;
    assert!(w.get("Alex", "").await["blackjack"]["can_double"].as_bool().unwrap());
    let (s, r) = w.post("Alex", "blackjack/double", json!({})).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    assert_eq!(r["game"]["player"].as_array().unwrap().len(), 3, "one card only");
    assert_eq!(r["game"]["status"], "done");
    assert_eq!(r["game"]["bet"], 200.0);
    assert!((w.balance("Alex").await - (before - 100.0 + r["payout"].as_f64().unwrap())).abs() < 0.01, "{r}");
    // No doubling after a hit.
    deal_live(&w).await;
    set("[0, 4, 4]", "[9, 8]").await;
    assert_eq!(w.post("Alex", "blackjack/double", json!({})).await.0, StatusCode::BAD_REQUEST);
    w.post("Alex", "blackjack/stand", json!({})).await;

    // A natural pays 3 to 2 immediately (rig by inserting a finished-in-one-step hand through a start that deals one).
    let before = w.balance("Alex").await;
    let mut natural = None;
    for _ in 0..400 {
        let (_, r) = w.post("Alex", "blackjack/start", json!({"bet": 100.0})).await;
        if r["game"]["status"] == "done" && r["game"]["outcome"] == "blackjack" { natural = Some(r); break; }
        if r["game"]["status"] == "active" { w.post("Alex", "blackjack/stand", json!({})).await; }
    }
    if let Some(r) = natural {
        assert_eq!(r["payout"], 250.0, "3 to 2 on a 100 bet");
        assert!(w.balance("Alex").await != before);
    }
    let h = w.get("Alex", "history").await;
    assert!(h["rounds"].as_array().unwrap().iter().any(|r| r["game"] == "blackjack"));
}

async fn deal_live(w: &World) {
    for _ in 0..60 {
        let (s, r) = w.post("Alex", "blackjack/start", json!({"bet": 100.0})).await;
        assert_eq!(s, StatusCode::OK, "{r}");
        if r["game"]["status"] == "active" { return; }
    }
    panic!("could not deal a live hand");
}

#[tokio::test]
async fn a_game_you_walk_away_from_pays_out_instead_of_taking_your_money() {
    let w = world().await;
    w.configure(|c| c["chaos"]["enabled"] = json!(false)).await;

    // Crash: the page polls while you watch. If it stops, the round is cashed out at the multiplier you last saw.
    w.post("Alex", "crash/start", json!({"bet": 100.0})).await;
    sqlx::query("UPDATE casino_crash SET crash_point = 50.0, started_ms = started_ms - 40000, last_poll_ms = last_poll_ms - 30000 WHERE status = 'active'").execute(&w.t.db).await.unwrap();
    let before = w.balance("Alex").await;
    let r = w.get("Alex", "crash/status").await;
    assert_eq!(r["game"]["status"], "cashed", "{r}");
    let payout = r["payout"].as_f64().unwrap();
    assert!((payout - 100.0 * (0.12f64 * 10.0).exp()).abs() < 6.0, "10s in is about 3.3x, not the 40s the clock says: {payout}");
    assert!((w.balance("Alex").await - (before + payout)).abs() < 0.01);
    // A round that was already past its crash point on your last visit is a loss, as ever.
    w.post("Alex", "crash/start", json!({"bet": 100.0})).await;
    sqlx::query("UPDATE casino_crash SET crash_point = 1.1, started_ms = started_ms - 40000 WHERE status = 'active'").execute(&w.t.db).await.unwrap();
    assert_eq!(w.get("Alex", "crash/status").await["game"]["status"], "busted");

    // Mines left open for a day: tiles already turned over are paid, and an untouched board is refunded.
    w.post("Alex", "mines/start", json!({"bet": 200.0, "mines": 3})).await;
    let layout: Vec<u32> = serde_json::from_str(&sqlx::query_scalar::<_, String>("SELECT layout FROM casino_mines WHERE status = 'active'").fetch_one(&w.t.db).await.unwrap()).unwrap();
    let safe = (0..25).find(|t| !layout.contains(t)).unwrap();
    w.post("Alex", "mines/reveal", json!({"tile": safe})).await;
    sqlx::query("UPDATE casino_mines SET created_at = '2000-01-01T00:00:00Z' WHERE status = 'active'").execute(&w.t.db).await.unwrap();
    let before = w.balance("Alex").await;
    w.settle().await;
    assert!(w.balance("Alex").await > before + 200.0, "one safe tile is worth more than the bet");
    assert!(w.get("Alex", "").await["mines"].is_null());
    w.post("Steve", "mines/start", json!({"bet": 300.0, "mines": 3})).await;
    sqlx::query("UPDATE casino_mines SET created_at = '2000-01-01T00:00:00Z' WHERE status = 'active'").execute(&w.t.db).await.unwrap();
    w.settle().await;
    assert_eq!(w.balance("Steve").await, 10000.0, "nothing turned over, nothing lost");

    // A Blackjack hand left open is a push.
    let mut live = false;
    for _ in 0..40 {
        let (_, r) = w.post("Mia", "blackjack/start", json!({"bet": 150.0})).await;
        if r["game"]["status"] == "active" { live = true; break; }
        // A natural ended the hand at once; any result is fine, just not live.
    }
    assert!(live);
    let before = w.balance("Mia").await;
    sqlx::query("UPDATE casino_blackjack SET created_at = '2000-01-01T00:00:00Z' WHERE status = 'active'").execute(&w.t.db).await.unwrap();
    w.settle().await;
    assert_eq!(w.balance("Mia").await, before + 150.0, "the bet comes back");
}
