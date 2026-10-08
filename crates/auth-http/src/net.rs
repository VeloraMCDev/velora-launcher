use crate::{error::AppError, state::AuthorityState};
use axum::{
    extract::{ConnectInfo, FromRequestParts},
    http::{request::Parts, HeaderMap},
};
use std::net::{IpAddr, SocketAddr};

fn header<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).and_then(|value| value.to_str().ok()).map(str::trim).filter(|value| !value.is_empty())
}

/// Resolve the legacy trusted-proxy chain without accepting a client-supplied prefix.
/// Missing socket-peer context does not authorize forwarded headers.
pub fn client_ip(parts: &Parts, trusted_proxies: &[IpAddr]) -> Option<String> {
    let peer = parts.extensions.get::<ConnectInfo<SocketAddr>>().map(|info| info.0.ip());
    if peer.is_some_and(|ip| trusted_proxies.contains(&ip)) {
        if let Some(chain) = header(&parts.headers, "x-forwarded-for") {
            for hop in chain.rsplit(',') {
                let Ok(ip) = hop.trim().parse::<IpAddr>() else { return None };
                if !trusted_proxies.contains(&ip) {
                    return Some(ip.to_string());
                }
            }
        }
        if let Some(ip) = header(&parts.headers, "x-real-ip").and_then(|value| value.parse::<IpAddr>().ok()) {
            return Some(ip.to_string());
        }
    }
    peer.map(|ip| ip.to_string())
}
pub async fn public_base(state: &AuthorityState, headers: &HeaderMap) -> String {
    state.host.public_base(headers).await
}
pub struct ClientIp(pub Option<String>);
impl FromRequestParts<AuthorityState> for ClientIp {
    type Rejection = AppError;
    async fn from_request_parts(parts: &mut Parts, state: &AuthorityState) -> Result<Self, Self::Rejection> {
        state.host.client_ip(parts).await.map(Self)
    }
}
pub fn host_of(url: &str) -> String {
    let rest = url.split("://").nth(1).unwrap_or(url);
    let authority = rest.split('/').next().unwrap_or(rest);
    let host = authority.rsplit('@').next().unwrap_or(authority);
    if host.starts_with('[') {
        return host.split(']').next().unwrap_or(host).trim_start_matches('[').to_string();
    }
    host.split(':').next().unwrap_or(host).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parts(peer: Option<&str>, forwarded: Option<&str>, real: Option<&str>) -> Parts {
        let mut request = axum::http::Request::builder();
        if let Some(value) = forwarded {
            request = request.header("x-forwarded-for", value);
        }
        if let Some(value) = real {
            request = request.header("x-real-ip", value);
        }
        let (mut parts, ()) = request.body(()).unwrap().into_parts();
        if let Some(peer) = peer {
            parts.extensions.insert(ConnectInfo(peer.parse::<SocketAddr>().unwrap()));
        }
        parts
    }
    #[test]
    fn forwarded_headers_require_a_present_trusted_socket_peer() {
        let trusted = ["127.0.0.1".parse().unwrap()];
        assert_eq!(client_ip(&parts(None, Some("203.0.113.8"), Some("203.0.113.9")), &trusted), None);
        assert_eq!(
            client_ip(&parts(Some("198.51.100.1:1234"), Some("203.0.113.8"), Some("203.0.113.9")), &trusted),
            Some("198.51.100.1".into())
        );
        assert_eq!(client_ip(&parts(Some("127.0.0.1:1234"), Some("203.0.113.8"), None), &[]), Some("127.0.0.1".into()));
    }
    #[test]
    fn chains_use_the_rightmost_untrusted_hop_and_keep_legacy_failure_precedence() {
        let trusted = ["127.0.0.1".parse().unwrap(), "192.0.2.1".parse().unwrap()];
        assert_eq!(
            client_ip(&parts(Some("127.0.0.1:1234"), Some("spoofed-prefix, 203.0.113.8, 192.0.2.1"), Some("203.0.113.9")), &trusted),
            Some("203.0.113.8".into())
        );
        assert_eq!(client_ip(&parts(Some("127.0.0.1:1234"), Some("203.0.113.8, invalid"), Some("203.0.113.9")), &trusted), None);
        assert_eq!(client_ip(&parts(Some("127.0.0.1:1234"), Some("203.0.113.8,"), Some("203.0.113.9")), &trusted), None);
    }
    #[test]
    fn all_trusted_or_empty_chains_retain_real_ip_then_peer_fallback() {
        let trusted = ["127.0.0.1".parse().unwrap(), "192.0.2.1".parse().unwrap()];
        assert_eq!(
            client_ip(&parts(Some("127.0.0.1:1234"), Some("192.0.2.1,127.0.0.1"), Some("2001:0db8:0:0:0:0:0:1")), &trusted),
            Some("2001:db8::1".into())
        );
        assert_eq!(client_ip(&parts(Some("127.0.0.1:1234"), Some(" "), Some("invalid")), &trusted), Some("127.0.0.1".into()));
        assert_eq!(client_ip(&parts(Some("127.0.0.1:1234"), None, None), &trusted), Some("127.0.0.1".into()));
    }
}
