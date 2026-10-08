//! The authlib-injector flow end to end: sign-in, server join, signed
//! skins/capes, token lifecycle and chat certificates.

mod common;
use base64::Engine;
use common::*;
use rsa::pkcs1v15::{Signature, VerifyingKey};
use rsa::pkcs8::DecodePublicKey;
use rsa::signature::Verifier;
use rsa::RsaPublicKey;

const Y: &str = "/api/yggdrasil";

fn b64(s: &str) -> Vec<u8> {
    base64::engine::general_purpose::STANDARD.decode(s).unwrap()
}

fn verify(public_pem: &str, data: &[u8], sig_b64: &str) -> bool {
    let key = RsaPublicKey::from_public_key_pem(public_pem).unwrap();
    let sig = Signature::try_from(b64(sig_b64).as_slice()).unwrap();
    VerifyingKey::<sha1::Sha1>::new(key).verify(data, &sig).is_ok()
}

fn skin_png() -> Vec<u8> {
    let img = image::RgbaImage::from_pixel(64, 64, image::Rgba([30, 120, 200, 255]));
    let mut out = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png).unwrap();
    out
}

fn cape_png() -> Vec<u8> {
    let img = image::RgbaImage::from_pixel(64, 32, image::Rgba([200, 30, 30, 255]));
    let mut out = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png).unwrap();
    out
}

async fn with_player(t: &TestApp) -> String {
    let admin = t.login("admin", "supersecret").await;
    let (s, v) = t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username": "Steve", "password": "password123"}))).await;
    assert_eq!(s, StatusCode::OK, "{v}");
    admin
}

async fn steve_id(t: &TestApp) -> String {
    t.uuid("Steve").await.replace('-', "")
}

