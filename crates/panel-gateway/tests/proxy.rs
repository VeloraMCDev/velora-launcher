use axum::{
    body::{Body, Bytes},
    extract::{Request, State},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Router,
};
use http_body_util::BodyExt;
use std::{
    convert::Infallible,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::{oneshot, Notify},
};
use velora_panel_gateway::{config::Configuration, Gateway};

struct Server {
    origin: String,
    task: tokio::task::JoinHandle<()>,
    stop: Option<oneshot::Sender<()>>,
}
impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}
async fn server(app: Router) -> Server {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let (stop, stopped) = oneshot::channel();
    let task = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = stopped.await;
            })
            .await
            .unwrap();
    });
    Server { origin, task, stop: Some(stop) }
}
async fn stop(mut server: Server) {
    server.stop.take().unwrap().send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(5), &mut server.task).await.unwrap().unwrap();
}
async fn gateway(mut config: Configuration) -> Server {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    config.bind = listener.local_addr().unwrap();
    let (stop, stopped) = oneshot::channel();
    let task = tokio::spawn(async move {
        Gateway::new(config)
            .serve(listener, async {
                let _ = stopped.await;
            })
            .await
            .unwrap();
    });
    Server { origin, task, stop: Some(stop) }
}
fn configuration(auth: &str, panel: Option<&str>, prefix: &str) -> Configuration {
    Configuration::from_lookup(|key| match key {
        "VELORA_GATEWAY_PUBLIC_ORIGIN" => Some(format!("https://example.invalid{prefix}")),
        "VELORA_GATEWAY_AUTH_UPSTREAM" => Some(auth.into()),
        "VELORA_GATEWAY_PANEL_UPSTREAM" => panel.map(str::to_owned),
        _ => None,
    })
    .unwrap()
}
fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .no_proxy()
        .no_gzip()
        .no_brotli()
        .no_deflate()
        .no_zstd()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap()
}
#[derive(Clone, Default)]
struct Fixture {
    requests: Arc<Mutex<Vec<serde_json::Value>>>,
    release: Arc<Notify>,
}
async fn fixture(State(state): State<Fixture>, request: Request) -> Response {
    let (parts, body) = request.into_parts();
    let bytes = body.collect().await.unwrap().to_bytes();
    state.requests.lock().unwrap().push(serde_json::json!({
        "method": parts.method.as_str(), "path": parts.uri.to_string(), "body": bytes.to_vec(),
        "host": parts.headers.get("host").and_then(|v| v.to_str().ok()),
        "ip": parts.headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()),
        "real": parts.headers.get("x-real-ip").and_then(|v| v.to_str().ok()),
        "proto": parts.headers.get("x-forwarded-proto").and_then(|v| v.to_str().ok()),
        "prefix": parts.headers.get("x-forwarded-prefix").and_then(|v| v.to_str().ok()),
        "forwarded": parts.headers.get("forwarded").and_then(|v| v.to_str().ok()),
        "hop": parts.headers.get("x-hop").and_then(|v| v.to_str().ok()),
        "bearer": parts.headers.get("authorization").and_then(|v| v.to_str().ok()),
        "instance": parts.headers.get("x-scopenet-instance").and_then(|v| v.to_str().ok()),
        "velora_instance": parts.headers.get("x-velora-instance").and_then(|v| v.to_str().ok()),
        "range": parts.headers.get("range").and_then(|v| v.to_str().ok()),
    }));
    match parts.uri.path() {
        "/health/ready" | "/healthz" => (StatusCode::OK, "ok").into_response(),
        "/api/yggdrasil/error" => (
            StatusCode::FORBIDDEN,
            [(header::CONTENT_TYPE, "application/json")],
            "{\"error\":\"ForbiddenOperationException\",\"errorMessage\":\"Invalid token.\"}",
        )
            .into_response(),
        "/api/yggdrasil/redirect" => {
            (StatusCode::TEMPORARY_REDIRECT, [(header::LOCATION, "https://example.invalid/login")], "redirect").into_response()
        }
        "/api/yggdrasil/unknown" => (StatusCode::NOT_FOUND, "owner missing route").into_response(),
        "/api/yggdrasil/slow" => {
            tokio::time::sleep(Duration::from_millis(300)).await;
            "late".into_response()
        }
        "/api/yggdrasil/stream" => {
            let stream = futures_util::stream::unfold((0, state.release), |(stage, release)| async move {
                match stage {
                    0 => Some((Ok::<_, Infallible>(Bytes::from_static(b"first")), (1, release))),
                    1 => {
                        release.notified().await;
                        Some((Ok(Bytes::from_static(b"second")), (2, release)))
                    }
                    _ => None,
                }
            });
            Body::from_stream(stream).into_response()
        }
        _ => {
            let mut response = (StatusCode::PARTIAL_CONTENT, bytes).into_response();
            response.headers_mut().insert(header::CONTENT_ENCODING, HeaderValue::from_static("gzip"));
            response.headers_mut().insert(header::ETAG, HeaderValue::from_static("\"synthetic-v1\""));
            response.headers_mut().insert(header::CONTENT_RANGE, HeaderValue::from_static("bytes 0-3/4"));
            response.headers_mut().insert(header::CONNECTION, HeaderValue::from_static("x-response-hop"));
            response.headers_mut().insert("x-response-hop", HeaderValue::from_static("remove"));
            response.headers_mut().append(header::SET_COOKIE, HeaderValue::from_static("a=1; Secure"));
            response.headers_mut().append(header::SET_COOKIE, HeaderValue::from_static("b=2; HttpOnly"));
            response
        }
    }
}
async fn upstream(state: Fixture) -> Server {
    server(Router::new().fallback(fixture).with_state(state)).await
}

