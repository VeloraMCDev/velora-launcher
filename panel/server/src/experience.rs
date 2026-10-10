//! The platform owns identity and installations; instances own all gameplay state.
//! A scoped pool reuses the existing domain handlers without query-string isolation.
use crate::store::InstancePresentation;
use crate::{
    auth,
    error::{AppError, AppResult},
    state::AppState,
    store,
};
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use velora_shared::{Experience, EXPERIENCE_FEATURES};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
    Acquire, Row, SqlitePool,
};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use tokio::sync::Mutex;

pub const PLATFORM_TABLES: &[&str] = &[
    "users",
    "groups",
    "user_groups",
    "reserved_usernames",
    "capes",
    "ygg_tokens",
    "ygg_sessions",
    "player_keys",
    "launcher_sessions",
    "instances",
    "instance_files",
    "friendships",
    "direct_messages",
    "game_invites",
    "user_profiles",
    "user_posts",
    "user_post_likes",
    "account_connections",
    "oauth_attempts",
    "password_resets",
    "email_log",
];

#[derive(Default)]
pub struct ExperienceStores {
    pools: Mutex<HashMap<String, (SqlitePool, Arc<crate::worldmap::WorldMap>)>>,
    workers_enabled: AtomicBool,
}

impl ExperienceStores {
    pub async fn state(&self, platform: &AppState, id: &str) -> AppResult<AppState> {
        let mut pools = self.pools.lock().await;
        let instance = store::get_instance(platform, id).await?;
        let fresh = !pools.contains_key(id);
        let (pool, worldmap) = if let Some(runtime) = pools.get(id) {
            runtime.clone()
        } else {
            let pool = self.open(platform, id).await?;
            let runtime = (pool, Arc::new(crate::worldmap::WorldMap::new(&platform.cfg.data_dir.join("experiences").join(id))));
            pools.insert(id.to_string(), runtime.clone());
            runtime
        };
        let mut state = platform.clone();
        state.db = pool;
        state.instance_id = Some(instance.id);
        // Assets and maps are owned by the experience, too.
        let mut cfg = (*platform.cfg).clone();
        cfg.data_dir = platform.cfg.data_dir.join("experiences").join(id);
        state.worldmap = worldmap;
        state.cfg = Arc::new(cfg);
        drop(pools);
        if fresh && self.workers_enabled.load(Ordering::Relaxed) {
            start_workers(state.clone());
        }
        Ok(state)
    }

    pub async fn enable_workers(&self, platform: &AppState) -> AppResult<()> {
        self.workers_enabled.store(true, Ordering::Relaxed);
        let instances = store::list_instances(platform).await?;

        for instance in instances {
            self.state(platform, &instance.id).await?;
        }
        Ok(())
    }

    /// Shared account reports aggregate current stores without copying gameplay
    /// back into platform tables. Scoped reports still see just one experience.
    pub async fn reporting_states(&self, platform: &AppState) -> AppResult<Vec<AppState>> {
        if platform.instance_id.is_some() || !platform.cfg.data_dir.join("panel.db").is_file() {
            return Ok(vec![platform.clone()]);
        }
        let instances = store::list_instances(platform).await?;
        if instances.is_empty() {
            return Ok(vec![platform.clone()]);
        }
        let mut states = Vec::new();
        for instance in instances {
            states.push(self.state(platform, &instance.id).await?);
        }
        Ok(states)
    }

    pub async fn retire(&self, id: &str) {
        if let Some((pool, _)) = self.pools.lock().await.remove(id) {
            pool.close().await;
        }
    }

