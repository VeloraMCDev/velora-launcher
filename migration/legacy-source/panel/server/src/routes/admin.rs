//! Admin API used by the panel web UI.

use crate::auth::{self, AdminUser, UserRow};
use crate::error::{AppError, AppResult};
use crate::packs::{self, Fallback};
use crate::state::AppState;
use crate::state::RequestState as State;
use crate::store::{self, AdminInstance, Settings};
use axum::extract::{Multipart, Path, Query};
use axum::Json;
use scopenet_core::paths::{safe_join, sanitize_id};
use scopenet_shared::{valid_username, Branding, Loader, MemoryDefaults, ServerEntry};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

// ---------------------------------------------------------------------------
// Dashboard
// ---------------------------------------------------------------------------

pub async fn stats(_: AdminUser, State(state): State<AppState>) -> AppResult<Json<Value>> {
    let users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users").fetch_one(&state.db).await?;
    let pending: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE status = 'pending'").fetch_one(&state.db).await?;
    let instances: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM instances").fetch_one(&state.db).await?;
    let since = (chrono::Utc::now() - chrono::Duration::days(13)).format("%Y-%m-%d").to_string();
    let daily: Vec<(String, i64)> = sqlx::query_as(
        "SELECT substr(created_at, 1, 10) AS day, COUNT(*) FROM events WHERE kind = 'launch' AND created_at >= ? GROUP BY day ORDER BY day",
    )
    .bind(&since)
    .fetch_all(&state.db)
    .await?;
    let top: Vec<(Option<String>, i64)> = sqlx::query_as(
        "SELECT instance_id, COUNT(*) AS c FROM events WHERE kind = 'launch' AND created_at >= ? GROUP BY instance_id ORDER BY c DESC LIMIT 5",
    )
    .bind(&since)
    .fetch_all(&state.db)
    .await?;
    let recent: Vec<(Option<String>, Option<String>, String)> =
        sqlx::query_as("SELECT instance_id, username, created_at FROM events ORDER BY id DESC LIMIT 12").fetch_all(&state.db).await?;
    // Anyone who launched the game or played on a connected server.
    let week = (chrono::Utc::now() - chrono::Duration::days(7)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let players_7d: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM (SELECT lower(username) FROM events WHERE username IS NOT NULL AND created_at >= ?1
         UNION SELECT lower(name) FROM player_stats WHERE last_seen >= ?1)",
    )
    .bind(&week)
    .fetch_one(&state.db)
    .await?;

    // Fill in empty days so the chart is continuous.
    let mut days = Vec::new();
    for i in (0..14).rev() {
        let d = (chrono::Utc::now() - chrono::Duration::days(i)).format("%Y-%m-%d").to_string();
        let count = daily.iter().find(|(day, _)| *day == d).map(|(_, c)| *c).unwrap_or(0);
        days.push(json!({ "day": d, "launches": count }));
    }
    let settings = store::settings(&state).await?;
    Ok(Json(json!({
        "users": users,
        "pending": pending,
        "instances": instances,
        "players_7d": players_7d,
        "launches": days,
        "top_instances": top.into_iter().map(|(id, c)| json!({"instance_id": id, "launches": c})).collect::<Vec<_>>(),
        "recent": recent.into_iter().map(|(i, u, t)| json!({"instance_id": i, "username": u, "at": t})).collect::<Vec<_>>(),
        "launcher_download_url": settings.launcher_download_url,
        "live": crate::routes::servers::live_summary(&state).await?,
        "version": env!("CARGO_PKG_VERSION"),
    })))
}

// ---------------------------------------------------------------------------
// Users & groups
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct AdminUserView {
    #[serde(flatten)]
    user: UserRow,
    groups: Vec<String>,
    /// Panel-relative texture URL.
    skin_url: Option<String>,
    /// Across all game servers reporting to the panel.
    playtime_secs: i64,
    last_seen_ingame: Option<String>,
}

async fn view(state: &AppState, user: UserRow) -> AppResult<AdminUserView> {
    let groups = auth::user_groups(state, user.id).await?;
    let mut playtime = 0;
    let mut last_seen: Option<String> = None;
    for scope in state.experiences.reporting_states(state).await? {
        let (seconds, seen): (Option<i64>, Option<String>) =
            sqlx::query_as("SELECT SUM(playtime_secs), MAX(last_seen) FROM player_stats WHERE uuid = ?")
                .bind(&user.uuid)
                .fetch_one(&scope.db)
                .await?;
        playtime += seconds.unwrap_or(0);
        last_seen = last_seen.max(seen);
    }
    Ok(AdminUserView {
        skin_url: user.skin_hash.as_deref().map(|h| format!("/textures/{h}")),
        playtime_secs: playtime,
        last_seen_ingame: last_seen,
        groups,
        user,
    })
}

