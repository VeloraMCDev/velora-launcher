//! Talking to the Velora Panel, and reading Velora Core jar names. Pure logic lives here so it can be tested without a Panel.
use serde::Deserialize;
use std::{sync::LazyLock, time::{Duration, Instant}};

#[derive(Debug, Clone, Deserialize)]
pub struct CoreFile {
    pub name: String,
    pub sha256: String,
    /// Relative to the Velora Panel address.
    pub path: String,
}

/// The release the Velora Panel currently serves (`/api/v1/core/latest`).
#[derive(Debug, Clone, Deserialize)]
pub struct Latest {
    pub version: String,
    pub minecraft: String,
    #[serde(default)]
    pub notes: String,
    pub server: CoreFile,
}

/// Accepts an http(s) address without credentials, query or fragment and returns it without a trailing slash.
pub fn normalize_panel_url(raw: &str) -> Result<String, String> {
    let value = raw.trim().trim_end_matches('/');
    let rest = value
        .strip_prefix("https://")
        .or_else(|| value.strip_prefix("http://"))
        .ok_or("The Velora Panel address must start with https:// or http://")?;
    let host = rest.split('/').next().unwrap_or_default();
    if host.is_empty() || host.contains('@') || value.contains('?') || value.contains('#') || value.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err("Use the plain Panel address, for example https://velora.example.com".into());
    }
    Ok(value.to_owned())
}

/// A Velora server token: `sn_` followed by 40 letters or digits.
pub fn token_ok(token: &str) -> bool {
    token.len() == 43 && token.starts_with("sn_") && token[3..].bytes().all(|b| b.is_ascii_alphanumeric())
}

/// The URL a Wings node should pull a jar from. Only the Panel's own jar path is accepted.
pub fn download_url(panel_url: &str, file: &CoreFile) -> Result<String, String> {
    if !file.path.starts_with("/api/v1/core/files/") || file.path.contains("..") || file.path.contains('?') || file.path.contains('#') {
        return Err("The Velora Panel returned an unexpected download path".into());
    }
    Ok(format!("{panel_url}{}", file.path))
}

/// What a file in `mods/` says about itself: the Velora Core server jar and its version, also for the pre-rename name.
pub fn parse_installed(name: &str) -> Option<InstalledJar> {
    let stem = name.strip_suffix(".jar")?;
    let (legacy, rest) = if let Some(rest) = stem.strip_prefix("velora-core-server-") {
        (false, rest)
    } else if let Some(rest) = stem.strip_prefix("scopenet-fabric-") {
        (true, rest)
    } else {
        return None;
    };
    // `<minecraft>-<version>`, such as `1.20.1-0.6.0`.
    let (minecraft, version) = rest.split_once('-')?;
    let numeric = |v: &str| !v.is_empty() && v.split('.').all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()));
    if !numeric(minecraft) || !numeric(version) {
        return None;
    }
    Some(InstalledJar { file: name.to_owned(), version: version.to_owned(), legacy })
}

#[derive(Debug, Clone, PartialEq)]
pub struct InstalledJar {
    pub file: String,
    pub version: String,
    /// Installed under the pre-rename `scopenet-fabric-*` name.
    pub legacy: bool,
}

/// Whether `candidate` is a newer dotted version than `installed`.
pub fn newer(candidate: &str, installed: &str) -> bool {
    let parts = |v: &str| v.split('.').map(|p| p.parse::<u64>().unwrap_or(0)).collect::<Vec<_>>();
    let (a, b) = (parts(candidate), parts(installed));
    for i in 0..a.len().max(b.len()) {
        let (x, y) = (a.get(i).copied().unwrap_or(0), b.get(i).copied().unwrap_or(0));
        if x != y {
            return x > y;
        }
    }
    false
}

/// The flag that makes a server use the Panel's sign-in.
pub fn javaagent_flag(panel_url: &str) -> String {
    format!("-javaagent:authlib-injector.jar={panel_url}/api/yggdrasil")
}

/// The minimal config the extension writes; the mod fills in every other option with its defaults.
pub fn config_file(panel_url: &str, token: &str) -> String {
    format!(
        "# Written by the Velora Core extension for Calagopus.\n# Delete this file and restart the server to generate the full, documented config.\npanel-url={panel_url}\ntoken={token}\n"
    )
}

type Cache = tokio::sync::Mutex<Option<(Instant, String, Option<Latest>)>>;
static CACHE: LazyLock<Cache> = LazyLock::new(|| tokio::sync::Mutex::new(None));

