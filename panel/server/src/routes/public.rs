//! Endpoints the launcher talks to.

use crate::store::InstancePresentation;
use crate::auth::{self, AuthUser, MaybeUser};
use crate::error::{AppError, AppResult};
use crate::net::{self, ClientIp};
use crate::state::AppState;
use crate::state::RequestState as State;
use crate::store;
use crate::yggdrasil;
use axum::extract::Path;
use axum::http::HeaderMap;
use axum::Json;
use velora_shared::*;

pub async fn health() -> &'static str {
    "ok"
}

/// Identity and readiness for deployment health checks (never includes secrets).
/// `commit` is stamped at build time; `schema` is the store's migration level.
pub async fn health_detail(axum::extract::State(state): axum::extract::State<AppState>) -> (axum::http::StatusCode, Json<serde_json::Value>) {
    let expected = crate::db::schema_version();
    let schema: Result<i64, _> = sqlx::query_scalar("PRAGMA user_version").fetch_one(&state.platform_db).await;
    let commit = option_env!("VELORA_BUILD_REVISION").unwrap_or("unknown");
    let (code, status, schema) = match schema {
        Ok(v) if v == expected => (axum::http::StatusCode::OK, "ok", Some(v)),
        Ok(v) => (axum::http::StatusCode::SERVICE_UNAVAILABLE, "schema_mismatch", Some(v)),
        Err(_) => (axum::http::StatusCode::SERVICE_UNAVAILABLE, "store_unavailable", None),
    };
    (code, Json(serde_json::json!({
        "status": status,
        "service": "velora-panel",
        "version": env!("CARGO_PKG_VERSION"),
        "commit": commit,
        "schema": schema,
        "expected_schema": expected,
    })))
}

pub async fn manifest(State(state): State<AppState>, headers: HeaderMap, MaybeUser(user): MaybeUser) -> AppResult<Json<LauncherManifest>> {
    let settings = store::settings(&state).await?;
    let groups = match &user {
        Some(u) => auth::user_groups(&state, u.id).await?,
        None => vec![],
    };
    let mut instances = Vec::new();
    for row in store::list_instances(&state).await? {
        if row.visible_to(user.as_ref(), &groups) {
            let stats = store::file_stats(&state, &row.id).await?;
            let mut summary = row.to_summary(stats);
            if state.cfg.data_dir.join("panel.db").is_file() && !summary.experience.modules.is_empty() {
                let experience = state.experiences.state(&state, &row.id).await?;
                for (module, value) in &mut summary.experience.modules {
                    let live: Option<serde_json::Value> = store::kv_get(&experience, &format!("experience_module:{module}")).await?;
                    if let Some(live) = live {
                        *value = live;
                    }
                }
            }
            instances.push(summary);
        }
    }
    let public_user = match &user {
        Some(u) => Some(auth::public_user(&state, u).await?),
        None => None,
    };
    let mut auth_cfg = settings.auth;
    auth_cfg.yggdrasil_url = Some(format!("{}{}", net::public_base(&state, &headers).await, yggdrasil::ROOT));
    Ok(Json(LauncherManifest {
        api_version: API_VERSION,
        panel_version: env!("CARGO_PKG_VERSION").into(),
        branding: store::branding(&state).await?,
        auth: auth_cfg,
        instances,
        user: public_user,
    }))
}

pub async fn instance_manifest(
    State(state): State<AppState>,
    MaybeUser(user): MaybeUser,
    Path(id): Path<String>,
) -> AppResult<Json<InstanceManifest>> {
    let row = store::get_instance(&state, &id).await?;
    let groups = match &user {
        Some(u) => auth::user_groups(&state, u.id).await?,
        None => vec![],
    };
    if !row.visible_to(user.as_ref(), &groups) {
        return Err(AppError::not_found("instance not found"));
    }
    let stats = store::file_stats(&state, &id).await?;
    let files = store::to_entries(store::instance_files(&state, &id).await?);
    Ok(Json(InstanceManifest { instance: row.to_summary(stats), files }))
}

pub async fn login(State(state): State<AppState>, Json(request): Json<LoginRequest>) -> AppResult<Json<AuthResponse>> {
    Ok(Json(auth::account::login(&state, request).await?))
}
pub async fn register(State(state): State<AppState>, Json(request): Json<RegisterRequest>) -> AppResult<Json<AuthResponse>> {
    Ok(Json(auth::account::register(&state, request).await?))
}

pub async fn me(State(state): State<AppState>, AuthUser(user): AuthUser) -> AppResult<Json<PublicUser>> {
    Ok(Json(auth::public_user(&state, &user).await?))
}

pub async fn event(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    ClientIp(ip): ClientIp,
    Json(ev): Json<LaunchEvent>,
) -> AppResult<Json<serde_json::Value>> {
    let kind = ev.kind.as_str();
    if !matches!(
        kind,
        "launch"
            | "launcher_open"
            | "launcher_close"
            | "settings_changed"
            | "account_selected"
            | "logout"
            | "install_start"
            | "install_complete"
            | "repair_start"
            | "repair_complete"
            | "launch_failed"
            | "launch_cancelled"
            | "game_exit"
            | "game_crash"
            | "game_killed"
            | "instance_deleted"
            | "cache_cleared"
            | "update_installed"
            | "folder_opened"
            | "link_opened"
    ) {
        return Err(AppError::bad_request("unknown launcher event"));
    }
    // Remember where signed-in players launch from, so game servers can
    // require "joined through the launcher" (see game server settings).
    if let (Some(ip), "launch") = (&ip, kind) {
        velora_auth_core::launcher_sessions::record(&state.db, user.id, ip, &user.username, &crate::db::now()).await?;
        let cutoff = (chrono::Utc::now() - chrono::Duration::days(2)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        velora_auth_core::launcher_sessions::prune(&state.db, &cutoff).await?;
    }
    // Identity comes exclusively from the verified bearer token.
    sqlx::query("INSERT INTO events (instance_id, username, uuid, kind, created_at) VALUES (?, ?, ?, ?, ?)")
        .bind(ev.instance_id.chars().take(64).collect::<String>())
        .bind(&user.username)
        .bind(&user.uuid)
        .bind(kind)
        .bind(crate::db::now())
        .execute(&state.db)
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}
