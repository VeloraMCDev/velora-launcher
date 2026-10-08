mod common;
use common::*;
use std::io::Write;

fn zip_with(entries: &[&str]) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut z = zip::ZipWriter::new(std::io::Cursor::new(&mut out));
        for e in entries {
            z.start_file(*e, zip::write::SimpleFileOptions::default()).unwrap();
            z.write_all(b"x").unwrap();
        }
        z.finish().unwrap();
    }
    out
}

async fn upload(t: &TestApp, token: &str, platform: &str, file: &str, bytes: &[u8]) -> (StatusCode, Value) {
    let (mime, body) = multipart(&[("platform", platform), ("version", "v0.9.4"), ("notes", "Fixes")], (file, bytes));
    t.send(
        Request::builder()
            .method("POST")
            .uri("/api/admin/mobile-apps")
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", mime)
            .body(Body::from(body))
            .unwrap(),
    )
    .await
}

#[tokio::test]
async fn publishes_apps_and_an_altstore_source() {
    let t = setup().await;
    let (status, none) = t.call("GET", "/api/v1/mobile-apps", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(none["android"].is_null() && none["ios"].is_null());
    let admin = t.login("admin", "supersecret").await;

    let apk = zip_with(&["AndroidManifest.xml", "classes.dex"]);
    let ipa = zip_with(&["Payload/App.app/Info.plist", "Payload/App.app/App"]);
    let (status, body) = upload(&t, &admin, "android", "scopenet-player.apk", &apk).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["android"]["version"], "0.9.4");
    assert_eq!(t.fetch("/api/v1/mobile-apps/android/download").await, (StatusCode::OK, apk));
    let (status, body) = upload(&t, &admin, "ios", "scopenet-player.ipa", &ipa).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(body["ios"]["altstore_url"].as_str().unwrap().ends_with("/api/v1/mobile-apps/altstore.json"));
    assert_eq!(t.fetch("/api/v1/mobile-apps/ios/download").await, (StatusCode::OK, ipa));

    let (status, source) = t.call("GET", "/api/v1/mobile-apps/altstore.json", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(source["apps"][0]["bundleIdentifier"], "net.scopenet.player");
    assert_eq!(source["apps"][0]["version"], "0.9.4");
    assert!(source["apps"][0]["downloadURL"].as_str().unwrap().ends_with("/api/v1/mobile-apps/ios/download"));

    let (_, landing) = t.call("GET", "/api/admin/landing", Some(&admin), None).await;
    assert_eq!(landing["hosted_downloads"].as_array().unwrap().len(), 2);
    let (status, _) = t.call("DELETE", "/api/admin/mobile-apps/ios", Some(&admin), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(t.fetch("/api/v1/mobile-apps/ios/download").await.0, StatusCode::NOT_FOUND);
    let (_, source) = t.call("GET", "/api/v1/mobile-apps/altstore.json", None, None).await;
    assert!(source["apps"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn rejects_wrong_files_and_non_admins() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    assert_eq!(upload(&t, &admin, "android", "x.apk", b"not a zip").await.0, StatusCode::BAD_REQUEST);
    assert_eq!(upload(&t, &admin, "android", "x.ipa", &zip_with(&["AndroidManifest.xml"])).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(upload(&t, &admin, "ios", "x.ipa", &zip_with(&["AndroidManifest.xml"])).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(upload(&t, &admin, "windows", "x.apk", &zip_with(&["AndroidManifest.xml"])).await.0, StatusCode::BAD_REQUEST);
    t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username":"Alex","password":"password123"}))).await;
    let player = t.login("Alex", "password123").await;
    assert_eq!(upload(&t, &player, "android", "x.apk", &zip_with(&["AndroidManifest.xml"])).await.0, StatusCode::FORBIDDEN);
}
