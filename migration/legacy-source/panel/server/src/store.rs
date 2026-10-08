//! Persistence helpers: key/value settings and instances.

use crate::auth::UserRow;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use scopenet_shared::{AuthConfig, Branding, Experience, FileEntry, InstanceSummary, Loader, MemoryDefaults, ServerEntry};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

pub async fn kv_get<T: DeserializeOwned + Default>(state: &AppState, key: &str) -> AppResult<T> {
    let pool = if matches!(key, "settings" | "branding" | "connections_settings" | "launcher_installer" | "mobile_apps" | "landing_page") {
        &state.platform_db
    } else {
        &state.db
    };
    let raw: Option<String> = sqlx::query_scalar("SELECT value FROM kv WHERE key = ?").bind(key).fetch_optional(pool).await?;
    Ok(raw.and_then(|r| serde_json::from_str(&r).ok()).unwrap_or_default())
}

pub async fn kv_set<T: Serialize>(state: &AppState, key: &str, value: &T) -> AppResult<()> {
    sqlx::query("INSERT INTO kv (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value")
        .bind(key)
        .bind(serde_json::to_string(value)?)
        .execute(&state.db)
        .await?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub auth: AuthConfig,
    pub curseforge_api_key: Option<String>,
    /// Where players can download the launcher (shown on the dashboard).
    pub launcher_download_url: Option<String>,
    /// Public address of the panel (e.g. https://panel.example.com). Used in
    /// skin URLs and the auth server metadata. Falls back to the request.
    pub public_url: Option<String>,
    /// Exact names by default; `*term*` also blocks the term inside names.
    pub username_blocklist: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            auth: AuthConfig::default(),
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

pub fn username_blocked(name: &str, entries: &[String]) -> bool {
    let normalized = name.to_ascii_lowercase().replace('_', "");
    entries.iter().any(|entry| {
        let rule = entry.trim().to_ascii_lowercase();
        if rule.is_empty() {
            return false;
        }
        if let Some(inner) = rule.strip_prefix('*').and_then(|r| r.strip_suffix('*')) {
            inner.len() >= 3 && normalized.contains(inner)
        } else {
            normalized == rule.replace('_', "")
        }
    })
}

pub async fn check_username(state: &AppState, name: &str) -> AppResult<()> {
    if username_blocked(name, &settings(state).await?.username_blocklist) {
        return Err(AppError::bad_request("that username is unavailable; choose another"));
    }
    Ok(())
}

#[cfg(test)]
mod username_tests {
    use super::*;

    #[test]
    fn blacklist_matches_exact_and_marked_substrings_without_overblocking() {
        let rules = vec!["*nazi*".into(), "shit".into()];
        assert!(username_blocked("naziFan", &rules));
        assert!(username_blocked("ShIt", &rules));
        assert!(!username_blocked("grapes", &rules));
        assert!(!username_blocked("Shitake", &rules));
    }
}

pub async fn settings(state: &AppState) -> AppResult<Settings> {
    kv_get(state, "settings").await
}

pub async fn branding(state: &AppState) -> AppResult<Branding> {
    if let Some(id) = &state.instance_id {
        let row = get_instance(state, id).await?;
        let experience: scopenet_shared::Experience = serde_json::from_str(&row.experience)?;
        if let Some(branding) = experience.branding {
            return Ok(branding);
        }
    }
    kv_get(state, "branding").await
}

/// Environment variable wins over the value saved in the panel.
pub async fn curseforge_key(state: &AppState) -> AppResult<String> {
    if let Some(k) = &state.cfg.curseforge_api_key {
        return Ok(k.clone());
    }
    settings(state)
        .await?
        .curseforge_api_key
        .filter(|k| !k.is_empty())
        .ok_or_else(|| AppError::bad_request("add a CurseForge API key in Settings first (get one at console.curseforge.com)"))
}

// ---------------------------------------------------------------------------
// Instances
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct InstanceRow {
    pub experience: String,
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon_url: Option<String>,
    pub banner_url: Option<String>,
    pub logo_url: Option<String>,
    pub mc_version: String,
    pub loader: String,
    pub loader_version: Option<String>,
    pub source_kind: String,
    pub source_label: String,
    pub source_ref: String,
    pub visibility: String,
    pub allowed_groups: String,
    pub memory_min: i64,
    pub memory_max: i64,
    pub jvm_args: String,
    pub server: Option<String>,
    pub featured: bool,
    pub enabled: bool,
    pub sort: i64,
    pub revision: i64,
    pub clean_epoch: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// Full instance as seen by admins.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AdminInstance {
    pub experience: Experience,
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon_url: Option<String>,
    pub banner_url: Option<String>,
    pub logo_url: Option<String>,
    pub mc_version: String,
    pub loader: Loader,
    pub loader_version: Option<String>,
    pub source_kind: String,
    pub source_label: String,
    pub source_ref: serde_json::Value,
    /// "public" | "members" | "groups"
    pub visibility: String,
    pub allowed_groups: Vec<String>,
    pub memory: MemoryDefaults,
    pub jvm_args: String,
    pub server: Option<ServerEntry>,
    pub featured: bool,
    pub enabled: bool,
    pub sort: i64,
    pub revision: i64,
    pub clean_epoch: i64,
    pub created_at: String,
    pub updated_at: String,
    pub file_count: u32,
    pub total_size: u64,
    pub missing_count: u32,
}

impl InstanceRow {
    pub fn to_admin(&self, stats: FileStats) -> AdminInstance {
        AdminInstance {
            experience: serde_json::from_str(&self.experience).unwrap_or_default(),
            id: self.id.clone(),
            name: self.name.clone(),
            description: self.description.clone(),
            icon_url: self.icon_url.clone(),
            banner_url: self.banner_url.clone(),
            logo_url: self.logo_url.clone(),
            mc_version: self.mc_version.clone(),
            loader: Loader::parse(&self.loader).unwrap_or_default(),
            loader_version: self.loader_version.clone(),
            source_kind: self.source_kind.clone(),
            source_label: self.source_label.clone(),
            source_ref: serde_json::from_str(&self.source_ref).unwrap_or_default(),
            visibility: self.visibility.clone(),
            allowed_groups: serde_json::from_str(&self.allowed_groups).unwrap_or_default(),
            memory: MemoryDefaults { min_mb: self.memory_min as u32, max_mb: self.memory_max as u32 },
            jvm_args: self.jvm_args.clone(),
            server: self.server.as_deref().and_then(|s| serde_json::from_str(s).ok()),
            featured: self.featured,
            enabled: self.enabled,
            sort: self.sort,
            revision: self.revision,
            clean_epoch: self.clean_epoch,
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
            file_count: stats.count,
            total_size: stats.size,
            missing_count: stats.missing,
        }
    }

    pub fn to_summary(&self, stats: FileStats) -> InstanceSummary {
        let a = self.to_admin(stats);
        InstanceSummary {
            experience: a.experience,
            id: a.id,
            name: a.name,
            description: a.description,
            icon_url: a.icon_url,
            banner_url: a.banner_url,
            logo_url: a.logo_url,
            mc_version: a.mc_version,
            loader: a.loader,
            loader_version: a.loader_version,
            revision: a.revision,
            clean_epoch: a.clean_epoch,
            server: a.server.filter(|s| !s.address.trim().is_empty()),
            memory: a.memory,
            jvm_args: a.jvm_args,
            featured: a.featured,
            source_label: a.source_label,
            file_count: a.file_count,
            total_size: a.total_size,
        }
    }

    /// Can this (possibly anonymous) user see the instance?
    pub fn visible_to(&self, user: Option<&UserRow>, groups: &[String]) -> bool {
        if !self.enabled {
            return false;
        }
        if user.is_some_and(|u| u.is_admin()) {
            return true;
        }
        match self.visibility.as_str() {
            "public" => true,
            "members" => user.is_some(),
            "groups" => {
                let allowed: Vec<String> = serde_json::from_str(&self.allowed_groups).unwrap_or_default();
                user.is_some() && allowed.iter().any(|g| groups.iter().any(|x| x.eq_ignore_ascii_case(g)))
            }
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct FileStats {
    pub count: u32,
    pub size: u64,
    pub missing: u32,
}

pub async fn file_stats(state: &AppState, instance_id: &str) -> AppResult<FileStats> {
    let (count, size, missing): (i64, Option<i64>, Option<i64>) =
        sqlx::query_as("SELECT COUNT(*), SUM(size), SUM(CASE WHEN url = '' THEN 1 ELSE 0 END) FROM instance_files WHERE instance_id = ?")
            .bind(instance_id)
            .fetch_one(&state.platform_db)
            .await?;
    Ok(FileStats { count: count as u32, size: size.unwrap_or(0) as u64, missing: missing.unwrap_or(0) as u32 })
}

pub async fn get_instance(state: &AppState, id: &str) -> AppResult<InstanceRow> {
    sqlx::query_as("SELECT * FROM instances WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.platform_db)
        .await?
        .ok_or_else(|| AppError::not_found("instance not found"))
}

pub async fn list_instances(state: &AppState) -> AppResult<Vec<InstanceRow>> {
    Ok(sqlx::query_as("SELECT * FROM instances ORDER BY featured DESC, sort ASC, name COLLATE NOCASE ASC")
        .fetch_all(&state.platform_db)
        .await?)
}

pub async fn bump_revision(state: &AppState, id: &str) -> AppResult<()> {
    sqlx::query("UPDATE instances SET revision = revision + 1, updated_at = ? WHERE id = ?")
        .bind(crate::db::now())
        .bind(id)
        .execute(&state.platform_db)
        .await?;
    Ok(())
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct FileRow {
    pub path: String,
    pub url: String,
    pub sha1: String,
    pub size: i64,
    pub origin: String,
    pub note: Option<String>,
}

pub async fn instance_files(state: &AppState, id: &str) -> AppResult<Vec<FileRow>> {
    Ok(sqlx::query_as("SELECT path, url, sha1, size, origin, note FROM instance_files WHERE instance_id = ? ORDER BY path")
        .bind(id)
        .fetch_all(&state.platform_db)
        .await?)
}

pub fn to_entries(files: Vec<FileRow>) -> Vec<FileEntry> {
    files
        .into_iter()
        .filter(|f| !f.url.is_empty())
        .map(|f| FileEntry { path: f.path, url: f.url, sha1: f.sha1, size: f.size as u64 })
        .collect()
}

/// URL-safe, human-readable id from a name, made unique.
pub async fn unique_slug(state: &AppState, name: &str) -> AppResult<String> {
    let mut base: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    base.truncate(40);
    if base.is_empty() {
        base = "instance".into();
    }
    let mut slug = base.clone();
    let mut n = 2;
    while sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM instances WHERE id = ?").bind(&slug).fetch_one(&state.platform_db).await? > 0
        || state.cfg.data_dir.join("experiences").join(&slug).exists()
    {
        slug = format!("{base}-{n}");
        n += 1;
    }
    Ok(slug)
}
