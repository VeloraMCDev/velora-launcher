//! End-to-end tests against the real router with an in-memory database.

mod common;
use common::*;

#[tokio::test]
async fn health_and_default_manifest() {
    let t = setup().await;
    let (s, v) = t.call("GET", "/healthz", None, None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(v, "ok");
    let (s, v) = t.call("GET", "/api/v1/launcher/manifest", None, None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(v["branding"]["name"], "SCOPENET");
    assert_eq!(v["api_version"], 1);
    assert!(v["user"].is_null());
}

#[tokio::test]
async fn admin_only_routes_are_guarded() {
    let t = setup().await;
    let (s, _) = t.call("GET", "/api/admin/users", None, None).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    let (s, _) = t.call("GET", "/api/admin/users", Some("garbage"), None).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);

    let admin = t.login("admin", "supersecret").await;
    let (s, v) = t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username": "Steve", "password": "password123"}))).await;
    assert_eq!(s, StatusCode::OK, "{v}");
    assert_eq!(uuid::Uuid::parse_str(v["uuid"].as_str().unwrap()).unwrap().get_version_num(), 4);

    let player = t.login("steve", "password123").await; // usernames are case-insensitive
    let (s, _) = t.call("GET", "/api/admin/users", Some(&player), None).await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    let (s, v) = t.call("GET", "/api/v1/auth/me", Some(&player), None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(v["username"], "Steve");
}

#[tokio::test]
async fn wrong_password_and_lockout() {
    let t = setup().await;
    for _ in 0..10 {
        let (s, _) = t.call("POST", "/api/v1/auth/login", None, Some(json!({"username": "admin", "password": "nope"}))).await;
        assert_eq!(s, StatusCode::UNAUTHORIZED);
    }
    let (s, _) = t.call("POST", "/api/v1/auth/login", None, Some(json!({"username": "admin", "password": "supersecret"}))).await;
    assert_eq!(s, StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn registration_modes() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    let reg = json!({"username": "Alex", "password": "password123"});
    let (s, _) = t.call("POST", "/api/v1/auth/register", None, Some(reg.clone())).await;
    assert_eq!(s, StatusCode::FORBIDDEN, "closed by default");

    let (s, v) = t.call("PUT", "/api/admin/settings", Some(&admin), Some(json!({"auth": {"registration": "approval"}}))).await;
    assert_eq!(s, StatusCode::OK, "{v}");
    let (s, v) = t.call("POST", "/api/v1/auth/register", None, Some(reg.clone())).await;
    assert_eq!(s, StatusCode::OK, "{v}");
    assert_eq!(v["pending"], true);
    let (s, _) = t.call("POST", "/api/v1/auth/login", None, Some(json!({"username": "Alex", "password": "password123"}))).await;
    assert_eq!(s, StatusCode::FORBIDDEN, "pending accounts can't sign in");

    let (s, _) = t.call("POST", "/api/v1/auth/register", None, Some(reg)).await;
    assert_eq!(s, StatusCode::CONFLICT);
}

#[tokio::test]
async fn instance_visibility_and_zip_upload() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    let (_, _) = t.call("POST", "/api/admin/groups", Some(&admin), Some(json!({"name": "VIP"}))).await;
    t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username": "Vip1", "password": "password123", "groups": ["VIP"]}))).await;
    t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username": "Pleb", "password": "password123"}))).await;

    let (s, v) = t
        .call(
            "POST",
            "/api/admin/instances",
            Some(&admin),
            Some(json!({"name": "VIP Survival!", "mc_version": "1.21.1", "visibility": "groups", "allowed_groups": ["VIP"],
                        "server": {"name": "SMP", "address": "play.example.net", "port": 25565, "auto_join": true, "inject": true}})),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{v}");
    assert_eq!(v["id"], "vip-survival");
    let (_, v) = t.call("POST", "/api/admin/instances", Some(&admin), Some(json!({"name": "Public", "mc_version": "1.20.1"}))).await;
    assert_eq!(v["id"], "public");

    let names = |v: &Value| v["instances"].as_array().unwrap().iter().map(|i| i["id"].as_str().unwrap().to_string()).collect::<Vec<_>>();
    let (_, anon) = t.call("GET", "/api/v1/launcher/manifest", None, None).await;
    assert_eq!(names(&anon), vec!["public"]);
    let vip = t.login("Vip1", "password123").await;
    let (_, m) = t.call("GET", "/api/v1/launcher/manifest", Some(&vip), None).await;
    assert_eq!(names(&m).len(), 2);
    assert_eq!(m["user"]["groups"][0], "VIP");
    let pleb = t.login("Pleb", "password123").await;
    let (_, m) = t.call("GET", "/api/v1/launcher/manifest", Some(&pleb), None).await;
    assert_eq!(names(&m), vec!["public"]);
    let (s, _) = t.call("GET", "/api/v1/launcher/instances/vip-survival", Some(&pleb), None).await;
    assert_eq!(s, StatusCode::NOT_FOUND);

    // Plain zip without version info → needs the fallback fields.
    let zip = zip_bytes(&[("MyPack/mods/cool.jar", b"jarjar"), ("MyPack/config/x.toml", b"a=1"), ("MyPack/logs/latest.log", b"junk")]);
    let (ct, body) = multipart(&[], ("pack.zip", &zip));
    let req = Request::post("/api/admin/instances/public/import/upload")
        .header("authorization", format!("Bearer {admin}"))
        .header("content-type", &ct)
        .body(Body::from(body))
        .unwrap();
    let (s, v) = t.send(req).await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "{v}");

    let (ct, body) = multipart(&[("mc_version", "1.20.1"), ("loader", "vanilla")], ("pack.zip", &zip));
    let req = Request::post("/api/admin/instances/public/import/upload")
        .header("authorization", format!("Bearer {admin}"))
        .header("content-type", &ct)
        .body(Body::from(body))
        .unwrap();
    let (s, v) = t.send(req).await;
    assert_eq!(s, StatusCode::OK, "{v}");
    let paths: Vec<&str> = v["files"].as_array().unwrap().iter().map(|f| f["path"].as_str().unwrap()).collect();
    assert_eq!(paths, vec!["config/x.toml", "mods/cool.jar"]);
    assert_eq!(v["instance"]["revision"], 2);

    // Launcher sees the files and can download them.
    let (_, im) = t.call("GET", "/api/v1/launcher/instances/public", None, None).await;
    let url = im["files"][1]["url"].as_str().unwrap().to_string();
    assert_eq!(url, "/files/public/mods/cool.jar");
    assert_eq!(im["files"][1]["sha1"], scopenet_core::http::sha1_bytes(b"jarjar"));
    let resp = t.router.clone().oneshot(Request::get(&url).body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    assert_eq!(&bytes[..], b"jarjar");
}

#[tokio::test]
async fn mrpack_upload_sets_versions_and_overrides() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    t.call("POST", "/api/admin/instances", Some(&admin), Some(json!({"name": "Pack", "mc_version": "1.20.1"}))).await;
    let index = json!({
        "formatVersion": 1, "game": "minecraft", "versionId": "6.2.0", "name": "Fabulous",
        "files": [
            {"path": "mods/sodium.jar", "hashes": {"sha1": "aaa", "sha512": "x"}, "downloads": ["https://cdn.modrinth.com/sodium.jar"], "fileSize": 10},
            {"path": "mods/server-only.jar", "hashes": {"sha1": "bbb"}, "env": {"client": "unsupported", "server": "required"}, "downloads": ["https://cdn.modrinth.com/s.jar"], "fileSize": 5},
            {"path": "../escape.jar", "hashes": {"sha1": "ccc"}, "downloads": ["https://x/e.jar"], "fileSize": 1}
        ],
        "dependencies": {"minecraft": "1.21.1", "fabric-loader": "0.16.9"}
    });
    let zip = zip_bytes(&[
        ("modrinth.index.json", index.to_string().as_bytes()),
        ("overrides/options.txt", b"fov:0.5"),
        ("overrides/config/a.json", b"{}"),
        ("client-overrides/config/a.json", b"{\"client\":1}"),
    ]);
    let (ct, body) = multipart(&[], ("fab.mrpack", &zip));
    let req = Request::post("/api/admin/instances/pack/import/upload")
        .header("authorization", format!("Bearer {admin}"))
        .header("content-type", &ct)
        .body(Body::from(body))
        .unwrap();
    let (s, v) = t.send(req).await;
    assert_eq!(s, StatusCode::OK, "{v}");
    assert_eq!(v["instance"]["mc_version"], "1.21.1");
    assert_eq!(v["instance"]["loader"], "fabric");
    assert_eq!(v["instance"]["loader_version"], "0.16.9");
    assert_eq!(v["instance"]["source_kind"], "modrinth");
    assert_eq!(v["instance"]["source_label"], "Fabulous 6.2.0");
    let files = v["files"].as_array().unwrap();
    let paths: Vec<&str> = files.iter().map(|f| f["path"].as_str().unwrap()).collect();
    assert_eq!(paths, vec!["config/a.json", "mods/sodium.jar", "options.txt"]);
    let cfg = files.iter().find(|f| f["path"] == "config/a.json").unwrap();
    assert_eq!(cfg["sha1"], scopenet_core::http::sha1_bytes(b"{\"client\":1}"), "client-overrides win");
}

#[tokio::test]
async fn branding_roundtrip_and_media_validation() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    let (_, mut b) = t.call("GET", "/api/admin/branding", Some(&admin), None).await;
    b["name"] = json!("Craftopia");
    b["colors"]["accent"] = json!("#ff5500");
    let (s, _) = t.call("PUT", "/api/admin/branding", Some(&admin), Some(b)).await;
    assert_eq!(s, StatusCode::OK);
    let (_, m) = t.call("GET", "/api/v1/launcher/manifest", None, None).await;
    assert_eq!(m["branding"]["name"], "Craftopia");
    assert_eq!(m["branding"]["colors"]["accent"], "#ff5500");

    let (ct, body) = multipart(&[], ("evil.svg", b"<svg onload=alert(1)>"));
    let req = Request::post("/api/admin/uploads")
        .header("authorization", format!("Bearer {admin}"))
        .header("content-type", &ct)
        .body(Body::from(body))
        .unwrap();
    let (s, _) = t.send(req).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let (ct, body) = multipart(&[], ("logo.png", b"\x89PNG"));
    let req = Request::post("/api/admin/uploads")
        .header("authorization", format!("Bearer {admin}"))
        .header("content-type", &ct)
        .body(Body::from(body))
        .unwrap();
    let (s, v) = t.send(req).await;
    assert_eq!(s, StatusCode::OK);
    assert!(v["url"].as_str().unwrap().starts_with("/uploads/"));
}

#[tokio::test]
async fn settings_never_leak_curseforge_key() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    let (s, v) = t.call("PUT", "/api/admin/settings", Some(&admin), Some(json!({"curseforge_api_key": "secret-key"}))).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(v["curseforge_key_set"], true);
    assert!(v["curseforge_api_key"].is_null());
    // Saving again with an empty key keeps it.
    let (_, v) = t.call("PUT", "/api/admin/settings", Some(&admin), Some(json!({"curseforge_api_key": ""}))).await;
    assert_eq!(v["curseforge_key_set"], true);
    let (s, _) = t.call("PUT", "/api/admin/settings", Some(&admin), Some(json!({"public_url": "ftp://nope"}))).await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "public URL must be http(s)");
}

