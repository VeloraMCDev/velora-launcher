use super::{account_api, err, ok_or_panel_error, Res};
use crate::panel_client::PanelRequest;
use crate::state::AppState;
use tauri::State;

// ---- Market & auction house -------------------------------------------------------------------------

#[tauri::command]
pub async fn market_listings(state: State<'_, AppState>, server_id: i64) -> Res<serde_json::Value> {
    let resp = account_api(&state, reqwest::Method::GET, &format!("/market/{server_id}")).await?.panel_send().await.map_err(err)?;
    ok_or_panel_error(resp, "unable to load the market").await
}

#[tauri::command]
pub async fn market_mine(state: State<'_, AppState>, server_id: i64) -> Res<serde_json::Value> {
    let resp = account_api(&state, reqwest::Method::GET, &format!("/market/{server_id}/mine")).await?.panel_send().await.map_err(err)?;
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
    let resp = req.json(&serde_json::json!({ "listing_id": listing_id, "amount": amount })).panel_send().await.map_err(err)?;
    ok_or_panel_error(resp, "the market refused that").await
}