/// Asks the Velora Panel for the approved release, reusing an answer fetched in the last `max_age`.
pub async fn latest(panel_url: &str, max_age: Duration) -> Result<Option<Latest>, anyhow::Error> {
    let mut cache = CACHE.lock().await;
    if let Some((at, url, value)) = cache.as_ref() {
        if url == panel_url && at.elapsed() < max_age {
            return Ok(value.clone());
        }
    }
    let response = reqwest::Client::new()
        .get(format!("{panel_url}/api/v1/core/latest"))
        .header("User-Agent", "velora-core-calagopus")
        .timeout(Duration::from_secs(15))
        .send()
        .await?
        .error_for_status()?;
    let value: Option<Latest> = response.json().await?;
    *cache = Some((Instant::now(), panel_url.to_owned(), value.clone()));
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_addresses_are_plain_http_urls() {
        assert_eq!(normalize_panel_url(" https://velora.example.com/ "), Ok("https://velora.example.com".into()));
        assert_eq!(normalize_panel_url("http://10.0.0.5:8080"), Ok("http://10.0.0.5:8080".into()));
        assert!(normalize_panel_url("velora.example.com").is_err());
        assert!(normalize_panel_url("https://user:pw@velora.example.com").is_err());
        assert!(normalize_panel_url("https://velora.example.com/?x=1").is_err());
        assert!(normalize_panel_url("https://velora.example.com/#a").is_err());
        assert!(normalize_panel_url("https://").is_err());
        assert!(normalize_panel_url("https://a b").is_err());
    }

    #[test]
    fn tokens_match_the_panel_format() {
        assert!(token_ok(&format!("sn_{}", "aB3".repeat(13) + "x")));
        assert!(!token_ok("sn_short"));
        assert!(!token_ok(&format!("sn_{}", "a".repeat(41))));
        assert!(!token_ok(&format!("xx_{}", "a".repeat(40))));
        assert!(!token_ok(&format!("sn_{}!", "a".repeat(39))));
    }

    #[test]
    fn jar_names_give_role_and_version() {
        assert_eq!(parse_installed("velora-core-server-1.20.1-0.6.0.jar"), Some(InstalledJar { file: "velora-core-server-1.20.1-0.6.0.jar".into(), version: "0.6.0".into(), legacy: false }));
        assert_eq!(parse_installed("scopenet-fabric-1.20.1-0.5.0.jar").map(|j| (j.version, j.legacy)), Some(("0.5.0".into(), true)));
        assert_eq!(parse_installed("velora-core-client-1.20.1-0.6.0.jar"), None, "the client jar is not a server mod");
        assert_eq!(parse_installed("fabric-api-0.92.2+1.20.1.jar"), None);
        assert_eq!(parse_installed("velora-core-server-1.20.1-0.6.0-dev.jar"), None);
        assert_eq!(parse_installed("velora-core-server-.jar"), None);
    }

    #[test]
    fn versions_compare_numerically() {
        assert!(newer("0.6.0", "0.5.0"));
        assert!(newer("0.10.0", "0.9.9"));
        assert!(newer("1.0", "0.9.9"));
        assert!(!newer("0.6.0", "0.6.0"));
        assert!(!newer("0.5.9", "0.6.0"));
    }

    #[test]
    fn only_the_panels_own_jar_path_is_fetched() {
        let file = |path: &str| CoreFile { name: "x.jar".into(), sha256: "ab".repeat(32), path: path.into() };
        assert_eq!(download_url("https://v.example.com", &file("/api/v1/core/files/ab/x.jar")), Ok("https://v.example.com/api/v1/core/files/ab/x.jar".into()));
        assert!(download_url("https://v.example.com", &file("https://evil.example/x.jar")).is_err());
        assert!(download_url("https://v.example.com", &file("/api/v1/core/files/../../x.jar")).is_err());
        assert!(download_url("https://v.example.com", &file("/api/v1/core/files/ab/x.jar?a=1")).is_err());
    }

    #[test]
    fn config_and_flag_text() {
        let config = config_file("https://v.example.com", "sn_x");
        assert!(config.contains("panel-url=https://v.example.com\ntoken=sn_x\n"));
        assert_eq!(javaagent_flag("https://v.example.com"), "-javaagent:authlib-injector.jar=https://v.example.com/api/yggdrasil");
    }
}
