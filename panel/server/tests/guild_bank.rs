//! Guild bank: deposits, officer-only spending, shop sales, guild-to-guild
//! payments and guild market trades, all idempotent on their operation id.

mod common;
use common::*;

struct Fixture {
    t: TestApp,
    server: String,
    sid: i64,
    alex: String,
    steve: String,
    alex_uuid: String,
    steve_uuid: String,
    gid: String,
}

async fn fixture() -> Fixture {
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
    // Mia leads a second guild.
    t.call("POST", "/api/v1/guilds", Some(&mia), Some(json!({"instance_id": "smp", "name": "Void", "tag": "VOID"}))).await;
    Fixture { t, server, sid, alex, steve, alex_uuid, steve_uuid, gid }
}

impl Fixture {
    async fn post(&self, path: &str, body: Value) -> (StatusCode, Value) {
        self.t.call("POST", &format!("/api/server/v1/{path}"), Some(&self.server), Some(body)).await
    }
    async fn guild_balance(&self) -> f64 {
        sqlx::query_scalar("SELECT COALESCE((SELECT balance FROM guild_wallets WHERE guild_id = ? AND server_id = ?), 0)")
            .bind(&self.gid)
            .bind(self.sid)
            .fetch_one(&self.t.db)
            .await
            .unwrap()
    }
}

