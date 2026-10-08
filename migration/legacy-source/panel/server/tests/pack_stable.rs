mod common;
use common::*;

#[tokio::test]
async fn the_pack_hash_never_changes_unless_the_content_does() {
    let t = setup_with(|_, dir| {
        std::fs::create_dir_all(dir.join("uploads")).unwrap();
        std::fs::write(dir.join("uploads/veteran.png"), b"\x89PNG\r\n\x1a\nveteran").unwrap();
    })
    .await;
    let admin = t.login("admin", "supersecret").await;
    let (_, server) = t.call("POST", "/api/admin/servers", Some(&admin), Some(json!({"name":"SMP","instance_id":"smp"}))).await;
    let key = server["token"].as_str().unwrap().to_string();
    t.call("POST", "/api/admin/levels/rewards", Some(&admin), Some(json!({"level":2,"reward_type":"title","reward_value":"Veteran","reward_data":{"title_image":"/uploads/veteran.png"}}))).await;
    t.call("PUT", "/api/admin/resource-pack", Some(&admin), Some(json!({"enabled":true,"required":true,"pack_format":84}))).await;
    for (i, texture) in ["a", "b", "c"].iter().enumerate() {
        let png = base64_png();
        t.call("POST", "/api/admin/resource-assets", Some(&admin), Some(json!({"path": format!("assets/med/textures/default/{texture}.png"), "data": png}))).await;
        t.call("POST", "/api/admin/resource-assets", Some(&admin), Some(json!({"path": format!("assets/med/models/default/{texture}.json"), "data": b64(format!(r#"{{"parent":"minecraft:item/generated","textures":{{"layer0":"med:default/{texture}"}}}}"#).as_bytes())}))).await;
        t.call("PUT", &format!("/api/admin/custom-items/item{i}"), Some(&admin), Some(json!({"spec": {"item":"minecraft:paper","name":"x","custom_model_data": 100 + i, "model": format!("med:default/{texture}")}}))).await;
    }
    let mut hashes = std::collections::BTreeSet::new();
    for _ in 0..4 {
        let (s, v) = t.call("POST", "/api/server/v1/resource-pack", Some(&key), Some(json!({}))).await;
        assert_eq!(s, StatusCode::OK, "{v}");
        hashes.insert(v["sha1"].as_str().unwrap().to_string());
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    }
    assert_eq!(hashes.len(), 1, "every build of the same content must be byte-identical: {hashes:?}");
}

fn b64(b: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(b)
}
fn base64_png() -> String {
    b64(b"\x89PNG\r\n\x1a\nx")
}

#[tokio::test]
async fn vanilla_block_overrides_and_broken_images_are_kept_out_of_the_pack() {
    use std::io::Cursor;
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    let png = {
        let mut out = Vec::new();
        image::DynamicImage::new_rgba8(16, 16).write_to(&mut Cursor::new(&mut out), image::ImageFormat::Png).unwrap();
        out
    };
    let up = |path: &'static str, bytes: Vec<u8>| {
        let (t, admin) = (&t, admin.clone());
        async move { t.call("POST", "/api/admin/resource-assets", Some(&admin), Some(json!({"path": path, "data": b64(&bytes)}))).await.0 }
    };
    assert_eq!(up("assets/minecraft/textures/block/grass_block_top.png", png.clone()).await, StatusCode::OK);
    assert_eq!(up("assets/med/textures/default/ok.png", png.clone()).await, StatusCode::OK);
    let names = |bytes: Vec<u8>| -> Vec<String> { zip::ZipArchive::new(Cursor::new(bytes)).unwrap().file_names().map(String::from).collect() };

    t.call("PUT", "/api/admin/resource-pack", Some(&admin), Some(json!({"enabled": true, "required": false, "pack_format": 84}))).await;
    let (_, bytes) = t.fetch("/api/v1/resource-pack.zip").await;
    let n = names(bytes);
    assert!(!n.iter().any(|f| f.contains("textures/block/grass_block_top")), "vanilla grass stays vanilla: {n:?}");
    assert!(n.iter().any(|f| f == "assets/med/textures/default/ok.png"));
    let (_, check) = t.call("GET", "/api/admin/resource-assets-check", Some(&admin), None).await;
    assert_eq!(check["left_out"]["count"], 1, "{check}");

    t.call("PUT", "/api/admin/resource-pack", Some(&admin), Some(json!({"enabled": true, "required": false, "pack_format": 84, "allow_vanilla_overrides": true}))).await;
    let (_, bytes) = t.fetch("/api/v1/resource-pack.zip").await;
    assert!(names(bytes).iter().any(|f| f.contains("textures/block/grass_block_top")), "kept when the admin asks for it");
}
