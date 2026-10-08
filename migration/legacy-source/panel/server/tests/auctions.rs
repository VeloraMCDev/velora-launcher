//! Market auctions: bidding holds money, outbid players are refunded, late bids extend the clock, and winnings wait in a mailbox.

mod common;
use common::*;

struct F {
    t: TestApp,
    admin: String,
    server: String,
    alex: (String, String),
    steve: (String, String),
    mia: (String, String),
}

async fn fixture() -> F {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    let mut ids = Vec::new();
    for n in ["Alex", "Steve", "Mia"] {
        t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username": n, "password": "password123"}))).await;
        ids.push((t.login(n, "password123").await, t.uuid(n).await));
    }
    let (_, srv) = t.call("POST", "/api/admin/servers", Some(&admin), Some(json!({"name": "SMP", "instance_id": "smp"}))).await;
    let server = srv["token"].as_str().unwrap().to_string();
    let mia = ids.pop().unwrap();
    let steve = ids.pop().unwrap();
    let alex = ids.pop().unwrap();
    F { t, admin, server, alex, steve, mia }
}

impl F {
    async fn post(&self, path: &str, body: Value) -> (StatusCode, Value) {
        self.t.call("POST", &format!("/api/server/v1/{path}"), Some(&self.server), Some(body)).await
    }
    async fn balance(&self, uuid: &str) -> f64 {
        sqlx::query_scalar("SELECT balance FROM server_economy WHERE uuid = ?").bind(uuid).fetch_optional(&self.t.db).await.unwrap().unwrap_or(1000.0)
    }
    async fn list(&self, op: &str, kind: &str, hours: i64, price: f64) -> i64 {
        let (s, r) = self.post("economy/market/list", json!({
            "operation_id": op, "seller_uuid": self.alex.1, "seller_name": "Alex", "item_id": "DIAMOND_SWORD", "item_name": "Diamond Sword",
            "amount": 1, "price": price, "kind": kind, "duration_hours": hours, "item_data": "data"
        })).await;
        assert_eq!(s, StatusCode::OK, "{r}");
        r["id"].as_i64().unwrap()
    }
    async fn bid(&self, op: &str, who: &(String, String), name: &str, id: i64, amount: Option<f64>) -> (StatusCode, Value) {
        self.post("economy/market/bid", json!({"operation_id": op, "listing_id": id, "bidder_uuid": who.1, "bidder_name": name, "amount": amount})).await
    }
    async fn unread(&self, tok: &str) -> Vec<String> {
        let (_, n) = self.t.call("GET", "/api/v1/notifications", Some(tok), None).await;
        n["items"].as_array().unwrap().iter().map(|i| i["kind"].as_str().unwrap().to_string()).collect()
    }
}

#[tokio::test]
async fn bidding_refunds_the_previous_leader_and_enforces_increments() {
    let f = fixture().await;
    let id = f.list("l1", "auction", 24, 100.0).await;

    let (s, r) = f.post("economy/market/buy", json!({"operation_id": "b0", "listing_id": id, "buyer_uuid": f.steve.1, "buyer_name": "Steve"})).await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "auctions can't be bought outright: {r}");
    assert_eq!(f.bid("x1", &f.alex, "Alex", id, Some(150.0)).await.0, StatusCode::FORBIDDEN, "no bidding on your own auction");
    let (s, r) = f.bid("x2", &f.steve, "Steve", id, Some(50.0)).await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "{r}");
    assert!(r["error"].as_str().unwrap().contains("$100.00"));

    let (s, r) = f.bid("x3", &f.steve, "Steve", id, None).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    assert_eq!(r["current_bid"], 100.0);
    assert_eq!(r["min_next"], 105.0);
    assert_eq!(f.balance(&f.steve.1).await, 900.0, "the bid is held");

    let (s, r) = f.bid("x4", &f.steve, "Steve", id, Some(102.0)).await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "needs the 5% step: {r}");
    let (s, r) = f.bid("x5", &f.mia, "Mia", id, Some(120.0)).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    assert_eq!(f.balance(&f.steve.1).await, 1000.0, "outbid player is refunded");
    assert_eq!(f.balance(&f.mia.1).await, 880.0);
    assert!(f.unread(&f.steve.0).await.contains(&"auction_outbid".to_string()));

    // Raising your own bid only holds the difference.
    f.bid("x6", &f.mia, "Mia", id, Some(130.0)).await;
    assert_eq!(f.balance(&f.mia.1).await, 870.0);

    // A bid you can't afford changes nothing.
    let (s, _) = f.bid("x7", &f.steve, "Steve", id, Some(5000.0)).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    assert_eq!(f.balance(&f.steve.1).await, 1000.0);
    assert_eq!(f.balance(&f.mia.1).await, 870.0);

    // Retrying the same operation never charges twice.
    let before = f.balance(&f.mia.1).await;
    f.bid("x6", &f.mia, "Mia", id, Some(130.0)).await;
    assert_eq!(f.balance(&f.mia.1).await, before);

    // The listing shows the lead.
    let (_, market) = f.post("economy/market", json!({})).await;
    assert_eq!(market[0]["kind"], "auction");
    assert_eq!(market[0]["current_bid"], 130.0);
    assert_eq!(market[0]["bidder_name"], "Mia");
    assert_eq!(market[0]["min_next_bid"], 136.5);
}