    async fn open(&self, platform: &AppState, id: &str) -> AppResult<SqlitePool> {
        // IDs come from a platform record, but still verify they are safe path components.
        if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
            return Err(AppError::bad_request("invalid instance storage ID"));
        }
        let platform_path = platform.cfg.data_dir.join("panel.db");
        if !platform_path.is_file() {
            return Err(AppError::bad_request("instance storage requires the persistent platform database"));
        }
        let dir = platform.cfg.data_dir.join("experiences").join(id);
        std::fs::create_dir_all(&dir)?;
        let options = SqliteConnectOptions::new()
            .filename(dir.join("panel.db"))
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(SqliteJournalMode::Wal);
        let initial = SqlitePoolOptions::new().max_connections(1).connect_with(options.clone()).await?;
        crate::db::migrate_mode(&initial, true).await.map_err(AppError::from)?;
        let initialized: Option<String> =
            sqlx::query_scalar("SELECT value FROM kv WHERE key = 'experience_storage_v1'").fetch_optional(&initial).await?;
        if initialized.is_none() {
            // Previously unassigned servers belong to the legacy primary experience.
            let primary: String =
                sqlx::query_scalar("SELECT id FROM instances ORDER BY created_at,id LIMIT 1").fetch_one(&platform.platform_db).await?;
            if primary == id {
                sqlx::query("UPDATE game_servers SET instance_id=? WHERE instance_id IS NULL OR instance_id='' ")
                    .bind(id)
                    .execute(&platform.platform_db)
                    .await?;
            }
            import_legacy(&initial, &platform_path, id).await?;
        }
        initial.close().await;
        let platform_path = platform_path.to_string_lossy().to_string();
        let pool = SqlitePoolOptions::new().max_connections(4).after_connect(move |conn, _| {
            let path = platform_path.clone();
            Box::pin(async move {
                sqlx::query("ATTACH DATABASE ? AS platform").bind(path).execute(&mut *conn).await?;
                // These views share current identity rows, never copied credentials.
                for table in PLATFORM_TABLES {
                    if *table == "groups" {
                        sqlx::query("CREATE TEMP VIEW groups AS SELECT g.id,g.name,g.color,COALESCE(l.luckperms_group,'') AS luckperms_group,COALESCE(l.discord_role,'') AS discord_role FROM platform.groups g LEFT JOIN main.experience_group_links l ON l.group_id=g.id").execute(&mut *conn).await?;
                        continue;
                    }
                    sqlx::query(&format!("CREATE TEMP VIEW \"{table}\" AS SELECT * FROM platform.\"{table}\""))
                        .execute(&mut *conn).await?;
                }
                Ok(())
            })
        }).connect_with(options).await?;
        Ok(pool)
    }
}

