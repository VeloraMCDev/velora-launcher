//! Minecraft textures: nothing is bundled, admins can upload a client jar, and the public endpoints only serve plain files.
mod common;
use common::*;
use std::io::Write;

fn fake_jar() -> Vec<u8> {
    let mut out = std::io::Cursor::new(Vec::new());
    let mut zip = zip::ZipWriter::new(&mut out);
    let opts = zip::write::SimpleFileOptions::default();
    for (name, data) in [
        ("version.json", &br#"{"id":"1.21.1"}"#[..]),
        ("assets/minecraft/textures/item/diamond_sword.png", b"sword"),
        ("assets/minecraft/textures/block/stone.png", b"stone"),
        ("assets/minecraft/textures/gui/sprites/hud/heart/full.png", b"heart"),
    ] {
        zip.start_file(name, opts).unwrap();
        zip.write_all(data).unwrap();
    }
    zip.finish().unwrap();
    out.into_inner()
}

fn multipart(file: &[u8]) -> (String, Vec<u8>) {
    let boundary = "----velora-test";
    let mut body = format!("--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"client.jar\"\r\nContent-Type: application/java-archive\r\n\r\n").into_bytes();
    body.extend_from_slice(file);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    (format!("multipart/form-data; boundary={boundary}"), body)
}

#[tokio::test]
async fn an_uploaded_client_jar_becomes_public_textures() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    let (s, st) = t.call("GET", "/api/v1/mc/status", None, None).await;
    assert_eq!((s, st["ready"].clone()), (StatusCode::OK, json!(false)));
    assert_eq!(t.fetch("/api/v1/mc/item/diamond_sword.png").await.0, StatusCode::NOT_FOUND);

    let (ctype, body) = multipart(&fake_jar());
    let up = |token: Option<&str>, body: Vec<u8>| {
        let mut req = Request::builder().method("POST").uri("/api/admin/mc-textures/upload").header("content-type", ctype.clone());
        if let Some(tk) = token {
            req = req.header("authorization", format!("Bearer {tk}"));
        }
        req.body(Body::from(body)).unwrap()
    };
    assert_eq!(t.send(up(None, body.clone())).await.0, StatusCode::UNAUTHORIZED);
    let (s, v) = t.send(up(Some(&admin), body)).await;
    assert_eq!(s, StatusCode::OK, "{v}");
    assert_eq!((v["ready"].clone(), v["version"].clone(), v["source"].clone()), (json!(true), json!("1.21.1"), json!("upload")));

    assert_eq!(t.fetch("/api/v1/mc/item/diamond_sword.png").await, (StatusCode::OK, b"sword".to_vec()));
    assert_eq!(t.fetch("/api/v1/mc/item/stone.png").await.1, b"stone", "blocks answer as items");
    assert_eq!(t.fetch("/api/v1/mc/gui/hud/heart/full.png").await.1, b"heart");
    for bad in ["/api/v1/mc/gui/..%2F..%2Fjwt.secret", "/api/v1/mc/gui/hud/nothing.png", "/api/v1/mc/secrets/x.png", "/api/v1/mc/item/x.txt"] {
        assert_eq!(t.fetch(bad).await.0, StatusCode::NOT_FOUND, "{bad}");
    }

    let (_, admin_view) = t.call("GET", "/api/admin/mc-textures", Some(&admin), None).await;
    assert_eq!(admin_view["items"], json!(["diamond_sword"]));
    let (s, junk) = t.send(up(Some(&admin), multipart(b"not a jar").1)).await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "{junk}");
    assert_eq!(t.fetch("/api/v1/mc/item/diamond_sword.png").await.0, StatusCode::OK, "a bad upload keeps the working set");
    let (_, cleared) = t.call("DELETE", "/api/admin/mc-textures", Some(&admin), None).await;
    assert_eq!(cleared["ready"], false);
}
