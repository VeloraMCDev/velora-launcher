//! Cross-server network: servers in one economy group share balances and guild banks.

mod common;
use common::*;

#[tokio::test]
async fn servers_in_an_economy_group_share_balances() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username": "Alex", "password": "password123"}))).await;
    let uuid = t.uuid("Alex").await;

    let mut tokens = vec![];
    let mut ids = vec![];
    for (name, group) in [("SMP", "network"), ("Survival", "Network"), ("Modded", "")] {
        let (s, r) = t
            .call(
                "POST",
                "/api/admin/servers",
                Some(&admin),
                Some(json!({"name": name, "instance_id": name.to_lowercase(), "economy_group": group})),
            )
            .await;
        assert_eq!(s, StatusCode::OK, "{r}");
        tokens.push(r["token"].as_str().unwrap().to_string());
        ids.push(r["server"]["id"].as_i64().unwrap());
    }
    let adjust = |token: String, op: &'static str, delta: f64| {
        let (t, uuid) = (&t, uuid.clone());
        async move {
            t.call(
                "POST",
                "/api/server/v1/economy/adjust",
                Some(&token),
                Some(json!({"uuid": uuid, "username": "Alex", "delta": delta, "operation_id": op, "description": "test"})),
            )
            .await
        }
    };
    let balance = |token: String| {
        let (t, uuid) = (&t, uuid.clone());
        async move { t.call("POST", "/api/server/v1/economy/balance", Some(&token), Some(json!({"uuid": uuid, "username": "Alex"}))).await }
    };

    // Earning on one server shows up on the other in the group (new accounts start at 1000) …
    let (s, r) = adjust(tokens[0].clone(), "a", 250.0).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    assert_eq!(balance(tokens[1].clone()).await.1["balance"], 1250.0);
    // … spending on the second is visible on the first …
    assert_eq!(adjust(tokens[1].clone(), "b", -50.0).await.0, StatusCode::OK);
    assert_eq!(balance(tokens[0].clone()).await.1["balance"], 1200.0);
    // … and a server outside the group keeps its own economy.
    assert_eq!(balance(tokens[2].clone()).await.1["balance"], 1000.0);

    // Bad group names are refused.
    let (s, _) = t
        .call(
            "PUT",
            &format!("/api/admin/servers/{}", ids[2]),
            Some(&admin),
            Some(json!({"name": "Modded", "instance_id": "modded", "economy_group": "a<b>"})),
        )
        .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);

    // Removing the server that holds the shared balances keeps them for the rest of the group.
    assert_eq!(t.call("DELETE", &format!("/api/admin/servers/{}", ids[0]), Some(&admin), None).await.0, StatusCode::OK);
    assert_eq!(balance(tokens[1].clone()).await.1["balance"], 1200.0);
}