#[tokio::test]
async fn owner_routing_preserves_wire_bytes_headers_query_and_methods() {
    let auth_state = Fixture::default();
    let panel_state = Fixture::default();
    let auth = upstream(auth_state.clone()).await;
    let panel = upstream(panel_state.clone()).await;
    let gateway = gateway(configuration(&auth.origin, Some(&panel.origin), "/velora")).await;
    let client = client();
    let response = client
        .put(format!("{}/velora/api/yggdrasil/echo?name=a%2Fb&name=two", gateway.origin))
        .header("authorization", "Bearer synthetic-token")
        .header("x-scopenet-instance", "synthetic-instance")
        .header("x-velora-instance", "synthetic-instance")
        .header("range", "bytes=0-3")
        .header("host", "spoof.invalid")
        .header("x-forwarded-host", "spoof.invalid")
        .header("x-forwarded-proto", "ftp")
        .header("x-forwarded-for", "198.51.100.77")
        .header("x-real-ip", "198.51.100.78")
        .header("forwarded", "for=198.51.100.79")
        .header("connection", "x-hop")
        .header("x-hop", "remove")
        .body(vec![0, 255, 3, 4])
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(response.headers()["content-encoding"], "gzip");
    assert_eq!(response.headers()["etag"], "\"synthetic-v1\"");
    assert_eq!(response.headers().get_all("set-cookie").iter().count(), 2);
    assert!(!response.headers().contains_key("x-response-hop"));
    assert_eq!(response.headers()["x-authlib-injector-api-location"], "https://example.invalid/velora/api/yggdrasil/");
    assert_eq!(response.bytes().await.unwrap().as_ref(), &[0, 255, 3, 4]);
    {
        let requests = auth_state.requests.lock().unwrap();
        let row = &requests[0];
        assert_eq!(row["method"], "PUT");
        assert_eq!(row["path"], "/api/yggdrasil/echo?name=a%2Fb&name=two");
        assert_eq!(row["host"], "example.invalid");
        assert_eq!(row["proto"], "https");
        assert_eq!(row["ip"], "127.0.0.1");
        assert_eq!(row["real"], "127.0.0.1");
        assert_eq!(row["prefix"], "/velora");
        assert_eq!(row["bearer"], "Bearer synthetic-token");
        assert_eq!(row["instance"], "synthetic-instance");
        assert_eq!(row["velora_instance"], "synthetic-instance");
        assert_eq!(row["range"], "bytes=0-3");
        assert!(row["forwarded"].is_null());
        assert!(row["hop"].is_null());
    }
    for path in ["/api/me", "/api/yggdrasilevil", "/texturesfoo", "/"] {
        client.get(format!("{}/velora{path}", gateway.origin)).send().await.unwrap();
    }
    assert_eq!(panel_state.requests.lock().unwrap().len(), 4);
    let response = client.get(format!("{}/velora/api/yggdrasil/unknown", gateway.origin)).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(response.text().await.unwrap(), "owner missing route");
    let response = client.get(format!("{}/velora/api/yggdrasil/error", gateway.origin)).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(response.text().await.unwrap(), "{\"error\":\"ForbiddenOperationException\",\"errorMessage\":\"Invalid token.\"}");
    let response = client.get(format!("{}/velora/api/yggdrasil/redirect", gateway.origin)).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
    assert_eq!(response.headers()["location"], "https://example.invalid/login");
    assert_eq!(panel_state.requests.lock().unwrap().len(), 4);
    client.get(format!("{}/velora/files/%41rt%20test.zip", gateway.origin)).send().await.unwrap();
    assert_eq!(panel_state.requests.lock().unwrap().last().unwrap()["path"], "/files/%41rt%20test.zip");
}

