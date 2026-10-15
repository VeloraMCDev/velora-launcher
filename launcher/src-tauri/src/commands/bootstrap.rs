use super::{aerr, Res};
use crate::accounts::Account;
use crate::panel_client::{ErrorPolicy, PanelClient};
use crate::settings::Settings;
use crate::state::{build, AppState};
use serde::Serialize;
use tauri::{AppHandle, Manager, State};
use velora_shared::LauncherManifest;

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
    let token = crate::panel_context::token_for_panel(state, panel);
    let client = PanelClient::new(state.http.clone(), panel, token.as_deref())?;
    let response = PanelClient::send(client.request(reqwest::Method::GET, "/launcher/manifest")?).await?;
    Ok(PanelClient::json::<LauncherManifest>(response, ErrorPolicy::Generic).await?)
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
    state.update_settings(|settings| settings.panel_url = Some(url)).map_err(aerr)?;
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
    state.update_settings(|settings| settings.selected_instance = instance_id).map_err(aerr)
}

#[tauri::command]
pub fn save_settings(state: State<'_, AppState>, settings: Settings) -> Res<()> {
    state
        .update_settings(|current| {
            *current =
                Settings { panel_url: current.panel_url.clone(), instance_game_options: current.instance_game_options.clone(), ..settings };
        })
        .map_err(aerr)
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
    state
        .update_settings(|settings| {
            settings.instance_game_options.insert(instance_id, options.clone());
        })
        .map_err(aerr)?;
    Ok(options)
}

#[tauri::command]
pub fn set_instance_fov(state: State<'_, AppState>, instance_id: String, fov: f64) -> Res<()> {
    if !(-1.0..=1.0).contains(&fov) || !fov.is_finite() {
        return Err("FOV must be between 30 and 110 degrees".into());
    }
    state
        .update_settings(|settings| {
            settings.instance_game_options.entry(instance_id).or_default().insert("fov".into(), format!("{fov:.3}"));
        })
        .map_err(aerr)
}

#[tauri::command]
pub fn show_window(app: AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        w.show().ok();
        w.set_focus().ok();
    }
}

#[cfg(test)]
#[path = "bootstrap/credential_tests.rs"]
mod credential_tests;