#[tokio::test]
async fn landing_page_config_and_permissions() {
    let t = setup().await;
    // 1. Public landing endpoint accessible without authentication
    let (s, v) = t.call("GET", "/api/v1/landing", None, None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(v["brand_name"], "SCOPENET");
    assert_eq!(v["enabled"], true);
    assert!(v["blocks"].as_array().unwrap().len() >= 5);
    assert_eq!(v["theme"]["accent"], "#6d6af5");

    // 2. Unauthenticated user cannot modify landing config
    let (s, _) = t.call("PUT", "/api/admin/landing", None, Some(json!({"brand_name": "Hacked"}))).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);

    // 3. Admin can update landing config
    let admin = t.login("admin", "supersecret").await;
    let mut updated = v.clone();
    updated["brand_name"] = json!("PixelCraft");
    updated["hero_title"] = json!("Welcome to PixelCraft");
    updated["theme"]["accent"] = json!("#17bebb");
    updated["blocks"].as_array_mut().unwrap().push(json!({
        "id": "b_custom_text", "type": "text", "enabled": true, "title": "Our story",
        "subtitle": "", "options": { "body": "Built by players." }
    }));
    let (s, v2) = t.call("PUT", "/api/admin/landing", Some(&admin), Some(updated)).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(v2["brand_name"], "PixelCraft");
    assert_eq!(v2["hero_title"], "Welcome to PixelCraft");
    assert_eq!(v2["theme"]["accent"], "#17bebb");
    assert_eq!(v2["blocks"].as_array().unwrap().last().unwrap()["type"], "text");

    // Duplicate IDs would break drag ordering and keyed public rendering.
    let mut duplicate = v2.clone();
    let first = duplicate["blocks"][0].clone();
    duplicate["blocks"].as_array_mut().unwrap().push(first);
    let (s, _) = t.call("PUT", "/api/admin/landing", Some(&admin), Some(duplicate)).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);

    // 4. Public endpoint reflects the updated config
    let (s, v3) = t.call("GET", "/api/v1/landing", None, None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(v3["brand_name"], "PixelCraft");
}