#[tokio::test]
async fn bank_deposits_withdrawals_and_roles() {
    let f = fixture().await;
    let (s, info) = f.post("guilds/bank", json!({"uuid": f.alex_uuid})).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!((info["guild"]["tag"].as_str(), info["role"].as_str(), info["balance"].as_f64()), (Some("IRON"), Some("leader"), Some(0.0)));
    assert_eq!(f.post("guilds/bank", json!({"uuid": "00000000-0000-0000-0000-000000000001"})).await.1["guild"], Value::Null);

    // Members can deposit from their own balance (new accounts start with 1000).
    let dep = json!({"operation_id": "op-1", "uuid": f.steve_uuid, "username": "Steve", "amount": 250.5, "action": "deposit"});
    let (s, r) = f.post("guilds/bank/transfer", dep.clone()).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    assert_eq!((r["balance"].as_f64(), r["my_balance"].as_f64()), (Some(250.5), Some(749.5)));
    // A retried request returns the same answer and moves no money twice.
    let (_, again) = f.post("guilds/bank/transfer", dep).await;
    assert_eq!(again, r);
    assert_eq!(f.guild_balance().await, 250.5);

    // Only officers and leaders withdraw.
    let wd = |op: &str, who: &str, name: &str, amount: f64| json!({"operation_id": op, "uuid": who, "username": name, "amount": amount, "action": "withdraw"});
    assert_eq!(f.post("guilds/bank/transfer", wd("op-2", &f.steve_uuid, "Steve", 10.0)).await.0, StatusCode::FORBIDDEN);
    let (s, r) = f.post("guilds/bank/transfer", wd("op-3", &f.alex_uuid, "Alex", 100.0)).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    assert_eq!(r["balance"], 150.5);
    // Not more than the bank holds, not more than a player holds, only valid amounts.
    assert_eq!(f.post("guilds/bank/transfer", wd("op-4", &f.alex_uuid, "Alex", 9999.0)).await.0, StatusCode::BAD_REQUEST);
    let big = json!({"operation_id": "op-5", "uuid": f.steve_uuid, "username": "Steve", "amount": 5000.0, "action": "deposit"});
    assert_eq!(f.post("guilds/bank/transfer", big).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(f.post("guilds/bank/transfer", wd("op-6", &f.alex_uuid, "Alex", 0.001)).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(f.guild_balance().await, 150.5);

    // The launcher sees the same history, with who did it.
    let (s, w) = f.t.call("GET", &format!("/api/v1/guilds/{}/wallet?server_id={}", f.gid, f.sid), Some(&f.steve), None).await;
    assert_eq!(s, StatusCode::OK, "{w}");
    assert_eq!(w["balance"], 150.5);
    assert_eq!(w["transactions"].as_array().unwrap().len(), 2);
    assert_eq!(w["role"], "member");
    let (_, w) = f.t.call("GET", &format!("/api/v1/guilds/{}/wallet?server_id={}", f.gid, f.sid), Some(&f.alex), None).await;
    assert_eq!(w["role"], "leader");
}

#[tokio::test]
async fn shop_sales_and_guild_payments() {
    let f = fixture().await;
    let credit =
        json!({"operation_id": "sale-1", "uuid": f.steve_uuid, "username": "Steve", "amount": 64.0, "description": "64x Cobblestone"});
    let (s, r) = f.post("guilds/bank/credit", credit.clone()).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    assert_eq!(f.post("guilds/bank/credit", credit).await.1, r);
    assert_eq!(f.guild_balance().await, 64.0);
    let (_, nobody) = f
        .post(
            "guilds/bank/credit",
            json!({"operation_id": "sale-2", "uuid": "00000000-0000-0000-0000-000000000009", "username": "X", "amount": 5.0}),
        )
        .await;
    assert!(nobody["error"].as_str().unwrap().contains("not in a guild"));

    let pay = |op: &str, who: &str, tag: &str, amount: f64| json!({"operation_id": op, "uuid": who, "to_tag": tag, "amount": amount});
    assert_eq!(f.post("guilds/bank/pay", pay("p1", &f.steve_uuid, "VOID", 10.0)).await.0, StatusCode::FORBIDDEN, "members can't spend");
    assert_eq!(f.post("guilds/bank/pay", pay("p2", &f.alex_uuid, "IRON", 10.0)).await.0, StatusCode::NOT_FOUND, "not to yourself");
    assert_eq!(f.post("guilds/bank/pay", pay("p3", &f.alex_uuid, "NOPE", 10.0)).await.0, StatusCode::NOT_FOUND);
    assert_eq!(f.post("guilds/bank/pay", pay("p4", &f.alex_uuid, "void", 500.0)).await.0, StatusCode::BAD_REQUEST, "insufficient");
    let (s, r) = f.post("guilds/bank/pay", pay("p5", &f.alex_uuid, "void", 24.0)).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    assert_eq!(r["balance"], 40.0);
    let void_balance: f64 =
        sqlx::query_scalar("SELECT balance FROM guild_wallets WHERE guild_id <> ?").bind(&f.gid).fetch_one(&f.t.db).await.unwrap();
    assert_eq!(void_balance, 24.0);
    let kinds: Vec<String> = sqlx::query_scalar("SELECT kind FROM guild_wallet_transactions ORDER BY id").fetch_all(&f.t.db).await.unwrap();
    assert_eq!(kinds, ["sale", "transfer_out", "transfer_in"]);
}

#[tokio::test]
async fn guild_market_listings_and_purchases() {
    let f = fixture().await;
    let list = |op: &str, who: &str, name: &str, guild: bool| json!({"operation_id": op, "seller_uuid": who, "seller_name": name, "item_id": "DIAMOND", "item_name": "Diamond", "amount": 4, "price": 120.0, "as_guild": guild});
    // Only officers list for the guild.
    assert_eq!(f.post("economy/market/list", list("l1", &f.steve_uuid, "Steve", true)).await.0, StatusCode::FORBIDDEN);
    let (s, l) = f.post("economy/market/list", list("l2", &f.alex_uuid, "Alex", true)).await;
    assert_eq!(s, StatusCode::OK, "{l}");
    let listing = l["id"].as_i64().unwrap();
    let (_, read) = f.post("economy/market", json!({})).await;
    assert_eq!(read[0]["seller_guild"], "IRON");
    let (_, public) = f.t.call("GET", &format!("/api/v1/servers/{}/economy/market", f.sid), None, None).await;
    assert_eq!(public[0]["seller_guild_tag"], "IRON");

    // Mia's guild buys it with guild money: she's an officer but the bank is empty first.
    let mia_uuid = f.t.uuid("Mia").await;
    let buy = |op: &str| json!({"operation_id": op, "listing_id": listing, "buyer_uuid": mia_uuid, "buyer_name": "Mia", "as_guild": true});
    sqlx::query(
        "INSERT INTO guild_wallets (server_id, guild_id, balance, updated_at) SELECT ?, id, 50, 'now' FROM guilds WHERE tag = 'VOID'",
    )
    .bind(f.sid)
    .execute(&f.t.db)
    .await
    .unwrap();
    let (s, e) = f.post("economy/market/buy", buy("b1")).await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "{e}");
    let still: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM server_market").fetch_one(&f.t.db).await.unwrap();
    assert_eq!(still, 1, "a failed purchase leaves the listing in place");
    sqlx::query("UPDATE guild_wallets SET balance = 500 WHERE guild_id <> ?").bind(&f.gid).execute(&f.t.db).await.unwrap();
    let (s, r) = f.post("economy/market/buy", buy("b2")).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    assert_eq!(r["new_balance"], 380.0);
    assert_eq!(f.guild_balance().await, 120.0, "the sale is paid into the listing guild's bank");
    let alex_balance: Option<f64> =
        sqlx::query_scalar("SELECT balance FROM server_economy WHERE uuid = ?").bind(&f.alex_uuid).fetch_optional(&f.t.db).await.unwrap();
    assert!(alex_balance.is_none(), "the lister is not paid personally");
    let kinds: Vec<String> = sqlx::query_scalar("SELECT kind FROM guild_wallet_transactions ORDER BY id").fetch_all(&f.t.db).await.unwrap();
    assert_eq!(kinds, ["purchase", "sale"]);
    // Retried purchase is the same answer and charges nothing more.
    let (_, again) = f.post("economy/market/buy", buy("b2")).await;
    assert_eq!(again["new_balance"], 380.0);

    // Plain player listings still pay the player.
    let (_, l2) = f.post("economy/market/list", list("l3", &f.steve_uuid, "Steve", false)).await;
    let pay = json!({"operation_id": "b3", "listing_id": l2["id"], "buyer_uuid": f.alex_uuid, "buyer_name": "Alex"});
    let (s, r) = f.post("economy/market/buy", pay).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    assert_eq!(r["new_balance"], 880.0);
    let steve_balance: f64 =
        sqlx::query_scalar("SELECT balance FROM server_economy WHERE uuid = ?").bind(&f.steve_uuid).fetch_one(&f.t.db).await.unwrap();
    assert_eq!(steve_balance, 1120.0);
}
