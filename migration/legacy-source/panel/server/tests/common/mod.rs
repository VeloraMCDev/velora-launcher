//! Shared helpers for the API tests.
#![allow(dead_code)]

pub use axum::body::Body;
pub use axum::http::{Request, StatusCode};
use scopenet_panel::{app, bootstrap_admin, build_state_with_keys, config::Config, db, yggdrasil::keys::Keys};
pub use serde_json::{json, Value};
use std::io::Write;
use std::sync::{Arc, OnceLock};
pub use tower::ServiceExt;

/// One auth-server key for the whole test run (key generation is slow).
pub fn test_keys() -> Arc<Keys> {
    static KEYS: OnceLock<Arc<Keys>> = OnceLock::new();
    KEYS.get_or_init(|| Arc::new(Keys::from_private(rsa::RsaPrivateKey::new(&mut rand::thread_rng(), 2048).unwrap()).unwrap())).clone()
}

pub struct TestApp {
    pub router: axum::Router,
    pub db: sqlx::SqlitePool,
    _dir: tempfile::TempDir,
}

pub async fn setup() -> TestApp {
    setup_with(|_, _| {}).await
}

/// Like [`setup`], letting the test adjust the config (given the data dir).
pub async fn setup_with(tweak: impl FnOnce(&mut Config, &std::path::Path)) -> TestApp {
    let dir = tempfile::tempdir().unwrap();
    let mut cfg = Config {
        bind: "127.0.0.1:0".into(),
        data_dir: dir.path().to_path_buf(),
        web_dir: dir.path().join("web"),
        icons_dir: dir.path().join("icons"),
        admin_username: "admin".into(),
        admin_password: Some("supersecret".into()),
        jwt_secret: Some("test-secret-test-secret-test-secret".into()),
        curseforge_api_key: None,
        max_upload_mb: 64,
        public_url: Some("https://panel.test".into()),
        trusted_proxies: vec!["127.0.0.1".parse().unwrap()],
    };
    tweak(&mut cfg, dir.path());
    let pool = db::connect_memory().await.unwrap();
    let state = build_state_with_keys(cfg, pool, test_keys()).await.unwrap();
    bootstrap_admin(&state).await.unwrap();
    TestApp { db: state.db.clone(), router: app(state), _dir: dir }
}

impl TestApp {
    pub async fn uuid(&self, name: &str) -> String {
        sqlx::query_scalar("SELECT uuid FROM users WHERE username = ?").bind(name).fetch_one(&self.db).await.unwrap()
    }
    pub async fn call(&self, method: &str, uri: &str, token: Option<&str>, body: Option<Value>) -> (StatusCode, Value) {
        let mut req = Request::builder().method(method).uri(uri);
        if let Some(t) = token {
            req = req.header("authorization", format!("Bearer {t}"));
        }
        let req = match body {
            Some(b) => req.header("content-type", "application/json").body(Body::from(b.to_string())).unwrap(),
            None => req.body(Body::empty()).unwrap(),
        };
        self.send(req).await
    }

    /// POST raw bytes (binary uploads).
    pub async fn call_raw(&self, method: &str, uri: &str, token: Option<&str>, body: Vec<u8>) -> (StatusCode, Value) {
        let mut req = Request::builder().method(method).uri(uri).header("content-type", "application/octet-stream");
        if let Some(t) = token {
            req = req.header("authorization", format!("Bearer {t}"));
        }
        self.send(req.body(Body::from(body)).unwrap()).await
    }

    /// GET and keep the raw bytes (images).
    pub async fn fetch(&self, uri: &str) -> (StatusCode, Vec<u8>) {
        let mut req = Request::builder().method("GET").uri(uri).body(Body::empty()).unwrap();
        req.extensions_mut().insert(axum::extract::ConnectInfo("127.0.0.1:12345".parse::<std::net::SocketAddr>().unwrap()));
        let resp = self.router.clone().oneshot(req).await.unwrap();
        let status = resp.status();
        (status, axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap().to_vec())
    }

    pub async fn send(&self, mut req: Request<Body>) -> (StatusCode, Value) {
        if req.extensions().get::<axum::extract::ConnectInfo<std::net::SocketAddr>>().is_none() {
            req.extensions_mut().insert(axum::extract::ConnectInfo("127.0.0.1:12345".parse::<std::net::SocketAddr>().unwrap()));
        }
        let resp = self.router.clone().oneshot(req).await.unwrap();
        let status = resp.status();
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap_or(Value::String(String::from_utf8_lossy(&bytes).into())))
    }

    pub async fn login(&self, user: &str, pass: &str) -> String {
        let (s, v) = self.call("POST", "/api/v1/auth/login", None, Some(json!({"username": user, "password": pass}))).await;
        assert_eq!(s, StatusCode::OK, "{v}");
        v["token"].as_str().unwrap().to_string()
    }
}

pub fn multipart(fields: &[(&str, &str)], file: (&str, &[u8])) -> (String, Vec<u8>) {
    let boundary = "----scopenettest";
    let mut body = Vec::new();
    for (k, v) in fields {
        write!(body, "--{boundary}\r\nContent-Disposition: form-data; name=\"{k}\"\r\n\r\n{v}\r\n").unwrap();
    }
    write!(
        body,
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{}\"\r\nContent-Type: application/octet-stream\r\n\r\n",
        file.0
    )
    .unwrap();
    body.extend_from_slice(file.1);
    write!(body, "\r\n--{boundary}--\r\n").unwrap();
    (format!("multipart/form-data; boundary={boundary}"), body)
}

pub fn zip_bytes(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut z = zip::ZipWriter::new(&mut buf);
        let opts = zip::write::SimpleFileOptions::default();
        for (name, data) in entries {
            z.start_file(*name, opts).unwrap();
            z.write_all(data).unwrap();
        }
        z.finish().unwrap();
    }
    buf.into_inner()
}

/// A real 16x16 PNG. The pack build drops images a client could not read, so tests that expect a texture in the pack need a valid one.
pub fn tiny_png() -> Vec<u8> {
    let mut out = Vec::new();
    image::DynamicImage::new_rgba8(16, 16).write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png).unwrap();
    out
}
