use super::{account_api, err, ok_or_panel_error, Res};
use crate::panel_client::{ErrorPolicy, PanelClient, PanelRequest};
use crate::state::AppState;
use tauri::State;
use velora_shared::{DirectMessage, FriendInfo, GameInvite, MemberProfile};

// ---------------------------------------------------------------------------
// Social: Friends, DMs, Invites, Profiles & Feed
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn get_friends(state: State<'_, AppState>) -> Res<Vec<FriendInfo>> {
    let req = account_api(&state, reqwest::Method::GET, "/friends").await?;
    let resp = req.panel_send().await.map_err(err)?;
    let resp = PanelClient::success(resp, ErrorPolicy::Fallback("unable to load friends")).await.map_err(err)?;
    PanelClient::json(resp, ErrorPolicy::Generic).await.map_err(err)
}

#[tauri::command]
pub async fn send_friend_request(state: State<'_, AppState>, friend_username: Option<String>, username: Option<String>) -> Res<()> {
    let target = friend_username.or(username).ok_or("username is required")?;
    let req = account_api(&state, reqwest::Method::POST, "/friends/request").await?;
    let resp = req.json(&serde_json::json!({ "username": target })).panel_send().await.map_err(err)?;
    PanelClient::empty(resp, ErrorPolicy::Fallback("unable to send friend request")).await.map_err(err)?;
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
    let resp = req.json(&serde_json::json!({ "target_uuid": target, "accept": accept })).panel_send().await.map_err(err)?;
    PanelClient::empty(resp, ErrorPolicy::Fallback("unable to respond to friend request")).await.map_err(err)?;
    Ok(())
}

#[tauri::command]
pub async fn remove_friend(state: State<'_, AppState>, friend_uuid: Option<String>, target_uuid: Option<String>) -> Res<()> {
    let target = friend_uuid.or(target_uuid).ok_or("target_uuid is required")?;
    let req = account_api(&state, reqwest::Method::DELETE, &format!("/friends/{target}")).await?;
    let resp = req.panel_send().await.map_err(err)?;
    PanelClient::empty(resp, ErrorPolicy::Fallback("unable to remove friend")).await.map_err(err)?;
    Ok(())
}

#[tauri::command]
pub async fn search_members(state: State<'_, AppState>, query: String) -> Res<Vec<MemberProfile>> {
    let req = account_api(&state, reqwest::Method::GET, "/social/members").await?;
    // `.query` percent-encodes, so names with spaces, `&` or `#` search correctly.
    let resp = req.query(&[("q", query.trim())]).panel_send().await.map_err(err)?;
    let resp = PanelClient::success(resp, ErrorPolicy::Fallback("unable to search members")).await.map_err(err)?;
    PanelClient::json(resp, ErrorPolicy::Generic).await.map_err(err)
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
    let resp = req.panel_send().await.map_err(err)?;
    let resp = PanelClient::success(resp, ErrorPolicy::Fallback("unable to load messages")).await.map_err(err)?;
    PanelClient::json(resp, ErrorPolicy::Generic).await.map_err(err)
}

#[tauri::command]
pub async fn send_direct_message(state: State<'_, AppState>, friend_uuid: String, content: String) -> Res<DirectMessage> {
    let req = account_api(&state, reqwest::Method::POST, &format!("/messages/{friend_uuid}")).await?;
    let resp = req.json(&serde_json::json!({ "content": content })).panel_send().await.map_err(err)?;
    let resp = PanelClient::success(resp, ErrorPolicy::Fallback("unable to send message")).await.map_err(err)?;
    PanelClient::json(resp, ErrorPolicy::Generic).await.map_err(err)
}

#[tauri::command]
pub async fn get_game_invites(state: State<'_, AppState>) -> Res<Vec<GameInvite>> {
    let req = account_api(&state, reqwest::Method::GET, "/invites").await?;
    let resp = req.panel_send().await.map_err(err)?;
    let resp = PanelClient::success(resp, ErrorPolicy::Fallback("unable to load invites")).await.map_err(err)?;
    PanelClient::json(resp, ErrorPolicy::Generic).await.map_err(err)
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
        .panel_send()
        .await
        .map_err(err)?;
    PanelClient::empty(resp, ErrorPolicy::Fallback("unable to send invite")).await.map_err(err)?;
    Ok(())
}

#[tauri::command]
pub async fn respond_game_invite(state: State<'_, AppState>, invite_id: String, accept: bool) -> Res<()> {
    let req = account_api(&state, reqwest::Method::POST, &format!("/invites/{invite_id}/respond")).await?;
    let resp = req.json(&serde_json::json!({ "action": if accept { "accept" } else { "decline" } })).panel_send().await.map_err(err)?;
    PanelClient::empty(resp, ErrorPolicy::Fallback("unable to respond to invite")).await.map_err(err)?;
    Ok(())
}

#[tauri::command]
pub async fn notifications_list(state: State<'_, AppState>) -> Res<serde_json::Value> {
    let req = account_api(&state, reqwest::Method::GET, "/notifications").await?;
    let resp = req.panel_send().await.map_err(err)?;
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
    let resp = req.panel_send().await.map_err(err)?;
    ok_or_panel_error(resp, "unable to update notifications").await.map(|_| ())
}
