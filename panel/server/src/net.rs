//! Request helpers: the panel's public URL and the client's IP address.

use crate::error::AppError;
use crate::state::AppState;
use crate::store;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::HeaderMap;

fn header<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).and_then(|v| v.to_str().ok()).map(str::trim).filter(|v| !v.is_empty())
}

/// The URL players reach the panel at, without a trailing slash.
///
/// Order: the admin's setting → `PUBLIC_URL` → what the request says
/// (honouring `X-Forwarded-*` from a reverse proxy).
pub async fn public_base(state: &AppState, headers: &HeaderMap) -> String {
    if let Ok(s) = store::settings(state).await {
        if let Some(u) = s.public_url.filter(|u| !u.is_empty()) {
            return u.trim_end_matches('/').to_string();
        }
    }
    if let Some(u) = &state.cfg.public_url {
        return u.trim_end_matches('/').to_string();
    }
    let host = header(headers, "x-forwarded-host").or_else(|| header(headers, "host")).unwrap_or("localhost:8080");
    let proto = header(headers, "x-forwarded-proto").map(|p| p.split(',').next().unwrap_or(p).trim()).unwrap_or("http");
    format!("{proto}://{}", host.split(',').next().unwrap_or(host).trim())
}

/// Host part of a URL (`https://a.b:8443/x` → `a.b`), used for authlib's
/// skin-domain allow-list.
pub fn host_of(url: &str) -> String {
    let rest = url.split("://").nth(1).unwrap_or(url);
    let authority = rest.split('/').next().unwrap_or(rest);
    let host = authority.rsplit('@').next().unwrap_or(authority);
    if host.starts_with('[') {
        return host.split(']').next().unwrap_or(host).trim_start_matches('[').to_string();
    }
    host.split(':').next().unwrap_or(host).to_string()
}

/// Forwarded addresses are accepted only from explicitly trusted proxies.
pub struct ClientIp(pub Option<String>);

impl FromRequestParts<AppState> for ClientIp {
    type Rejection = AppError;
    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        Ok(ClientIp(velora_auth_http::net::client_ip(parts, &state.cfg.trusted_proxies)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hosts() {
        assert_eq!(host_of("https://panel.example.com/api"), "panel.example.com");
        assert_eq!(host_of("http://localhost:8080"), "localhost");
        assert_eq!(host_of("https://[::1]:8443/"), "::1");
    }
}
