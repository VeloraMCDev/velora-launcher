use super::{account_api, err, ok_or_panel_error, Res};
use crate::panel_client::PanelRequest;
use crate::state::AppState;
use tauri::State;

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
    let resp = account_api(&state, reqwest::Method::GET, &format!("/casino/{server_id}{path}")).await?.panel_send().await.map_err(err)?;
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
        .panel_send()
        .await
        .map_err(err)?;
    ok_or_panel_error(resp, "the casino refused that").await
}

/// Buy orders and contracts, served from `/board/{server}`. Same path rules as the casino.
#[tauri::command]
pub async fn vault_get(state: State<'_, AppState>, server_id: i64) -> Res<serde_json::Value> {
    let resp = account_api(&state, reqwest::Method::GET, &format!("/vaults/{server_id}")).await?.panel_send().await.map_err(err)?;
    ok_or_panel_error(resp, "unable to load vaults").await
}

#[tauri::command]
pub async fn vault_post(state: State<'_, AppState>, server_id: i64, action: String, body: serde_json::Value) -> Res<serde_json::Value> {
    if !matches!(action.as_str(), "move" | "buy") {
        return Err("unknown vault action".into());
    }
    let resp = account_api(&state, reqwest::Method::POST, &format!("/vaults/{server_id}/{action}"))
        .await?
        .json(&body)
        .panel_send()
        .await
        .map_err(err)?;
    ok_or_panel_error(resp, "the vault refused that").await
}

#[tauri::command]
pub async fn board_get(state: State<'_, AppState>, server_id: i64, path: String) -> Res<serde_json::Value> {
    let path = casino_path(&path)?;
    let resp = account_api(&state, reqwest::Method::GET, &format!("/board/{server_id}{path}")).await?.panel_send().await.map_err(err)?;
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
        .panel_send()
        .await
        .map_err(err)?;
    ok_or_panel_error(resp, "the board refused that").await
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
