use super::Res;
use crate::panel_client::{ErrorPolicy, PanelClient};
use crate::state::AppState;

pub(super) async fn account_api(state: &AppState, method: reqwest::Method, path: &str) -> Res<reqwest::RequestBuilder> {
    let context = crate::panel_context::current(state).ok_or("no panel configured")?;
    let token = context.token.ok_or("sign in with a server account to change your skin")?;
    let mut request = PanelClient::new(state.http.clone(), &context.panel, Some(&token))
        .and_then(|client| client.request(method, path))
        .map_err(|failure| failure.to_string())?;
    if let Some(id) = &context.selected_instance {
        request = request.header("X-Velora-Instance", id);
    }
    Ok(request)
}

// ---------------------------------------------------------------------------
// Map actions and guild invitations
// ---------------------------------------------------------------------------

pub(super) async fn ok_or_panel_error(resp: reqwest::Response, fallback: &'static str) -> Res<serde_json::Value> {
    PanelClient::json_or_null(resp, ErrorPolicy::Fallback(fallback)).await.map_err(|failure| failure.to_string())
}

#[cfg(test)]
#[path = "panel_client/context_tests.rs"]
mod context_tests;
