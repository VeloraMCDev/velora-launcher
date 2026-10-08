//! Admin economy: overview numbers, balance and guild adjustments, and market listing removal.

mod common;
use common::*;

struct F {
    t: TestApp,
    admin: String,
    alex: String,
    sid: i64,
    token: String,
    alex_uuid: String,
    steve_uuid: String,
}

async fn fixture() -> F {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    for n in ["Alex", "Steve"] {
        t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username": n, "password": "password123"}))).await;
    }
    let alex = t.login("Alex", "password123").await;
    let (_, srv) = t.call("POST", "/api/admin/servers", Some(&admin), Some(json!({"name": "SMP", "instance_id": "smp"}))).await;
    let sid = srv["server"]["id"].as_i64().unwrap();
    let token = srv["token"].as_str().unwrap().to_string();
    let (alex_uuid, steve_uuid) = (t.uuid("Alex").await, t.uuid("Steve").await);
    F { t, admin, alex, sid, token, alex_uuid, steve_uuid }
}

impl F {
    async fn get(&self, path: &str) -> (StatusCode, Value) {
        self.t.call("GET", &format!("/api/admin/economy/{path}"), Some(&self.admin), None).await
    }
    async fn post(&self, path: &str, mut body: Value) -> (StatusCode, Value) {
        body["server_id"] = json!(self.sid);
        self.t.call("POST", &format!("/api/admin/economy/{path}"), Some(&self.admin), Some(body)).await
    }
    async fn balance(&self, uuid: &str) -> f64 {
        sqlx::query_scalar("SELECT balance FROM server_economy WHERE uuid = ?").bind(uuid).fetch_one(&self.t.db).await.unwrap()
    }
    async fn seed_balance(&self, uuid: &str, name: &str, bal: f64) {
        sqlx::query("INSERT OR REPLACE INTO server_economy(server_id, uuid, username, balance, updated_at) VALUES (?, ?, ?, ?, '2026-01-01T00:00:00Z')")
            .bind(self.sid).bind(uuid).bind(name).bind(bal).execute(&self.t.db).await.unwrap();
    }
    async fn list(&self, op: &str, kind: &str) -> i64 {
        let (s, r) = self.t.call("POST", "/api/server/v1/economy/market/list", Some(&self.token), Some(json!({
            "operation_id": op, "seller_uuid": self.alex_uuid, "seller_name": "Alex", "item_id": "DIAMOND_SWORD", "item_name": "Diamond Sword",
            "amount": 1, "price": 100.0, "kind": kind, "duration_hours": 24, "item_data": "data"
        }))).await;
        assert_eq!(s, StatusCode::OK, "{r}");
        r["id"].as_i64().unwrap()
    }
}

