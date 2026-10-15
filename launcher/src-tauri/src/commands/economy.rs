use super::{account_api, err, Res};
use crate::panel_client::{ErrorPolicy, PanelClient, PanelRequest};
use crate::state::AppState;
use tauri::State;
use velora_shared::{BaltopEntry, EconomyTransaction, ServerEconomyBalance};

#[tauri::command]
pub async fn get_economy_balances(state: State<'_, AppState>) -> Res<Vec<ServerEconomyBalance>> {
    let req = account_api(&state, reqwest::Method::GET, "/economy/me").await?;
    let resp = req.panel_send().await.map_err(err)?;
    let resp = PanelClient::success(resp, ErrorPolicy::Fallback("unable to load economy balances")).await.map_err(err)?;
    PanelClient::json(resp, ErrorPolicy::Generic).await.map_err(err)
}

#[tauri::command]
pub async fn get_server_baltop(state: State<'_, AppState>, server_id: i64) -> Res<Vec<BaltopEntry>> {
    let req = account_api(&state, reqwest::Method::GET, &format!("/servers/{server_id}/economy/baltop")).await?;
    let resp = req.panel_send().await.map_err(err)?;
    let resp = PanelClient::success(resp, ErrorPolicy::Fallback("unable to load baltop leaderboard")).await.map_err(err)?;
    PanelClient::json(resp, ErrorPolicy::Generic).await.map_err(err)
}

#[tauri::command]
pub async fn get_transactions(state: State<'_, AppState>) -> Res<Vec<EconomyTransaction>> {
    let req = account_api(&state, reqwest::Method::GET, "/economy/transactions").await?;
    let resp = req.panel_send().await.map_err(err)?;
    let resp = PanelClient::success(resp, ErrorPolicy::Fallback("unable to load transactions")).await.map_err(err)?;
    PanelClient::json(resp, ErrorPolicy::Generic).await.map_err(err)
}