/// One-time, transactional adoption of legacy data. Never modify the source.
/// Server/guild/instance-owned rows follow their owner. Ambiguous network-wide
/// history is retained once, by the oldest instance, rather than duplicated.
async fn import_legacy(pool: &SqlitePool, path: &std::path::Path, id: &str) -> AppResult<()> {
    let mut conn = pool.acquire().await?;
    sqlx::query("ATTACH DATABASE ? AS legacy").bind(path.to_string_lossy().as_ref()).execute(&mut *conn).await?;
    let primary: Option<String> =
        sqlx::query_scalar("SELECT id FROM legacy.instances ORDER BY created_at, id LIMIT 1").fetch_optional(&mut *conn).await?;
    let is_primary = primary.as_deref() == Some(id);
    let tables: Vec<String> =
        sqlx::query_scalar("SELECT name FROM main.sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY rowid")
            .fetch_all(&mut *conn)
            .await?;
    let triggers: Vec<(String, String)> =
        sqlx::query_as("SELECT name, sql FROM main.sqlite_master WHERE type='trigger'").fetch_all(&mut *conn).await?;
    let mut tx = conn.begin().await?;
    // Copying already-earned history must not fire XP/reward triggers again.
    for (name, _) in &triggers {
        sqlx::query(&format!("DROP TRIGGER \"{name}\"")).execute(&mut *tx).await?;
    }
    for table in tables {
        if PLATFORM_TABLES.contains(&table.as_str()) {
            continue;
        }
        let columns = sqlx::query(&format!("PRAGMA main.table_info(\"{table}\")")).fetch_all(&mut *tx).await?;
        let names: Vec<String> = columns.iter().map(|r| r.get("name")).collect();
        let has = |n: &str| names.iter().any(|c| c == n);
        let filter = if table == "kv" {
            if !is_primary {
                continue;
            }
            "key NOT IN ('settings','branding','connections_settings','landing_page','launcher_installer','mobile_apps','experience_storage_v1') AND key NOT LIKE 'pack_location:%'".to_string()
        } else if table == "game_servers" {
            "instance_id = ?1".to_string()
        } else if has("server_id") {
            format!("(server_id IN (SELECT id FROM main.game_servers) {})", if is_primary { "OR server_id IS NULL" } else { "" })
        } else if has("instance_id") {
            format!("(instance_id = ?1 {})", if is_primary { "OR instance_id IS NULL OR instance_id = ''" } else { "" })
        } else if has("guild_id") {
            "guild_id IN (SELECT id FROM main.guilds)".to_string()
        } else {
            if !is_primary {
                continue;
            }
            "1".to_string()
        };
        // Parent references keep dependent rows inside the selected experience.
        let foreign_keys = sqlx::query(&format!("PRAGMA main.foreign_key_list(\"{table}\")")).fetch_all(&mut *tx).await?;
        let mut predicates = vec![filter];
        for fk in foreign_keys {
            let parent: String = fk.get("table");
            let from: String = fk.get("from");
            let to: String = fk.get("to");
            if !PLATFORM_TABLES.contains(&parent.as_str()) && parent != table {
                predicates.push(format!("(\"{from}\" IS NULL OR \"{from}\" IN (SELECT \"{to}\" FROM main.\"{parent}\"))"));
            }
        }
        // Preserve seeded defaults for fresh experiences; replace only legacy rows adopted here.
        let sql = format!("INSERT OR REPLACE INTO main.\"{table}\" SELECT * FROM legacy.\"{table}\" WHERE {}", predicates.join(" AND "));
        if sql.contains("?1") {
            sqlx::query(&sql).bind(id).execute(&mut *tx).await?;
        } else {
            sqlx::query(&sql).execute(&mut *tx).await?;
        }
    }
    if is_primary {
        sqlx::query("INSERT OR IGNORE INTO main.experience_group_links SELECT id,luckperms_group,discord_role FROM legacy.groups")
            .execute(&mut *tx)
            .await?;
    }
    for (_, sql) in &triggers {
        sqlx::query(sql).execute(&mut *tx).await?;
    }
    sqlx::query("INSERT INTO kv(key,value) VALUES('experience_storage_v1','true')").execute(&mut *tx).await?;
    // Map images have the same server ownership as their database rows. Copy
    // before committing the marker so an interrupted migration can safely retry.
    let servers: Vec<i64> = sqlx::query_scalar("SELECT id FROM main.game_servers").fetch_all(&mut *tx).await?;
    let source = path.parent().unwrap().join("map");
    let destination = path.parent().unwrap().join("experiences").join(id).join("map");
    tokio::task::spawn_blocking(move || {
        for server in servers {
            copy_tree(&source.join(server.to_string()), &destination.join(server.to_string()))?;
        }
        Ok::<_, std::io::Error>(())
    })
    .await
    .map_err(anyhow::Error::from)??;
    tx.commit().await?;
    sqlx::query("DETACH DATABASE legacy").execute(&mut *conn).await?;
    Ok(())
}

fn copy_tree(source: &std::path::Path, destination: &std::path::Path) -> std::io::Result<()> {
    let metadata = match std::fs::symlink_metadata(source) {
        Ok(metadata) => metadata,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };
    if metadata.file_type().is_symlink() {
        return Ok(());
    }
    if metadata.is_dir() {
        std::fs::create_dir_all(destination)?;
        for entry in std::fs::read_dir(source)? {
            let entry = entry?;
            copy_tree(&entry.path(), &destination.join(entry.file_name()))?;
        }
    } else if !destination.exists() {
        std::fs::copy(source, destination)?;
    }
    Ok(())
}

pub fn feature_for_path(path: &str) -> Option<&'static str> {
    if path.starts_with("/api/experience-map/") || path.starts_with("/api/map/") {
        return Some("maps");
    }
    if path == "/api/v1/account/stats" {
        return Some("progression");
    }
    let p = path.strip_prefix("/api/admin/").or_else(|| path.strip_prefix("/api/v1/")).or_else(|| path.strip_prefix("/api/server/v1/"))?;
    let head = p.split('/').next().unwrap_or("");
    if head.starts_with("resource-") || head == "resource-pack.zip" {
        return Some("content");
    }
    if p.contains("/map") || head == "map" || path.starts_with("/api/experience-map/") {
        return Some("maps");
    }
    if p.starts_with("player/xp") {
        return Some("progression");
    }
    if p.starts_with("player/quest-objective") {
        return Some("quests");
    }
    if p.starts_with("players/") && p.ends_with("/collections") {
        return Some("collections");
    }
    match head {
        "casino" => Some("casino"),
        "economy" | "market" | "board" => Some("economy"),
        "guilds" | "guild" | "admin-claims" | "guild-flag-policy" => Some("guilds"),
        "leveling" | "levels" | "leaderboard" | "progression" | "stats" | "bulk" | "luckperms" | "reward-bundles" | "reward-deliveries"
        | "rewards" => Some("progression"),
        "quests" | "quest-chains" => Some("quests"),
        "achievements" => Some("achievements"),
        "collections" | "cosmetics" => Some("collections"),
        "items" | "content" | "resource-assets" | "resource-pack" => Some("content"),
        "utilities" | "commands" | "chat" | "kits" => Some("commands"),
        "events" => Some("events"),
        "companion" => Some("companion"),
        _ => None,
    }
}