#[tokio::test]
async fn overview_reports_seeded_numbers() {
    let f = fixture().await;
    f.seed_balance(&f.alex_uuid, "Alex", 300.0).await;
    f.seed_balance(&f.steve_uuid, "Steve", 100.0).await;
    f.seed_balance("00000000-0000-0000-0000-0000000000aa", "Mia", 200.0).await;
    let now = chrono::Utc::now().to_rfc3339();
    for (from, fname, to, tname, amt) in
        [(&f.alex_uuid, "Alex", &f.steve_uuid, "Steve", 40.0), (&f.steve_uuid, "Steve", &f.alex_uuid, "Alex", 10.0)]
    {
        sqlx::query("INSERT INTO economy_transactions(server_id, from_uuid, from_name, to_uuid, to_name, amount, description, created_at) VALUES (?, ?, ?, ?, ?, ?, 'Pay', ?)")
            .bind(f.sid).bind(from).bind(fname).bind(to).bind(tname).bind(amt).bind(&now).execute(&f.t.db).await.unwrap();
    }
    f.list("l1", "buy_now").await;
    f.list("l2", "auction").await;
    let (s, o) = f.get(&format!("overview?server_id={}", f.sid)).await;
    assert_eq!(s, StatusCode::OK, "{o}");
    assert_eq!(o["totals"]["circulation"], 600.0);
    assert_eq!(o["totals"]["accounts"], 3);
    assert_eq!(o["totals"]["average"], 200.0);
    assert_eq!(o["totals"]["median"], 200.0);
    assert_eq!(o["totals"]["richest"]["name"], "Alex");
    assert_eq!(o["volume"]["today"], 50.0);
    assert_eq!(o["volume"]["tx_7d"], 2);
    assert_eq!(o["daily"].as_array().unwrap().len(), 14);
    assert_eq!(o["daily"][13]["volume"], 50.0);
    assert_eq!(o["top_earners"][0]["name"], "Steve");
    assert_eq!(o["top_spenders"][0]["name"], "Alex");
    assert_eq!(o["market"]["fixed_listings"], 1);
    assert_eq!(o["market"]["auctions"], 1);
    assert_eq!(o["market"]["total_value"], 200.0);
    assert_eq!(o["market"]["ending_soon"], 1);
    assert_eq!(o["casino"]["house_net_7d"], 0.0);

    let (_, p) = f.get(&format!("players?server_id={}&q=ste&sort=name&dir=asc", f.sid)).await;
    assert_eq!(p["total"], 1);
    assert_eq!(p["rows"][0]["username"], "Steve");
    let (_, p) = f.get(&format!("players?server_id={}&q=%25", f.sid)).await;
    assert_eq!(p["total"], 0, "wildcards are escaped");
    let (_, l) = f.get(&format!("transactions?server_id={}&q=steve", f.sid)).await;
    assert_eq!(l["rows"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn adjusting_balances() {
    let f = fixture().await;
    let u = f.alex_uuid.clone();
    f.seed_balance(&u, "Alex", 100.0).await;

    let (s, r) = f.post("players/adjust", json!({"uuid": u, "mode": "add", "amount": 50.555, "reason": "event prize"})).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    assert_eq!(r["balance"], 150.56);
    assert_eq!(r["delta"], 50.56);
    let (s, r) = f.post("players/adjust", json!({"uuid": u, "mode": "remove", "amount": 1000})).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    assert!(r["error"].as_str().unwrap().contains("Insufficient"));
    assert_eq!(f.balance(&u).await, 150.56, "failed removal changes nothing");
    let (_, r) = f.post("players/adjust", json!({"uuid": u, "mode": "remove", "amount": 0.56})).await;
    assert_eq!(r["balance"], 150.0);
    let (_, r) = f.post("players/adjust", json!({"uuid": u, "mode": "set", "amount": 20})).await;
    assert_eq!((r["balance"].as_f64(), r["delta"].as_f64()), (Some(20.0), Some(-130.0)));
    let (_, r) = f.post("players/adjust", json!({"uuid": u, "mode": "set", "amount": 20})).await;
    assert_eq!(r["delta"], 0.0);
    for bad in [
        json!({"mode": "add", "amount": -5}),
        json!({"mode": "add", "amount": 2e12}),
        json!({"mode": "add", "amount": 0}),
        json!({"mode": "nope", "amount": 1}),
    ] {
        let mut b = bad.clone();
        b["uuid"] = json!(u);
        assert_eq!(f.post("players/adjust", b).await.0, StatusCode::BAD_REQUEST, "{bad}");
    }

    let n: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM economy_transactions WHERE description LIKE 'Admin:%'").fetch_one(&f.t.db).await.unwrap();
    assert_eq!(n, 3, "add, remove and set (non-zero) are logged");
    let d: String =
        sqlx::query_scalar("SELECT description FROM economy_transactions ORDER BY id LIMIT 1").fetch_one(&f.t.db).await.unwrap();
    assert_eq!(d, "Admin: event prize");

    // A brand new player gets an account created.
    let (s, r) = f
        .post("players/adjust", json!({"uuid": "00000000-0000-0000-0000-0000000000bb", "username": "Newbie", "mode": "add", "amount": 5}))
        .await;
    assert_eq!(s, StatusCode::OK, "{r}");
    assert_eq!(r["balance"], 1005.0);

    // Notified.
    let (_, n) = f.t.call("GET", "/api/v1/notifications", Some(&f.alex), None).await;
    assert!(n["items"].as_array().unwrap().iter().any(|i| i["kind"] == "economy_adjust"));

    // Admins only.
    let (s, _) =
        f.t.call(
            "POST",
            "/api/admin/economy/players/adjust",
            Some(&f.alex),
            Some(json!({"server_id": f.sid, "uuid": u, "mode": "add", "amount": 1})),
        )
        .await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    let (s, _) = f.t.call("GET", &format!("/api/admin/economy/overview?server_id={}", f.sid), None, None).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    assert_eq!(f.balance(&u).await, 20.0);
}

#[tokio::test]
async fn adjusting_guild_banks() {
    let f = fixture().await;
    let (_, g) =
        f.t.call("POST", "/api/v1/guilds", Some(&f.alex), Some(json!({"instance_id": "smp", "name": "Iron", "tag": "IRON"}))).await;
    let gid = g["id"].as_str().unwrap().to_string();
    let (s, list) = f.get(&format!("guilds?server_id={}", f.sid)).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!((list[0]["tag"].as_str(), list[0]["balance"].as_f64(), list[0]["members"].as_i64()), (Some("IRON"), Some(0.0), Some(1)));

    let (s, r) = f.post("guilds/adjust", json!({"guild_id": gid, "mode": "add", "amount": 250, "reason": "grant"})).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    assert_eq!(r["balance"], 250.0);
    let (s, _) = f.post("guilds/adjust", json!({"guild_id": gid, "mode": "remove", "amount": 999})).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let (_, r) = f.post("guilds/adjust", json!({"guild_id": gid, "mode": "set", "amount": 100})).await;
    assert_eq!((r["balance"].as_f64(), r["delta"].as_f64()), (Some(100.0), Some(-150.0)));
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM guild_wallet_transactions WHERE guild_id = ? AND kind = 'admin'")
        .bind(&gid)
        .fetch_one(&f.t.db)
        .await
        .unwrap();
    assert_eq!(n, 2, "the bank's own ledger records it");
    assert_eq!(f.post("guilds/adjust", json!({"guild_id": "nope", "mode": "add", "amount": 1})).await.0, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn removing_listings() {
    let f = fixture().await;
    let steve = f.steve_uuid.clone();
    f.seed_balance(&steve, "Steve", 1000.0).await;
    let fixed = f.list("l1", "buy_now").await;
    let auction = f.list("l2", "auction").await;

    let (_, m) = f.get(&format!("market?server_id={}&kind=auction", f.sid)).await;
    assert_eq!((m["total"].as_i64(), m["rows"][0]["id"].as_i64()), (Some(1), Some(auction)));

    let (s, r) = f.post(&format!("market/{fixed}/remove"), json!({"reason": "scam"})).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    let mail: (String, i64, String) =
        sqlx::query_as("SELECT uuid, to_vault, note FROM market_mailbox WHERE item_id = 'DIAMOND_SWORD'").fetch_one(&f.t.db).await.unwrap();
    assert_eq!(mail, (f.alex_uuid.clone(), 1, "removed by admin".into()));
    assert_eq!(f.post(&format!("market/{fixed}/remove"), json!({})).await.0, StatusCode::NOT_FOUND);

    // Steve leads the auction; removing it refunds him and returns the item.
    let (s, r) =
        f.t.call(
            "POST",
            "/api/server/v1/economy/market/bid",
            Some(&f.token),
            Some(json!({"operation_id": "b1", "listing_id": auction, "bidder_uuid": steve, "bidder_name": "Steve", "amount": 150.0})),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{r}");
    assert_eq!(f.balance(&steve).await, 850.0);
    let (s, r) = f.post(&format!("market/{auction}/extend"), json!({"hours": 500})).await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "{r}");
    assert_eq!(f.post(&format!("market/{auction}/extend"), json!({"hours": 2})).await.0, StatusCode::OK);
    let (s, r) = f.post(&format!("market/{auction}/remove"), json!({})).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    assert_eq!(r["refunded"], 150.0);
    assert_eq!(f.balance(&steve).await, 1000.0);
    let mails: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM market_mailbox WHERE uuid = ?").bind(&f.alex_uuid).fetch_one(&f.t.db).await.unwrap();
    assert_eq!(mails, 2);
    let refund: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM economy_transactions WHERE description LIKE 'Auction refund%' AND to_uuid = ?")
            .bind(&steve)
            .fetch_one(&f.t.db)
            .await
            .unwrap();
    assert_eq!(refund, 1);
    let left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM server_market").fetch_one(&f.t.db).await.unwrap();
    assert_eq!(left, 0);
}

#[tokio::test]
async fn players_without_accounts_can_be_fixed_in_bulk_or_by_taking_over_an_old_account() {
    let f = fixture().await;
    // Nobody has an account yet (accounts appear when a game server first asks for one). The admin account is a player too.
    let (s, m) = f.get(&format!("accounts/missing?server_id={}", f.sid)).await;
    assert_eq!(s, StatusCode::OK, "{m}");
    let names: Vec<_> = m["missing"].as_array().unwrap().iter().map(|r| r["name"].as_str().unwrap().to_string()).collect();
    assert!(names.contains(&"Alex".to_string()) && names.contains(&"Steve".to_string()), "{m}");

    // Alex's money sits under an old (offline-mode) UUID with the same name: the repair re-links it instead of opening a second account.
    sqlx::query("INSERT INTO server_economy(server_id, uuid, username, balance, updated_at) VALUES (?, 'old-uuid', 'alex', 777.5, 'x')").bind(f.sid).execute(&f.t.db).await.unwrap();
    sqlx::query("INSERT INTO economy_transactions(server_id, from_uuid, from_name, to_uuid, to_name, amount, description, created_at) VALUES (?, 'server', 'Server', 'old-uuid', 'alex', 5, 'gift', 'x')").bind(f.sid).execute(&f.t.db).await.unwrap();
    let (_, m) = f.get(&format!("accounts/missing?server_id={}", f.sid)).await;
    let alex_row = m["missing"].as_array().unwrap().iter().find(|r| r["name"] == "Alex").unwrap();
    assert_eq!(alex_row["suggested_from"], "old-uuid");
    let (s, r) = f.post("accounts/repair", json!({})).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    assert_eq!(r["linked"], 1, "{r}");
    assert!(r["created"].as_i64().unwrap() >= 2, "Steve and the admin get new accounts: {r}");
    assert_eq!(r["remaining"], 0);
    assert_eq!(f.balance(&f.alex_uuid).await, 777.5, "the old balance moved to Alex");
    let history: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM economy_transactions WHERE to_uuid = ?").bind(&f.alex_uuid).fetch_one(&f.t.db).await.unwrap();
    assert_eq!(history, 1, "and so did the history");
    let accounts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM server_economy WHERE uuid = 'old-uuid'").fetch_one(&f.t.db).await.unwrap();
    assert_eq!(accounts, 0);
    // Running it again changes nothing.
    let (_, again) = f.post("accounts/repair", json!({})).await;
    assert_eq!((again["created"].as_i64(), again["linked"].as_i64()), (Some(0), Some(0)));

    // Assigning by hand: a player who already has one is refused; an unknown player is refused.
    let (s, _) = f.post("accounts/assign", json!({"uuid": f.steve_uuid})).await;
    assert_eq!(s, StatusCode::CONFLICT);
    let (s, _) = f.post("accounts/assign", json!({"uuid": "not-a-player"})).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    sqlx::query("DELETE FROM server_economy WHERE uuid = ?").bind(&f.steve_uuid).execute(&f.t.db).await.unwrap();
    sqlx::query("INSERT INTO server_economy(server_id, uuid, username, balance, updated_at) VALUES (?, 'stray', 'SteveOld', 40, 'x')").bind(f.sid).execute(&f.t.db).await.unwrap();
    let (_, m) = f.get(&format!("accounts/missing?server_id={}", f.sid)).await;
    assert_eq!(m["orphans"][0]["uuid"], "stray");
    let (s, r) = f.post("accounts/assign", json!({"uuid": f.steve_uuid, "from_uuid": "stray"})).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    assert_eq!(f.balance(&f.steve_uuid).await, 40.0);
    let name: String = sqlx::query_scalar("SELECT username FROM server_economy WHERE uuid = ?").bind(&f.steve_uuid).fetch_one(&f.t.db).await.unwrap();
    assert_eq!(name, "Steve", "the account takes the player's current name");
    // A brand-new account with a chosen opening balance.
    sqlx::query("DELETE FROM server_economy WHERE uuid = ?").bind(&f.steve_uuid).execute(&f.t.db).await.unwrap();
    let (s, _) = f.post("accounts/assign", json!({"uuid": f.steve_uuid, "balance": 250.0})).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(f.balance(&f.steve_uuid).await, 250.0);
    // The same repair runs as a scheduled job.
    sqlx::query("DELETE FROM server_economy WHERE uuid = ?").bind(&f.steve_uuid).execute(&f.t.db).await.unwrap();
    let (s, run) = f.t.call("POST", "/api/admin/tasks/economy_accounts/run", Some(&f.admin), None).await;
    assert_eq!(s, StatusCode::OK, "{run}");
    assert_eq!(f.balance(&f.steve_uuid).await, 1000.0, "the job gave Steve the starting balance back: {run}");
}
