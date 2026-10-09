//! Persistence helpers: key/value settings and instances.

use crate::auth::UserRow;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use velora_shared::{AuthConfig, Branding, Experience, FileEntry, InstanceSummary, Loader, MemoryDefaults, ServerEntry};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

pub async fn kv_get<T: DeserializeOwned + Default>(state: &AppState, key: &str) -> AppResult<T> {
    let pool = if matches!(key, "settings" | "branding" | "connections_settings" | "launcher_installer" | "mobile_apps" | "landing_page") {
        &state.platform_db
    } else {
        &state.db
    };
    Ok(velora_panel_settings::get(pool, key).await?)
}

pub async fn kv_set<T: Serialize>(state: &AppState, key: &str, value: &T) -> AppResult<()> {
    velora_panel_settings::set(&state.db, key, value).await.map_err(|e| match e {
        velora_panel_settings::WriteError::Json(e) => AppError::from(e),
        velora_panel_settings::WriteError::Database(e) => AppError::from(e),
    })
}

pub type Settings = velora_panel_settings::Settings<AuthConfig>;

pub use velora_auth_core::identity::username_blocked;

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
        let experience: velora_shared::Experience = serde_json::from_str(&row.experience)?;
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

pub use velora_panel_instances::{FileRow, FileStats, InstanceRow};

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

/// Compatibility host projections retain its existing private Experience defaults and live visibility policy.
pub trait InstancePresentation {
    fn to_admin(&self, stats: FileStats) -> AdminInstance;
    fn to_summary(&self, stats: FileStats) -> InstanceSummary;
    fn visible_to(&self, user: Option<&UserRow>, groups: &[String]) -> bool;
}

impl InstancePresentation for InstanceRow {
    fn to_admin(&self, stats: FileStats) -> AdminInstance {
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

    fn to_summary(&self, stats: FileStats) -> InstanceSummary {
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
    fn visible_to(&self, user: Option<&UserRow>, groups: &[String]) -> bool {
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

pub async fn file_stats(state: &AppState, instance_id: &str) -> AppResult<FileStats> {
    Ok(velora_panel_instances::file_stats(&state.platform_db, instance_id).await?)
}

pub async fn get_instance(state: &AppState, id: &str) -> AppResult<InstanceRow> {
    velora_panel_instances::get_instance(&state.platform_db, id).await?.ok_or_else(|| AppError::not_found("instance not found"))
}

pub async fn list_instances(state: &AppState) -> AppResult<Vec<InstanceRow>> {
    Ok(velora_panel_instances::list_instances(&state.platform_db).await?)
}

pub async fn bump_revision(state: &AppState, id: &str) -> AppResult<()> {
    Ok(velora_panel_instances::bump_revision(&state.platform_db, id, &crate::db::now()).await?)
}

pub async fn instance_files(state: &AppState, id: &str) -> AppResult<Vec<FileRow>> {
    Ok(velora_panel_instances::instance_files(&state.platform_db, id).await?)
}

pub fn to_entries(files: Vec<FileRow>) -> Vec<FileEntry> {
    files
        .into_iter()
        .filter(|f| !f.url.is_empty())
        .map(|f| FileEntry { path: f.path, url: f.url, sha1: f.sha1, size: f.size as u64 })
        .collect()
}

/// Reserved experience storage keeps its existing host-owned namespace.
pub async fn unique_slug(state: &AppState, name: &str) -> AppResult<String> {
    Ok(velora_panel_instances::unique_slug(&state.platform_db, name, &state.cfg.data_dir.join("experiences")).await?)
}
