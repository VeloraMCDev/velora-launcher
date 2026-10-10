//! Everything the UI can ask the backend to do.

use crate::accounts::{self, Account};
use crate::game;
use crate::settings::Settings;
use crate::state::{build, AppState};
use crate::updater;
use velora_launcher_core::ping::ServerStatus;
use velora_shared::{
    Achievement, BaltopEntry, DirectMessage, EconomyTransaction, FriendInfo, GameInvite, Guild, GuildClaim, GuildMember, GuildPost,
    LauncherManifest, MemberProfile, PlayerProfile, ServerEconomyBalance, SkinProfile, UserLevelInfo, UserPost, UserProfileView, UserQuest,
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_opener::OpenerExt;

/// Commands return `Err(String)`; the UI shows it as-is.
type Res<T> = Result<T, String>;

#[tauri::command]
pub async fn record_activity(state: State<'_, AppState>, kind: String, instance_id: Option<String>) -> Res<()> {
    crate::telemetry::send(&state, &kind, instance_id.as_deref().unwrap_or("")).await.map_err(aerr)
}

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

/// anyhow's alternate format keeps the context chain readable.
fn aerr(e: anyhow::Error) -> String {
    format!("{e:#}")
}

#[derive(Serialize)]
pub struct AppInfo {
    version: String,
    default_panel_url: Option<String>,
    panel_locked: bool,
    repo: Option<String>,
    os: String,
    arch: String,
    total_ram_mb: u64,
    data_dir: String,
}

#[derive(Serialize)]
pub struct Bootstrap {
    app: AppInfo,
    panel_url: Option<String>,
    settings: Settings,
    accounts: Vec<Account>,
    active_account: Option<String>,
    manifest: Option<LauncherManifest>,
    game_running: Vec<RunningGameInfo>,
}

#[derive(Serialize)]
struct RunningGameInfo {
    run_id: String,
    instance_id: String,
}

#[tauri::command]
pub fn bootstrap(state: State<'_, AppState>) -> Bootstrap {
    let accounts = state.accounts.read().unwrap().clone();
    Bootstrap {
        app: AppInfo {
            version: env!("CARGO_PKG_VERSION").into(),
            default_panel_url: build::PANEL_URL.filter(|u| !u.is_empty()).map(String::from),
            panel_locked: build::panel_locked(),
            repo: build::REPO.map(String::from),
            os: std::env::consts::OS.into(),
            arch: std::env::consts::ARCH.into(),
            total_ram_mb: velora_launcher_core::sys::total_memory_mb(),
            data_dir: state.data_dir.to_string_lossy().into_owned(),
        },
        panel_url: state.panel_url(),
        settings: state.settings.read().unwrap().clone(),
        accounts: accounts.accounts,
        active_account: accounts.active,
        manifest: state.manifest.read().unwrap().clone(),
        game_running: state
            .game
            .lock()
            .unwrap()
            .iter()
            .map(|g| RunningGameInfo { run_id: g.run_id.clone(), instance_id: g.instance_id.clone() })
            .collect(),
    }
}

async fn fetch_manifest(state: &AppState, panel: &str) -> anyhow::Result<LauncherManifest> {
    let mut req = state.http.get(format!("{panel}/api/v1/launcher/manifest"));
    if let Some(t) = accounts::panel_token(state) {
        req = req.bearer_auth(t);
    }
    let resp = req.send().await.map_err(|e| anyhow::anyhow!("couldn't reach {panel}: {e}"))?;
    if !resp.status().is_success() {
        anyhow::bail!("{panel} returned {}", resp.status());
    }
    let m: LauncherManifest = resp.json().await.map_err(|_| anyhow::anyhow!("{panel} doesn't look like a Velora panel"))?;
    Ok(m)
}

#[tauri::command]
pub async fn refresh_manifest(state: State<'_, AppState>) -> Res<LauncherManifest> {
    let panel = state.panel_url().ok_or("no panel configured")?;
    let m = fetch_manifest(&state, &panel).await.map_err(aerr)?;
    state.cache_manifest(&m);
    Ok(m)
}

#[tauri::command]
pub async fn set_panel_url(state: State<'_, AppState>, url: String) -> Res<LauncherManifest> {
    if build::panel_locked() {
        return Err("this launcher is locked to its server".into());
    }
    let mut url = url.trim().trim_end_matches('/').to_string();
    if !url.starts_with("http://") && !url.starts_with("https://") {
        url = format!("https://{url}");
    }
    let m = fetch_manifest(&state, &url).await.map_err(aerr)?;
    state.settings.write().unwrap().panel_url = Some(url);
    state.save_settings().map_err(aerr)?;
    state.cache_manifest(&m);
    Ok(m)
}

#[tauri::command]
pub fn select_experience(state: State<'_, AppState>, instance_id: Option<String>) -> Res<()> {
    if let Some(id) = &instance_id {
        if !state.manifest.read().unwrap().as_ref().is_some_and(|m| m.instances.iter().any(|i| &i.id == id)) {
            return Err("instance is unavailable".into());
        }
    }
    state.settings.write().unwrap().selected_instance = instance_id;
    state.save_settings().map_err(aerr)
}

#[tauri::command]
pub fn save_settings(state: State<'_, AppState>, settings: Settings) -> Res<()> {
    let mut current = state.settings.write().unwrap();
    let panel = current.panel_url.clone();
    let captured = current.instance_game_options.clone();
    *current = Settings { panel_url: panel, instance_game_options: captured, ..settings };
    drop(current);
    state.save_settings().map_err(aerr)
}

#[tauri::command]
pub fn save_instance_options(state: State<'_, AppState>, instance_id: String) -> Res<std::collections::BTreeMap<String, String>> {
    if state.game.lock().unwrap().iter().any(|g| g.instance_id == instance_id) {
        return Err("close this instance before saving its game settings".into());
    }
    let options = velora_launcher_core::options::read_vanilla_preferences(&state.layout.instance_dir(&instance_id)).map_err(aerr)?;
    if options.is_empty() {
        return Err("no vanilla options.txt settings found yet".into());
    }
    state.settings.write().unwrap().instance_game_options.insert(instance_id, options.clone());
    state.save_settings().map_err(aerr)?;
    Ok(options)
}

#[tauri::command]
pub fn set_instance_fov(state: State<'_, AppState>, instance_id: String, fov: f64) -> Res<()> {
    if !(-1.0..=1.0).contains(&fov) || !fov.is_finite() {
        return Err("FOV must be between 30 and 110 degrees".into());
    }
    state.settings.write().unwrap().instance_game_options.entry(instance_id).or_default().insert("fov".into(), format!("{fov:.3}"));
    state.save_settings().map_err(aerr)
}

// ---- accounts ----

#[tauri::command]
pub async fn login_panel(state: State<'_, AppState>, username: String, password: String) -> Res<Account> {
    accounts::login_panel(&state, &username, &password).await.map_err(aerr)
}

#[tauri::command]
pub async fn discord_sign_in_start(state: State<'_, AppState>) -> Res<serde_json::Value> {
    let panel = state.panel_url().ok_or("no panel configured")?;
    let response = state.http.get(format!("{panel}/api/v1/auth/discord/start")).send().await.map_err(err)?;
    if !response.status().is_success() {
        return Err(accounts::panel_error(response).await.to_string());
    }
    response.json().await.map_err(err)
}

#[tauri::command]
pub async fn discord_sign_in_poll(state: State<'_, AppState>, oauth_state: String) -> Res<serde_json::Value> {
    let panel = state.panel_url().ok_or("no panel configured")?;
    let response =
        state.http.get(format!("{panel}/api/v1/auth/discord/poll")).query(&[("state", oauth_state)]).send().await.map_err(err)?;
    if !response.status().is_success() {
        return Err(accounts::panel_error(response).await.to_string());
    }
    let result: serde_json::Value = response.json().await.map_err(err)?;
    if result.get("token").is_some() {
        let auth = serde_json::from_value(result.clone()).map_err(err)?;
        let account = accounts::save_panel_account(&state, &panel, auth).map_err(aerr)?;
        return Ok(serde_json::json!({"account":account}));
    }
    Ok(result)
}

#[tauri::command]
pub async fn request_password_reset(state: State<'_, AppState>, email: String) -> Res<String> {
    let panel = state.panel_url().ok_or("no panel configured")?;
    let response = state
        .http
        .post(format!("{panel}/api/v1/auth/forgot-password"))
        .json(&serde_json::json!({"email":email}))
        .send()
        .await
        .map_err(err)?;
    if !response.status().is_success() {
        return Err(accounts::panel_error(response).await.to_string());
    }
    let body: serde_json::Value = response.json().await.map_err(err)?;
    Ok(body["message"].as_str().unwrap_or("Check your email for a reset link.").to_string())
}

#[tauri::command]
pub async fn discord_connection(state: State<'_, AppState>) -> Res<serde_json::Value> {
    let req = account_api(&state, reqwest::Method::GET, "/account/connections/discord").await?;
    let response = req.send().await.map_err(err)?;
    if !response.status().is_success() {
        return Err(accounts::panel_error(response).await.to_string());
    }
    response.json().await.map_err(err)
}

/// The community invite link and whether this account is linked.
#[tauri::command]
pub async fn discord_info(state: State<'_, AppState>) -> Res<serde_json::Value> {
    let req = account_api(&state, reqwest::Method::GET, "/account/discord").await?;
    let response = req.send().await.map_err(err)?;
    if !response.status().is_success() {
        return Err(accounts::panel_error(response).await.to_string());
    }
    response.json().await.map_err(err)
}

#[tauri::command]
pub async fn discord_link_start(state: State<'_, AppState>) -> Res<serde_json::Value> {
    let req = account_api(&state, reqwest::Method::GET, "/auth/discord/start?kind=link").await?;
    let response = req.send().await.map_err(err)?;
    if !response.status().is_success() {
        return Err(accounts::panel_error(response).await.to_string());
    }
    response.json().await.map_err(err)
}

#[tauri::command]
pub async fn discord_unlink(state: State<'_, AppState>) -> Res<()> {
    let req = account_api(&state, reqwest::Method::DELETE, "/account/connections/discord").await?;
    let response = req.send().await.map_err(err)?;
    if !response.status().is_success() {
        return Err(accounts::panel_error(response).await.to_string());
    }
    Ok(())
}

#[derive(Serialize)]
pub struct RegisterResult {
    account: Option<Account>,
    pending: bool,
}

#[tauri::command]
pub async fn register_panel(state: State<'_, AppState>, username: String, password: String, email: Option<String>) -> Res<RegisterResult> {
    let (account, pending) = accounts::register_panel(&state, &username, &password, email).await.map_err(aerr)?;
    Ok(RegisterResult { account, pending })
}

#[tauri::command]
pub fn add_offline(state: State<'_, AppState>, username: String) -> Res<Account> {
    accounts::add_offline(&state, &username).map_err(aerr)
}

#[tauri::command]
pub fn select_account(state: State<'_, AppState>, id: String) -> Res<()> {
    let mut accounts = state.accounts.write().unwrap();
    if !accounts.accounts.iter().any(|a| a.id == id) {
        return Err("account not found".into());
    }
    accounts.active = Some(id);
    drop(accounts);
    state.save_accounts().map_err(aerr)
}

#[tauri::command]
pub fn remove_account(state: State<'_, AppState>, id: String) -> Res<()> {
    crate::telemetry::background(&state, "logout", "");
    let mut accounts = state.accounts.write().unwrap();
    accounts.accounts.retain(|a| a.id != id);
    if accounts.active.as_deref() == Some(id.as_str()) {
        accounts.active = accounts.accounts.first().map(|a| a.id.clone());
    }
    drop(accounts);
    state.secrets.remove(&id).map_err(aerr)?;
    state.save_accounts().map_err(aerr)
}

// ---- skin & cape (panel accounts) ----

async fn account_api(state: &AppState, method: reqwest::Method, path: &str) -> Res<reqwest::RequestBuilder> {
    let panel = state.panel_url().ok_or("no panel configured")?;
    let token = accounts::panel_token(state).ok_or("sign in with a server account to change your skin")?;
    let mut request = state.http.request(method, format!("{panel}/api/v1{path}")).bearer_auth(token);
    if let Some(id) = &state.settings.read().unwrap().selected_instance {
        request = request.header("X-Velora-Instance", id);
    }
    Ok(request)
}

async fn profile_response(resp: reqwest::Response) -> Res<PlayerProfile> {
    if !resp.status().is_success() {
        let body: serde_json::Value = resp.json().await.unwrap_or_default();
        return Err(body["error"].as_str().unwrap_or("the server refused the change").to_string());
    }
    resp.json().await.map_err(err)
}

#[tauri::command]
pub async fn account_profile(state: State<'_, AppState>) -> Res<PlayerProfile> {
    let req = account_api(&state, reqwest::Method::GET, "/account/profile").await?;
    profile_response(req.send().await.map_err(err)?).await
}

/// `data` is the PNG as base64 (read in the UI from a file picker).
#[tauri::command]
pub async fn upload_skin(state: State<'_, AppState>, data: String, model: String) -> Res<PlayerProfile> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD.decode(data.trim()).map_err(|_| "couldn't read that image".to_string())?;
    let form = reqwest::multipart::Form::new()
        .text("model", model)
        .part("file", reqwest::multipart::Part::bytes(bytes).file_name("skin.png").mime_str("image/png").map_err(err)?);
    let req = account_api(&state, reqwest::Method::POST, "/account/skin").await?;
    profile_response(req.multipart(form).send().await.map_err(err)?).await
}

#[tauri::command]
pub async fn set_skin_model(state: State<'_, AppState>, model: String) -> Res<PlayerProfile> {
    let req = account_api(&state, reqwest::Method::PUT, "/account/skin/model").await?;
    profile_response(req.json(&serde_json::json!({ "model": model })).send().await.map_err(err)?).await
}

#[tauri::command]
pub async fn delete_skin(state: State<'_, AppState>) -> Res<PlayerProfile> {
    let req = account_api(&state, reqwest::Method::DELETE, "/account/skin").await?;
    profile_response(req.send().await.map_err(err)?).await
}

#[tauri::command]
pub async fn set_cape(state: State<'_, AppState>, cape_id: Option<i64>) -> Res<PlayerProfile> {
    let req = account_api(&state, reqwest::Method::PUT, "/account/cape").await?;
    profile_response(req.json(&serde_json::json!({ "cape_id": cape_id })).send().await.map_err(err)?).await
}

fn load_skin_profiles(data_dir: &std::path::Path) -> Vec<SkinProfile> {
    let path = data_dir.join("skin_profiles.json");
    std::fs::read(&path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

fn write_skin_profiles(data_dir: &std::path::Path, profiles: &[SkinProfile]) -> anyhow::Result<()> {
    let path = data_dir.join("skin_profiles.json");
    let json = serde_json::to_vec_pretty(profiles)?;
    std::fs::write(path, json)?;
    Ok(())
}

#[tauri::command]
pub fn get_skin_profiles(state: State<'_, AppState>) -> Res<Vec<SkinProfile>> {
    Ok(load_skin_profiles(&state.data_dir))
}

#[tauri::command]
pub async fn save_skin_profile(state: State<'_, AppState>, mut profile: SkinProfile) -> Res<Vec<SkinProfile>> {
    use base64::Engine;
    if let Some(ref mut data) = profile.skin_data {
        if data.starts_with('/') {
            if let Some(panel) = state.panel_url() {
                *data = format!("{panel}{data}");
            }
        }
        if data.starts_with("http://") || data.starts_with("https://") {
            if let Ok(resp) = state.http.get(&*data).send().await {
                if resp.status().is_success() {
                    if let Ok(bytes) = resp.bytes().await {
                        let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                        *data = format!("data:image/png;base64,{b64}");
                    }
                }
            }
        } else if !data.starts_with("data:") && !data.is_empty() {
            *data = format!("data:image/png;base64,{data}");
        }
    }
    if profile.created_at.is_empty() {
        let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs().to_string()).unwrap_or_default();
        profile.created_at = ts;
    }
    let mut profiles = load_skin_profiles(&state.data_dir);
    if let Some(idx) = profiles.iter().position(|x| x.id == profile.id) {
        profiles[idx] = profile;
    } else {
        profiles.push(profile);
    }
    write_skin_profiles(&state.data_dir, &profiles).map_err(aerr)?;
    Ok(profiles)
}

#[tauri::command]
pub fn delete_skin_profile(state: State<'_, AppState>, id: String) -> Res<Vec<SkinProfile>> {
    let mut profiles = load_skin_profiles(&state.data_dir);
    profiles.retain(|x| x.id != id);
    write_skin_profiles(&state.data_dir, &profiles).map_err(aerr)?;
    Ok(profiles)
}

#[tauri::command]
pub async fn apply_skin_profile(state: State<'_, AppState>, id: String) -> Res<PlayerProfile> {
    use base64::Engine;
    let profiles = load_skin_profiles(&state.data_dir);
    let p = profiles.into_iter().find(|x| x.id == id).ok_or_else(|| "profile not found".to_string())?;

    if let Some(skin_data) = p.skin_data {
        if !skin_data.is_empty() {
            let bytes = if skin_data.starts_with("data:") {
                let comma = skin_data.find(',').ok_or_else(|| "invalid data uri".to_string())?;
                base64::engine::general_purpose::STANDARD.decode(skin_data[comma + 1..].trim()).map_err(err)?
            } else if skin_data.starts_with("http://") || skin_data.starts_with("https://") {
                state.http.get(&skin_data).send().await.map_err(err)?.bytes().await.map_err(err)?.to_vec()
            } else {
                base64::engine::general_purpose::STANDARD.decode(skin_data.trim()).map_err(err)?
            };
            let form = reqwest::multipart::Form::new()
                .text("model", p.skin_model.clone())
                .part("file", reqwest::multipart::Part::bytes(bytes).file_name("skin.png").mime_str("image/png").map_err(err)?);
            let req = account_api(&state, reqwest::Method::POST, "/account/skin").await?;
            profile_response(req.multipart(form).send().await.map_err(err)?).await?;
        } else {
            let req = account_api(&state, reqwest::Method::PUT, "/account/skin/model").await?;
            profile_response(req.json(&serde_json::json!({ "model": p.skin_model })).send().await.map_err(err)?).await?;
        }
    } else {
        let req = account_api(&state, reqwest::Method::PUT, "/account/skin/model").await?;
        profile_response(req.json(&serde_json::json!({ "model": p.skin_model })).send().await.map_err(err)?).await?;
    }

    let req = account_api(&state, reqwest::Method::PUT, "/account/cape").await?;
    profile_response(req.json(&serde_json::json!({ "cape_id": p.cape_id })).send().await.map_err(err)?).await?;

    account_profile(state).await
}

// ---- game ----

#[tauri::command]
pub async fn set_username(state: State<'_, AppState>, username: String, password: String) -> Res<accounts::Account> {
    accounts::rename_panel(&state, &username, &password).await.map_err(aerr)
}

#[tauri::command]
pub fn launch(app: AppHandle, state: State<'_, AppState>, instance_id: String) -> Res<()> {
    start_task(app, &state, instance_id, true, false)
}

#[tauri::command]
pub fn repair_instance(app: AppHandle, state: State<'_, AppState>, instance_id: String) -> Res<()> {
    start_task(app, &state, instance_id, false, true)
}

fn start_task(app: AppHandle, state: &AppState, instance_id: String, start: bool, deep: bool) -> Res<()> {
    if deep && state.game.lock().unwrap().iter().any(|g| g.instance_id == instance_id) {
        return Err("Close this instance's game windows before repairing it".into());
    }
    let mut task = state.task.lock().unwrap();
    if task.as_ref().is_some_and(|t| !t.is_finished()) {
        return Err("already preparing an instance".into());
    }
    let app2 = app.clone();
    let handle = tauri::async_runtime::spawn(async move {
        if let Err(e) = game::run(app2.clone(), instance_id.clone(), start, deep).await {
            crate::telemetry::background(&app2.state::<AppState>(), "launch_failed", &instance_id);
            tracing::error!("launch failed: {e:#}");
            game::emit_state(&app2, &instance_id, "error", Some(format!("{e:#}")));
        }
    });
    *task = Some(handle.inner().abort_handle());
    Ok(())
}

#[tauri::command]
pub fn cancel_launch(app: AppHandle, state: State<'_, AppState>, instance_id: String) {
    if let Some(t) = state.task.lock().unwrap().take() {
        t.abort();
    }
    game::emit_state(&app, &instance_id, "idle", Some("Cancelled".into()));
}

#[tauri::command]
pub fn kill_game(state: State<'_, AppState>, run_id: String) {
    if let Some(g) = state.game.lock().unwrap().iter_mut().find(|g| g.run_id == run_id) {
        if let Some(kill) = g.kill.take() {
            kill.send(()).ok();
        }
    }
}

#[derive(Serialize)]
pub struct InstanceLocal {
    installed: bool,
    revision: i64,
    size: u64,
}

#[tauri::command]
pub async fn instance_local(state: State<'_, AppState>, instance_id: String) -> Res<InstanceLocal> {
    let dir = state.layout.instance_dir(&instance_id);
    let s = velora_launcher_core::sync::load_state(&dir);
    let size = tokio::task::spawn_blocking(move || velora_launcher_core::sys::dir_size(&dir)).await.unwrap_or(0);
    Ok(InstanceLocal { installed: s.revision > 0, revision: s.revision, size })
}

#[tauri::command]
pub async fn ping_server(host: String, port: u16) -> Res<ServerStatus> {
    velora_launcher_core::ping::ping(&host, port).await.map_err(aerr)
}

#[tauri::command]
pub fn open_folder(app: AppHandle, state: State<'_, AppState>, kind: String, instance_id: Option<String>) -> Res<()> {
    let base = match &instance_id {
        Some(id) => state.layout.instance_dir(id),
        None => state.data_dir.clone(),
    };
    let path = match kind.as_str() {
        "instance" | "data" => base,
        "logs" | "screenshots" | "mods" | "resourcepacks" | "shaderpacks" | "saves" | "crash-reports" => base.join(&kind),
        _ => return Err("unknown folder".into()),
    };
    std::fs::create_dir_all(&path).map_err(err)?;
    app.opener().open_path(path.to_string_lossy(), None::<&str>).map_err(err)
}

#[tauri::command]
pub fn open_file(app: AppHandle, path: String) -> Res<()> {
    app.opener().open_path(path, None::<&str>).map_err(err)
}

#[derive(Serialize)]
pub struct Storage {
    shared: u64,
    runtimes: u64,
    instances: Vec<(String, u64)>,
}

#[tauri::command]
pub async fn storage_info(state: State<'_, AppState>) -> Res<Storage> {
    let layout = state.layout.clone();
    tokio::task::spawn_blocking(move || {
        use velora_launcher_core::sys::dir_size;
        let shared = dir_size(&layout.libraries()) + dir_size(&layout.assets()) + dir_size(&layout.versions());
        let runtimes = dir_size(&layout.runtimes());
        let instances = std::fs::read_dir(layout.instances())
            .map(|rd| rd.filter_map(|e| e.ok()).map(|e| (e.file_name().to_string_lossy().into_owned(), dir_size(&e.path()))).collect())
            .unwrap_or_default();
        Storage { shared, runtimes, instances }
    })
    .await
    .map_err(err)
}

#[tauri::command]
pub async fn delete_instance_data(state: State<'_, AppState>, instance_id: String) -> Res<()> {
    if state.game.lock().unwrap().iter().any(|g| g.instance_id == instance_id) {
        return Err("close the game first".into());
    }
    let dir = state.layout.instance_dir(&instance_id);
    tokio::fs::remove_dir_all(dir).await.or_else(|e| if e.kind() == std::io::ErrorKind::NotFound { Ok(()) } else { Err(e) }).map_err(err)
}

#[tauri::command]
pub async fn clear_cache(state: State<'_, AppState>) -> Res<()> {
    let cache = state.layout.cache();
    tokio::fs::remove_dir_all(cache).await.ok();
    Ok(())
}

#[tauri::command]
pub async fn detect_java(path: String) -> Option<u32> {
    velora_launcher_core::java::detect_major(std::path::Path::new(&path)).await
}

#[tauri::command]
pub async fn check_update(state: State<'_, AppState>) -> Res<Option<updater::UpdateInfo>> {
    updater::check(&state.http, state.panel_url().as_deref()).await.map_err(aerr)
}

#[tauri::command]
pub async fn install_update(app: AppHandle, state: State<'_, AppState>, url: String) -> Res<()> {
    let update = updater::check(&state.http, state.panel_url().as_deref()).await.map_err(aerr)?.ok_or("No launcher update is available")?;
    if update.url != url {
        return Err("The available release changed. Check for updates again.".into());
    }
    updater::download_and_run(&state.http, &update).await.map_err(aerr)?;
    app.exit(0);
    Ok(())
}

#[tauri::command]
pub fn show_window(app: AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        w.show().ok();
        w.set_focus().ok();
    }
}

// ---- stats & leaderboards ----

#[tauri::command]
pub async fn get_player_stats(state: State<'_, AppState>) -> Res<serde_json::Value> {
    let req = account_api(&state, reqwest::Method::GET, "/account/stats").await?;
    let resp = req.send().await.map_err(err)?;
    if !resp.status().is_success() {
        return Err("unable to load player stats".into());
    }
    resp.json().await.map_err(err)
}

#[tauri::command]
pub async fn get_leaderboard(state: State<'_, AppState>, server_id: Option<i64>, sort: Option<String>) -> Res<serde_json::Value> {
    let panel = state.panel_url().ok_or("no panel configured")?;
    let sort_query = sort.map(|s| format!("?sort={s}")).unwrap_or_default();
    let url = if let Some(id) = server_id {
        format!("{panel}/api/v1/servers/{id}/leaderboard{sort_query}")
    } else {
        format!("{panel}/api/v1/leaderboard{sort_query}")
    };
    let mut request = state.http.get(&url);
    if let Some(id) = &state.settings.read().unwrap().selected_instance {
        request = request.header("X-Velora-Instance", id);
    }
    if let Some(token) = accounts::panel_token(&state) {
        request = request.bearer_auth(token);
    }
    let resp = request.send().await.map_err(err)?;
    if !resp.status().is_success() {
        return Err("unable to load leaderboard".into());
    }
    resp.json().await.map_err(err)
}

#[tauri::command]
pub async fn get_public_servers(state: State<'_, AppState>) -> Res<serde_json::Value> {
    let panel = state.panel_url().ok_or("no panel configured")?;
    let mut request = state.http.get(format!("{panel}/api/v1/servers/public"));
    if let Some(id) = &state.settings.read().unwrap().selected_instance {
        request = request.header("X-Velora-Instance", id);
    }
    if let Some(token) = accounts::panel_token(&state) {
        request = request.bearer_auth(token);
    }
    let resp = request.send().await.map_err(err)?;
    if !resp.status().is_success() {
        return Err("unable to load servers".into());
    }
    resp.json().await.map_err(err)
}

/// What the Velora Map needs for one game server: which dimensions have tiles, the tile key, and where tiles live (made absolute,
/// because the map loads them straight from the panel as images).
#[tauri::command]
pub async fn get_map_info(state: State<'_, AppState>, server_id: i64) -> Res<serde_json::Value> {
    let panel = state.panel_url().ok_or("no panel configured")?;
    let req = account_api(&state, reqwest::Method::GET, &format!("/servers/{server_id}/map")).await?;
    let resp = req.send().await.map_err(err)?;
    if !resp.status().is_success() {
        let body: serde_json::Value = resp.json().await.unwrap_or_default();
        return Err(body["error"].as_str().unwrap_or("unable to load the map").to_string());
    }
    let mut info: serde_json::Value = resp.json().await.map_err(err)?;
    if let Some(base) = info["tile_base"].as_str().map(str::to_owned) {
        info["tile_base"] = serde_json::json!(format!("{}{}", panel.trim_end_matches('/'), base));
    }
    Ok(info)
}

/// Players now, plus the claims and pins the game server last sent.
#[tauri::command]
pub async fn get_map_overlay(state: State<'_, AppState>, server_id: i64) -> Res<serde_json::Value> {
    let req = account_api(&state, reqwest::Method::GET, &format!("/servers/{server_id}/map/overlay")).await?;
    let resp = req.send().await.map_err(err)?;
    if !resp.status().is_success() {
        let body: serde_json::Value = resp.json().await.unwrap_or_default();
        return Err(body["error"].as_str().unwrap_or("unable to load the map").to_string());
    }
    resp.json().await.map_err(err)
}

// ---------------------------------------------------------------------------
// Leveling & Quests
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn get_my_level(state: State<'_, AppState>) -> Res<UserLevelInfo> {
    let req = account_api(&state, reqwest::Method::GET, "/levels/me").await?;
    let resp = req.send().await.map_err(err)?;
    if !resp.status().is_success() {
        return Err("unable to load user level".into());
    }
    resp.json().await.map_err(err)
}

/// What quests, achievements and rank milestones hand out besides XP, as short lines keyed `quest:<id>`.
#[tauri::command]
pub async fn get_reward_summaries(state: State<'_, AppState>) -> Res<serde_json::Value> {
    let req = account_api(&state, reqwest::Method::GET, "/reward-bundles").await?;
    let resp = req.send().await.map_err(err)?;
    if !resp.status().is_success() {
        return Ok(serde_json::json!({}));
    }
    resp.json().await.map_err(err)
}

#[tauri::command]
pub async fn get_my_quests(state: State<'_, AppState>) -> Res<Vec<UserQuest>> {
    let req = account_api(&state, reqwest::Method::GET, "/quests/my").await?;
    let resp = req.send().await.map_err(err)?;
    if !resp.status().is_success() {
        return Err("unable to load quests".into());
    }
    resp.json().await.map_err(err)
}

#[tauri::command]
pub async fn claim_quest(state: State<'_, AppState>, quest_id: String) -> Res<UserLevelInfo> {
    let req = account_api(&state, reqwest::Method::POST, &format!("/quests/{quest_id}/claim")).await?;
    let resp = req.send().await.map_err(err)?;
    if !resp.status().is_success() {
        let body: serde_json::Value = resp.json().await.unwrap_or_default();
        return Err(body["error"].as_str().unwrap_or("unable to claim quest").to_string());
    }
    get_my_level(state).await
}

// ---------------------------------------------------------------------------
// Achievements
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn get_my_achievements(state: State<'_, AppState>) -> Res<Vec<Achievement>> {
    let req = account_api(&state, reqwest::Method::GET, "/achievements/my").await?;
    let resp = req.send().await.map_err(err)?;
    if !resp.status().is_success() {
        return Err("unable to load achievements".into());
    }
    resp.json().await.map_err(err)
}

#[tauri::command]
pub async fn get_my_collections(state: State<'_, AppState>) -> Res<Vec<serde_json::Value>> {
    let req = account_api(&state, reqwest::Method::GET, "/collections/me").await?;
    let resp = req.send().await.map_err(err)?;
    if !resp.status().is_success() {
        return Err("unable to load collections".into());
    }
    resp.json().await.map_err(err)
}

#[tauri::command]
pub async fn equip_collection_item(state: State<'_, AppState>, unlock_key: String, equipped: bool) -> Res<()> {
    let resp = account_api(&state, reqwest::Method::POST, "/collections/equip")
        .await?
        .json(&serde_json::json!({"unlock_key":unlock_key,"equipped":equipped}))
        .send()
        .await
        .map_err(err)?;
    if !resp.status().is_success() {
        return Err(guild_response_error(resp, "unable to update collection item").await);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Guilds & Land Claims
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn get_guilds(state: State<'_, AppState>, instance_id: Option<String>) -> Res<Vec<Guild>> {
    let q = instance_id.map(|i| format!("?instance_id={i}")).unwrap_or_default();
    let resp = account_api(&state, reqwest::Method::GET, &format!("/guilds{q}")).await?.send().await.map_err(err)?;
    if !resp.status().is_success() {
        return Err("unable to load factions".into());
    }
    resp.json().await.map_err(err)
}

#[tauri::command]
pub async fn get_my_guild(state: State<'_, AppState>, instance_id: Option<String>) -> Res<Option<Guild>> {
    let q = instance_id.map(|i| format!("?instance_id={i}")).unwrap_or_default();
    let req = account_api(&state, reqwest::Method::GET, &format!("/guilds/my{q}")).await?;
    let resp = req.send().await.map_err(err)?;
    if !resp.status().is_success() {
        return Ok(None);
    }
    resp.json().await.map_err(err)
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GuildMembership {
    id: String,
    instance_id: String,
    name: String,
    tag: String,
    role: String,
    server_count: i64,
    primary_server_count: i64,
}

#[tauri::command]
pub async fn get_my_guild_memberships(state: State<'_, AppState>, instance_id: Option<String>) -> Res<Vec<GuildMembership>> {
    let mut req = account_api(&state, reqwest::Method::GET, "/guilds/memberships").await?;
    if let Some(id) = instance_id {
        req = req.query(&[("instance_id", id)]);
    }
    let resp = req.send().await.map_err(err)?;
    if !resp.status().is_success() {
        return Err(guild_response_error(resp, "could not load faction memberships").await);
    }
    resp.json().await.map_err(err)
}

#[tauri::command]
pub async fn set_primary_guild(state: State<'_, AppState>, guild_id: String, server_id: i64) -> Res<()> {
    let resp = account_api(&state, reqwest::Method::POST, &format!("/guilds/{guild_id}/primary"))
        .await?
        .json(&serde_json::json!({"server_id":server_id}))
        .send()
        .await
        .map_err(err)?;
    if !resp.status().is_success() {
        return Err(guild_response_error(resp, "could not set primary faction").await);
    }
    Ok(())
}

#[tauri::command]
pub async fn get_guild_relations(state: State<'_, AppState>, guild_id: String) -> Res<Vec<serde_json::Value>> {
    let resp = account_api(&state, reqwest::Method::GET, &format!("/guilds/{guild_id}/relations")).await?.send().await.map_err(err)?;
    if !resp.status().is_success() {
        return Err(guild_response_error(resp, "could not load faction relations").await);
    }
    resp.json().await.map_err(err)
}

#[tauri::command]
pub async fn create_guild_relation(state: State<'_, AppState>, guild_id: String, other_guild_id: String, relation: String, reward_cents: Option<i64>, reward_bps: Option<i64>, duration_hours: Option<i64>) -> Res<()> {
    let resp = account_api(&state, reqwest::Method::POST, &format!("/guilds/{guild_id}/relations"))
        .await?
        .json(&serde_json::json!({"other_guild_id":other_guild_id,"relation":relation,"reward_cents":reward_cents,"reward_bps":reward_bps,"duration_hours":duration_hours}))
        .send()
        .await
        .map_err(err)?;
    if !resp.status().is_success() {
        return Err(guild_response_error(resp, "could not create faction relation").await);
    }
    Ok(())
}

#[tauri::command]
pub async fn respond_guild_relation(state: State<'_, AppState>, guild_id: String, relation_id: i64, accept: bool, expected_revision: Option<String>) -> Res<()> {
    let resp = account_api(&state, reqwest::Method::POST, &format!("/guilds/{guild_id}/relations/{relation_id}/respond"))
        .await?
        .json(&serde_json::json!({"accept":accept,"expected_revision":expected_revision}))
        .send()
        .await
        .map_err(err)?;
    if !resp.status().is_success() {
        return Err(guild_response_error(resp, "could not respond to faction relation").await);
    }
    Ok(())
}

async fn guild_response_error(resp: reqwest::Response, fallback: &str) -> String {
    let body: serde_json::Value = resp.json().await.unwrap_or_default();
    body["error"].as_str().unwrap_or(fallback).to_string()
}

/// A guild's land rules (what visitors may do, PVP, mob spawning…) with the catalogue the page draws.
#[tauri::command]
pub async fn get_guild_claim_flags(state: State<'_, AppState>, guild_id: String) -> Res<serde_json::Value> {
    let resp = account_api(&state, reqwest::Method::GET, &format!("/guilds/{guild_id}/claim-flags")).await?.send().await.map_err(err)?;
    if !resp.status().is_success() {
        return Err(guild_response_error(resp, "could not load the land rules").await);
    }
    resp.json().await.map_err(err)
}

/// Change some of a guild's land rules. Only the rules in `flags` change.
#[tauri::command]
pub async fn set_guild_claim_flags(state: State<'_, AppState>, guild_id: String, flags: serde_json::Value) -> Res<serde_json::Value> {
    let resp = account_api(&state, reqwest::Method::PUT, &format!("/guilds/{guild_id}/claim-flags"))
        .await?
        .json(&serde_json::json!({ "flags": flags }))
        .send()
        .await
        .map_err(err)?;
    if !resp.status().is_success() {
        return Err(guild_response_error(resp, "could not change the land rules").await);
    }
    resp.json().await.map_err(err)
}

#[tauri::command]
pub async fn request_guild_join(state: State<'_, AppState>, guild_id: String, message: String) -> Res<()> {
    let req = account_api(&state, reqwest::Method::POST, &format!("/guilds/{guild_id}/requests")).await?;
    let resp = req.json(&serde_json::json!({"message": message})).send().await.map_err(err)?;
    if !resp.status().is_success() {
        return Err(guild_response_error(resp, "could not request to join faction").await);
    }
    Ok(())
}

#[tauri::command]
pub async fn get_guild_join_requests(state: State<'_, AppState>, guild_id: String) -> Res<serde_json::Value> {
    let resp = account_api(&state, reqwest::Method::GET, &format!("/guilds/{guild_id}/requests")).await?.send().await.map_err(err)?;
    if !resp.status().is_success() {
        return Err(guild_response_error(resp, "could not load faction requests").await);
    }
    resp.json().await.map_err(err)
}

#[tauri::command]
pub async fn respond_guild_join_request(state: State<'_, AppState>, guild_id: String, uuid: String, accept: bool) -> Res<()> {
    let req = account_api(&state, reqwest::Method::POST, &format!("/guilds/{guild_id}/requests/{uuid}/respond")).await?;
    let resp = req.json(&serde_json::json!({"accept": accept})).send().await.map_err(err)?;
    if !resp.status().is_success() {
        return Err(guild_response_error(resp, "could not review faction request").await);
    }
    Ok(())
}

#[tauri::command]
pub async fn get_guild_roles(state: State<'_, AppState>, guild_id: String) -> Res<serde_json::Value> {
    let resp = account_api(&state, reqwest::Method::GET, &format!("/guilds/{guild_id}/roles")).await?.send().await.map_err(err)?;
    if !resp.status().is_success() {
        return Err(guild_response_error(resp, "could not load faction roles").await);
    }
    resp.json().await.map_err(err)
}

#[tauri::command]
pub async fn create_guild_role(state: State<'_, AppState>, guild_id: String, role: serde_json::Value) -> Res<()> {
    let resp =
        account_api(&state, reqwest::Method::POST, &format!("/guilds/{guild_id}/roles")).await?.json(&role).send().await.map_err(err)?;
    if !resp.status().is_success() {
        return Err(guild_response_error(resp, "could not create faction role").await);
    }
    Ok(())
}

#[tauri::command]
pub async fn assign_guild_role(state: State<'_, AppState>, guild_id: String, uuid: String, role: String) -> Res<()> {
    let resp = account_api(&state, reqwest::Method::PUT, &format!("/guilds/{guild_id}/members/{uuid}/role"))
        .await?
        .json(&serde_json::json!({"role":role}))
        .send()
        .await
        .map_err(err)?;
    if !resp.status().is_success() {
        return Err(guild_response_error(resp, "could not assign faction role").await);
    }
    Ok(())
}

/// Remove a member (leaders and officers; only the leader can remove officers).
#[tauri::command]
pub async fn kick_guild_member(state: State<'_, AppState>, guild_id: String, uuid: String) -> Res<()> {
    let resp =
        account_api(&state, reqwest::Method::DELETE, &format!("/guilds/{guild_id}/members/{uuid}")).await?.send().await.map_err(err)?;
    if !resp.status().is_success() {
        return Err(guild_response_error(resp, "could not remove that member").await);
    }
    Ok(())
}

/// Hand the guild to another member; the old leader becomes an officer.
#[tauri::command]
pub async fn transfer_guild_leader(state: State<'_, AppState>, guild_id: String, uuid: String) -> Res<()> {
    let resp = account_api(&state, reqwest::Method::PUT, &format!("/guilds/{guild_id}/leader"))
        .await?
        .json(&serde_json::json!({ "uuid": uuid }))
        .send()
        .await
        .map_err(err)?;
    if !resp.status().is_success() {
        return Err(guild_response_error(resp, "could not hand over leadership").await);
    }
    Ok(())
}

#[tauri::command]
pub async fn delete_guild_role(state: State<'_, AppState>, guild_id: String, role_id: i64) -> Res<()> {
    let resp =
        account_api(&state, reqwest::Method::DELETE, &format!("/guilds/{guild_id}/roles/{role_id}")).await?.send().await.map_err(err)?;
    if !resp.status().is_success() {
        return Err(guild_response_error(resp, "could not delete faction role").await);
    }
    Ok(())
}

#[tauri::command]
pub async fn update_guild(
    state: State<'_, AppState>,
    guild_id: String,
    description: String,
    motd: String,
    icon_url: String,
    banner_url: String,
) -> Res<()> {
    let resp = account_api(&state, reqwest::Method::PUT, &format!("/guilds/{guild_id}"))
        .await?
        .json(&serde_json::json!({"description":description,"motd":motd,"icon_url":icon_url,"banner_url":banner_url}))
        .send()
        .await
        .map_err(err)?;
    if !resp.status().is_success() {
        return Err(guild_response_error(resp, "could not update faction").await);
    }
    Ok(())
}

#[tauri::command]
pub async fn create_guild(
    state: State<'_, AppState>,
    instance_id: String,
    name: String,
    tag: String,
    description: String,
    icon_url: Option<String>,
    banner_url: Option<String>,
) -> Res<Guild> {
    let req = account_api(&state, reqwest::Method::POST, "/guilds").await?;
    let resp = req
        .json(&serde_json::json!({
            "instance_id": instance_id,
            "name": name,
            "tag": tag,
            "description": description,
            "icon_url": icon_url,
            "banner_url": banner_url,
        }))
        .send()
        .await
        .map_err(err)?;
    if !resp.status().is_success() {
        let body: serde_json::Value = resp.json().await.unwrap_or_default();
        return Err(body["error"].as_str().unwrap_or("unable to create faction").to_string());
    }
    resp.json().await.map_err(err)
}

#[tauri::command]
pub async fn get_guild_claims(
    state: State<'_, AppState>,
    instance_id: String,
    server_id: i64,
    dimension: Option<String>,
    center_x: i32,
    center_z: i32,
    guild_id: Option<String>,
    radius: Option<i32>,
) -> Res<Vec<GuildClaim>> {
    let dim = dimension.unwrap_or_else(|| "minecraft:overworld".into());
    let mut query = vec![
        ("instance_id", instance_id),
        ("server_id", server_id.to_string()),
        ("dimension", dim),
        ("center_x", center_x.to_string()),
        ("center_z", center_z.to_string()),
    ];
    if let Some(guild) = guild_id.filter(|g| !g.is_empty()) {
        query.push(("guild_id", guild));
    }
    if let Some(radius) = radius {
        query.push(("radius", radius.to_string()));
    }
    let resp = account_api(&state, reqwest::Method::GET, "/guilds/claims/grid").await?.query(&query).send().await.map_err(err)?;
    if !resp.status().is_success() {
        return Err("unable to load claims".into());
    }
    resp.json().await.map_err(err)
}

#[tauri::command]
pub async fn claim_guild_chunk(
    state: State<'_, AppState>,
    guild_id: String,
    instance_id: String,
    server_id: i64,
    dimension: String,
    chunk_x: i32,
    chunk_z: i32,
) -> Res<GuildClaim> {
    let req = account_api(&state, reqwest::Method::POST, &format!("/guilds/{guild_id}/claim")).await?;
    let resp = req
        .json(&serde_json::json!({
            "instance_id": instance_id,
            "server_id": server_id,
            "dimension": dimension,
            "chunk_x": chunk_x,
            "chunk_z": chunk_z,
        }))
        .send()
        .await
        .map_err(err)?;
    if !resp.status().is_success() {
        let body: serde_json::Value = resp.json().await.unwrap_or_default();
        return Err(body["error"].as_str().unwrap_or("unable to claim chunk").to_string());
    }
    resp.json().await.map_err(err)
}

/// The guild leader renames their guild (and optionally changes its tag).
#[tauri::command]
pub async fn rename_guild(state: State<'_, AppState>, guild_id: String, name: String, tag: Option<String>) -> Res<serde_json::Value> {
    let req = account_api(&state, reqwest::Method::PUT, &format!("/guilds/{guild_id}/name")).await?;
    let resp = req.json(&serde_json::json!({ "name": name, "tag": tag })).send().await.map_err(err)?;
    if !resp.status().is_success() {
        let body: serde_json::Value = resp.json().await.unwrap_or_default();
        return Err(body["error"].as_str().unwrap_or("unable to rename the faction").to_string());
    }
    resp.json().await.map_err(err)
}

/// The guild leader disbands their guild. The reply says how much money from the treasury was paid back.
#[tauri::command]
pub async fn disband_guild(state: State<'_, AppState>, guild_id: String) -> Res<serde_json::Value> {
    let req = account_api(&state, reqwest::Method::DELETE, &format!("/guilds/{guild_id}")).await?;
    let resp = req.send().await.map_err(err)?;
    if !resp.status().is_success() {
        let body: serde_json::Value = resp.json().await.unwrap_or_default();
        return Err(body["error"].as_str().unwrap_or("unable to disband the faction").to_string());
    }
    resp.json().await.map_err(err)
}

#[tauri::command]
pub async fn unclaim_guild_chunk(state: State<'_, AppState>, claim_id: i64) -> Res<()> {
    let req = account_api(&state, reqwest::Method::DELETE, &format!("/guilds/claims/{claim_id}")).await?;
    req.send().await.map_err(err)?.error_for_status().map_err(err)?;
    Ok(())
}

#[tauri::command]
pub async fn get_guild_posts(state: State<'_, AppState>, guild_id: String) -> Res<Vec<GuildPost>> {
    let req = account_api(&state, reqwest::Method::GET, &format!("/guilds/{guild_id}/posts")).await?;
    let resp = req.send().await.map_err(err)?;
    if !resp.status().is_success() {
        return Err("unable to load faction posts".into());
    }
    resp.json().await.map_err(err)
}

#[tauri::command]
pub async fn create_guild_post(state: State<'_, AppState>, guild_id: String, title: String, content: String) -> Res<GuildPost> {
    let req = account_api(&state, reqwest::Method::POST, &format!("/guilds/{guild_id}/posts")).await?;
    let resp = req
        .json(&serde_json::json!({
            "title": title,
            "content": content,
        }))
        .send()
        .await
        .map_err(err)?;
    if !resp.status().is_success() {
        let body: serde_json::Value = resp.json().await.unwrap_or_default();
        return Err(body["error"].as_str().unwrap_or("unable to post announcement").to_string());
    }
    resp.json().await.map_err(err)
}

#[tauri::command]
pub async fn get_guild_members(state: State<'_, AppState>, guild_id: String) -> Res<Vec<GuildMember>> {
    let req = account_api(&state, reqwest::Method::GET, &format!("/guilds/{guild_id}/members")).await?;
    let resp = req.send().await.map_err(err)?;
    if !resp.status().is_success() {
        return Err("unable to load members".into());
    }
    resp.json().await.map_err(err)
}

#[tauri::command]
pub async fn get_guild_wallet(state: State<'_, AppState>, guild_id: String, server_id: i64) -> Res<serde_json::Value> {
    let req = account_api(&state, reqwest::Method::GET, &format!("/guilds/{guild_id}/wallet?server_id={server_id}")).await?;
    req.send().await.map_err(err)?.error_for_status().map_err(err)?.json().await.map_err(err)
}

#[tauri::command]
pub async fn transfer_guild_wallet(
    state: State<'_, AppState>,
    guild_id: String,
    server_id: i64,
    amount: f64,
    withdraw: bool,
) -> Res<serde_json::Value> {
    let action = if withdraw { "withdraw" } else { "deposit" };
    let req = account_api(&state, reqwest::Method::POST, &format!("/guilds/{guild_id}/wallet/{action}")).await?;
    let resp = req.json(&serde_json::json!({"server_id":server_id,"amount":amount})).send().await.map_err(err)?;
    if !resp.status().is_success() {
        let body: serde_json::Value = resp.json().await.unwrap_or_default();
        return Err(body["error"].as_str().unwrap_or("Faction wallet transfer failed").to_string());
    }
    resp.json().await.map_err(err)
}

// ---------------------------------------------------------------------------
// Social: Friends, DMs, Invites, Profiles & Feed
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn get_friends(state: State<'_, AppState>) -> Res<Vec<FriendInfo>> {
    let req = account_api(&state, reqwest::Method::GET, "/friends").await?;
    let resp = req.send().await.map_err(err)?;
    if !resp.status().is_success() {
        return Err("unable to load friends".into());
    }
    resp.json().await.map_err(err)
}

#[tauri::command]
pub async fn send_friend_request(state: State<'_, AppState>, friend_username: Option<String>, username: Option<String>) -> Res<()> {
    let target = friend_username.or(username).ok_or("username is required")?;
    let req = account_api(&state, reqwest::Method::POST, "/friends/request").await?;
    let resp = req.json(&serde_json::json!({ "username": target })).send().await.map_err(err)?;
    if !resp.status().is_success() {
        let body: serde_json::Value = resp.json().await.unwrap_or_default();
        return Err(body["error"].as_str().unwrap_or("unable to send friend request").to_string());
    }
    Ok(())
}

#[tauri::command]
pub async fn respond_friend_request(
    state: State<'_, AppState>,
    friend_uuid: Option<String>,
    target_uuid: Option<String>,
    accept: bool,
) -> Res<()> {
    let target = friend_uuid.or(target_uuid).ok_or("target_uuid is required")?;
    let req = account_api(&state, reqwest::Method::POST, "/friends/respond").await?;
    let resp = req.json(&serde_json::json!({ "target_uuid": target, "accept": accept })).send().await.map_err(err)?;
    if !resp.status().is_success() {
        let body: serde_json::Value = resp.json().await.unwrap_or_default();
        return Err(body["error"].as_str().unwrap_or("unable to respond to friend request").to_string());
    }
    Ok(())
}

#[tauri::command]
pub async fn remove_friend(state: State<'_, AppState>, friend_uuid: Option<String>, target_uuid: Option<String>) -> Res<()> {
    let target = friend_uuid.or(target_uuid).ok_or("target_uuid is required")?;
    let req = account_api(&state, reqwest::Method::DELETE, &format!("/friends/{target}")).await?;
    let resp = req.send().await.map_err(err)?;
    if !resp.status().is_success() {
        let body: serde_json::Value = resp.json().await.unwrap_or_default();
        return Err(body["error"].as_str().unwrap_or("unable to remove friend").to_string());
    }
    Ok(())
}

#[tauri::command]
pub async fn search_members(state: State<'_, AppState>, query: String) -> Res<Vec<MemberProfile>> {
    let req = account_api(&state, reqwest::Method::GET, "/social/members").await?;
    // `.query` percent-encodes, so names with spaces, `&` or `#` search correctly.
    let resp = req.query(&[("q", query.trim())]).send().await.map_err(err)?;
    if !resp.status().is_success() {
        let body: serde_json::Value = resp.json().await.unwrap_or_default();
        return Err(body["error"].as_str().unwrap_or("unable to search members").to_string());
    }
    resp.json().await.map_err(err)
}

#[tauri::command]
pub async fn get_economy_balances(state: State<'_, AppState>) -> Res<Vec<ServerEconomyBalance>> {
    let req = account_api(&state, reqwest::Method::GET, "/economy/me").await?;
    let resp = req.send().await.map_err(err)?;
    if !resp.status().is_success() {
        return Err("unable to load economy balances".into());
    }
    resp.json().await.map_err(err)
}

#[tauri::command]
pub async fn get_server_baltop(state: State<'_, AppState>, server_id: i64) -> Res<Vec<BaltopEntry>> {
    let req = account_api(&state, reqwest::Method::GET, &format!("/servers/{server_id}/economy/baltop")).await?;
    let resp = req.send().await.map_err(err)?;
    if !resp.status().is_success() {
        return Err("unable to load baltop leaderboard".into());
    }
    resp.json().await.map_err(err)
}

#[tauri::command]
pub async fn get_transactions(state: State<'_, AppState>) -> Res<Vec<EconomyTransaction>> {
    let req = account_api(&state, reqwest::Method::GET, "/economy/transactions").await?;
    let resp = req.send().await.map_err(err)?;
    if !resp.status().is_success() {
        return Err("unable to load transactions".into());
    }
    resp.json().await.map_err(err)
}

#[tauri::command]
pub async fn get_direct_messages(
    state: State<'_, AppState>,
    friend_uuid: String,
    before_id: Option<i64>,
    mark_read: Option<bool>,
) -> Res<Vec<DirectMessage>> {
    // FIX #13: Support optional before_id for loading older messages beyond the first 100.
    let mut path = if let Some(before) = before_id {
        format!("/messages/{friend_uuid}?before_id={before}")
    } else {
        format!("/messages/{friend_uuid}")
    };
    if mark_read == Some(false) {
        path.push_str(if before_id.is_some() { "&mark_read=false" } else { "?mark_read=false" });
    }
    let req = account_api(&state, reqwest::Method::GET, &path).await?;
    let resp = req.send().await.map_err(err)?;
    if !resp.status().is_success() {
        return Err("unable to load messages".into());
    }
    resp.json().await.map_err(err)
}

#[tauri::command]
pub async fn send_direct_message(state: State<'_, AppState>, friend_uuid: String, content: String) -> Res<DirectMessage> {
    let req = account_api(&state, reqwest::Method::POST, &format!("/messages/{friend_uuid}")).await?;
    let resp = req.json(&serde_json::json!({ "content": content })).send().await.map_err(err)?;
    if !resp.status().is_success() {
        let body: serde_json::Value = resp.json().await.unwrap_or_default();
        return Err(body["error"].as_str().unwrap_or("unable to send message").to_string());
    }
    resp.json().await.map_err(err)
}

#[tauri::command]
pub async fn get_game_invites(state: State<'_, AppState>) -> Res<Vec<GameInvite>> {
    let req = account_api(&state, reqwest::Method::GET, "/invites").await?;
    let resp = req.send().await.map_err(err)?;
    if !resp.status().is_success() {
        return Err("unable to load invites".into());
    }
    resp.json().await.map_err(err)
}

#[tauri::command]
pub async fn send_game_invite(state: State<'_, AppState>, recipient_uuid: String, instance_id: String, server_id: Option<i64>) -> Res<()> {
    let req = account_api(&state, reqwest::Method::POST, "/invites").await?;
    let resp = req
        .json(&serde_json::json!({
            "recipient_uuid": recipient_uuid,
            "instance_id": instance_id,
            "server_id": server_id,
        }))
        .send()
        .await
        .map_err(err)?;
    if !resp.status().is_success() {
        let body: serde_json::Value = resp.json().await.unwrap_or_default();
        return Err(body["error"].as_str().unwrap_or("unable to send invite").to_string());
    }
    Ok(())
}

#[tauri::command]
pub async fn respond_game_invite(state: State<'_, AppState>, invite_id: String, accept: bool) -> Res<()> {
    let req = account_api(&state, reqwest::Method::POST, &format!("/invites/{invite_id}/respond")).await?;
    let resp = req.json(&serde_json::json!({ "action": if accept { "accept" } else { "decline" } })).send().await.map_err(err)?;
    if !resp.status().is_success() {
        let body: serde_json::Value = resp.json().await.unwrap_or_default();
        return Err(body["error"].as_str().unwrap_or("unable to respond to invite").to_string());
    }
    Ok(())
}

#[tauri::command]
pub async fn get_user_profile(state: State<'_, AppState>, uuid: String) -> Res<UserProfileView> {
    let req = account_api(&state, reqwest::Method::GET, &format!("/profiles/{uuid}")).await?;
    let resp = req.send().await.map_err(err)?;
    if !resp.status().is_success() {
        return Err("unable to load user profile".into());
    }
    resp.json().await.map_err(err)
}

#[tauri::command]
pub async fn update_my_profile(
    state: State<'_, AppState>,
    bio: String,
    banner_url: Option<String>,
    custom_badge: Option<String>,
    featured_achievement_id: Option<String>,
    accent_color: Option<String>,
) -> Res<UserProfileView> {
    let req = account_api(&state, reqwest::Method::PUT, "/profiles/me").await?;
    let resp = req
        .json(&serde_json::json!({
            "bio": bio,
            "banner_url": banner_url,
            "custom_badge": custom_badge,
            "featured_achievement_id": featured_achievement_id,
            "accent_color": accent_color,
        }))
        .send()
        .await
        .map_err(err)?;
    if !resp.status().is_success() {
        let body: serde_json::Value = resp.json().await.unwrap_or_default();
        return Err(body["error"].as_str().unwrap_or("unable to update profile").to_string());
    }
    let uuid = state.accounts.read().unwrap().active().ok_or("no active account")?.uuid.clone();
    get_user_profile(state, uuid).await
}

#[tauri::command]
pub async fn get_user_posts(state: State<'_, AppState>, user_uuid: Option<String>) -> Res<Vec<UserPost>> {
    let path = user_uuid.map(|u| format!("/posts?uuid={u}")).unwrap_or_else(|| "/posts".into());
    let req = account_api(&state, reqwest::Method::GET, &path).await?;
    let resp = req.send().await.map_err(err)?;
    if !resp.status().is_success() {
        return Err("unable to load posts".into());
    }
    resp.json().await.map_err(err)
}

#[tauri::command]
pub async fn create_user_post(state: State<'_, AppState>, content: String, image_url: Option<String>) -> Res<UserPost> {
    let req = account_api(&state, reqwest::Method::POST, "/profiles/me/posts").await?;
    let resp = req
        .json(&serde_json::json!({
            "content": content,
            "image_url": image_url,
        }))
        .send()
        .await
        .map_err(err)?;
    if !resp.status().is_success() {
        let body: serde_json::Value = resp.json().await.unwrap_or_default();
        return Err(body["error"].as_str().unwrap_or("unable to publish post").to_string());
    }
    resp.json().await.map_err(err)
}

#[tauri::command]
pub async fn like_user_post(state: State<'_, AppState>, post_id: i64) -> Res<bool> {
    let req = account_api(&state, reqwest::Method::POST, &format!("/posts/{post_id}/like")).await?;
    let resp = req.send().await.map_err(err)?;
    if !resp.status().is_success() {
        return Err("unable to like post".into());
    }
    let body: serde_json::Value = resp.json().await.map_err(err)?;
    Ok(body["liked"].as_bool().unwrap_or(false))
}

// ---------------------------------------------------------------------------
// Map actions and guild invitations
// ---------------------------------------------------------------------------

async fn ok_or_panel_error(resp: reqwest::Response, fallback: &str) -> Res<serde_json::Value> {
    if resp.status().is_success() {
        return Ok(resp.json().await.unwrap_or(serde_json::Value::Null));
    }
    let body: serde_json::Value = resp.json().await.unwrap_or_default();
    Err(body["error"].as_str().or(body["message"].as_str()).unwrap_or(fallback).to_string())
}

/// From the map: ask a player to teleport to, or send them a message in game.
#[tauri::command]
pub async fn map_action(state: State<'_, AppState>, server_id: i64, action: String, target_uuid: String, text: Option<String>) -> Res<()> {
    let req = account_api(&state, reqwest::Method::POST, &format!("/servers/{server_id}/map/actions")).await?;
    let resp = req
        .json(&serde_json::json!({ "action": action, "target_uuid": target_uuid, "text": text.unwrap_or_default() }))
        .send()
        .await
        .map_err(err)?;
    ok_or_panel_error(resp, "unable to do that right now").await.map(|_| ())
}

/// Invite a player to a guild you lead or officiate.
#[tauri::command]
pub async fn guild_send_invite(state: State<'_, AppState>, guild_id: String, target_uuid: String) -> Res<serde_json::Value> {
    let req = account_api(&state, reqwest::Method::POST, &format!("/guilds/{guild_id}/invites")).await?;
    let resp = req.json(&serde_json::json!({ "uuid": target_uuid })).send().await.map_err(err)?;
    ok_or_panel_error(resp, "unable to send the invitation").await
}

#[tauri::command]
pub async fn notifications_list(state: State<'_, AppState>) -> Res<serde_json::Value> {
    let req = account_api(&state, reqwest::Method::GET, "/notifications").await?;
    let resp = req.send().await.map_err(err)?;
    ok_or_panel_error(resp, "unable to load notifications").await
}

/// Mark some (or, with no ids, all) notifications as read; `clear` deletes them instead.
#[tauri::command]
pub async fn notifications_mark(state: State<'_, AppState>, ids: Option<Vec<i64>>, clear: Option<bool>) -> Res<()> {
    let req = if clear.unwrap_or(false) {
        account_api(&state, reqwest::Method::DELETE, "/notifications").await?
    } else {
        account_api(&state, reqwest::Method::POST, "/notifications/read").await?.json(&serde_json::json!({ "ids": ids }))
    };
    let resp = req.send().await.map_err(err)?;
    ok_or_panel_error(resp, "unable to update notifications").await.map(|_| ())
}

// ---- Market & auction house -------------------------------------------------------------------------

#[tauri::command]
pub async fn market_listings(state: State<'_, AppState>, server_id: i64) -> Res<serde_json::Value> {
    let resp = account_api(&state, reqwest::Method::GET, &format!("/market/{server_id}")).await?.send().await.map_err(err)?;
    ok_or_panel_error(resp, "unable to load the market").await
}

#[tauri::command]
pub async fn market_mine(state: State<'_, AppState>, server_id: i64) -> Res<serde_json::Value> {
    let resp = account_api(&state, reqwest::Method::GET, &format!("/market/{server_id}/mine")).await?.send().await.map_err(err)?;
    ok_or_panel_error(resp, "unable to load your market activity").await
}

/// `action` is `buy`, `bid` or `cancel`. `amount` is only used for bids (leave out to bid the minimum).
#[tauri::command]
pub async fn market_act(
    state: State<'_, AppState>,
    server_id: i64,
    action: String,
    listing_id: i64,
    amount: Option<f64>,
) -> Res<serde_json::Value> {
    if !["buy", "bid", "cancel"].contains(&action.as_str()) {
        return Err("unknown market action".into());
    }
    let req = account_api(&state, reqwest::Method::POST, &format!("/market/{server_id}/{action}")).await?;
    let resp = req.json(&serde_json::json!({ "listing_id": listing_id, "amount": amount })).send().await.map_err(err)?;
    ok_or_panel_error(resp, "the market refused that").await
}

// ---- Casino ----------------------------------------------------------------------------------------

/// A casino sub-path such as `/bounties` or `/markets/4/bet`: lowercase words, digits and slashes, plus an optional `?q=` search.
fn casino_path(path: &str) -> Res<String> {
    let (route, query) = path.split_once('?').unwrap_or((path, ""));
    let ok_route = route.is_empty()
        || (route.starts_with('/')
            && !route.contains("//")
            && route.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '/' || c == '_'));
    let ok_query = query.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '=' | '&' | '_' | '-' | '.' | '%'));
    if !ok_route || !ok_query || path.len() > 120 {
        return Err("unknown casino request".into());
    }
    Ok(path.to_string())
}

#[tauri::command]
pub async fn casino_get(state: State<'_, AppState>, server_id: i64, path: String) -> Res<serde_json::Value> {
    let path = casino_path(&path)?;
    let resp = account_api(&state, reqwest::Method::GET, &format!("/casino/{server_id}{path}")).await?.send().await.map_err(err)?;
    ok_or_panel_error(resp, "unable to reach the casino").await
}

#[tauri::command]
pub async fn casino_post(
    state: State<'_, AppState>,
    server_id: i64,
    path: String,
    body: Option<serde_json::Value>,
) -> Res<serde_json::Value> {
    let path = casino_path(&path)?;
    let resp = account_api(&state, reqwest::Method::POST, &format!("/casino/{server_id}{path}"))
        .await?
        .json(&body.unwrap_or_else(|| serde_json::json!({})))
        .send()
        .await
        .map_err(err)?;
    ok_or_panel_error(resp, "the casino refused that").await
}

/// Buy orders and contracts, served from `/board/{server}`. Same path rules as the casino.
#[tauri::command]
pub async fn vault_get(state: State<'_, AppState>, server_id: i64) -> Res<serde_json::Value> {
    let resp = account_api(&state, reqwest::Method::GET, &format!("/vaults/{server_id}"))
        .await?.send().await.map_err(err)?;
    ok_or_panel_error(resp, "unable to load vaults").await
}

#[tauri::command]
pub async fn faction_upgrades(state: State<'_, AppState>, guild_id: String, body: Option<serde_json::Value>) -> Res<serde_json::Value> {
    let method = if body.is_some() { reqwest::Method::POST } else { reqwest::Method::GET };
    uuid::Uuid::parse_str(&guild_id).map_err(|_| "invalid faction ID".to_string())?;
    let path = format!("/guilds/{guild_id}/upgrades");
    let mut req = account_api(&state, method, &path).await?;
    if let Some(body) = body { req = req.json(&body); }
    ok_or_panel_error(req.send().await.map_err(err)?, "unable to access faction upgrades").await
}

#[tauri::command]
pub async fn vault_post(state: State<'_, AppState>, server_id: i64, action: String, body: serde_json::Value) -> Res<serde_json::Value> {
    if !matches!(action.as_str(), "move" | "buy") { return Err("unknown vault action".into()); }
    let resp = account_api(&state, reqwest::Method::POST, &format!("/vaults/{server_id}/{action}"))
        .await?.json(&body).send().await.map_err(err)?;
    ok_or_panel_error(resp, "the vault refused that").await
}

#[tauri::command]
pub async fn board_get(state: State<'_, AppState>, server_id: i64, path: String) -> Res<serde_json::Value> {
    let path = casino_path(&path)?;
    let resp = account_api(&state, reqwest::Method::GET, &format!("/board/{server_id}{path}")).await?.send().await.map_err(err)?;
    ok_or_panel_error(resp, "unable to reach the board").await
}

#[tauri::command]
pub async fn board_post(
    state: State<'_, AppState>,
    server_id: i64,
    path: String,
    body: Option<serde_json::Value>,
) -> Res<serde_json::Value> {
    let path = casino_path(&path)?;
    let resp = account_api(&state, reqwest::Method::POST, &format!("/board/{server_id}{path}"))
        .await?
        .json(&body.unwrap_or_else(|| serde_json::json!({})))
        .send()
        .await
        .map_err(err)?;
    ok_or_panel_error(resp, "the board refused that").await
}

#[tauri::command]
pub async fn my_guild_invites(state: State<'_, AppState>) -> Res<serde_json::Value> {
    let req = account_api(&state, reqwest::Method::GET, "/guilds/invites").await?;
    let resp = req.send().await.map_err(err)?;
    ok_or_panel_error(resp, "unable to load invitations").await
}

#[tauri::command]
pub async fn respond_guild_invite(state: State<'_, AppState>, invite_id: i64, accept: bool) -> Res<serde_json::Value> {
    let path = format!("/guilds/invites/{invite_id}/{}", if accept { "accept" } else { "decline" });
    let req = account_api(&state, reqwest::Method::POST, &path).await?;
    let resp = req.send().await.map_err(err)?;
    ok_or_panel_error(resp, "unable to answer the invitation").await
}

#[tauri::command]
pub fn textures_status(state: State<'_, AppState>) -> velora_launcher_core::textures::Status {
    velora_launcher_core::textures::status(&state.layout)
}

/// Item textures as data URIs, keyed by the name asked for. Names with no texture are left out.
#[tauri::command]
pub fn item_textures(state: State<'_, AppState>, names: Vec<String>) -> std::collections::HashMap<String, String> {
    use base64::Engine;
    names
        .into_iter()
        .take(400)
        .filter_map(|n| {
            let png = velora_launcher_core::textures::find(&state.layout, &n)?;
            Some((n, format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(png))))
        })
        .collect()
}

/// Other Minecraft sprites (HUD hearts, effect icons, panorama…) as data URIs. Names are `kind/path.png`, e.g. `gui/hud/heart/full.png`.
#[tauri::command]
pub fn mc_sprites(state: State<'_, AppState>, names: Vec<String>) -> std::collections::HashMap<String, String> {
    use base64::Engine;
    names
        .into_iter()
        .take(200)
        .filter_map(|n| {
            let (kind, rel) = n.split_once('/')?;
            let png = velora_launcher_core::textures::find_sprite(&state.layout, kind, rel)?;
            Some((n, format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(png))))
        })
        .collect()
}

#[cfg(test)]
mod casino_path_tests {
    use super::casino_path;

    #[test]
    fn only_casino_sub_paths_are_allowed() {
        for ok in ["", "/slots", "/mines/start", "/bounties/12/cancel", "/players?q=ste", "/markets/3/bet"] {
            assert!(casino_path(ok).is_ok(), "{ok}");
        }
        for bad in ["/../admin/casino", "slots", "//slots", "/Slots", "/slots?x=<script>", "/a b", &"/a".repeat(80)] {
            assert!(casino_path(bad).is_err(), "{bad}");
        }
    }
}
