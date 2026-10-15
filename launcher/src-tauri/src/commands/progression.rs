use super::{account_api, err, Res};
use crate::panel_client::{ErrorPolicy, PanelClient, PanelRequest};
use crate::state::AppState;
use tauri::State;
use velora_shared::{Achievement, UserLevelInfo, UserQuest};

// ---------------------------------------------------------------------------
// Leveling & Quests
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn get_my_level(state: State<'_, AppState>) -> Res<UserLevelInfo> {
    let req = account_api(&state, reqwest::Method::GET, "/levels/me").await?;
    let resp = req.panel_send().await.map_err(err)?;
    let resp = PanelClient::success(resp, ErrorPolicy::Fallback("unable to load user level")).await.map_err(err)?;
    PanelClient::json(resp, ErrorPolicy::Generic).await.map_err(err)
}

/// What quests, achievements and rank milestones hand out besides XP, as short lines keyed `quest:<id>`.
#[tauri::command]
pub async fn get_reward_summaries(state: State<'_, AppState>) -> Res<serde_json::Value> {
    let req = account_api(&state, reqwest::Method::GET, "/reward-bundles").await?;
    let resp = req.panel_send().await.map_err(err)?;
    let resp = match PanelClient::success(resp, ErrorPolicy::Generic).await {
        Ok(response) => response,
        Err(_) => {
            return Ok(serde_json::json!({}));
        }
    };
    PanelClient::json(resp, ErrorPolicy::Generic).await.map_err(err)
}

#[tauri::command]
pub async fn get_my_quests(state: State<'_, AppState>) -> Res<Vec<UserQuest>> {
    let req = account_api(&state, reqwest::Method::GET, "/quests/my").await?;
    let resp = req.panel_send().await.map_err(err)?;
    let resp = PanelClient::success(resp, ErrorPolicy::Fallback("unable to load quests")).await.map_err(err)?;
    PanelClient::json(resp, ErrorPolicy::Generic).await.map_err(err)
}

#[tauri::command]
pub async fn claim_quest(state: State<'_, AppState>, quest_id: String) -> Res<UserLevelInfo> {
    let req = account_api(&state, reqwest::Method::POST, &format!("/quests/{quest_id}/claim")).await?;
    let resp = req.panel_send().await.map_err(err)?;
    PanelClient::empty(resp, ErrorPolicy::Fallback("unable to claim quest")).await.map_err(err)?;
    get_my_level(state).await
}

// ---------------------------------------------------------------------------
// Achievements
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn get_my_achievements(state: State<'_, AppState>) -> Res<Vec<Achievement>> {
    let req = account_api(&state, reqwest::Method::GET, "/achievements/my").await?;
    let resp = req.panel_send().await.map_err(err)?;
    let resp = PanelClient::success(resp, ErrorPolicy::Fallback("unable to load achievements")).await.map_err(err)?;
    PanelClient::json(resp, ErrorPolicy::Generic).await.map_err(err)
}

#[tauri::command]
pub async fn get_my_collections(state: State<'_, AppState>) -> Res<Vec<serde_json::Value>> {
    let req = account_api(&state, reqwest::Method::GET, "/collections/me").await?;
    let resp = req.panel_send().await.map_err(err)?;
    let resp = PanelClient::success(resp, ErrorPolicy::Fallback("unable to load collections")).await.map_err(err)?;
    PanelClient::json(resp, ErrorPolicy::Generic).await.map_err(err)
}

#[tauri::command]
pub async fn equip_collection_item(state: State<'_, AppState>, unlock_key: String, equipped: bool) -> Res<()> {
    let resp = account_api(&state, reqwest::Method::POST, "/collections/equip")
        .await?
        .json(&serde_json::json!({"unlock_key":unlock_key,"equipped":equipped}))
        .panel_send()
        .await
        .map_err(err)?;
    PanelClient::empty(resp, ErrorPolicy::Fallback("unable to update collection item")).await.map_err(err)?;
    Ok(())
}