pub async fn list_users(_: AdminUser, State(state): State<AppState>) -> AppResult<Json<Vec<AdminUserView>>> {
    let users: Vec<UserRow> =
        sqlx::query_as("SELECT * FROM users ORDER BY status = 'pending' DESC, username COLLATE NOCASE").fetch_all(&state.db).await?;
    let mut out = Vec::with_capacity(users.len());
    for u in users {
        out.push(view(&state, u).await?);
    }
    Ok(Json(out))
}

#[derive(Deserialize)]
pub struct UserInput {
    username: Option<String>,
    password: Option<String>,
    email: Option<String>,
    role: Option<String>,
    status: Option<String>,
    status_reason: Option<String>,
    groups: Option<Vec<String>>,
}

fn check_role(role: &str) -> AppResult<()> {
    if !matches!(role, "admin" | "player") {
        return Err(AppError::bad_request("role must be admin or player"));
    }
    Ok(())
}

fn check_status(status: &str) -> AppResult<()> {
    if !matches!(status, "active" | "pending" | "disabled") {
        return Err(AppError::bad_request("status must be active, pending or disabled"));
    }
    Ok(())
}

async fn set_groups(state: &AppState, user_id: i64, groups: &[String]) -> AppResult<()> {
    sqlx::query("DELETE FROM user_groups WHERE user_id = ?").bind(user_id).execute(&state.db).await?;
    for g in groups {
        sqlx::query("INSERT OR IGNORE INTO user_groups (user_id, group_id) SELECT ?, id FROM groups WHERE name = ?")
            .bind(user_id)
            .bind(g)
            .execute(&state.db)
            .await?;
    }
    Ok(())
}

pub async fn create_user(_: AdminUser, State(state): State<AppState>, Json(input): Json<UserInput>) -> AppResult<Json<AdminUserView>> {
    let username = input.username.as_deref().map(str::trim).unwrap_or_default();
    if !valid_username(username) {
        return Err(AppError::bad_request("usernames are 3-16 letters, numbers or underscores"));
    }
    let password = input.password.unwrap_or_default();
    auth::validate_password(&password)?;
    let role = input.role.unwrap_or_else(|| "player".into());
    check_role(&role)?;
    let status = input.status.unwrap_or_else(|| "active".into());
    check_status(&status)?;
    let id = auth::create_user(&state, username, &password, input.email.as_deref(), &role, &status).await?;
    if let Some(groups) = input.groups {
        set_groups(&state, id, &groups).await?;
    }
    let user = sqlx::query_as("SELECT * FROM users WHERE id = ?").bind(id).fetch_one(&state.db).await?;
    Ok(Json(view(&state, user).await?))
}

