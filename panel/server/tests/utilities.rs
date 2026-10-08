//! Utility command settings, kits and custom items: validated in the panel, delivered to game servers with the sync.

mod common;
use common::*;

#[tokio::test]
async fn utilities_kits_and_custom_items_are_validated_and_synced() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    let (_, srv) = t.call("POST", "/api/admin/servers", Some(&admin), Some(json!({"name": "SMP", "instance_id": "smp"}))).await;
    let server = srv["token"].as_str().unwrap().to_string();

    // A custom item with lore, over-the-top enchantments and an attribute bonus.
    let blade = json!({
        "title": "Stormbreaker",
        "spec": {
            "item": "minecraft:netherite_axe", "name": "&b&lStormbreaker", "lore": ["&7Forged in lightning", "&eUnbreakable"],
            "enchants": {"sharpness": 50, "unbreaking": 10}, "unbreakable": true, "glow": true, "hide_flags": true,
            "attributes": [{"attribute": "generic.attack_damage", "amount": 40, "operation": "add", "slot": "mainhand"}],
            "junk": "dropped"
        }
    });
    let (s, r) = t.call("PUT", "/api/admin/custom-items/stormbreaker", Some(&admin), Some(blade)).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    assert!(r["spec"].get("junk").is_none(), "unknown keys are dropped");
    for (bad, why) in [
        (json!({"item": "Not An Id!"}), "bad id"),
        (json!({"item": "minecraft:stick", "amount": 99}), "amount"),
        (json!({"item": "minecraft:stick", "enchants": {"sharpness": 999}}), "enchant level"),
        (json!({"item": "minecraft:stick", "attributes": [{"attribute": "generic.nope", "amount": 1}]}), "attribute"),
        (json!({"item": "minecraft:stick", "lore": ["a\nb"]}), "multi-line lore"),
    ] {
        let (s, _) = t.call("PUT", "/api/admin/custom-items/bad", Some(&admin), Some(json!({"title": "x", "spec": bad}))).await;
        assert_eq!(s, StatusCode::BAD_REQUEST, "{why}");
    }

    // Utilities with a kit for a LuckPerms group, one referencing the custom item.
    let (_, d) = t.call("GET", "/api/admin/utilities", Some(&admin), None).await;
    assert_eq!(d["settings"]["vault"]["count"], 3);
    let mut u = d["settings"].clone();
    u["heal"] = json!({"enabled": true, "amount": 10, "cooldown_secs": 60});
    u["vault"]["count"] = json!(5);
    u["kits"] = json!([{
        "id": "VIP", "name": "VIP kit", "cooldown_secs": 86400, "groups": ["VIP", "Staff"],
        "items": [{"item": "minecraft:diamond", "amount": 16}, {"custom": "stormbreaker"}], "commands": ["/give {player} apple 1"]
    }]);
    let (s, r) = t.call("PUT", "/api/admin/utilities", Some(&admin), Some(u.clone())).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    assert_eq!(r["settings"]["kits"][0]["id"], "vip", "ids are lower-cased");
    assert_eq!(r["settings"]["kits"][0]["groups"], json!(["vip", "staff"]));
    assert_eq!(r["settings"]["kits"][0]["commands"][0], "give {player} apple 1");

    let mut bad = u.clone();
    bad["kits"][0]["items"] = json!([{"custom": "missing"}]);
    assert_eq!(t.call("PUT", "/api/admin/utilities", Some(&admin), Some(bad)).await.0, StatusCode::BAD_REQUEST);
    let mut bad = u.clone();
    bad["vault"]["rows"] = json!(9);
    assert_eq!(t.call("PUT", "/api/admin/utilities", Some(&admin), Some(bad)).await.0, StatusCode::BAD_REQUEST);
    let mut bad = u.clone();
    bad["kits"] = json!([{"id": "a"}, {"id": "A"}]);
    assert_eq!(t.call("PUT", "/api/admin/utilities", Some(&admin), Some(bad)).await.0, StatusCode::BAD_REQUEST, "duplicate ids");

    // A kit still using an item keeps it from being deleted.
    assert_eq!(t.call("DELETE", "/api/admin/custom-items/stormbreaker", Some(&admin), None).await.0, StatusCode::BAD_REQUEST);

    let (_, sync) = t.call("POST", "/api/server/v1/sync", Some(&server), Some(json!({"online": []}))).await;
    assert_eq!(sync["utilities"]["heal"]["amount"], 10.0);
    assert_eq!(sync["utilities"]["vault"]["count"], 5);
    assert_eq!(sync["custom_items"][0]["id"], "stormbreaker");
    assert_eq!(sync["custom_items"][0]["spec"]["enchants"]["sharpness"], 50);
}