#[tokio::test]
async fn trusted_chain_outage_timeout_and_readiness_never_fall_back() {
    let auth_state = Fixture::default();
    let panel_state = Fixture::default();
    let auth = upstream(auth_state.clone()).await;
    let panel = upstream(panel_state.clone()).await;
    let mut config = configuration(&auth.origin, Some(&panel.origin), "");
    config.trusted_proxies = vec!["127.0.0.1".parse().unwrap(), "192.0.2.1".parse().unwrap()];
    config.header_timeout = Duration::from_millis(100);
    let gateway = gateway(config).await;
    let client = client();
    assert_eq!(client.get(format!("{}/health/ready", gateway.origin)).send().await.unwrap().status(), StatusCode::OK);
    client
        .get(format!("{}/api/yggdrasil/echo", gateway.origin))
        .header("x-forwarded-for", "spoofed-prefix, 203.0.113.9, 192.0.2.1")
        .send()
        .await
        .unwrap();
    assert_eq!(auth_state.requests.lock().unwrap().last().unwrap()["ip"], "203.0.113.9");
    let response = client
        .get(format!("{}/api/yggdrasil/echo", gateway.origin))
        .header("x-forwarded-for", "203.0.113.9, invalid")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let response = client.get(format!("{}/api/yggdrasil/slow", gateway.origin)).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::GATEWAY_TIMEOUT);
    assert_eq!(response.text().await.unwrap(), "{\"error\":\"upstream response timed out\"}");
    stop(auth).await;
    let status = client.get(format!("{}/api/yggdrasil/error", gateway.origin)).send().await.unwrap().status();
    assert!(matches!(status, StatusCode::BAD_GATEWAY | StatusCode::GATEWAY_TIMEOUT));
    assert_eq!(client.get(format!("{}/health/ready", gateway.origin)).send().await.unwrap().status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(client.get(format!("{}/health/live", gateway.origin)).send().await.unwrap().status(), StatusCode::OK);
    assert_eq!(panel_state.requests.lock().unwrap().len(), 1); // readiness only
    client.get(format!("{}/api/me", gateway.origin)).send().await.unwrap();
    assert_eq!(panel_state.requests.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn streams_before_completion_and_drains_on_shutdown() {
    let state = Fixture::default();
    let auth = upstream(state.clone()).await;
    let mut gateway = gateway(configuration(&auth.origin, None, "")).await;
    let mut response = client().get(format!("{}/api/yggdrasil/stream", gateway.origin)).send().await.unwrap();
    assert_eq!(tokio::time::timeout(Duration::from_secs(2), response.chunk()).await.unwrap().unwrap().unwrap().as_ref(), b"first");
    gateway.stop.take().unwrap().send(()).unwrap();
    // A live response must complete before graceful serve finishes.
    assert!(tokio::time::timeout(Duration::from_millis(30), &mut gateway.task).await.is_err());
    state.release.notify_one();
    assert_eq!(response.bytes().await.unwrap().as_ref(), b"second");
    tokio::time::timeout(Duration::from_secs(2), &mut gateway.task).await.unwrap().unwrap();
}

#[tokio::test]
async fn panel_outage_leaves_authentication_owned_and_public_only_mode_ready() {
    let auth = upstream(Fixture::default()).await;
    let panel = upstream(Fixture::default()).await;
    let config = configuration(&auth.origin, Some(&panel.origin), "");
    let gateway = gateway(config).await;
    let client = client();
    assert_eq!(client.get(format!("{}/health/ready", gateway.origin)).send().await.unwrap().status(), StatusCode::OK);
    stop(panel).await;
    assert_eq!(client.get(format!("{}/health/ready", gateway.origin)).send().await.unwrap().status(), StatusCode::SERVICE_UNAVAILABLE);
    let status = client.get(format!("{}/api/me", gateway.origin)).send().await.unwrap().status();
    assert!(matches!(status, StatusCode::BAD_GATEWAY | StatusCode::GATEWAY_TIMEOUT));
    assert_eq!(client.get(format!("{}/api/yggdrasil/error", gateway.origin)).send().await.unwrap().status(), StatusCode::FORBIDDEN);
    let public_only = crate::gateway(configuration(&auth.origin, None, "")).await;
    assert_eq!(client.get(format!("{}/health/ready", public_only.origin)).send().await.unwrap().status(), StatusCode::OK);
    assert_eq!(client.get(format!("{}/api/me", public_only.origin)).send().await.unwrap().status(), StatusCode::SERVICE_UNAVAILABLE);
}
#[tokio::test]
async fn ambiguous_paths_prefix_escapes_and_upgrades_are_rejected_before_proxying() {
    let state = Fixture::default();
    let auth = upstream(state.clone()).await;
    let gateway = gateway(configuration(&auth.origin, None, "/velora")).await;
    let address = gateway.origin.trim_start_matches("http://");
    for path in [
        "/velora/%61pi/yggdrasil",
        "/velora/api%2fyggdrasil",
        "/velora/api/../api/yggdrasil",
        "/velora//api/yggdrasil",
        "/velora/api/%252e%252e",
        "/velora/api/%00",
    ] {
        let mut socket = tokio::net::TcpStream::connect(address).await.unwrap();
        socket.write_all(format!("GET {path} HTTP/1.1\r\nHost: example.invalid\r\nConnection: close\r\n\r\n").as_bytes()).await.unwrap();
        let mut response = String::new();
        socket.read_to_string(&mut response).await.unwrap();
        assert!(response.starts_with("HTTP/1.1 400"), "{path}: {response}");
    }
    let client = client();
    assert_eq!(client.get(format!("{}/velora-other/api/yggdrasil", gateway.origin)).send().await.unwrap().status(), StatusCode::NOT_FOUND);
    assert_eq!(client.get(format!("{}/velora/api/me", gateway.origin)).send().await.unwrap().status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        client.get(format!("{}/velora/api/yggdrasil", gateway.origin)).header("upgrade", "websocket").send().await.unwrap().status(),
        StatusCode::NOT_IMPLEMENTED
    );
    assert!(state.requests.lock().unwrap().is_empty());
}

#[test]
fn invalid_configuration_and_cli_never_echo_credentials() {
    for (key, value) in [
        ("VELORA_GATEWAY_PUBLIC_ORIGIN", "https://user:synthetic-secret@example.invalid"),
        ("VELORA_GATEWAY_PUBLIC_ORIGIN", "https://example.invalid/a/../b"),
        ("VELORA_GATEWAY_PUBLIC_ORIGIN", "https://example.invalid/%61pi"),
        ("VELORA_GATEWAY_AUTH_UPSTREAM", "https://example.invalid"),
        ("VELORA_GATEWAY_AUTH_UPSTREAM", "http://example.invalid/prefix"),
        ("VELORA_GATEWAY_BIND", "localhost:8080"),
        ("VELORA_GATEWAY_TRUSTED_PROXIES", "not-an-ip"),
        ("VELORA_GATEWAY_HEADER_TIMEOUT_SECS", "0"),
    ] {
        let config = Configuration::from_lookup(|name| {
            if name == key {
                Some(value.into())
            } else {
                match name {
                    "VELORA_GATEWAY_PUBLIC_ORIGIN" => Some("https://example.invalid".into()),
                    "VELORA_GATEWAY_AUTH_UPSTREAM" => Some("http://127.0.0.1:8081".into()),
                    _ => None,
                }
            }
        });
        let error = match config {
            Ok(_) => panic!("accepted invalid {key}"),
            Err(error) => format!("{error:#}"),
        };
        assert!(!error.contains("synthetic-secret"));
    }
    let help = std::process::Command::new(env!("CARGO_BIN_EXE_velora-gateway")).env_clear().arg("--help").output().unwrap();
    assert!(help.status.success());
    assert!(String::from_utf8_lossy(&help.stdout).contains("Velora HTTP gateway"));
    let invalid = std::process::Command::new(env!("CARGO_BIN_EXE_velora-gateway"))
        .env_clear()
        .env("VELORA_GATEWAY_PUBLIC_ORIGIN", "https://user:synthetic-secret@example.invalid")
        .output()
        .unwrap();
    assert!(!invalid.status.success());
    assert!(!String::from_utf8_lossy(&invalid.stderr).contains("synthetic-secret"));
}
