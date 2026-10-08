mod common;
use base64::{engine::general_purpose::STANDARD, Engine};
use common::*;
use std::io::{Cursor, Read};

#[tokio::test]
async fn inventory_kits_require_server_auth_and_preserve_metadata() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    let (_, server) = t.call("POST", "/api/admin/servers", Some(&admin), Some(json!({"name":"SMP","instance_id":"smp"}))).await;
    let key = server["token"].as_str().unwrap();
    let items = json!([{"item":"minecraft:diamond_sword","amount":1,"data":{"format":"nbt","value":"{id:\"minecraft:diamond_sword\",Count:1b,tag:{Damage:7}}"}}]);
    let body = json!({"id":"Legend","items":items});
    assert_eq!(t.call("POST", "/api/server/v1/kits/create", None, Some(body.clone())).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(t.call("POST", "/api/server/v1/kits/create", Some(key), Some(body.clone())).await.0, StatusCode::OK);
    assert_eq!(t.call("POST", "/api/server/v1/kits/create", Some(key), Some(body)).await.0, StatusCode::BAD_REQUEST);
    for bad in [json!({"id":"empty","items":[]}), json!({"id":"create","items":items}), json!({"id":"bad name","items":items})] {
        assert_eq!(t.call("POST", "/api/server/v1/kits/create", Some(key), Some(bad)).await.0, StatusCode::BAD_REQUEST);
    }
    let (_, sync) = t.call("POST", "/api/server/v1/sync", Some(key), Some(json!({"online":[]}))).await;
    assert_eq!(sync["utilities"]["kits"][0]["id"], "legend");
    assert_eq!(sync["utilities"]["kits"][0]["items"], items);
    assert_eq!(sync["utilities"]["vault"]["count"], 3);
}

#[tokio::test]
async fn custom_rewards_validate_references_and_snapshot_styled_items() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    let uuid = t.uuid("admin").await;
    let (_, server) = t.call("POST", "/api/admin/servers", Some(&admin), Some(json!({"name":"SMP","instance_id":"smp"}))).await;
    let key = server["token"].as_str().unwrap();
    let actions = json!({"actions":[{"type":"custom_item","custom":"blade","amount":128}]});
    assert_eq!(t.call("PUT", "/api/admin/reward-bundles/quest/test", Some(&admin), Some(actions.clone())).await.0, StatusCode::BAD_REQUEST);
    let spec = json!({"item":"minecraft:diamond_sword","amount":1,"name":"&bBlade","lore":["Legendary"],"enchants":{"sharpness":10},"custom_model_data":10000});
    assert_eq!(
        t.call("PUT", "/api/admin/custom-items/blade", Some(&admin), Some(json!({"title":"Blade","spec":spec}))).await.0,
        StatusCode::OK
    );
    assert_eq!(t.call("PUT", "/api/admin/reward-bundles/quest/test", Some(&admin), Some(actions)).await.0, StatusCode::OK);
    assert_eq!(t.call("DELETE", "/api/admin/custom-items/blade", Some(&admin), None).await.0, StatusCode::BAD_REQUEST);
    sqlx::query("INSERT INTO reward_queue(uuid,source_type,source_id,created_at) VALUES (?, 'quest','test','2026-10-01')")
        .bind(&uuid)
        .execute(&t.db)
        .await
        .unwrap();
    t.call("POST", "/api/server/v1/rewards/poll", Some(key), Some(json!({}))).await;
    let raw: String = sqlx::query_scalar("SELECT payload FROM reward_deliveries WHERE uuid = ?").bind(uuid).fetch_one(&t.db).await.unwrap();
    let payload: Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(payload["spec"], spec);
    assert_eq!(payload["amount"], 128);
}