pub async fn update_user(
    AdminUser(me): AdminUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(input): Json<UserInput>,
) -> AppResult<Json<AdminUserView>> {
    let user: UserRow = sqlx::query_as("SELECT * FROM users WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::not_found("user not found"))?;
    if let Some(password) = input.password.filter(|p| !p.is_empty()) {
        auth::validate_password(&password)?;
        let mut tx = state.db.begin().await?;
        sqlx::query("UPDATE users SET password_hash = ?, auth_version = auth_version + 1 WHERE id = ?")
            .bind(auth::hash_password(&password)?)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM ygg_tokens WHERE user_id=?").bind(id).execute(&mut *tx).await?;
        sqlx::query("DELETE FROM ygg_sessions WHERE user_id=?").bind(id).execute(&mut *tx).await?;
        tx.commit().await?;
    }
    if let Some(email) = input.email {
        sqlx::query("UPDATE users SET email = ? WHERE id = ?")
            .bind(Some(email.trim()).filter(|e| !e.is_empty()))
            .bind(id)
            .execute(&state.db)
            .await?;
    }
    if let Some(role) = input.role {
        check_role(&role)?;
        if me.id == id && role != "admin" {
            return Err(AppError::bad_request("you can't remove your own admin role"));
        }
        sqlx::query("UPDATE users SET role = ? WHERE id = ?").bind(role).bind(id).execute(&state.db).await?;
    }
    if let Some(reason) = input.status_reason {
        sqlx::query("UPDATE users SET status_reason = ? WHERE id = ?")
            .bind(Some(reason.trim()).filter(|r| !r.is_empty()))
            .bind(id)
            .execute(&state.db)
            .await?;
    }
    if let Some(status) = input.status {
        check_status(&status)?;
        if me.id == id && status != "active" {
            return Err(AppError::bad_request("you can't disable your own account"));
        }
        sqlx::query("UPDATE users SET status = ? WHERE id = ?").bind(status).bind(id).execute(&state.db).await?;
    }
    if let Some(groups) = input.groups {
        set_groups(&state, user.id, &groups).await?;
    }
    let user = sqlx::query_as("SELECT * FROM users WHERE id = ?").bind(id).fetch_one(&state.db).await?;
    Ok(Json(view(&state, user).await?))
}

pub async fn delete_user(AdminUser(me): AdminUser, State(state): State<AppState>, Path(id): Path<i64>) -> AppResult<Json<Value>> {
    if me.id == id {
        return Err(AppError::bad_request("you can't delete your own account"));
    }
    let user: UserRow = sqlx::query_as("SELECT * FROM users WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::not_found("player not found"))?;
    // Removes the account and everything tied to its UUID, not just the login.
    let report = crate::purge::purge_user(&state, &user).await?;
    crate::routes::activity::record(
        &state,
        &me,
        "panel",
        "account_deleted",
        Some(&format!("{} ({} records removed)", user.username, report.total())),
    )
    .await?;
    Ok(Json(json!({ "ok": true, "report": report })))
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct Group {
    #[serde(default)]
    id: i64,
    name: String,
    #[serde(default = "default_color")]
    color: String,
    #[serde(default)]
    #[sqlx(default)]
    members: i64,
    /// LuckPerms group this panel group is mapped to (empty: none).
    #[serde(default)]
    #[sqlx(default)]
    luckperms_group: String,
    #[serde(default)]
    #[sqlx(default)]
    discord_role: String,
}
fn default_color() -> String {
    "#7c5cff".into()
}

pub async fn list_groups(_: AdminUser, State(state): State<AppState>) -> AppResult<Json<Vec<Group>>> {
    Ok(Json(
        sqlx::query_as(
            "SELECT g.id, g.name, g.color, (SELECT COUNT(*) FROM user_groups ug WHERE ug.group_id = g.id) AS members, g.luckperms_group, g.discord_role FROM groups g ORDER BY g.name",
        )
        .fetch_all(&state.db)
        .await?,
    ))
}

pub async fn create_group(_: AdminUser, State(state): State<AppState>, Json(g): Json<Group>) -> AppResult<Json<Value>> {
    let name = g.name.trim();
    if name.is_empty() || name.len() > 32 {
        return Err(AppError::bad_request("group names are 1-32 characters"));
    }
    sqlx::query("INSERT INTO groups (name, color) VALUES (?, ?)")
        .bind(name)
        .bind(&g.color)
        .execute(&state.db)
        .await
        .map_err(|_| AppError::conflict("a group with that name exists"))?;
    Ok(Json(json!({ "ok": true })))
}

pub async fn delete_group(_: AdminUser, State(state): State<AppState>, Path(id): Path<i64>) -> AppResult<Json<Value>> {
    sqlx::query("DELETE FROM groups WHERE id = ?").bind(id).execute(&state.db).await?;
    Ok(Json(json!({ "ok": true })))
}

// ---------------------------------------------------------------------------
// Branding & settings
// ---------------------------------------------------------------------------

pub async fn get_branding(_: AdminUser, State(state): State<AppState>) -> AppResult<Json<Branding>> {
    Ok(Json(store::branding(&state).await?))
}

pub async fn put_branding(_: AdminUser, State(state): State<AppState>, Json(b): Json<Branding>) -> AppResult<Json<Branding>> {
    if b.name.trim().is_empty() {
        return Err(AppError::bad_request("the launcher needs a name"));
    }
    store::kv_set(&state, "branding", &b).await?;
    Ok(Json(b))
}

#[derive(Serialize)]
pub struct SettingsView {
    #[serde(flatten)]
    settings: Settings,
    curseforge_key_set: bool,
    curseforge_key_from_env: bool,
}

pub async fn get_settings(_: AdminUser, State(state): State<AppState>) -> AppResult<Json<SettingsView>> {
    settings_view(&state).await
}

async fn settings_view(state: &AppState) -> AppResult<Json<SettingsView>> {
    let mut s = store::settings(state).await?;
    let set = s.curseforge_api_key.as_deref().is_some_and(|k| !k.is_empty());
    s.curseforge_api_key = None; // never send secrets back to the browser
    Ok(Json(SettingsView { settings: s, curseforge_key_set: set, curseforge_key_from_env: state.cfg.curseforge_api_key.is_some() }))
}

pub async fn put_settings(_: AdminUser, State(state): State<AppState>, Json(mut s): Json<Settings>) -> AppResult<Json<SettingsView>> {
    let old = store::settings(&state).await?;
    // An empty key means "keep the current one"; "-" clears it.
    s.curseforge_api_key = match s.curseforge_api_key.as_deref().map(str::trim) {
        None | Some("") => old.curseforge_api_key,
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
        return Err(AppError::bad_request(
            "blacklist entries must be 3–16 letters, numbers or underscores; use *word* to match within names",
        ));
    }
    if let Some(u) = &s.public_url {
        if !u.starts_with("http://") && !u.starts_with("https://") {
            return Err(AppError::bad_request("the public URL must start with https:// (or http://)"));
        }
    }
    s.auth.yggdrasil_url = None; // derived, never stored
    store::kv_set(&state, "settings", &s).await?;
    settings_view(&state).await
}

// ---------------------------------------------------------------------------
// Instances
// ---------------------------------------------------------------------------

pub async fn list_instances(_: AdminUser, State(state): State<AppState>) -> AppResult<Json<Vec<AdminInstance>>> {
    let mut out = Vec::new();
    for row in store::list_instances(&state).await? {
        let stats = store::file_stats(&state, &row.id).await?;
        out.push(row.to_admin(stats));
    }
    Ok(Json(out))
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct InstanceInput {
    name: String,
    description: String,
    icon_url: Option<String>,
    banner_url: Option<String>,
    logo_url: Option<String>,
    mc_version: String,
    loader: Loader,
    loader_version: Option<String>,
    visibility: Option<String>,
    allowed_groups: Vec<String>,
    memory: Option<MemoryDefaults>,
    jvm_args: String,
    server: Option<ServerEntry>,
    featured: bool,
    enabled: Option<bool>,
    sort: i64,
}

async fn validated(state: &AppState, mut input: InstanceInput) -> AppResult<InstanceInput> {
    input.name = input.name.trim().to_string();
    if input.name.is_empty() {
        return Err(AppError::bad_request("give the instance a name"));
    }
    if input.mc_version.trim().is_empty() {
        return Err(AppError::bad_request("pick a Minecraft version"));
    }
    let vis = input.visibility.clone().unwrap_or_else(|| "public".into());
    if !matches!(vis.as_str(), "public" | "members" | "groups") {
        return Err(AppError::bad_request("visibility must be public, members or groups"));
    }
    input.visibility = Some(vis);
    let mem = input.memory.unwrap_or_default();
    if mem.max_mb < 512 || mem.min_mb > mem.max_mb {
        return Err(AppError::bad_request("memory: max must be at least 512 MB and not below min"));
    }
    // Pin "latest" to a concrete loader version when we can reach the meta
    // servers; otherwise the launcher resolves it at install time.
    if input.loader == Loader::Vanilla {
        input.loader_version = None;
    } else {
        let requested = input.loader_version.clone();
        match scopenet_core::meta::resolve_loader_version(&state.http, input.loader, &input.mc_version, requested.as_deref()).await {
            Ok(v) => input.loader_version = v,
            Err(e) => tracing::warn!("couldn't resolve loader version: {e:#}"),
        }
    }
    Ok(input)
}

pub async fn create_instance(
    _: AdminUser,
    State(state): State<AppState>,
    Json(input): Json<InstanceInput>,
) -> AppResult<Json<AdminInstance>> {
    let input = validated(&state, input).await?;
    let id = store::unique_slug(&state, &input.name).await?;
    let now = crate::db::now();
    let mem = input.memory.unwrap_or_default();
    sqlx::query(
        "INSERT INTO instances (id, name, description, icon_url, banner_url, logo_url, mc_version, loader, loader_version, source_kind, source_label,
         visibility, allowed_groups, memory_min, memory_max, jvm_args, server, featured, enabled, sort, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'vanilla', ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&input.name)
    .bind(&input.description)
    .bind(&input.icon_url)
    .bind(&input.banner_url)
    .bind(&input.logo_url)
    .bind(&input.mc_version)
    .bind(input.loader.as_str())
    .bind(&input.loader_version)
    .bind(format!("Minecraft {}", input.mc_version))
    .bind(input.visibility.as_deref().unwrap_or("public"))
    .bind(serde_json::to_string(&input.allowed_groups)?)
    .bind(mem.min_mb as i64)
    .bind(mem.max_mb as i64)
    .bind(&input.jvm_args)
    .bind(input.server.as_ref().map(serde_json::to_string).transpose()?)
    .bind(input.featured)
    .bind(input.enabled.unwrap_or(true))
    .bind(input.sort)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await?;
    detail(&state, id).await.map(|Json(v)| Json(v.instance))
}

#[derive(Serialize)]
pub struct InstanceDetail {
    instance: AdminInstance,
    files: Vec<store::FileRow>,
}

pub async fn get_instance(_: AdminUser, State(state): State<AppState>, Path(id): Path<String>) -> AppResult<Json<InstanceDetail>> {
    detail(&state, id).await
}

async fn detail(state: &AppState, id: String) -> AppResult<Json<InstanceDetail>> {
    let row = store::get_instance(state, &id).await?;
    let stats = store::file_stats(state, &id).await?;
    Ok(Json(InstanceDetail { instance: row.to_admin(stats), files: store::instance_files(state, &id).await? }))
}

pub async fn update_instance(
    _: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<InstanceInput>,
) -> AppResult<Json<InstanceDetail>> {
    let existing = store::get_instance(&state, &id).await?;
    let input = validated(&state, input).await?;
    let mem = input.memory.unwrap_or_default();
    // Modpack instances keep the versions the pack declared unless the
    // admin explicitly changes them here.
    let label = if existing.source_kind == "vanilla" { format!("Minecraft {}", input.mc_version) } else { existing.source_label.clone() };
    sqlx::query(
        "UPDATE instances SET name = ?, description = ?, icon_url = ?, banner_url = ?, logo_url = ?, mc_version = ?, loader = ?, loader_version = ?,
         source_label = ?, visibility = ?, allowed_groups = ?, memory_min = ?, memory_max = ?, jvm_args = ?, server = ?, featured = ?,
         enabled = ?, sort = ?, revision = revision + 1, updated_at = ? WHERE id = ?",
    )
    .bind(&input.name)
    .bind(&input.description)
    .bind(input.icon_url.filter(|s| !s.is_empty()))
    .bind(input.banner_url.filter(|s| !s.is_empty()))
    .bind(input.logo_url.filter(|s| !s.is_empty()))
    .bind(&input.mc_version)
    .bind(input.loader.as_str())
    .bind(&input.loader_version)
    .bind(label)
    .bind(input.visibility.as_deref().unwrap_or("public"))
    .bind(serde_json::to_string(&input.allowed_groups)?)
    .bind(mem.min_mb as i64)
    .bind(mem.max_mb as i64)
    .bind(&input.jvm_args)
    .bind(input.server.as_ref().map(serde_json::to_string).transpose()?)
    .bind(input.featured)
    .bind(input.enabled.unwrap_or(true))
    .bind(input.sort)
    .bind(crate::db::now())
    .bind(&id)
    .execute(&state.db)
    .await?;
    detail(&state, id).await
}

pub async fn delete_instance(_: AdminUser, State(state): State<AppState>, Path(id): Path<String>) -> AppResult<Json<Value>> {
    store::get_instance(&state, &id).await?;
    let mut tx = state.platform_db.begin().await?;
    sqlx::query("DELETE FROM game_servers WHERE instance_id=?").bind(&id).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM instances WHERE id = ?").bind(&id).execute(&mut *tx).await?;
    tx.commit().await?;
    state.experiences.retire(&id).await;
    let dir = state.cfg.files_dir().join(sanitize_id(&id));
    tokio::fs::remove_dir_all(dir).await.ok();
    Ok(Json(json!({ "ok": true })))
}

/// Drop the modpack, keeping only hand-uploaded files.
pub async fn reset_source(_: AdminUser, State(state): State<AppState>, Path(id): Path<String>) -> AppResult<Json<InstanceDetail>> {
    let row = store::get_instance(&state, &id).await?;
    let info = packs::PackInfo {
        mc_version: row.mc_version.clone(),
        loader: Loader::parse(&row.loader).unwrap_or_default(),
        loader_version: row.loader_version.clone(),
        ..Default::default()
    };
    packs::apply(&state, &id, &info, "vanilla", &format!("Minecraft {}", row.mc_version), json!({})).await?;
    detail(&state, id).await
}

/// Make every player's launcher throw away this instance's old files (mods, configs and anything else the panel ever sent, plus the
/// cached server resource pack) and download everything again on their next launch. Worlds, screenshots and options are never touched.
pub async fn clean_update(_: AdminUser, State(state): State<AppState>, Path(id): Path<String>) -> AppResult<Json<InstanceDetail>> {
    store::get_instance(&state, &id).await?;
    sqlx::query("UPDATE instances SET clean_epoch = clean_epoch + 1, revision = revision + 1, updated_at = ? WHERE id = ?")
        .bind(crate::db::now())
        .bind(&id)
        .execute(&state.db)
        .await?;
    detail(&state, id).await
}

#[derive(Deserialize)]
pub struct ModrinthImport {
    version_id: String,
    #[serde(default)]
    project_id: Option<String>,
}

pub async fn import_modrinth(
    _: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<ModrinthImport>,
) -> AppResult<Json<InstanceDetail>> {
    let row = store::get_instance(&state, &id).await?;
    let (bytes, label, icon) = packs::fetch_modrinth(&state, &req.version_id).await?;
    let (info, _) = packs::import_zip(&state, &id, bytes, Fallback::default()).await?;
    packs::apply(&state, &id, &info, "modrinth", &label, json!({ "version_id": req.version_id, "project_id": req.project_id })).await?;
    set_icon_if_missing(&state, &row, icon).await?;
    detail(&state, id).await
}

#[derive(Deserialize)]
pub struct CurseForgeImport {
    mod_id: i64,
    file_id: i64,
}

pub async fn import_curseforge(
    _: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<CurseForgeImport>,
) -> AppResult<Json<InstanceDetail>> {
    let row = store::get_instance(&state, &id).await?;
    let (bytes, label, icon) = packs::fetch_curseforge(&state, req.mod_id, req.file_id).await?;
    let (info, _) = packs::import_zip(&state, &id, bytes, Fallback::default()).await?;
    packs::apply(&state, &id, &info, "curseforge", &label, json!({ "mod_id": req.mod_id, "file_id": req.file_id })).await?;
    set_icon_if_missing(&state, &row, icon).await?;
    detail(&state, id).await
}

async fn set_icon_if_missing(state: &AppState, row: &store::InstanceRow, icon: Option<String>) -> AppResult<()> {
    if row.icon_url.is_none() {
        if let Some(icon) = icon {
            sqlx::query("UPDATE instances SET icon_url = ? WHERE id = ?").bind(icon).bind(&row.id).execute(&state.db).await?;
        }
    }
    Ok(())
}

/// Upload a .mrpack / CurseForge zip / plain instance zip.
pub async fn import_upload(
    _: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    mut form: Multipart,
) -> AppResult<Json<InstanceDetail>> {
    store::get_instance(&state, &id).await?;
    let mut bytes = None;
    let mut filename = String::from("upload.zip");
    let mut fallback = Fallback::default();
    while let Some(field) = form.next_field().await? {
        match field.name().unwrap_or_default() {
            "file" => {
                filename = field.file_name().unwrap_or("upload.zip").to_string();
                bytes = Some(field.bytes().await?.to_vec());
            }
            "mc_version" => fallback.mc_version = Some(field.text().await?),
            "loader" => fallback.loader = Loader::parse(&field.text().await?),
            "loader_version" => fallback.loader_version = Some(field.text().await?),
            _ => {}
        }
    }
    let bytes = bytes.ok_or_else(|| AppError::bad_request("no file uploaded"))?;
    let (mut info, kind) = packs::import_zip(&state, &id, bytes, fallback).await?;
    if kind == "zip" && info.loader != Loader::Vanilla {
        info.loader_version =
            scopenet_core::meta::resolve_loader_version(&state.http, info.loader, &info.mc_version, info.loader_version.as_deref())
                .await
                .ok()
                .flatten();
    }
    let label = match kind {
        "modrinth" | "curseforge" if !info.name.is_empty() => format!("{} {}", info.name, info.version).trim().to_string(),
        _ => format!("Uploaded · {filename}"),
    };
    packs::apply(&state, &id, &info, kind, &label, json!({ "filename": filename })).await?;
    detail(&state, id).await
}

/// Add individual files (extra mods, configs, or a CurseForge mod that
/// can't be downloaded automatically).
pub async fn upload_files(
    _: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    mut form: Multipart,
) -> AppResult<Json<InstanceDetail>> {
    store::get_instance(&state, &id).await?;
    let root = state.cfg.files_dir().join(sanitize_id(&id));
    let mut folder = String::from("mods");
    let mut added = 0;
    while let Some(field) = form.next_field().await? {
        match field.name().unwrap_or_default() {
            "folder" => folder = field.text().await?.trim().trim_matches('/').to_string(),
            "file" => {
                let name = field.file_name().unwrap_or("file").rsplit(['/', '\\']).next().unwrap_or("file").to_string();
                let rel = if folder.is_empty() { name.clone() } else { format!("{folder}/{name}") };
                let dest = safe_join(&root, &rel).ok_or_else(|| AppError::bad_request("invalid path"))?;
                let data = field.bytes().await?;
                tokio::fs::create_dir_all(dest.parent().unwrap()).await?;
                tokio::fs::write(&dest, &data).await?;
                let sha1 = scopenet_core::http::sha1_bytes(&data);
                // Uploading a file with the same name as a "missing" CurseForge
                // entry replaces it.
                sqlx::query("DELETE FROM instance_files WHERE instance_id = ? AND url = '' AND path LIKE ?")
                    .bind(&id)
                    .bind(format!("%/{name}"))
                    .execute(&state.db)
                    .await?;
                sqlx::query(
                    "INSERT INTO instance_files (instance_id, path, url, sha1, size, origin) VALUES (?, ?, ?, ?, ?, 'upload')
                     ON CONFLICT(instance_id, path) DO UPDATE SET url = excluded.url, sha1 = excluded.sha1, size = excluded.size, origin = 'upload', note = NULL",
                )
                .bind(&id)
                .bind(&rel)
                .bind(packs::files_url(&id, &rel))
                .bind(sha1)
                .bind(data.len() as i64)
                .execute(&state.db)
                .await?;
                added += 1;
            }
            _ => {}
        }
    }
    if added == 0 {
        return Err(AppError::bad_request("no files uploaded"));
    }
    store::bump_revision(&state, &id).await?;
    detail(&state, id).await
}

#[derive(Deserialize)]
pub struct PathQuery {
    path: String,
}

pub async fn delete_file(
    _: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<PathQuery>,
) -> AppResult<Json<InstanceDetail>> {
    sqlx::query("DELETE FROM instance_files WHERE instance_id = ? AND path = ?").bind(&id).bind(&q.path).execute(&state.db).await?;
    store::bump_revision(&state, &id).await?;
    packs::gc_files(&state, &id).await?;
    detail(&state, id).await
}

// ---------------------------------------------------------------------------
// Media uploads (logos, backgrounds, icons)
// ---------------------------------------------------------------------------

pub async fn upload_media(_: AdminUser, State(state): State<AppState>, mut form: Multipart) -> AppResult<Json<Value>> {
    while let Some(field) = form.next_field().await? {
        if field.name() != Some("file") {
            continue;
        }
        let name = field.file_name().unwrap_or_default().to_lowercase();
        let ext = name.rsplit('.').next().unwrap_or_default().to_string();
        // SVG is excluded on purpose: it can carry scripts.
        if !matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "gif" | "webp" | "ico" | "mp4" | "webm" | "avif") {
            return Err(AppError::bad_request("supported: png, jpg, gif, webp, avif, ico, mp4, webm"));
        }
        let data = field.bytes().await?;
        let file = format!("{}.{ext}", uuid::Uuid::new_v4().simple());
        tokio::fs::create_dir_all(state.cfg.uploads_dir()).await?;
        tokio::fs::write(state.cfg.uploads_dir().join(&file), &data).await?;
        return Ok(Json(json!({ "url": format!("/uploads/{file}") })));
    }
    Err(AppError::bad_request("no file uploaded"))
}

// ---------------------------------------------------------------------------
// Skins & capes
// ---------------------------------------------------------------------------

pub async fn admin_set_skin(
    _: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    mut form: Multipart,
) -> AppResult<Json<AdminUserView>> {
    let (bytes, model) = crate::routes::account::read_texture_form(&mut form).await?;
    crate::yggdrasil::set_skin(&state, id, &bytes, &model).await?;
    let user = sqlx::query_as("SELECT * FROM users WHERE id = ?").bind(id).fetch_one(&state.db).await?;
    Ok(Json(view(&state, user).await?))
}

pub async fn admin_delete_skin(_: AdminUser, State(state): State<AppState>, Path(id): Path<i64>) -> AppResult<Json<AdminUserView>> {
    sqlx::query("UPDATE users SET skin_hash = NULL WHERE id = ?").bind(id).execute(&state.db).await?;
    let user = sqlx::query_as("SELECT * FROM users WHERE id = ?").bind(id).fetch_one(&state.db).await?;
    Ok(Json(view(&state, user).await?))
}

#[derive(Deserialize)]
pub struct AdminModelInput {
    model: String,
}

pub async fn admin_set_model(
    _: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(input): Json<AdminModelInput>,
) -> AppResult<Json<AdminUserView>> {
    if !matches!(input.model.as_str(), "classic" | "slim") {
        return Err(AppError::bad_request("model must be classic or slim"));
    }
    sqlx::query("UPDATE users SET skin_model = ? WHERE id = ?").bind(&input.model).bind(id).execute(&state.db).await?;
    let user = sqlx::query_as("SELECT * FROM users WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::not_found("user not found"))?;
    Ok(Json(view(&state, user).await?))
}

#[derive(Deserialize)]
pub struct AdminCapeInput {
    cape_id: Option<i64>,
}

/// Admins can give any cape to anyone (including "private" ones).
pub async fn admin_set_cape(
    _: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(input): Json<AdminCapeInput>,
) -> AppResult<Json<AdminUserView>> {
    if let Some(cape) = input.cape_id {
        let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM capes WHERE id = ?").bind(cape).fetch_one(&state.db).await?;
        if exists == 0 {
            return Err(AppError::not_found("cape not found"));
        }
    }
    sqlx::query("UPDATE users SET cape_id = ? WHERE id = ?").bind(input.cape_id).bind(id).execute(&state.db).await?;
    let user = sqlx::query_as("SELECT * FROM users WHERE id = ?").bind(id).fetch_one(&state.db).await?;
    Ok(Json(view(&state, user).await?))
}

#[derive(Serialize)]
pub struct CapeView {
    id: i64,
    name: String,
    url: String,
    visibility: String,
    allowed_groups: Vec<String>,
    wearers: i64,
    created_at: String,
}

async fn cape_views(state: &AppState) -> AppResult<Vec<CapeView>> {
    let rows: Vec<crate::yggdrasil::CapeRow> =
        sqlx::query_as("SELECT * FROM capes ORDER BY name COLLATE NOCASE").fetch_all(&state.db).await?;
    let mut out = Vec::new();
    for c in rows {
        let wearers: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE cape_id = ?").bind(c.id).fetch_one(&state.db).await?;
        out.push(CapeView {
            id: c.id,
            url: format!("/textures/{}", c.hash),
            allowed_groups: serde_json::from_str(&c.allowed_groups).unwrap_or_default(),
            name: c.name,
            visibility: c.visibility,
            wearers,
            created_at: c.created_at,
        });
    }
    Ok(out)
}

pub async fn list_capes(_: AdminUser, State(state): State<AppState>) -> AppResult<Json<Vec<CapeView>>> {
    Ok(Json(cape_views(&state).await?))
}

fn check_visibility(v: &str) -> AppResult<()> {
    if !matches!(v, "public" | "groups" | "private") {
        return Err(AppError::bad_request("visibility must be public, groups or private"));
    }
    Ok(())
}

/// Multipart: `name`, `visibility`, `allowed_groups` (JSON array), `file`.
pub async fn create_cape(_: AdminUser, State(state): State<AppState>, mut form: Multipart) -> AppResult<Json<Vec<CapeView>>> {
    let (mut name, mut visibility, mut groups, mut file) = (String::new(), String::from("public"), String::from("[]"), None);
    while let Some(field) = form.next_field().await? {
        match field.name().unwrap_or_default() {
            "name" => name = field.text().await?.trim().to_string(),
            "visibility" => visibility = field.text().await?,
            "allowed_groups" => groups = field.text().await?,
            "file" => file = Some(field.bytes().await?.to_vec()),
            _ => {}
        }
    }
    if name.is_empty() || name.len() > 40 {
        return Err(AppError::bad_request("give the cape a name (up to 40 characters)"));
    }
    check_visibility(&visibility)?;
    let groups: Vec<String> = serde_json::from_str(&groups).map_err(|_| AppError::bad_request("allowed_groups must be a JSON list"))?;
    let bytes = file.ok_or_else(|| AppError::bad_request("no image uploaded"))?;
    let dir = state.cfg.textures_dir();
    let hash = tokio::task::spawn_blocking(move || crate::textures::store(&dir, crate::textures::Kind::Cape, &bytes))
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))??;
    sqlx::query("INSERT INTO capes (name, hash, visibility, allowed_groups, created_at) VALUES (?, ?, ?, ?, ?)")
        .bind(&name)
        .bind(hash)
        .bind(&visibility)
        .bind(serde_json::to_string(&groups)?)
        .bind(crate::db::now())
        .execute(&state.db)
        .await?;
    Ok(Json(cape_views(&state).await?))
}

#[derive(Deserialize)]
pub struct CapeUpdate {
    name: String,
    visibility: String,
    #[serde(default)]
    allowed_groups: Vec<String>,
}

pub async fn update_cape(
    _: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(input): Json<CapeUpdate>,
) -> AppResult<Json<Vec<CapeView>>> {
    check_visibility(&input.visibility)?;
    if input.name.trim().is_empty() {
        return Err(AppError::bad_request("the cape needs a name"));
    }
    sqlx::query("UPDATE capes SET name = ?, visibility = ?, allowed_groups = ? WHERE id = ?")
        .bind(input.name.trim())
        .bind(&input.visibility)
        .bind(serde_json::to_string(&input.allowed_groups)?)
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(Json(cape_views(&state).await?))
}

pub async fn delete_cape(_: AdminUser, State(state): State<AppState>, Path(id): Path<i64>) -> AppResult<Json<Vec<CapeView>>> {
    sqlx::query("UPDATE users SET cape_id = NULL WHERE cape_id = ?").bind(id).execute(&state.db).await?;
    sqlx::query("DELETE FROM capes WHERE id = ?").bind(id).execute(&state.db).await?;
    Ok(Json(cape_views(&state).await?))
}

/// Auth server details for the Settings page (what to put on game servers).
pub async fn auth_server_info(_: AdminUser, State(state): State<AppState>, headers: axum::http::HeaderMap) -> AppResult<Json<Value>> {
    let base = crate::net::public_base(&state, &headers).await;
    Ok(Json(json!({
        "public_url": base,
        "yggdrasil_url": format!("{base}{}", crate::yggdrasil::ROOT),
        "public_key": state.ygg.public_pem,
    })))
}