#[tokio::test]
async fn ended_auctions_pay_the_seller_and_fill_the_mailbox() {
    let f = fixture().await;
    let won = f.list("l1", "auction", 1, 50.0).await;
    let unsold = f.list("l2", "auction", 1, 10.0).await;
    f.bid("a", &f.steve, "Steve", won, Some(80.0)).await;
    sqlx::query("UPDATE server_market SET ends_at = '2000-01-01T00:00:00Z' WHERE kind = 'auction'").execute(&f.t.db).await.unwrap();

    let (_, r) = f.t.call("POST", "/api/admin/tasks/settle_auctions/run", Some(&f.admin), None).await;
    assert_eq!(r["message"], "1 sold, 1 returned unsold", "{r}");
    let _ = unsold;
    assert_eq!(f.balance(&f.alex.1).await, 1080.0, "seller is paid the winning bid");
    assert_eq!(f.balance(&f.steve.1).await, 920.0);

    let (_, w) = f.post("economy/market/mailbox", json!({"uuid": f.steve.1})).await;
    assert_eq!(w["waiting"], 1);
    let (s, c) = f.post("economy/market/mailbox/claim", json!({"operation_id": "c1", "uuid": f.steve.1})).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(c["items"][0]["item_name"], "Diamond Sword");
    assert_eq!(c["items"][0]["item_data"], "data");
    let (_, again) = f.post("economy/market/mailbox/claim", json!({"operation_id": "c1", "uuid": f.steve.1})).await;
    assert_eq!(again["items"].as_array().unwrap().len(), 1, "a retry returns the same answer");
    let (_, empty) = f.post("economy/market/mailbox/claim", json!({"operation_id": "c2", "uuid": f.steve.1})).await;
    assert!(empty["items"].as_array().unwrap().is_empty(), "nothing is handed out twice");

    // The unsold sword is back with its seller.
    let (_, c) = f.post("economy/market/mailbox/claim", json!({"operation_id": "c3", "uuid": f.alex.1})).await;
    assert_eq!(c["items"].as_array().unwrap().len(), 1);
    assert!(f.unread(&f.steve.0).await.contains(&"auction_won".to_string()));
    let alex_kinds = f.unread(&f.alex.0).await;
    assert!(alex_kinds.contains(&"auction_sold".to_string()) && alex_kinds.contains(&"auction_unsold".to_string()), "{alex_kinds:?}");
    let (_, market) = f.post("economy/market", json!({})).await;
    assert!(market.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn late_bids_extend_the_clock_and_listings_can_be_cancelled_until_someone_bids() {
    let f = fixture().await;
    let id = f.list("l1", "auction", 1, 20.0).await;
    let soon = (chrono::Utc::now() + chrono::Duration::seconds(30)).to_rfc3339();
    sqlx::query("UPDATE server_market SET ends_at = ? WHERE id = ?").bind(&soon).bind(id).execute(&f.t.db).await.unwrap();
    let (_, r) = f.bid("late", &f.steve, "Steve", id, None).await;
    let ends: chrono::DateTime<chrono::Utc> = r["ends_at"].as_str().unwrap().parse().unwrap();
    assert!((ends - chrono::Utc::now()).num_seconds() > 100, "anti-sniping gives everyone two more minutes");

    let (s, e) = f.post("economy/market/cancel", json!({"operation_id": "k1", "listing_id": id, "uuid": f.alex.1})).await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "bids are binding: {e}");

    let buy_now = f.list("l2", "buy_now", 24, 30.0).await;
    assert_eq!(f.post("economy/market/cancel", json!({"operation_id": "k2", "listing_id": buy_now, "uuid": f.mia.1})).await.0, StatusCode::FORBIDDEN);
    let (s, c) = f.post("economy/market/cancel", json!({"operation_id": "k3", "listing_id": buy_now, "uuid": f.alex.1})).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(c["items"][0]["item_id"], "DIAMOND_SWORD");

    // Bad listings are refused.
    let (s, _) = f.post("economy/market/list", json!({"operation_id": "l3", "seller_uuid": f.alex.1, "seller_name": "Alex", "item_id": "STONE", "item_name": "Stone", "amount": 1, "price": 5.0, "kind": "auction", "duration_hours": 999})).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let (s, _) = f.post("economy/market/list", json!({"operation_id": "l4", "seller_uuid": f.alex.1, "seller_name": "Alex", "item_id": "STONE", "item_name": "Stone", "amount": 1, "price": 5.0, "kind": "raffle"})).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
}
