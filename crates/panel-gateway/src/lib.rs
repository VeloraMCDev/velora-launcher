//! Streaming, fixed-owner HTTP gateway. No database, credential keys or private imports.
pub mod config;
use axum::{
    body::Body,
    extract::{ConnectInfo, Request, State},
    http::{header, HeaderMap, HeaderName, HeaderValue, StatusCode, Uri},
    response::{IntoResponse, Response},
    Router,
};
use config::Configuration;
use http_body_util::BodyExt;
use hyper_util::{
    client::legacy::{connect::HttpConnector, Client},
    rt::TokioExecutor,
};
use std::{
    future::Future,
    net::{IpAddr, SocketAddr},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

type HttpClient = Client<HttpConnector, Body>;
#[derive(Clone)]
struct StateData {
    config: Configuration,
    client: HttpClient,
    serving: Arc<AtomicBool>,
}

pub struct Gateway {
    state: StateData,
}
impl Gateway {
    pub fn new(config: Configuration) -> Self {
        let mut connector = HttpConnector::new();
        connector.set_connect_timeout(Some(Duration::from_secs(5)));
        // Native HTTP only, no environment proxy, decompression, redirects or request retries.
        let client = Client::builder(TokioExecutor::new()).retry_canceled_requests(false).build(connector);
        Self { state: StateData { config, client, serving: Arc::new(AtomicBool::new(true)) } }
    }
    pub fn router(&self) -> Router {
        Router::new().fallback(proxy).with_state(self.state.clone())
    }
    pub async fn serve(self, listener: tokio::net::TcpListener, shutdown: impl Future<Output = ()> + Send + 'static) -> anyhow::Result<()> {
        let serving = self.state.serving.clone();
        axum::serve(listener, self.router().into_make_service_with_connect_info::<SocketAddr>())
            .with_graceful_shutdown(async move {
                shutdown.await;
                serving.store(false, Ordering::SeqCst);
            })
            .await?;
        Ok(())
    }
}

fn failure(status: StatusCode, error: &'static str) -> Response {
    (status, [(header::CONTENT_TYPE, "application/json")], format!("{{\"error\":\"{error}\"}}")).into_response()
}
fn hop_headers(headers: &mut HeaderMap) {
    let nominated: Vec<HeaderName> = headers
        .get_all(header::CONNECTION)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(','))
        .filter_map(|v| v.trim().parse().ok())
        .collect();
    for name in nominated {
        headers.remove(name);
    }
    for name in ["connection", "keep-alive", "proxy-authenticate", "proxy-authorization", "te", "trailer", "transfer-encoding", "upgrade"] {
        headers.remove(name);
    }
}
/// Reject aliases that intermediaries could normalize into another route owner.
fn canonical_path(path: &str) -> Option<String> {
    if !path.starts_with('/') || path.contains(['\\', '#']) || path.contains("//") {
        return None;
    }
    let bytes = path.as_bytes();
    let mut decoded_path = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = std::str::from_utf8(bytes.get(i + 1..i + 3)?).ok()?;
            let decoded = u8::from_str_radix(hex, 16).ok()?;
            // Slash/backslash and recursive decoding can change owner boundaries.
            if decoded <= 31 || b"/\\%".contains(&decoded) || decoded == 127 {
                return None;
            }
            decoded_path.push(decoded);
            i += 3;
        } else {
            decoded_path.push(bytes[i]);
            i += 1;
        }
    }
    let decoded = String::from_utf8(decoded_path).ok()?;
    if decoded.split('/').any(|p| matches!(p, "." | "..")) {
        return None;
    }
    Some(decoded)
}
fn client_ip(peer: IpAddr, headers: &HeaderMap, trusted: &[IpAddr]) -> Option<IpAddr> {
    if trusted.contains(&peer) {
        if let Some(chain) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()).map(str::trim).filter(|v| !v.is_empty()) {
            for hop in chain.rsplit(',') {
                let ip = hop.trim().parse().ok()?;
                if !trusted.contains(&ip) {
                    return Some(ip);
                }
            }
        }
        if let Some(ip) = headers.get("x-real-ip").and_then(|v| v.to_str().ok()).and_then(|v| v.trim().parse().ok()) {
            return Some(ip);
        }
    }
    Some(peer)
}
fn owns_auth(path: &str) -> bool {
    path == "/api/yggdrasil" || path.starts_with("/api/yggdrasil/") || path == "/textures" || path.starts_with("/textures/")
}
fn destination(upstream: &Uri, path_and_query: &str) -> Uri {
    // Already validated origin + raw origin-form URI; no URL resolution/normalization.
    format!("http://{}{path_and_query}", upstream.authority().expect("validated upstream")).parse().expect("validated request URI")
}
async fn healthy(state: &StateData, upstream: &Uri, path: &str) -> bool {
    let request = Request::builder().uri(destination(upstream, path)).body(Body::empty()).expect("static health request");
    let checked = async {
        let Ok(response) = state.client.request(request).await else {
            return false;
        };
        if response.status() != StatusCode::OK {
            return false;
        }
        // Bound readiness probe consumption; it is not a content/download route.
        http_body_util::Limited::new(response.into_body(), 4096).collect().await.is_ok()
    };
    tokio::time::timeout(state.config.header_timeout.min(Duration::from_secs(5)), checked).await.unwrap_or(false)
}
async fn proxy(State(state): State<StateData>, ConnectInfo(peer): ConnectInfo<SocketAddr>, request: Request) -> Response {
    let location = HeaderValue::from_str(&format!("{}/api/yggdrasil/", state.config.origin)).expect("validated origin");
    let mut response = forward(State(state), ConnectInfo(peer), request).await;
    response.headers_mut().insert("x-authlib-injector-api-location", location);
    response
}
async fn forward(State(state): State<StateData>, ConnectInfo(peer): ConnectInfo<SocketAddr>, request: Request) -> Response {
    let (mut parts, body) = request.into_parts();
    let path = parts.uri.path();
    let Some(decoded) = canonical_path(path) else {
        return failure(StatusCode::BAD_REQUEST, "ambiguous request path");
    };
    let Some(path) = path.strip_prefix(&state.config.prefix).filter(|p| p.is_empty() || p.starts_with('/')) else {
        return failure(StatusCode::NOT_FOUND, "outside configured public prefix");
    };
    let path = if path.is_empty() { "/" } else { path };
    let decoded = decoded.strip_prefix(&state.config.prefix).unwrap_or(&decoded);
    if owns_auth(path) != owns_auth(decoded) || (matches!(decoded, "/health/live" | "/health/ready") && decoded != path) {
        return failure(StatusCode::BAD_REQUEST, "ambiguous request path");
    }

    if matches!(path, "/health/live" | "/health/ready") {
        if parts.method != axum::http::Method::GET && parts.method != axum::http::Method::HEAD {
            return failure(StatusCode::METHOD_NOT_ALLOWED, "health requires GET or HEAD");
        }
        let ready = path == "/health/live"
            || (state.serving.load(Ordering::SeqCst)
                && healthy(&state, &state.config.auth, "/health/ready").await
                && match &state.config.panel {
                    Some(panel) => healthy(&state, panel, "/healthz").await,
                    None => true,
                });
        return if ready {
            (StatusCode::OK, "ok").into_response()
        } else {
            failure(StatusCode::SERVICE_UNAVAILABLE, "upstream unavailable")
        };
    }
    if !state.serving.load(Ordering::SeqCst) {
        return failure(StatusCode::SERVICE_UNAVAILABLE, "gateway stopping");
    }
    if parts.headers.contains_key(header::UPGRADE) || parts.method == axum::http::Method::CONNECT {
        return failure(StatusCode::NOT_IMPLEMENTED, "protocol upgrade is not supported");
    }
    let upstream = if owns_auth(path) {
        &state.config.auth
    } else {
        let Some(panel) = state.config.panel.as_ref() else {
            return failure(StatusCode::SERVICE_UNAVAILABLE, "Panel upstream is not configured");
        };
        panel
    };
    let Some(ip) = client_ip(peer.ip(), &parts.headers, &state.config.trusted_proxies) else {
        return failure(StatusCode::BAD_REQUEST, "invalid trusted proxy chain");
    };
    let raw = match parts.uri.query() {
        Some(query) => format!("{path}?{query}"),
        None => path.to_owned(),
    };
    parts.uri = destination(upstream, &raw);
    // Connection-nominated hop fields are removed before reconstructing trusted metadata.
    hop_headers(&mut parts.headers);
    for name in
        ["forwarded", "x-forwarded-for", "x-real-ip", "x-forwarded-host", "x-forwarded-proto", "x-forwarded-port", "x-forwarded-prefix"]
    {
        parts.headers.remove(name);
    }
    parts.headers.insert(header::HOST, state.config.public_host.clone());
    parts.headers.insert("x-forwarded-host", state.config.public_host.clone());
    parts.headers.insert("x-forwarded-proto", state.config.public_scheme.clone());
    let address = HeaderValue::from_str(&ip.to_string()).expect("IP header");
    parts.headers.insert("x-forwarded-for", address.clone());
    parts.headers.insert("x-real-ip", address);
    if !state.config.prefix.is_empty() {
        parts.headers.insert("x-forwarded-prefix", HeaderValue::from_str(&state.config.prefix).expect("validated prefix"));
    }
    parts.version = axum::http::Version::HTTP_11;
    let response = tokio::time::timeout(state.config.header_timeout, state.client.request(Request::from_parts(parts, body))).await;
    let mut response = match response {
        Err(_) => return failure(StatusCode::GATEWAY_TIMEOUT, "upstream response timed out"),
        Ok(Err(_)) => return failure(StatusCode::BAD_GATEWAY, "upstream unavailable"),
        Ok(Ok(response)) => response,
    };
    hop_headers(response.headers_mut());
    // Upstream error/redirect/cookie/cache/signature/body bytes remain intact.
    let (parts, body) = response.into_parts();
    Response::from_parts(parts, Body::new(body))
}