#[tokio::test]
async fn a_clean_update_raises_the_epoch_launchers_compare_against() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    t.call("POST", "/api/admin/instances", Some(&admin), Some(json!({"name": "Pack", "mc_version": "1.20.1"}))).await;
    let (_, before) = t.call("GET", "/api/v1/launcher/instances/pack", None, None).await;
    let (epoch, revision) = (before["instance"]["clean_epoch"].as_i64().unwrap(), before["instance"]["revision"].as_i64().unwrap());
    let (s, after) = t.call("POST", "/api/admin/instances/pack/clean-update", Some(&admin), None).await;
    assert_eq!(s, StatusCode::OK, "{after}");
    let (_, manifest) = t.call("GET", "/api/v1/launcher/instances/pack", None, None).await;
    assert_eq!(manifest["instance"]["clean_epoch"].as_i64().unwrap(), epoch + 1);
    assert!(manifest["instance"]["revision"].as_i64().unwrap() > revision, "launchers only re-sync when the revision moves");
    let (s, _) = t.call("POST", "/api/admin/instances/nope/clean-update", Some(&admin), None).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    let (s, _) = t.call("POST", "/api/admin/instances/pack/clean-update", None, None).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn mac_and_linux_installers_are_uploaded_listed_with_short_names_and_removed() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    let post = |platform: &'static str, name: &'static str, bytes: Vec<u8>| {
        let (mime, body) = multipart(&[("platform", platform)], (name, &bytes));
        let t = &t;
        let admin = admin.clone();
        async move {
            t.send(
                Request::builder()
                    .method("POST")
                    .uri("/api/admin/landing/upload-launcher")
                    .header("authorization", format!("Bearer {admin}"))
                    .header("content-type", mime)
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
        }
    };
    let (s, _) = post("mac", "evil.exe", b"x".to_vec()).await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "a Windows file is not a macOS installer");
    let (s, _) = post("windows", "a.dmg", b"x".to_vec()).await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "Windows has its own publish step");
    let (s, v) = post("mac", "SCOPENET Launcher_1.0.1_universal.dmg", b"disk image".to_vec()).await;
    assert_eq!(s, StatusCode::OK, "{v}");
    let (s, _) = post("linux", "Launcher_1.0.1_amd64.AppImage", b"appimage".to_vec()).await;
    assert_eq!(s, StatusCode::OK);
    let (s, _) = post("linux", "Launcher_1.0.1_amd64.deb", b"deb".to_vec()).await;
    assert_eq!(s, StatusCode::OK);
    let (_, landing) = t.call("GET", "/api/v1/landing", None, None).await;
    let list = landing["hosted_downloads"].as_array().unwrap();
    let names: Vec<_> = list.iter().map(|d| d["display_name"].as_str().unwrap().to_string()).collect();
    assert_eq!(names, vec!["SCOPENET-1.0.1-macOS.dmg", "SCOPENET-1.0.1-Linux.AppImage", "SCOPENET-1.0.1-Linux.deb"], "{landing}");
    // Each file downloads under its short name.
    let url = list[1]["file_url"].as_str().unwrap();
    let resp = t.router.clone().oneshot(Request::get(url).body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.headers()["content-disposition"], "attachment; filename=\"SCOPENET-1.0.1-Linux.AppImage\"");
    // A new AppImage replaces the old one; the .deb stays.
    post("linux", "Launcher_1.0.2_amd64.AppImage", b"newer".to_vec()).await;
    let (_, landing) = t.call("GET", "/api/v1/landing", None, None).await;
    assert_eq!(landing["hosted_downloads"].as_array().unwrap().len(), 4 - 1 + 0, "{landing}");
    let (s, after) = t.call("DELETE", "/api/admin/landing/launcher/linux", Some(&admin), None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(after["hosted_downloads"].as_array().unwrap().len(), 1, "only macOS is left");
}
