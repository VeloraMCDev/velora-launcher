//! Neutral settings/KV helpers. The host selects pools, authorization and auth config/defaults.
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sqlx::SqlitePool;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, bound(deserialize = "A: Deserialize<'de> + Default"))]
pub struct Settings<A = velora_platform_contracts::AuthConfig> {
    pub auth: A,
    pub curseforge_api_key: Option<String>,
    /// Where players can download the launcher (shown on the dashboard).
    pub launcher_download_url: Option<String>,
    /// Public address of the panel (e.g. https://panel.example.com). Used in
    /// skin URLs and the auth server metadata. Falls back to the request.
    pub public_url: Option<String>,
    /// Exact names by default; `*term*` also blocks the term inside names.
    pub username_blocklist: Vec<String>,
}

impl<A: Default> Default for Settings<A> {
    fn default() -> Self {
        Self {
            auth: A::default(),
            curseforge_api_key: None,
            launcher_download_url: None,
            public_url: None,
            username_blocklist: default_username_blocklist(),
        }
    }
}

fn default_username_blocklist() -> Vec<String> {
    ["*nazi*", "*hitler*", "*nigger*", "*faggot*", "*pedophile*", "fuck", "shit", "bitch", "cunt", "rape"]
        .into_iter()
        .map(str::to_string)
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsError(pub &'static str);
impl std::fmt::Display for SettingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
impl std::error::Error for SettingsError {}

pub fn normalize<A>(mut s: Settings<A>, saved_key: Option<String>) -> Result<Settings<A>, SettingsError> {
    // An empty key means "keep the current one"; "-" clears it.
    s.curseforge_api_key = match s.curseforge_api_key.as_deref().map(str::trim) {
        None | Some("") => saved_key,
        Some("-") => None,
        Some(k) => Some(k.to_string()),
    };
    s.public_url = s.public_url.map(|u| u.trim().trim_end_matches('/').to_string()).filter(|u| !u.is_empty());
    s.username_blocklist =
        s.username_blocklist.into_iter().map(|entry| entry.trim().to_ascii_lowercase()).filter(|entry| !entry.is_empty()).collect();
    if s.username_blocklist.len() > 200
        || s.username_blocklist.iter().any(|entry| {
            let word = entry.trim_matches('*');
            word.len() < 3
                || word.len() > 16
                || !word.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                || (entry.contains('*') && !(entry.starts_with('*') && entry.ends_with('*') && entry.matches('*').count() == 2))
        })
    {
        return Err(SettingsError("blacklist entries must be 3–16 letters, numbers or underscores; use *word* to match within names"));
    }
    if let Some(u) = &s.public_url {
        if !u.starts_with("http://") && !u.starts_with("https://") {
            return Err(SettingsError("the public URL must start with https:// (or http://)"));
        }
    }
    Ok(s)
}

pub fn validate_branding_name(name: &str) -> Result<(), SettingsError> {
    if name.trim().is_empty() {
        Err(SettingsError("the launcher needs a name"))
    } else {
        Ok(())
    }
}

pub async fn get<T: DeserializeOwned + Default>(pool: &SqlitePool, key: &str) -> Result<T, sqlx::Error> {
    let raw: Option<String> = sqlx::query_scalar("SELECT value FROM kv WHERE key = ?").bind(key).fetch_optional(pool).await?;
    Ok(raw.and_then(|r| serde_json::from_str(&r).ok()).unwrap_or_default())
}

#[derive(Debug)]
pub enum WriteError {
    Json(serde_json::Error),
    Database(sqlx::Error),
}
impl From<serde_json::Error> for WriteError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}
impl From<sqlx::Error> for WriteError {
    fn from(e: sqlx::Error) -> Self {
        Self::Database(e)
    }
}
pub async fn set<T: Serialize>(pool: &SqlitePool, key: &str, value: &T) -> Result<(), WriteError> {
    sqlx::query("INSERT INTO kv (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value")
        .bind(key)
        .bind(serde_json::to_string(value)?)
        .execute(pool)
        .await?;
    Ok(())
}
impl std::fmt::Display for WriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json(e) => write!(f, "invalid JSON: {e}"),
            Self::Database(e) => write!(f, "database error: {e}"),
        }
    }
}
impl std::error::Error for WriteError {}