pub fn platform_path(path: &str) -> bool {
    // Operations aggregates the shared account and audit store across instances.
    if path == "/api/admin/operations/summary" {
        return true;
    }
    if path.starts_with("/api/yggdrasil") {
        return true;
    }
    if path == "/api/v1/account/stats" || path == "/api/v1/account/discord" {
        return false;
    }
    if path.starts_with("/api/admin/groups/") && (path.ends_with("/luckperms") || path.ends_with("/discord-role")) {
        return false;
    }
    let p = path.strip_prefix("/api/admin/").or_else(|| path.strip_prefix("/api/v1/"));
    if path.starts_with("/api/experience-map/") {
        return false;
    }
    match p.and_then(|p| p.split('/').next()) {
        Some(
            "auth" | "auth-server" | "account" | "profile" | "social" | "friends" | "messages" | "invites" | "members" | "users" | "groups"
            | "capes" | "instances" | "branding" | "settings" | "landing" | "uploads" | "avatar" | "email" | "email-templates" | "meta"
            | "icons" | "mc" | "mc-textures" | "core" | "launcher" | "launcher-updates" | "mobile-apps" | "connections" | "emails" | "activity",
        ) => true,
        _ => !path.starts_with("/api/"),
    }
}

pub async fn scope_request(State(platform): State<AppState>, mut req: Request, next: Next) -> Response {
    let result: AppResult<()> = async {
        let path = req.uri().path().to_string();
        if platform_path(&path) {
            return Ok(());
        }
        let mut id = req
            .headers()
            .get("x-velora-instance")
            .map(|v| v.to_str().map(str::to_owned))
            .transpose()
            .map_err(|_| AppError::bad_request("invalid instance header"))?;
        let legacy = req
            .headers()
            .get("x-scopenet-instance")
            .map(|v| v.to_str().map(str::to_owned))
            .transpose()
            .map_err(|_| AppError::bad_request("invalid instance header"))?;
        if let Some(legacy) = legacy {
            if id.as_ref().is_some_and(|id| id != &legacy) {
                return Err(AppError::bad_request("conflicting instance context"));
            }
            id = Some(legacy);
        }
        let query = axum::extract::Query::<HashMap<String, String>>::try_from_uri(req.uri())
            .map_err(|_| AppError::bad_request("invalid experience query"))?
            .0;
        if let Some(selected) = query.get("instance") {
            if id.as_ref().is_some_and(|id| id != selected) {
                return Err(AppError::bad_request("conflicting instance context"));
            }
            id = Some(selected.clone());
        }
        let pack_snapshot = path == "/api/v1/resource-pack.zip"
            && query.get("revision").is_some_and(|r| r.len() == 40 && r.chars().all(|c| c.is_ascii_hexdigit()));
        if id.is_none() && pack_snapshot {
            // Versioned resource packs are immutable public artifacts. Older game
            // integrations download them without custom headers; locate by checksum.
            let raw: Option<String> = sqlx::query_scalar("SELECT value FROM kv WHERE key=?")
                .bind(format!("pack_location:{}", query["revision"]))
                .fetch_optional(&platform.platform_db)
                .await?;
            id = raw.and_then(|v| serde_json::from_str(&v).ok());
        }
        if path.starts_with("/api/map/") && platform.cfg.data_dir.join("panel.db").is_file() {
            if let Some(server) = path.split('/').nth(3).and_then(|s| s.parse::<i64>().ok()) {
                id = sqlx::query_scalar("SELECT instance_id FROM game_servers WHERE id=?")
                    .bind(server)
                    .fetch_optional(&platform.platform_db)
                    .await?;
            }
        }
        if path.starts_with("/api/experience-map/") {
            id = path.split('/').nth(3).map(str::to_owned);
        }
        if path.starts_with("/api/server/") {
            // Server credentials, rather than client-supplied headers, determine ownership.
            let token = req.headers().get("authorization").and_then(|v| v.to_str().ok()).and_then(|v| v.strip_prefix("Bearer "));
            if let Some(token) = token {
                use sha2::{Digest, Sha256};
                let hash = hex::encode(Sha256::digest(token.trim().as_bytes()));
                // Platform server records are ownership locators; scoped records hold runtime state.
                let owner: Option<String> = sqlx::query_scalar("SELECT instance_id FROM game_servers WHERE token_hash=?")
                    .bind(hash)
                    .fetch_optional(&platform.platform_db)
                    .await?;
                if let Some(owner) = owner.filter(|s| !s.is_empty()) {
                    if id.as_ref().is_some_and(|id| *id != owner) {
                        return Err(AppError::forbidden("server belongs to another instance"));
                    }
                    if platform.cfg.data_dir.join("panel.db").is_file() {
                        id = Some(owner);
                    }
                }
            }
        }
        if id.is_none() && platform.cfg.data_dir.join("panel.db").is_file() {
            let instances = store::list_instances(&platform).await?;
            if instances.len() == 1 {
                id = Some(instances[0].id.clone());
            } else {
                return Err(AppError::bad_request("select an instance using X-Velora-Instance"));
            }
        }
        if let Some(id) = id {
            let instance = store::get_instance(&platform, &id).await?;
            if !path.starts_with("/api/server/")
                && !path.starts_with("/api/experience-map/")
                && !path.starts_with("/api/map/")
                && !pack_snapshot
            {
                let token = req.headers().get("authorization").and_then(|v| v.to_str().ok()).and_then(|v| v.strip_prefix("Bearer "));
                let user = if let Some(token) = token { Some(auth::authenticate(&platform, token).await?) } else { None };
                let groups = match &user {
                    Some(u) => auth::user_groups(&platform, u.id).await?,
                    None => vec![],
                };
                if !instance.visible_to(user.as_ref(), &groups) && !user.as_ref().is_some_and(|u| u.is_admin()) {
                    return Err(AppError::not_found("instance not found"));
                }
            }
            let experience: Experience = serde_json::from_str(&instance.experience)?;
            if !instance.enabled && !path.starts_with("/api/admin/") {
                return Err(AppError::forbidden("this experience is disabled"));
            }
            if let Some(policy) = crate::velora_core::policy(&experience)? {
                if let Some(module) = crate::velora_core::module_for_path(&path) {
                    if !policy.enabled(module) {
                        return Err(AppError::forbidden("this Velora Core module is disabled"));
                    }
                }
            }
            if let Some(feature) = feature_for_path(&path) {
                let core_transport = experience.kind == crate::velora_core::KIND && path == "/api/server/v1/companion";
                if !core_transport && !experience.enabled(feature) {
                    return Err(AppError::forbidden("this feature is disabled for the instance"));
                }
            }
            let state = platform.experiences.state(&platform, &id).await?;
            req.extensions_mut().insert(state);
        }
        Ok(())
    }
    .await;
    match result {
        Ok(()) => next.run(req).await,
        Err(e) => e.into_response(),
    }
}

