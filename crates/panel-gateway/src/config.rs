use anyhow::{Context, Result};
use axum::http::{HeaderValue, Uri};
use std::{
    net::{IpAddr, SocketAddr},
    time::Duration,
};

/// Operator destinations only. Request headers/paths never select a host.
#[derive(Clone)]
pub struct Configuration {
    pub bind: SocketAddr,
    pub origin: String,
    pub prefix: String,
    pub public_host: HeaderValue,
    pub public_scheme: HeaderValue,
    pub auth: Uri,
    pub panel: Option<Uri>,
    pub trusted_proxies: Vec<IpAddr>,
    pub header_timeout: Duration,
}

fn origin(value: &str, upstream: bool) -> Result<(String, url::Url)> {
    // Require canonical paths instead of silently normalizing dot segments/escapes.
    let clean = value.trim_end_matches('/');
    let parsed = url::Url::parse(clean).map_err(|_| anyhow::anyhow!("expected an absolute operator HTTP(S) origin"))?;
    anyhow::ensure!(
        matches!(parsed.scheme(), "http" | "https")
            && parsed.host_str().is_some()
            && parsed.username().is_empty()
            && parsed.password().is_none()
            && parsed.query().is_none()
            && parsed.fragment().is_none(),
        "origin must have a host and no credentials, query or fragment"
    );
    let canonical = parsed.as_str().trim_end_matches('/');
    anyhow::ensure!(clean == canonical, "origin must use a canonical URL without normalized path segments");
    let path = parsed.path().trim_end_matches('/');
    anyhow::ensure!(path.bytes().all(|c| c.is_ascii_alphanumeric() || b"/-_~.".contains(&c)), "origin prefix must use plain path segments");
    anyhow::ensure!(!path.contains("//") && !path.split('/').any(|p| matches!(p, "." | "..")), "origin prefix must be canonical");
    if upstream {
        anyhow::ensure!(
            parsed.scheme() == "http" && path.is_empty(),
            "upstream must be a private HTTP origin without a path prefix; terminate TLS outside this component"
        );
    }
    Ok((clean.to_owned(), parsed))
}

impl Configuration {
    pub fn from_env() -> Result<Self> {
        Self::from_lookup(|key| std::env::var(key).ok())
    }
    pub fn from_lookup(mut lookup: impl FnMut(&str) -> Option<String>) -> Result<Self> {
        let mut read = |key| lookup(key).map(|v| v.trim().to_owned()).filter(|v| !v.is_empty());
        let (public, parsed) = origin(&read("VELORA_GATEWAY_PUBLIC_ORIGIN").context("VELORA_GATEWAY_PUBLIC_ORIGIN is required")?, false)
            .context("invalid VELORA_GATEWAY_PUBLIC_ORIGIN")?;
        let (auth, _) = origin(&read("VELORA_GATEWAY_AUTH_UPSTREAM").context("VELORA_GATEWAY_AUTH_UPSTREAM is required")?, true)
            .context("invalid VELORA_GATEWAY_AUTH_UPSTREAM")?;
        let panel = read("VELORA_GATEWAY_PANEL_UPSTREAM")
            .map(|v| origin(&v, true).map(|(v, _)| v.parse::<Uri>()))
            .transpose()
            .context("invalid VELORA_GATEWAY_PANEL_UPSTREAM")?
            .transpose()?;
        let auth: Uri = auth.parse()?;
        anyhow::ensure!(panel.as_ref() != Some(&auth), "Authentication and Panel must be distinct upstreams");
        let bind = read("VELORA_GATEWAY_BIND").unwrap_or_else(|| "127.0.0.1:8080".into()).parse().context("invalid VELORA_GATEWAY_BIND")?;
        let trusted_proxies = read("VELORA_GATEWAY_TRUSTED_PROXIES")
            .map(|v| v.split(',').map(|s| s.trim().parse()).collect::<Result<Vec<IpAddr>, _>>())
            .transpose()
            .context("VELORA_GATEWAY_TRUSTED_PROXIES must contain comma-separated IP addresses")?
            .unwrap_or_default();
        let timeout: u64 = read("VELORA_GATEWAY_HEADER_TIMEOUT_SECS")
            .unwrap_or_else(|| "30".into())
            .parse()
            .context("invalid VELORA_GATEWAY_HEADER_TIMEOUT_SECS")?;
        anyhow::ensure!((1..=3600).contains(&timeout), "header timeout must be 1-3600 seconds");
        let public_uri: Uri = public.parse()?;
        Ok(Self {
            bind,
            prefix: parsed.path().trim_end_matches('/').to_owned(),
            origin: public,
            public_host: HeaderValue::from_str(public_uri.authority().context("public origin needs authority")?.as_str())?,
            public_scheme: HeaderValue::from_str(parsed.scheme())?,
            auth,
            panel,
            trusted_proxies,
            header_timeout: Duration::from_secs(timeout),
        })
    }
}
