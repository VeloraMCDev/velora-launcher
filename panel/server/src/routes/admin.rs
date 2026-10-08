//! Admin API used by the panel web UI.

use crate::store::InstancePresentation;
use crate::auth::account::{context, host_error};
use crate::auth::{self, AdminUser, UserRow};
use crate::error::{AppError, AppResult};
use crate::packs::{self, Fallback};
use crate::state::AppState;
use crate::state::RequestState as State;
use crate::store::{self, AdminInstance, Settings};
pub use authority::{Group, UserInput};
use axum::extract::{Multipart, Path, Query};
use axum::Json;
use scopenet_shared::{Branding, Loader, MemoryDefaults, ServerEntry};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use velora_auth_http::admin as authority;
use velora_platform_utils::paths::{safe_join, sanitize_id};

// ---------------------------------------------------------------------------
// Dashboard
// ---------------------------------------------------------------------------

pub async fn stats(_: AdminUser, State(state): State<AppState>) -> AppResult<Json<Value>> {
    let (users, pending) = authority::counts(&state.db).await.map_err(host_error)?;
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
    let users = authority::list_users(&state.db).await.map_err(host_error)?;
    let mut out = Vec::with_capacity(users.len());
    for u in users {
        out.push(view(&state, u).await?);
    }
    Ok(Json(out))
}

pub async fn create_user(_: AdminUser, State(state): State<AppState>, Json(input): Json<UserInput>) -> AppResult<Json<AdminUserView>> {
    let user = authority::create_user(&context(&state), input).await.map_err(host_error)?;
    Ok(Json(view(&state, user).await?))
}

pub async fn update_user(
    AdminUser(me): AdminUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(input): Json<UserInput>,
) -> AppResult<Json<AdminUserView>> {
    let user = authority::update_user(&context(&state), &me, id, input).await.map_err(host_error)?;
    Ok(Json(view(&state, user).await?))
}

pub async fn delete_user(AdminUser(me): AdminUser, State(state): State<AppState>, Path(id): Path<i64>) -> AppResult<Json<Value>> {
    let user = authority::deletion_target(&state.db, &me, id).await.map_err(host_error)?;
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

pub async fn list_groups(_: AdminUser, State(state): State<AppState>) -> AppResult<Json<Vec<Group>>> {
    Ok(Json(authority::list_groups(&state.db).await.map_err(host_error)?))
}
pub async fn create_group(_: AdminUser, State(state): State<AppState>, Json(group): Json<Group>) -> AppResult<Json<Value>> {
    authority::create_group(&state.db, group).await.map_err(host_error)?;
    Ok(Json(json!({"ok": true})))
}
pub async fn delete_group(_: AdminUser, State(state): State<AppState>, Path(id): Path<i64>) -> AppResult<Json<Value>> {
    authority::delete_group(&state.db, id).await.map_err(host_error)?;
    Ok(Json(json!({"ok": true})))
}
// ---------------------------------------------------------------------------
// Branding & settings
// ---------------------------------------------------------------------------

pub async fn get_branding(_: AdminUser, State(state): State<AppState>) -> AppResult<Json<Branding>> {
    Ok(Json(store::branding(&state).await?))
}

pub async fn put_branding(_: AdminUser, State(state): State<AppState>, Json(b): Json<Branding>) -> AppResult<Json<Branding>> {
    velora_panel_settings::validate_branding_name(&b.name).map_err(|e| AppError::bad_request(e.0))?;
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
    s = velora_panel_settings::normalize(s, old.curseforge_api_key).map_err(|e| AppError::bad_request(e.0))?;
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
        match velora_platform_utils::meta::resolve_loader_version(&state.http, input.loader, &input.mc_version, requested.as_deref()).await
        {
            Ok(v) => input.loader_version = v,
            Err(e) => tracing::warn!("couldn't resolve loader version: {e:#}"),
        }
    }
    Ok(input)
}

fn instance_write(input: &InstanceInput) -> AppResult<velora_panel_instances::mutations::InstanceWrite> {
    let mem = input.memory.unwrap_or_default();
    Ok(velora_panel_instances::mutations::InstanceWrite {
        name: input.name.clone(), description: input.description.clone(), icon_url: input.icon_url.clone(),
        banner_url: input.banner_url.clone(), logo_url: input.logo_url.clone(), mc_version: input.mc_version.clone(),
        loader: input.loader.as_str().into(), loader_version: input.loader_version.clone(),
        visibility: input.visibility.clone().unwrap_or_else(|| "public".into()),
        allowed_groups: serde_json::to_string(&input.allowed_groups)?, memory_min: mem.min_mb as i64, memory_max: mem.max_mb as i64,
        jvm_args: input.jvm_args.clone(), server: input.server.as_ref().map(serde_json::to_string).transpose()?,
        featured: input.featured, enabled: input.enabled.unwrap_or(true), sort: input.sort,
    })
}

