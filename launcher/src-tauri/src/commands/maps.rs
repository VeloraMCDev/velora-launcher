use super::{account_api, err, ok_or_panel_error, Res};
use crate::panel_client::{ErrorPolicy, PanelClient, PanelRequest};
use crate::state::AppState;
use tauri::State;

// ---- stats & leaderboards ----

#[tauri::command]
pub async fn get_player_stats(state: State<'_, AppState>) -> Res<serde_json::Value> {
    let req = account_api(&state, reqwest::Method::GET, "/account/stats").await?;
    let resp = req.panel_send().await.map_err(err)?;
    let resp = PanelClient::success(resp, ErrorPolicy::Fallback("unable to load player stats")).await.map_err(err)?;
    PanelClient::json(resp, ErrorPolicy::Generic).await.map_err(err)
}

#[tauri::command]
pub async fn get_leaderboard(state: State<'_, AppState>, server_id: Option<i64>, sort: Option<String>) -> Res<serde_json::Value> {
    let context = crate::panel_context::current(&state).ok_or("no panel configured")?;
    let endpoint = if let Some(id) = server_id { format!("/servers/{id}/leaderboard") } else { "/leaderboard".to_string() };
    let mut request = PanelClient::new(state.http.clone(), &context.panel, context.token.as_deref())
        .map_err(err)?
        .request(reqwest::Method::GET, &endpoint)
        .map_err(err)?;
    if let Some(sort) = sort {
        request = request.query(&[("sort", sort)]);
    }
    if let Some(id) = &context.selected_instance {
        request = request.header("X-Velora-Instance", id);
    }
    let resp = request.panel_send().await.map_err(err)?;
    let resp = PanelClient::success(resp, ErrorPolicy::Fallback("unable to load leaderboard")).await.map_err(err)?;
    PanelClient::json(resp, ErrorPolicy::Generic).await.map_err(err)
}

#[tauri::command]
pub async fn get_public_servers(state: State<'_, AppState>) -> Res<serde_json::Value> {
    let context = crate::panel_context::current(&state).ok_or("no panel configured")?;
    let mut request = PanelClient::new(state.http.clone(), &context.panel, context.token.as_deref())
        .map_err(err)?
        .request(reqwest::Method::GET, "/servers/public")
        .map_err(err)?;
    if let Some(id) = &context.selected_instance {
        request = request.header("X-Velora-Instance", id);
    }
    let resp = request.panel_send().await.map_err(err)?;
    let resp = PanelClient::success(resp, ErrorPolicy::Fallback("unable to load servers")).await.map_err(err)?;
    PanelClient::json(resp, ErrorPolicy::Generic).await.map_err(err)
}

/// What the Velora Map needs for one game server: which dimensions have tiles, the tile key, and where tiles live (made absolute,
/// because the map loads them straight from the panel as images).
#[tauri::command]
pub async fn get_map_info(state: State<'_, AppState>, server_id: i64) -> Res<serde_json::Value> {
    let panel = state.panel_url().ok_or("no panel configured")?;
    let req = account_api(&state, reqwest::Method::GET, &format!("/servers/{server_id}/map")).await?;
    let resp = req.panel_send().await.map_err(err)?;
    let resp = PanelClient::success(resp, ErrorPolicy::Fallback("unable to load the map")).await.map_err(err)?;
    let mut info: serde_json::Value = PanelClient::json(resp, ErrorPolicy::Generic).await.map_err(err)?;
    if let Some(base) = info["tile_base"].as_str().map(str::to_owned) {
        info["tile_base"] = serde_json::json!(format!("{}{}", panel.trim_end_matches('/'), base));
    }
    Ok(info)
}

/// Players now, plus the claims and pins the game server last sent.
#[tauri::command]
pub async fn get_map_overlay(state: State<'_, AppState>, server_id: i64) -> Res<serde_json::Value> {
    let req = account_api(&state, reqwest::Method::GET, &format!("/servers/{server_id}/map/overlay")).await?;
    let resp = req.panel_send().await.map_err(err)?;
    let resp = PanelClient::success(resp, ErrorPolicy::Fallback("unable to load the map")).await.map_err(err)?;
    PanelClient::json(resp, ErrorPolicy::Generic).await.map_err(err)
}

/// From the map: ask a player to teleport to, or send them a message in game.
#[tauri::command]
pub async fn map_action(state: State<'_, AppState>, server_id: i64, action: String, target_uuid: String, text: Option<String>) -> Res<()> {
    let req = account_api(&state, reqwest::Method::POST, &format!("/servers/{server_id}/map/actions")).await?;
    let resp = req
        .json(&serde_json::json!({ "action": action, "target_uuid": target_uuid, "text": text.unwrap_or_default() }))
        .panel_send()
        .await
        .map_err(err)?;
    ok_or_panel_error(resp, "unable to do that right now").await.map(|_| ())
}
