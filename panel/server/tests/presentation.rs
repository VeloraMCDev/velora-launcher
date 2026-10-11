//! Presentation settings use temporary stores and synthetic data only.
mod common;
use common::*;

#[tokio::test]
async fn guide_and_icons_publish_to_manifest_and_reject_invalid_edits() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    let (_, mut brand) = t.call("GET", "/api/admin/branding", Some(&admin), None).await;
    brand["player_guide"] = json!({
        "title": "Test Player Guide", "introduction": "Synthetic server help", "show_commands": false,
        "articles": [{ "id": "rules", "category": "Server", "title": "Rules", "body": "Be kind.", "icon": "book", "links": [{ "label": "Help", "url": "https://example.com/help" }] }]
    });
    brand["interface_icons"] = json!({"nav:commands": "enchanted_book", "ui:Search": "compass", "item:diamond": "emerald"});
    assert_eq!(t.call("PUT", "/api/admin/branding", None, Some(brand.clone())).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(t.call("PUT", "/api/admin/branding", Some(&admin), Some(brand.clone())).await.0, StatusCode::OK);
    let (_, manifest) = t.call("GET", "/api/v1/launcher/manifest", None, None).await;
    assert_eq!(manifest["branding"]["player_guide"], brand["player_guide"]);
    assert_eq!(manifest["branding"]["interface_icons"], brand["interface_icons"]);
    let mut invalid = brand.clone();
    invalid["player_guide"]["articles"][0]["links"][0]["url"] = json!("javascript:alert(1)");
    assert_eq!(t.call("PUT", "/api/admin/branding", Some(&admin), Some(invalid)).await.0, StatusCode::BAD_REQUEST);
    let (_, saved) = t.call("GET", "/api/admin/branding", Some(&admin), None).await;
    assert_eq!(saved, brand);
}

#[tokio::test]
async fn logo_dimensions_survive_instance_updates_without_replacing_experience() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    let (status, created) = t.call("POST", "/api/admin/instances", Some(&admin), Some(json!({
        "name": "Synthetic Pack", "mc_version": "1.20.1", "loader": "vanilla",
        "logo_layout": { "width": 720, "max_height": 240, "title_size": 64 }
    }))).await;
    assert_eq!(status, StatusCode::OK);
    let id = created["id"].as_str().unwrap();
    let path = format!("/api/admin/instances/{id}");
    let (_, detail) = t.call("GET", &path, Some(&admin), None).await;
    assert_eq!(detail["instance"]["experience"]["modules"]["launcher_presentation"]["width"], 720);
    // Seed unrelated module metadata directly: this fixture uses an in-memory
    // platform database, whereas experience configuration opens disk stores.
    sqlx::query("UPDATE instances SET experience=json_set(experience,'$.modules.synthetic_extension',json(?)) WHERE id=?")
        .bind(r#"{"enabled":true}"#).bind(id).execute(&t.db).await.unwrap();
    let mut input = detail["instance"].clone();
    input["logo_layout"] = json!({"width": 900, "max_height": 320, "title_size": 72});
    let (status, saved) = t.call("PUT", &path, Some(&admin), Some(input.clone())).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(saved["instance"]["experience"]["modules"]["launcher_presentation"]["width"], 900);
    assert_eq!(saved["instance"]["experience"]["modules"]["synthetic_extension"]["enabled"], true);
    assert_eq!(saved["instance"]["id"], id);
    input["logo_layout"]["width"] = json!(0);
    assert_eq!(t.call("PUT", &path, Some(&admin), Some(input)).await.0, StatusCode::BAD_REQUEST);
    let (_, persisted) = t.call("GET", &path, Some(&admin), None).await;
    assert_eq!(persisted["instance"]["experience"]["modules"]["launcher_presentation"]["width"], 900);
}