#[tokio::test]
async fn metadata_advertises_key_and_skin_domain() {
    let t = setup().await;
    for path in [Y, "/api/yggdrasil/"] {
        let (s, v) = t.call("GET", path, None, None).await;
        assert_eq!(s, StatusCode::OK, "{path}");
        assert_eq!(v["skinDomains"], json!(["panel.test"]));
        assert!(v["signaturePublickey"].as_str().unwrap().starts_with("-----BEGIN PUBLIC KEY-----"));
        assert_eq!(v["meta"]["feature.non_email_login"], true);
        assert_eq!(v["meta"]["feature.enable_profile_key"], true);
    }
    // API Location Indication header on every response.
    let resp = t.router.clone().oneshot(Request::get("/healthz").body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(resp.headers()["x-authlib-injector-api-location"], "/api/yggdrasil/");
    let (_, m) = t.call("GET", "/api/v1/launcher/manifest", None, None).await;
    assert_eq!(m["auth"]["yggdrasil_url"], "https://panel.test/api/yggdrasil");
}

#[tokio::test]
async fn full_authlib_flow() {
    let t = setup().await;
    with_player(&t).await;
    let (_, meta) = t.call("GET", Y, None, None).await;
    let public_pem = meta["signaturePublickey"].as_str().unwrap().to_string();

    // Sign in (username, not email) exactly as authlib does.
    let (s, auth) = t
        .call(
            "POST",
            &format!("{Y}/authserver/authenticate"),
            None,
            Some(json!({"agent": {"name": "Minecraft", "version": 1}, "username": "Steve", "password": "password123", "clientToken": "ct1", "requestUser": true})),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{auth}");
    assert_eq!(auth["clientToken"], "ct1");
    assert_eq!(auth["selectedProfile"]["id"], steve_id(&t).await);
    assert_eq!(auth["selectedProfile"]["name"], "Steve");
    assert_eq!(auth["availableProfiles"].as_array().unwrap().len(), 1);
    assert!(auth["user"]["id"].is_string());
    let token = auth["accessToken"].as_str().unwrap().to_string();

    let (s, v) =
        t.call("POST", &format!("{Y}/authserver/authenticate"), None, Some(json!({"username": "Steve", "password": "nope"}))).await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    assert_eq!(v["error"], "ForbiddenOperationException");

    // Validate.
    let (s, _) = t.call("POST", &format!("{Y}/authserver/validate"), None, Some(json!({"accessToken": token}))).await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    let (s, _) =
        t.call("POST", &format!("{Y}/authserver/validate"), None, Some(json!({"accessToken": token, "clientToken": "other"}))).await;
    assert_eq!(s, StatusCode::FORBIDDEN, "client token must match");

    // Upload a skin through the launcher API, pick a cape.
    let panel = t.login("Steve", "password123").await;
    let (ct, body) = multipart(&[("model", "slim")], ("skin.png", &skin_png()));
    let req = Request::post("/api/v1/account/skin")
        .header("authorization", format!("Bearer {panel}"))
        .header("content-type", &ct)
        .body(Body::from(body))
        .unwrap();
    let (s, profile) = t.send(req).await;
    assert_eq!(s, StatusCode::OK, "{profile}");
    let skin_url = profile["skin_url"].as_str().unwrap().to_string();
    assert!(skin_url.starts_with("https://panel.test/textures/"));
    assert_eq!(profile["skin_model"], "slim");

    let admin = t.login("admin", "supersecret").await;
    let (ct, body) = multipart(&[("name", "Founder"), ("visibility", "public"), ("allowed_groups", "[]")], ("cape.png", &cape_png()));
    let req = Request::post("/api/admin/capes")
        .header("authorization", format!("Bearer {admin}"))
        .header("content-type", &ct)
        .body(Body::from(body))
        .unwrap();
    let (s, capes) = t.send(req).await;
    assert_eq!(s, StatusCode::OK, "{capes}");
    let cape_id = capes[0]["id"].as_i64().unwrap();
    let (s, profile) = t.call("PUT", "/api/v1/account/cape", Some(&panel), Some(json!({"cape_id": cape_id}))).await;
    assert_eq!(s, StatusCode::OK, "{profile}");
    assert_eq!(profile["cape"]["name"], "Founder");

    // Texture file is served as PNG.
    let path = skin_url.trim_start_matches("https://panel.test");
    let resp = t.router.clone().oneshot(Request::get(path).body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.headers()["content-type"], "image/png");

    // Client joins; server verifies.
    let (s, _) = t
        .call(
            "POST",
            &format!("{Y}/sessionserver/session/minecraft/join"),
            None,
            Some(json!({"accessToken": token, "selectedProfile": steve_id(&t).await, "serverId": "-4b1d2f"})),
        )
        .await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    let (s, _) = t
        .call(
            "POST",
            &format!("{Y}/sessionserver/session/minecraft/join"),
            None,
            Some(json!({"accessToken": token, "selectedProfile": "0".repeat(32), "serverId": "x"})),
        )
        .await;
    assert_eq!(s, StatusCode::FORBIDDEN, "can't join as someone else");

    let (s, joined) =
        t.call("GET", &format!("{Y}/sessionserver/session/minecraft/hasJoined?username=Steve&serverId=-4b1d2f"), None, None).await;
    assert_eq!(s, StatusCode::OK, "{joined}");
    assert_eq!(joined["id"], steve_id(&t).await);
    let textures = joined["properties"].as_array().unwrap().iter().find(|p| p["name"] == "textures").unwrap();
    let value = textures["value"].as_str().unwrap();
    assert!(
        verify(&public_pem, value.as_bytes(), textures["signature"].as_str().unwrap()),
        "textures signature must verify with the metadata key"
    );
    let decoded: Value = serde_json::from_slice(&b64(value)).unwrap();
    assert_eq!(decoded["profileName"], "Steve");
    assert_eq!(decoded["textures"]["SKIN"]["url"], skin_url);
    assert_eq!(decoded["textures"]["SKIN"]["metadata"]["model"], "slim");
    assert!(decoded["textures"]["CAPE"]["url"].as_str().unwrap().starts_with("https://panel.test/textures/"));

    let (s, _) = t.call("GET", &format!("{Y}/sessionserver/session/minecraft/hasJoined?username=Alex&serverId=-4b1d2f"), None, None).await;
    assert_eq!(s, StatusCode::NO_CONTENT, "wrong name");
    let (s, _) = t.call("GET", &format!("{Y}/sessionserver/session/minecraft/hasJoined?username=Steve&serverId=other"), None, None).await;
    assert_eq!(s, StatusCode::NO_CONTENT, "wrong server id");

    // Profile lookups.
    let (_, unsigned) = t.call("GET", &format!("{Y}/sessionserver/session/minecraft/profile/{}", steve_id(&t).await), None, None).await;
    assert!(unsigned["properties"][0].get("signature").is_none());
    let (_, signed) =
        t.call("GET", &format!("{Y}/sessionserver/session/minecraft/profile/{}?unsigned=false", steve_id(&t).await), None, None).await;
    assert!(signed["properties"][0]["signature"].is_string());
    let (_, found) = t.call("POST", &format!("{Y}/api/profiles/minecraft"), None, Some(json!(["steve", "nobody"]))).await;
    assert_eq!(found, json!([{"id": steve_id(&t).await, "name": "Steve"}]));

    // Refresh rotates the token.
    let (s, refreshed) =
        t.call("POST", &format!("{Y}/authserver/refresh"), None, Some(json!({"accessToken": token, "clientToken": "ct1"}))).await;
    assert_eq!(s, StatusCode::OK, "{refreshed}");
    let new_token = refreshed["accessToken"].as_str().unwrap().to_string();
    assert_ne!(new_token, token);
    assert_eq!(refreshed["clientToken"], "ct1");
    let (s, _) = t.call("POST", &format!("{Y}/authserver/validate"), None, Some(json!({"accessToken": token}))).await;
    assert_eq!(s, StatusCode::FORBIDDEN, "old token revoked");

    // Invalidate.
    let (s, _) =
        t.call("POST", &format!("{Y}/authserver/invalidate"), None, Some(json!({"accessToken": new_token, "clientToken": "ct1"}))).await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    let (s, _) = t.call("POST", &format!("{Y}/authserver/validate"), None, Some(json!({"accessToken": new_token}))).await;
    assert_eq!(s, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn launcher_login_returns_game_session() {
    let t = setup().await;
    with_player(&t).await;
    let (s, v) = t.call("POST", "/api/v1/auth/login", None, Some(json!({"username": "Steve", "password": "password123"}))).await;
    assert_eq!(s, StatusCode::OK);
    let token = v["yggdrasil"]["access_token"].as_str().unwrap();
    assert_eq!(v["user"]["uuid"], t.uuid("Steve").await);
    let (s, _) = t.call("POST", &format!("{Y}/authserver/validate"), None, Some(json!({"accessToken": token}))).await;
    assert_eq!(s, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn disabled_accounts_are_refused_with_reason() {
    let t = setup().await;
    let admin = with_player(&t).await;
    let (_, users) = t.call("GET", "/api/admin/users", Some(&admin), None).await;
    let id = users.as_array().unwrap().iter().find(|u| u["username"] == "Steve").unwrap()["id"].as_i64().unwrap();
    let (s, v) = t
        .call(
            "PATCH",
            &format!("/api/admin/users/{id}"),
            Some(&admin),
            Some(json!({"status": "disabled", "status_reason": "Griefing spawn"})),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{v}");
    let (s, v) =
        t.call("POST", &format!("{Y}/authserver/authenticate"), None, Some(json!({"username": "Steve", "password": "password123"}))).await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    assert_eq!(v["errorMessage"], "Griefing spawn");
}

#[tokio::test]
async fn chat_certificates_are_signed_like_mojang() {
    let t = setup().await;
    with_player(&t).await;
    let (_, meta) = t.call("GET", Y, None, None).await;
    let public_pem = meta["signaturePublickey"].as_str().unwrap().to_string();
    let (_, auth) =
        t.call("POST", &format!("{Y}/authserver/authenticate"), None, Some(json!({"username": "Steve", "password": "password123"}))).await;
    let token = auth["accessToken"].as_str().unwrap();

    let (s, cert) = t.call("POST", &format!("{Y}/minecraftservices/player/certificates"), Some(token), None).await;
    assert_eq!(s, StatusCode::OK, "{cert}");
    let pem = cert["keyPair"]["publicKey"].as_str().unwrap();
    assert!(pem.starts_with("-----BEGIN RSA PUBLIC KEY-----"));
    assert!(cert["keyPair"]["privateKey"].as_str().unwrap().starts_with("-----BEGIN RSA PRIVATE KEY-----"));
    let expires = chrono::DateTime::parse_from_rfc3339(cert["expiresAt"].as_str().unwrap()).unwrap();

    // Rebuild the V2 payload the way Minecraft does and check the signature.
    let body: String = pem.lines().filter(|l| !l.starts_with("-----")).map(|l| l.trim()).collect();
    let der = b64(&body);
    let uuid = uuid::Uuid::parse_str(&t.uuid("Steve").await).unwrap();
    let mut payload = uuid.as_bytes().to_vec();
    payload.extend_from_slice(&expires.timestamp_millis().to_be_bytes());
    payload.extend_from_slice(&der);
    assert!(verify(&public_pem, &payload, cert["publicKeySignatureV2"].as_str().unwrap()));
    let v1 = format!("{}{}", expires.timestamp_millis(), pem);
    assert!(verify(&public_pem, v1.as_bytes(), cert["publicKeySignature"].as_str().unwrap()));

    // Cached until refresh time.
    let (_, again) = t.call("POST", &format!("{Y}/minecraftservices/player/certificates"), Some(token), None).await;
    assert_eq!(again["keyPair"]["publicKey"], cert["keyPair"]["publicKey"]);

    let (s, _) = t.call("POST", &format!("{Y}/minecraftservices/player/certificates"), None, None).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    let (_, keys) = t.call("GET", &format!("{Y}/minecraftservices/publickeys"), None, None).await;
    assert!(keys["playerCertificateKeys"][0]["publicKey"].is_string());
}

#[tokio::test]
async fn avatars_and_skin_validation() {
    let t = setup().await;
    with_player(&t).await;
    let panel = t.login("Steve", "password123").await;
    let (s, _) = t.call("GET", &format!("/api/v1/avatar/{}", steve_id(&t).await), None, None).await;
    assert_eq!(s, StatusCode::NOT_FOUND, "no skin yet");

    let (ct, body) = multipart(&[], ("bad.png", b"GIF89a not a png"));
    let req = Request::post("/api/v1/account/skin")
        .header("authorization", format!("Bearer {panel}"))
        .header("content-type", &ct)
        .body(Body::from(body))
        .unwrap();
    let (s, _) = t.send(req).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);

    let (ct, body) = multipart(&[], ("skin.png", &skin_png()));
    let req = Request::post("/api/v1/account/skin")
        .header("authorization", format!("Bearer {panel}"))
        .header("content-type", &ct)
        .body(Body::from(body))
        .unwrap();
    assert_eq!(t.send(req).await.0, StatusCode::OK);
    for id in [steve_id(&t).await, "Steve".to_string()] {
        let resp =
            t.router.clone().oneshot(Request::get(format!("/api/v1/avatar/{id}?size=32")).body(Body::empty()).unwrap()).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        assert_eq!(image::load_from_memory(&bytes).unwrap().width(), 32);
    }

    // Private capes can't be self-selected.
    let admin = t.login("admin", "supersecret").await;
    let (ct, body) = multipart(&[("name", "Staff"), ("visibility", "private"), ("allowed_groups", "[]")], ("cape.png", &cape_png()));
    let req = Request::post("/api/admin/capes")
        .header("authorization", format!("Bearer {admin}"))
        .header("content-type", &ct)
        .body(Body::from(body))
        .unwrap();
    let (_, capes) = t.send(req).await;
    let (s, _) = t.call("PUT", "/api/v1/account/cape", Some(&panel), Some(json!({"cape_id": capes[0]["id"]}))).await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    let (_, profile) = t.call("GET", "/api/v1/account/profile", Some(&panel), None).await;
    assert_eq!(profile["available_capes"], json!([]));
}
