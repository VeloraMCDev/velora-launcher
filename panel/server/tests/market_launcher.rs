//! The market in the launcher: browse, buy, bid and cancel; purchases are delivered to the vault by the game server.

mod common;
use common::*;

#[tokio::test]
async fn launcher_buys_bids_and_gets_items_delivered_to_the_vault() {
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
    let game = |path: &str, body: Value| {
        let (t, server, path) = (&t, server.clone(), format!("/api/server/v1/{path}"));
        async move { t.call("POST", &path, Some(&server), Some(body)).await }
    };
    let list = |op: &str, kind: &str, price: f64| {
        json!({"operation_id": op, "seller_uuid": uuid["Alex"], "seller_name": "Alex", "item_id": "DIAMOND_SWORD", "item_name": "Diamond Sword", "amount": 1,
               "price": price, "kind": kind, "duration_hours": 2, "item_data": "snbt"})
    };
    let (_, a) = game("economy/market/list", list("l1", "buy_now", 100.0)).await;
    let (_, b) = game("economy/market/list", list("l2", "auction", 50.0)).await;
    let (buy_id, auction_id) = (a["id"].as_i64().unwrap(), b["id"].as_i64().unwrap());

    // Browsing shows both kinds, who owns them and what I could pay.
    let (s, v) = t.call("GET", &format!("/api/v1/market/{sid}"), Some(&tok["Steve"]), None).await;
    assert_eq!(s, StatusCode::OK, "{v}");
    assert_eq!(v["listings"].as_array().unwrap().len(), 2);
    assert_eq!(v["listings"][0]["kind"], "auction");
    assert_eq!(v["listings"][0]["min_next_bid"], 50.0);
    let (_, mine) = t.call("GET", &format!("/api/v1/market/{sid}"), Some(&tok["Alex"]), None).await;
    assert_eq!(mine["listings"][0]["mine"], true);

    // Buying from the launcher charges the buyer, pays the seller and tells them; the item goes to the vault queue.
    let (s, r) = t.call("POST", &format!("/api/v1/market/{sid}/buy"), Some(&tok["Steve"]), Some(json!({"listing_id": buy_id}))).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    assert_eq!(r["to_vault"], true);
    assert!(r["item_data"].is_null(), "the item is not handed back to the launcher");
    assert_eq!(t.call("POST", &format!("/api/v1/market/{sid}/buy"), Some(&tok["Mia"]), Some(json!({"listing_id": buy_id}))).await.0, StatusCode::NOT_FOUND, "sold once");
    let (_, bell) = t.call("GET", "/api/v1/notifications", Some(&tok["Alex"]), None).await;
    assert!(bell["items"].as_array().unwrap().iter().any(|i| i["kind"] == "market_sold"), "{bell}");
    let (_, theirs) = t.call("GET", &format!("/api/v1/market/{sid}/mine"), Some(&tok["Alex"]), None).await;
    assert_eq!(theirs["history"][0]["kind"], "sold");
    assert_eq!(theirs["history"][0]["amount"], 100.0);
    let (_, steves) = t.call("GET", &format!("/api/v1/market/{sid}/mine"), Some(&tok["Steve"]), None).await;
    assert_eq!((steves["history"][0]["kind"].as_str(), steves["waiting"].as_i64()), (Some("bought"), Some(1)));

    // The game server collects it for the vault. While leased, an in-game /market claim doesn't also hand it over.
    let (_, pulled) = game("economy/market/vault", json!({})).await;
    let d = &pulled["deliveries"][0];
    assert_eq!((d["uuid"].as_str(), d["item_id"].as_str(), d["item_data"].as_str()), (Some(uuid["Steve"].as_str()), Some("DIAMOND_SWORD"), Some("snbt")));
    let (_, claim) = game("economy/market/mailbox/claim", json!({"operation_id": "c1", "uuid": uuid["Steve"]})).await;
    assert!(claim["items"].as_array().unwrap().is_empty(), "leased items are not claimable twice");
    let (_, again) = game("economy/market/vault", json!({})).await;
    assert!(again["deliveries"].as_array().unwrap().is_empty());
    game("economy/market/vault/ack", json!({"ids": [d["id"]]})).await;
    let (_, steves) = t.call("GET", &format!("/api/v1/market/{sid}/mine"), Some(&tok["Steve"]), None).await;
    assert_eq!(steves["waiting"], 0);

    // Bidding, getting outbid, and cancelling.
    let (s, r) = t.call("POST", &format!("/api/v1/market/{sid}/bid"), Some(&tok["Steve"]), Some(json!({"listing_id": auction_id}))).await;
    assert_eq!((s, r["current_bid"].as_f64()), (StatusCode::OK, Some(50.0)), "{r}");
    let (_, v) = t.call("GET", &format!("/api/v1/market/{sid}"), Some(&tok["Steve"]), None).await;
    assert_eq!(v["listings"][0]["leading"], true);
    assert_eq!(t.call("POST", &format!("/api/v1/market/{sid}/bid"), Some(&tok["Mia"]), Some(json!({"listing_id": auction_id, "amount": 60.0}))).await.0, StatusCode::OK);
    let (_, bell) = t.call("GET", "/api/v1/notifications", Some(&tok["Steve"]), None).await;
    assert!(bell["items"].as_array().unwrap().iter().any(|i| i["kind"] == "auction_outbid"));
    assert_eq!(t.call("POST", &format!("/api/v1/market/{sid}/cancel"), Some(&tok["Alex"]), Some(json!({"listing_id": auction_id}))).await.0, StatusCode::BAD_REQUEST, "bids are binding");

    let (_, c) = game("economy/market/list", list("l3", "buy_now", 10.0)).await;
    assert_eq!(t.call("POST", &format!("/api/v1/market/{sid}/cancel"), Some(&tok["Mia"]), Some(json!({"listing_id": c["id"]}))).await.0, StatusCode::FORBIDDEN);
    let (s, r) = t.call("POST", &format!("/api/v1/market/{sid}/cancel"), Some(&tok["Alex"]), Some(json!({"listing_id": c["id"]}))).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    let (_, pulled) = game("economy/market/vault", json!({})).await;
    assert_eq!(pulled["deliveries"][0]["uuid"], uuid["Alex"].as_str(), "a cancelled listing returns to the seller's vault");

    // A vault with no room sends the item back to the mailbox and tells the player.
    game("economy/market/vault/ack", json!({"ids": [], "failed": [pulled["deliveries"][0]["id"]]})).await;
    let (_, bell) = t.call("GET", "/api/v1/notifications", Some(&tok["Alex"]), None).await;
    assert!(bell["items"].as_array().unwrap().iter().any(|i| i["kind"] == "market_vault_full"));
    let (_, claim) = game("economy/market/mailbox/claim", json!({"operation_id": "c2", "uuid": uuid["Alex"]})).await;
    assert_eq!(claim["items"].as_array().unwrap().len(), 1, "still claimable by hand");
}