pub async fn configure(
    _: auth::AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(mut experience): Json<Experience>,
) -> AppResult<Json<Experience>> {
    let instance = store::get_instance(&state, &id).await?;
    if let Some(policy) = crate::velora_core::policy(&experience)? {
        if instance.mc_version != "1.20.1" || instance.loader != "fabric" {
            return Err(AppError::bad_request("Velora SMP requires Fabric 1.20.1"));
        }
        experience.features = policy.features();
    }
    if experience.features.iter().any(|f| !EXPERIENCE_FEATURES.contains(&f.as_str())) {
        return Err(AppError::bad_request("unknown experience capability"));
    }
    if experience.navigation.len() > 32 || experience.widgets.len() > 32 || experience.kind.len() > 48 {
        return Err(AppError::bad_request("experience configuration exceeds limits"));
    }
    let mut ids = std::collections::HashSet::new();
    for widget in &experience.widgets {
        if widget.id.is_empty() || widget.id.len() > 64 || !ids.insert(&widget.id) || !widget.config.is_object() {
            return Err(AppError::bad_request("widgets need unique, nonempty IDs up to 64 characters and an object configuration"));
        }
    }
    ids.clear();
    for page in &experience.navigation {
        if page.id.is_empty() || !ids.insert(&page.id) || page.label.len() > 100 {
            return Err(AppError::bad_request("navigation needs unique IDs and labels up to 100 characters"));
        }
    }
    if experience.branding.as_ref().is_some_and(|b| b.name.trim().is_empty()) {
        return Err(AppError::bad_request("instance branding needs a name"));
    }
    state.experiences.state(&state, &id).await?;
    sqlx::query("UPDATE instances SET experience = ?, revision=revision+1, updated_at=? WHERE id=?")
        .bind(serde_json::to_string(&experience)?)
        .bind(crate::db::now())
        .bind(&id)
        .execute(&state.platform_db)
        .await?;
    Ok(Json(experience))
}

