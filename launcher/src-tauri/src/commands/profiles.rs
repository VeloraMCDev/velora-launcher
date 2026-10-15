use super::{account_api, aerr, err, Res};
use crate::panel_client::{ErrorPolicy, PanelClient, PanelRequest};
use crate::state::AppState;
use tauri::State;
use velora_shared::{PlayerProfile, SkinProfile, UserPost, UserProfileView};

async fn profile_response(resp: reqwest::Response) -> Res<PlayerProfile> {
    let resp = PanelClient::success(resp, ErrorPolicy::Fallback("the server refused the change")).await.map_err(err)?;
    PanelClient::json(resp, ErrorPolicy::Generic).await.map_err(err)
}

#[tauri::command]
pub async fn account_profile(state: State<'_, AppState>) -> Res<PlayerProfile> {
    let req = account_api(&state, reqwest::Method::GET, "/account/profile").await?;
    profile_response(req.panel_send().await.map_err(err)?).await
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
    profile_response(req.multipart(form).panel_send().await.map_err(err)?).await
}

#[tauri::command]
pub async fn set_skin_model(state: State<'_, AppState>, model: String) -> Res<PlayerProfile> {
    let req = account_api(&state, reqwest::Method::PUT, "/account/skin/model").await?;
    profile_response(req.json(&serde_json::json!({ "model": model })).panel_send().await.map_err(err)?).await
}

#[tauri::command]
pub async fn delete_skin(state: State<'_, AppState>) -> Res<PlayerProfile> {
    let req = account_api(&state, reqwest::Method::DELETE, "/account/skin").await?;
    profile_response(req.panel_send().await.map_err(err)?).await
}

#[tauri::command]
pub async fn set_cape(state: State<'_, AppState>, cape_id: Option<i64>) -> Res<PlayerProfile> {
    let req = account_api(&state, reqwest::Method::PUT, "/account/cape").await?;
    profile_response(req.json(&serde_json::json!({ "cape_id": cape_id })).panel_send().await.map_err(err)?).await
}

#[tauri::command]
pub fn get_skin_profiles(state: State<'_, AppState>) -> Res<Vec<SkinProfile>> {
    let _guard = state.profiles.lock().unwrap();
    crate::profiles::load(&state.data_dir).map_err(aerr)
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
    let _work = state.shutdown.enter().map_err(|message| message.to_owned())?;
    let _guard = state.profiles.lock().unwrap();
    crate::profiles::update(&state.data_dir, |profiles| {
        if let Some(index) = profiles.iter().position(|existing| existing.id == profile.id) {
            profiles[index] = profile;
        } else {
            profiles.push(profile);
        }
    })
    .map_err(aerr)
}

#[tauri::command]
pub fn delete_skin_profile(state: State<'_, AppState>, id: String) -> Res<Vec<SkinProfile>> {
    let _work = state.shutdown.enter().map_err(|message| message.to_owned())?;
    let _guard = state.profiles.lock().unwrap();
    crate::profiles::update(&state.data_dir, |profiles| profiles.retain(|profile| profile.id != id)).map_err(aerr)
}

#[tauri::command]
pub async fn apply_skin_profile(state: State<'_, AppState>, id: String) -> Res<PlayerProfile> {
    use base64::Engine;
    let profiles = {
        let _guard = state.profiles.lock().unwrap();
        crate::profiles::load(&state.data_dir).map_err(aerr)?
    };
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
            profile_response(req.multipart(form).panel_send().await.map_err(err)?).await?;
        } else {
            let req = account_api(&state, reqwest::Method::PUT, "/account/skin/model").await?;
            profile_response(req.json(&serde_json::json!({ "model": p.skin_model })).panel_send().await.map_err(err)?).await?;
        }
    } else {
        let req = account_api(&state, reqwest::Method::PUT, "/account/skin/model").await?;
        profile_response(req.json(&serde_json::json!({ "model": p.skin_model })).panel_send().await.map_err(err)?).await?;
    }

    let req = account_api(&state, reqwest::Method::PUT, "/account/cape").await?;
    profile_response(req.json(&serde_json::json!({ "cape_id": p.cape_id })).panel_send().await.map_err(err)?).await?;

    account_profile(state).await
}

#[tauri::command]
pub async fn get_user_profile(state: State<'_, AppState>, uuid: String) -> Res<UserProfileView> {
    let req = account_api(&state, reqwest::Method::GET, &format!("/profiles/{uuid}")).await?;
    let resp = req.panel_send().await.map_err(err)?;
    let resp = PanelClient::success(resp, ErrorPolicy::Fallback("unable to load user profile")).await.map_err(err)?;
    PanelClient::json(resp, ErrorPolicy::Generic).await.map_err(err)
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
        .panel_send()
        .await
        .map_err(err)?;
    PanelClient::empty(resp, ErrorPolicy::Fallback("unable to update profile")).await.map_err(err)?;
    let uuid = state.accounts.read().unwrap().active().ok_or("no active account")?.uuid.clone();
    get_user_profile(state, uuid).await
}

#[tauri::command]
pub async fn get_user_posts(state: State<'_, AppState>, user_uuid: Option<String>) -> Res<Vec<UserPost>> {
    let mut req = account_api(&state, reqwest::Method::GET, "/posts").await?;
    if let Some(uuid) = user_uuid {
        req = req.query(&[("uuid", uuid)]);
    }
    let resp = req.panel_send().await.map_err(err)?;
    let resp = PanelClient::success(resp, ErrorPolicy::Fallback("unable to load posts")).await.map_err(err)?;
    PanelClient::json(resp, ErrorPolicy::Generic).await.map_err(err)
}

#[tauri::command]
pub async fn create_user_post(state: State<'_, AppState>, content: String, image_url: Option<String>) -> Res<UserPost> {
    let req = account_api(&state, reqwest::Method::POST, "/profiles/me/posts").await?;
    let resp = req
        .json(&serde_json::json!({
            "content": content,
            "image_url": image_url,
        }))
        .panel_send()
        .await
        .map_err(err)?;
    let resp = PanelClient::success(resp, ErrorPolicy::Fallback("unable to publish post")).await.map_err(err)?;
    PanelClient::json(resp, ErrorPolicy::Generic).await.map_err(err)
}

#[tauri::command]
pub async fn like_user_post(state: State<'_, AppState>, post_id: i64) -> Res<bool> {
    let req = account_api(&state, reqwest::Method::POST, &format!("/posts/{post_id}/like")).await?;
    let resp = req.panel_send().await.map_err(err)?;
    let resp = PanelClient::success(resp, ErrorPolicy::Fallback("unable to like post")).await.map_err(err)?;
    let body: serde_json::Value = PanelClient::json(resp, ErrorPolicy::Generic).await.map_err(err)?;
    Ok(body["liked"].as_bool().unwrap_or(false))
}