#[tokio::test]
async fn assets_generate_stable_packs_and_reject_unsafe_paths() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    let (_, server) = t.call("POST", "/api/admin/servers", Some(&admin), Some(json!({"name":"SMP","instance_id":"smp"}))).await;
    let key = server["token"].as_str().unwrap();
    let mut png = Cursor::new(Vec::new());
    image::DynamicImage::new_rgba8(16, 16).write_to(&mut png, image::ImageFormat::Png).unwrap();
    let data = STANDARD.encode(png.into_inner());
    assert_eq!(
        t.call("POST", "/api/admin/resource-assets", None, Some(json!({"path":"assets/scopenet/textures/item/blade.png","data":data})))
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    for path in ["assets/scopenet/../../blade.png", "assets/scopenet/textures/../blade.png", "assets/scopenet/textures/item/blade.exe"] {
        assert_eq!(
            t.call("POST", "/api/admin/resource-assets", Some(&admin), Some(json!({"path":path,"data":data}))).await.0,
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(
        t.call(
            "POST",
            "/api/admin/resource-assets",
            Some(&admin),
            Some(json!({"path":"assets/scopenet/textures/item/blade.png","data":data}))
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        t.call(
            "PUT",
            "/api/admin/custom-items/blade",
            Some(&admin),
            Some(
                json!({"title":"Blade","spec":{"item":"minecraft:diamond_sword","custom_model_data":10000,"texture":"scopenet:item/blade"}})
            )
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        t.call("PUT", "/api/admin/resource-pack", Some(&admin), Some(json!({"enabled":true,"required":true,"pack_format":15}))).await.0,
        StatusCode::OK
    );
    let (status, bytes) = t.fetch("/api/v1/resource-pack.zip").await;
    assert_eq!(status, StatusCode::OK);
    let (_, again) = t.fetch("/api/v1/resource-pack.zip").await;
    assert_eq!(bytes, again, "pack hashes must stay stable without changes");
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    for path in [
        "pack.mcmeta",
        "assets/scopenet/textures/item/blade.png",
        "assets/scopenet/models/item/blade.json",
        "assets/minecraft/models/item/diamond_sword.json",
        "assets/minecraft/items/diamond_sword.json",
    ] {
        assert!(zip.by_name(path).is_ok(), "{path}");
    }
    let mut text = String::new();
    zip.by_name("assets/minecraft/models/item/diamond_sword.json").unwrap().read_to_string(&mut text).unwrap();
    let model: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(model["overrides"][0]["predicate"]["custom_model_data"], 10000);
    let (_, pack) = t.call("POST", "/api/server/v1/resource-pack", Some(key), Some(json!({}))).await;
    assert_eq!(pack["sha1"].as_str().unwrap().len(), 40);
    assert_eq!(pack["required"], true);
    let revision = pack["sha1"].as_str().unwrap();
    let (_, snapshot) = t.fetch(&format!("/api/v1/resource-pack.zip?revision={revision}")).await;
    let (_, original) = t.fetch("/api/v1/resource-pack.zip").await;
    assert_eq!(snapshot, original);
    assert_eq!(
        t.call("DELETE", "/api/admin/resource-assets/assets/scopenet/textures/item/blade.png", Some(&admin), None).await.0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(t.call("PUT", "/api/admin/custom-items/duplicate", Some(&admin), Some(json!({"title":"Duplicate","spec":{"item":"minecraft:diamond_sword","custom_model_data":10000,"texture":"scopenet:item/blade"}}))).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(
        t.call("PUT", "/api/admin/resource-pack", Some(&admin), Some(json!({"enabled":true,"required":true,"pack_format":34}))).await.0,
        StatusCode::OK
    );
    let (_, changed) = t.fetch("/api/v1/resource-pack.zip").await;
    assert_ne!(changed, snapshot);
    assert_eq!(
        t.fetch(&format!("/api/v1/resource-pack.zip?revision={revision}")).await.1,
        snapshot,
        "an offered hash must keep its exact ZIP after edits"
    );
}

#[tokio::test]
async fn imports_an_itemsadder_pack_into_assets_items_and_the_generated_pack() {
    use std::io::Write;
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    let yml = "info:\n  namespace: ruby\nitems:\n  ruby_sword:\n    display_name: \"&cRuby Sword\"\n    resource:\n      material: DIAMOND_SWORD\n      textures:\n        - item/ruby_sword.png\n";
    let png = tiny_png();
    let mut z = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let o = zip::write::SimpleFileOptions::default();
    for (name, data) in [
        ("contents/ruby/configs/items.yml", yml.as_bytes()),
        ("contents/ruby/resourcepack/ruby/textures/item/ruby_sword.png", png.as_slice()),
    ] {
        z.start_file(name, o).unwrap();
        z.write_all(data).unwrap();
    }
    let data = STANDARD.encode(z.finish().unwrap().into_inner());

    assert_eq!(t.call("POST", "/api/admin/resource-assets-import", None, Some(json!({"data":data}))).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(t.call("POST", "/api/admin/resource-assets-import", Some(&admin), Some(json!({"data":"@@"}))).await.0, StatusCode::BAD_REQUEST);

    // A preview changes nothing.
    let (status, preview) = t.call("POST", "/api/admin/resource-assets-import", Some(&admin), Some(json!({"data":data,"dry_run":true}))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!((preview["source"].as_str(), preview["files_added"].as_i64()), (Some("itemsadder"), Some(1)));
    assert_eq!(preview["items"][0]["status"], "would_create");
    assert!(preview["items"][0]["preview"].as_str().unwrap().starts_with("data:image/png;base64,"), "previews show what the item looks like");

    // Deselected items are listed but not created; edits made in the preview apply on import.
    let (_, none) = t.call("POST", "/api/admin/resource-assets-import", Some(&admin), Some(json!({"data":data,"only":[]}))).await;
    assert_eq!(none["items"][0]["status"], "not_selected");
    assert_eq!(t.call("GET", "/api/admin/custom-items", Some(&admin), None).await.1["items"].as_array().unwrap().len(), 0);
    let (_, browse) = t.call("GET", "/api/admin/resource-assets-browse?kind=texture&q=ruby", Some(&admin), None).await;
    assert_eq!((browse["total"].as_i64(), browse["items"][0]["ref"].as_str()), (Some(1), Some("ruby:item/ruby_sword")));
    assert!(browse["items"][0]["preview"].as_str().unwrap().starts_with("data:image/png"));
    let (_, one) = t.call("GET", "/api/admin/resource-assets-preview?texture=ruby:item/ruby_sword", Some(&admin), None).await;
    assert!(one["preview"].is_string());
    assert_eq!(t.call("GET", "/api/admin/custom-items", Some(&admin), None).await.1["items"].as_array().unwrap().len(), 0);

    let (status, done) = t
        .call("POST", "/api/admin/resource-assets-import", Some(&admin), Some(json!({"data":data,"edits":{"ruby_sword":{"item":"netherite_sword","name":"&bRuby Blade"}}})))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(done["items"][0]["status"], "created");
    let (_, items) = t.call("GET", "/api/admin/custom-items", Some(&admin), None).await;
    let spec = &items["items"][0]["spec"];
    assert_eq!((spec["item"].as_str(), spec["texture"].as_str(), spec["name"].as_str()), (Some("minecraft:netherite_sword"), Some("ruby:item/ruby_sword"), Some("&bRuby Blade")));
    let (_, listed) = t.call("GET", "/api/admin/resource-assets", Some(&admin), None).await;
    assert_eq!(listed["files"][0], "assets/ruby/textures/item/ruby_sword.png");

    // Importing again leaves the existing item alone.
    let (_, again) = t.call("POST", "/api/admin/resource-assets-import", Some(&admin), Some(json!({"data":data}))).await;
    assert_eq!((again["items"][0]["status"].as_str(), again["files_replaced"].as_i64()), (Some("exists"), Some(1)));

    // The generated pack carries the imported texture and a model override for the item.
    let (_, bytes) = t.fetch("/api/v1/resource-pack.zip").await;
    let mut pack = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    assert!(pack.by_name("assets/ruby/textures/item/ruby_sword.png").is_ok());
    let mut model = String::new();
    pack.by_name("assets/minecraft/models/item/netherite_sword.json").unwrap().read_to_string(&mut model).unwrap();
    assert!(model.contains("custom_model_data") && model.contains("100000"));
}

#[tokio::test]
async fn models_that_dont_resolve_are_reported_and_repaired_in_the_pack() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    let mut png = Cursor::new(Vec::new());
    image::DynamicImage::new_rgba8(16, 16).write_to(&mut png, image::ImageFormat::Png).unwrap();
    let png = STANDARD.encode(png.into_inner());
    let model = STANDARD.encode(br#"{"parent":"item/handheld","textures":{"layer0":"weapons/head"},"elements":[]}"#);
    for (path, data) in [("assets/med/textures/weapons/head.png", &png), ("assets/med/models/item/hammer.json", &model)] {
        let (s, r) = t.call("POST", "/api/admin/resource-assets", Some(&admin), Some(json!({"path":path,"data":data}))).await;
        assert_eq!(s, StatusCode::OK, "{r}");
    }
    // As stored, `weapons/head` means minecraft:weapons/head, which doesn't exist: the health check says so.
    let (_, check) = t.call("GET", "/api/admin/resource-assets-check", Some(&admin), None).await;
    assert_eq!((check["problems"][0]["kind"].as_str(), check["problems"][0]["reference"].as_str()), (Some("texture"), Some("minecraft:weapons/head")));

    // The pack fixes the reference to the namespace that holds the file and registers the texture in the atlases.
    let (_, bytes) = t.fetch("/api/v1/resource-pack.zip").await;
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut model = String::new();
    zip.by_name("assets/med/models/item/hammer.json").unwrap().read_to_string(&mut model).unwrap();
    assert!(model.contains("med:weapons/head"), "{model}");
    let mut atlas = String::new();
    zip.by_name("assets/minecraft/atlases/blocks.json").unwrap().read_to_string(&mut atlas).unwrap();
    assert!(atlas.contains("med:weapons/head"), "{atlas}");
}