pub async fn create_instance(
    _: AdminUser,
    State(state): State<AppState>,
    Json(input): Json<InstanceInput>,
) -> AppResult<Json<AdminInstance>> {
    let input = validated(&state, input).await?;
    let id = store::unique_slug(&state, &input.name).await?;
    let now = crate::db::now();
    velora_panel_instances::mutations::create_instance(&state.db, &id, &instance_write(&input)?, &now).await?;
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
    // Modpack instances keep the versions the pack declared unless the
    // admin explicitly changes them here.
    let label = if existing.source_kind == "vanilla" { format!("Minecraft {}", input.mc_version) } else { existing.source_label.clone() };
    velora_panel_instances::mutations::update_instance(&state.db, &id, &instance_write(&input)?, &label, &crate::db::now()).await?;
    detail(&state, id).await
}

pub async fn delete_instance(_: AdminUser, State(state): State<AppState>, Path(id): Path<String>) -> AppResult<Json<Value>> {
    store::get_instance(&state, &id).await?;
    velora_panel_instances::mutations::delete_instance(&state.platform_db, &id).await?;
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
    velora_panel_instances::mutations::clean_update(&state.db, &id, &crate::db::now()).await?;
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
            velora_panel_instances::mutations::set_icon(&state.db, &row.id, &icon).await?;
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
            velora_platform_utils::meta::resolve_loader_version(&state.http, info.loader, &info.mc_version, info.loader_version.as_deref())
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
                let sha1 = velora_platform_utils::http::sha1_bytes(&data);
                // Uploading a file with the same name as a "missing" CurseForge
                // entry replaces it.
                velora_panel_instances::mutations::record_upload(&state.db, &id, velora_panel_instances::mutations::UploadedFile {
                    path: &rel, filename: &name, url: &packs::files_url(&id, &rel), sha1: &sha1, size: data.len() as i64,
                }).await?;
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
    velora_panel_instances::mutations::delete_file(&state.db, &id, &q.path).await?;
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

pub use cosmetics::{CapeUpdate, CapeView};
use velora_auth_http::cosmetics;

pub async fn admin_set_skin(
    _: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    mut form: Multipart,
) -> AppResult<Json<AdminUserView>> {
    let (bytes, model) = crate::routes::account::read_texture_form(&mut form).await?;
    let user = cosmetics::set_skin(&crate::yggdrasil::context(&state), id, &bytes, &model).await.map_err(host_error)?;
    Ok(Json(view(&state, user).await?))
}
pub async fn admin_delete_skin(_: AdminUser, State(state): State<AppState>, Path(id): Path<i64>) -> AppResult<Json<AdminUserView>> {
    let user = cosmetics::delete_skin(&crate::yggdrasil::context(&state), id).await.map_err(host_error)?;
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
    let user = cosmetics::set_model(&crate::yggdrasil::context(&state), id, &input.model).await.map_err(host_error)?;
    Ok(Json(view(&state, user).await?))
}
#[derive(Deserialize)]
pub struct AdminCapeInput {
    cape_id: Option<i64>,
}
pub async fn admin_set_cape(
    _: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(input): Json<AdminCapeInput>,
) -> AppResult<Json<AdminUserView>> {
    let user = cosmetics::set_cape(&crate::yggdrasil::context(&state), id, input.cape_id).await.map_err(host_error)?;
    Ok(Json(view(&state, user).await?))
}
pub async fn list_capes(_: AdminUser, State(state): State<AppState>) -> AppResult<Json<Vec<CapeView>>> {
    Ok(Json(cosmetics::list_capes(&crate::yggdrasil::context(&state)).await.map_err(host_error)?))
}
/// Multipart extraction stays with the host's existing request/body-limit handling.
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
    Ok(Json(cosmetics::create_cape(&crate::yggdrasil::context(&state), name, visibility, groups, file).await.map_err(host_error)?))
}
pub async fn update_cape(
    _: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(input): Json<CapeUpdate>,
) -> AppResult<Json<Vec<CapeView>>> {
    Ok(Json(cosmetics::update_cape(&crate::yggdrasil::context(&state), id, input).await.map_err(host_error)?))
}
pub async fn delete_cape(_: AdminUser, State(state): State<AppState>, Path(id): Path<i64>) -> AppResult<Json<Vec<CapeView>>> {
    Ok(Json(cosmetics::delete_cape(&crate::yggdrasil::context(&state), id).await.map_err(host_error)?))
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
