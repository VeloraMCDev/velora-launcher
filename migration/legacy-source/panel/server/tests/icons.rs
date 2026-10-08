mod common;
use common::*;
use std::io::Write;

#[tokio::test]
async fn icons_are_public_svgs_and_search_is_admin_only() {
    let t = setup_with(|cfg, dir| {
        let icons = dir.join("icons");
        std::fs::create_dir_all(&icons).unwrap();
        let mut gz = flate2::write::GzEncoder::new(std::fs::File::create(icons.join("lucide.json.gz")).unwrap(), flate2::Compression::fast());
        gz.write_all(br#"{"width":24,"height":24,"icons":{"rocket":{"body":"<path stroke=\"currentColor\"/>"}}}"#).unwrap();
        gz.finish().unwrap();
        std::fs::write(icons.join("packs.json"), br#"[{"id":"lucide","label":"Lucide","palette":false,"license":"ISC","total":1}]"#).unwrap();
        cfg.icons_dir = icons;
    })
    .await;
    let admin = t.login("admin", "supersecret").await;

    let (status, bytes) = t.fetch("/api/v1/icons/lucide/rocket.svg?color=ff0000").await;
    assert_eq!(status, StatusCode::OK);
    let svg = String::from_utf8(bytes).unwrap();
    assert!(svg.starts_with("<svg") && svg.contains("#ff0000"));
    assert_eq!(t.fetch("/api/v1/icons/lucide/nothing.svg").await.0, StatusCode::NOT_FOUND);
    assert_eq!(t.fetch("/api/v1/icons/lucide/rocket.png").await.0, StatusCode::NOT_FOUND);

    assert_eq!(t.call("GET", "/api/admin/icons/search?q=rock", None, None).await.0, StatusCode::UNAUTHORIZED);
    let (_, found) = t.call("GET", "/api/admin/icons/search?q=rock", Some(&admin), None).await;
    assert_eq!((found["total"].as_i64(), found["icons"][0]["name"].as_str()), (Some(1), Some("rocket")));
    let (_, packs) = t.call("GET", "/api/admin/icons/packs", Some(&admin), None).await;
    assert_eq!(packs["packs"][0]["id"], "lucide");
}