/// A trusted game integration can publish its own module's live overview data.
/// Module state stays in the experience DB, separate from presentation configuration.
pub async fn publish_module(
    crate::routes::servers::GameServer(_): crate::routes::servers::GameServer,
    crate::state::RequestState(state): crate::state::RequestState,
    axum::extract::Path(module): axum::extract::Path<String>,
    Json(value): Json<serde_json::Value>,
) -> AppResult<Json<serde_json::Value>> {
    let id = state.instance_id.as_deref().ok_or_else(|| AppError::bad_request("server must be assigned to an instance"))?;
    let row = store::get_instance(&state, id).await?;
    let experience: Experience = serde_json::from_str(&row.experience)?;
    if !experience.modules.contains_key(&module) {
        return Err(AppError::forbidden("module is not registered for this instance"));
    }
    if !value.is_object() || value.to_string().len() > 32 * 1024 {
        return Err(AppError::bad_request("module snapshots must be objects up to 32 KB"));
    }
    store::kv_set(&state, &format!("experience_module:{module}"), &value).await?;
    Ok(Json(serde_json::json!({"ok":true})))
}

/// Processes are restarted once per experience, never shared across gameplay stores.
fn start_workers(state: AppState) {
    let map = state.worldmap.clone();
    tokio::task::spawn_blocking(move || map.backfill_pyramids());
    let initial = state.clone();
    tokio::spawn(async move {
        if let Err(e) = crate::routes::leveling::refresh_global_titles(&initial).await {
            tracing::warn!(instance = ?initial.instance_id, "initial title refresh failed: {}", e.message);
        }
    });
    crate::routes::discord::spawn_worker(state.clone());
    crate::scheduler::spawn(state.clone());
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            if state.db.is_closed() {
                return;
            }
            if let Err(e) = crate::rewards::process_queue(&state).await {
                tracing::warn!(instance = ?state.instance_id, "reward queue failed: {}", e.message);
            }
        }
    });
}

#[cfg(test)]
mod platform_path_tests {
    #[test]
    fn minecraft_textures_need_no_instance_selection() {
        for path in ["/api/v1/mc/status", "/api/v1/mc/item/diamond_sword.png", "/api/v1/mc/gui/hud/heart/full.png", "/api/admin/mc-textures", "/api/admin/mc-textures/upload"] {
            assert!(super::platform_path(path), "{path}");
        }
    }

    /// Velora Core releases belong to the platform, not to any one instance: deployment tools such as the Calagopus
    /// extension call these without an instance header, and the Panel serves several instances.
    #[test]
    fn velora_core_releases_need_no_instance_selection() {
        for path in [
            "/api/v1/core/latest",
            "/api/v1/core/files/abababababababababababababababababababababababababababababababab/velora-core-server-1.20.1-0.6.0.jar",
            "/api/admin/core/releases",
            "/api/admin/core/releases/velora-core-v0.6.0/approve",
        ] {
            assert!(super::platform_path(path), "{path}");
        }
        assert!(!super::platform_path("/api/v1/corefoo/latest"), "only the exact core segment is platform-level");
    }
}
