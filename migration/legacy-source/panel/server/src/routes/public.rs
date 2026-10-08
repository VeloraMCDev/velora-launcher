//! Endpoints the launcher talks to.

use crate::auth::{self, AuthUser, MaybeUser, UserRow};
use crate::error::{AppError, AppResult};
use crate::net::{self, ClientIp};
use crate::state::AppState;
use crate::state::RequestState as State;
use crate::store;
use crate::yggdrasil;
use axum::extract::Path;
use axum::http::HeaderMap;
use axum::Json;
use scopenet_shared::*;

pub async fn health() -> &'static str {
    "ok"
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

async fn find_user(state: &AppState, username: &str) -> AppResult<Option<UserRow>> {
    auth::find_user_by_name(state, username).await
}

/// Panel token + a fresh game session for authlib-injector.
pub(crate) async fn signed_in(state: &AppState, user: &UserRow) -> AppResult<AuthResponse> {
    let (access_token, client_token) = yggdrasil::issue_token(state, user.id, None).await?;
    Ok(AuthResponse {
        token: state.keys.issue(user)?,
        user: auth::public_user(state, user).await?,
        pending: false,
        yggdrasil: Some(YggdrasilTokens { access_token, client_token }),
    })
}

pub async fn login(State(state): State<AppState>, Json(req): Json<LoginRequest>) -> AppResult<Json<AuthResponse>> {
    let username = req.username.trim().to_string();
    state.login_guard.check(&username)?;
    let settings = store::settings(&state).await?;
    let user = find_user(&state, &username).await?;
    let Some(user) = user.filter(|u| auth::verify_password(&req.password, &u.password_hash)) else {
        state.login_guard.fail(&username);
        return Err(AppError::unauthorized("wrong username or password"));
    };
    state.login_guard.succeed(&username);
    match user.status.as_str() {
        "pending" => return Err(AppError::forbidden("your account is waiting for an admin to approve it")),
        "disabled" => {
            return Err(AppError::forbidden(user.status_reason.clone().unwrap_or_else(|| "this account has been disabled".into())))
        }
        _ => {}
    }
    if !settings.auth.panel_accounts && !user.is_admin() {
        return Err(AppError::forbidden("account sign-in is currently disabled"));
    }
    sqlx::query("UPDATE users SET last_login = ? WHERE id = ?").bind(crate::db::now()).bind(user.id).execute(&state.db).await?;
    super::activity::record(&state, &user, "auth", "login", None).await?;
    Ok(Json(signed_in(&state, &user).await?))
}

pub async fn register(State(state): State<AppState>, Json(req): Json<RegisterRequest>) -> AppResult<Json<AuthResponse>> {
    let settings = store::settings(&state).await?;
    if !settings.auth.panel_accounts || settings.auth.registration == RegistrationMode::Closed {
        return Err(AppError::forbidden("sign-ups are closed — ask an admin for an account"));
    }
    let username = req.username.trim();
    if !valid_username(username) {
        return Err(AppError::bad_request("usernames are 3-16 letters, numbers or underscores"));
    }
    auth::validate_password(&req.password)?;
    if find_user(&state, username).await?.is_some() {
        return Err(AppError::conflict("that username is taken"));
    }
    let status = if settings.auth.registration == RegistrationMode::Approval { "pending" } else { "active" };
    let id = auth::create_user(&state, username, &req.password, req.email.as_deref(), "player", status).await?;
    let user: UserRow = sqlx::query_as("SELECT * FROM users WHERE id = ?").bind(id).fetch_one(&state.db).await?;
    if status == "pending" {
        return Ok(Json(AuthResponse {
            token: String::new(),
            user: auth::public_user(&state, &user).await?,
            pending: true,
            yggdrasil: None,
        }));
    }
    Ok(Json(signed_in(&state, &user).await?))
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
        sqlx::query("INSERT INTO launcher_sessions (user_id, ip, username, created_at) VALUES (?, ?, ?, ?)")
            .bind(user.id)
            .bind(ip)
            .bind(&user.username)
            .bind(crate::db::now())
            .execute(&state.db)
            .await?;
        let cutoff = (chrono::Utc::now() - chrono::Duration::days(2)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        sqlx::query("DELETE FROM launcher_sessions WHERE created_at < ?").bind(cutoff).execute(&state.db).await?;
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
