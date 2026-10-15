use super::{account_api, aerr, err, Res};
use crate::accounts::{self, Account};
use crate::panel_client::{ErrorPolicy, PanelClient, PanelRequest};
use crate::state::AppState;
use serde::Serialize;
use tauri::State;
use velora_shared::launcher_contracts::{DiscordConnectionResponse, DiscordInfoResponse, DiscordPollResponse, DiscordStartResponse};

// ---- accounts ----

#[tauri::command]
pub async fn login_panel(state: State<'_, AppState>, username: String, password: String) -> Res<Account> {
    accounts::login_panel(&state, &username, &password).await.map_err(aerr)
}

#[tauri::command]
pub async fn discord_sign_in_start(state: State<'_, AppState>) -> Res<serde_json::Value> {
    let panel = state.panel_url().ok_or("no panel configured")?;
    let response = PanelClient::new(state.http.clone(), &panel, None)
        .map_err(err)?
        .request(reqwest::Method::GET, "/auth/discord/start")
        .map_err(err)?
        .panel_send()
        .await
        .map_err(err)?;
    PanelClient::json_fields::<DiscordStartResponse>(response, ErrorPolicy::Account, &["url", "state"]).await.map_err(err)
}

#[tauri::command]
pub async fn discord_sign_in_poll(state: State<'_, AppState>, oauth_state: String) -> Res<serde_json::Value> {
    let panel = state.panel_url().ok_or("no panel configured")?;
    let response = PanelClient::new(state.http.clone(), &panel, None)
        .map_err(err)?
        .request(reqwest::Method::GET, "/auth/discord/poll")
        .map_err(err)?
        .query(&[("state", oauth_state)])
        .panel_send()
        .await
        .map_err(err)?;
    discord_poll_result(&state, &panel, PanelClient::json::<DiscordPollResponse>(response, ErrorPolicy::Account).await.map_err(err)?)
}

fn discord_poll_result(state: &AppState, panel: &str, response: DiscordPollResponse) -> Res<serde_json::Value> {
    match response {
        DiscordPollResponse::Authenticated(auth) => {
            let account = accounts::save_panel_account(state, panel, auth.into_auth_response()).map_err(aerr)?;
            Ok(serde_json::json!({"account":account}))
        }
        DiscordPollResponse::Linked(_) => Ok(serde_json::json!({"linked":true})),
        DiscordPollResponse::Pending(_) => Ok(serde_json::json!({"pending":true})),
    }
}

#[tauri::command]
pub async fn request_password_reset(state: State<'_, AppState>, email: String) -> Res<String> {
    let panel = state.panel_url().ok_or("no panel configured")?;
    let response = PanelClient::new(state.http.clone(), &panel, None)
        .map_err(err)?
        .request(reqwest::Method::POST, "/auth/forgot-password")
        .map_err(err)?
        .json(&serde_json::json!({"email":email}))
        .panel_send()
        .await
        .map_err(err)?;
    let response = PanelClient::success(response, ErrorPolicy::Account).await.map_err(err)?;
    let body: serde_json::Value = PanelClient::json(response, ErrorPolicy::Generic).await.map_err(err)?;
    Ok(body["message"].as_str().unwrap_or("Check your email for a reset link.").to_string())
}

#[tauri::command]
pub async fn discord_connection(state: State<'_, AppState>) -> Res<serde_json::Value> {
    let req = account_api(&state, reqwest::Method::GET, "/account/connections/discord").await?;
    let response = req.panel_send().await.map_err(err)?;
    PanelClient::json_fields::<DiscordConnectionResponse>(response, ErrorPolicy::Account, &["id", "name"]).await.map_err(err)
}

/// The community invite link and whether this account is linked.
#[tauri::command]
pub async fn discord_info(state: State<'_, AppState>) -> Res<serde_json::Value> {
    let req = account_api(&state, reqwest::Method::GET, "/account/discord").await?;
    let response = req.panel_send().await.map_err(err)?;
    PanelClient::json_fields::<DiscordInfoResponse>(response, ErrorPolicy::Account, &["invite_url", "linked", "display_name"])
        .await
        .map_err(err)
}

#[tauri::command]
pub async fn discord_link_start(state: State<'_, AppState>) -> Res<serde_json::Value> {
    let req = account_api(&state, reqwest::Method::GET, "/auth/discord/start?kind=link").await?;
    let response = req.panel_send().await.map_err(err)?;
    PanelClient::json_fields::<DiscordStartResponse>(response, ErrorPolicy::Account, &["url", "state"]).await.map_err(err)
}

#[tauri::command]
pub async fn discord_unlink(state: State<'_, AppState>) -> Res<()> {
    let req = account_api(&state, reqwest::Method::DELETE, "/account/connections/discord").await?;
    let response = req.panel_send().await.map_err(err)?;
    PanelClient::empty(response, ErrorPolicy::Account).await.map_err(err)?;
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
    accounts::select(&state, id).map_err(aerr)
}

#[tauri::command]
pub fn remove_account(state: State<'_, AppState>, id: String) -> Res<()> {
    crate::telemetry::background(&state, "logout", "");
    accounts::remove(&state, &id).map_err(aerr)
}

// ---- game ----

#[tauri::command]
pub async fn set_username(state: State<'_, AppState>, username: String, password: String) -> Res<accounts::Account> {
    accounts::rename_panel(&state, &username, &password).await.map_err(aerr)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Directory(std::path::PathBuf);

    impl Directory {
        fn new() -> Self {
            let directory = Self(std::env::temp_dir().join(format!("velora-discord-consumer-{}", uuid::Uuid::new_v4())));
            std::fs::create_dir(&directory.0).unwrap();
            directory
        }
    }

    impl Drop for Directory {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }

    fn authenticated() -> serde_json::Value {
        serde_json::json!({
            "token":"synthetic-panel-token",
            "user":{"id":17,"username":"FixturePlayer","uuid":"00000000-0000-0000-0000-000000000017","role":"player","groups":[]},
            "yggdrasil":{"access_token":"synthetic-game-token","client_token":"synthetic-client-token"},
            "account":{"username":"InjectedPlayer"},
            "password":"synthetic-extra-password"
        })
    }

    #[test]
    fn pending_and_linked_adapter_strip_injected_account_and_secrets() {
        let directory = Directory::new();
        let state = AppState::new(directory.0.clone());
        for field in ["pending", "linked"] {
            let mut fixture = serde_json::json!({
                "account":{"username":"InjectedPlayer"},
                "access_token":"synthetic-injected-token",
                "password":"synthetic-injected-password",
                "yggdrasil":{"access_token":"synthetic-injected-game-token"}
            });
            fixture[field] = serde_json::Value::Bool(true);
            let response = serde_json::from_value::<DiscordPollResponse>(fixture).unwrap();
            let actual = discord_poll_result(&state, "https://example.invalid/panel", response).unwrap();
            let mut expected = serde_json::json!({});
            expected[field] = serde_json::Value::Bool(true);
            assert_eq!(actual, expected);
        }
        assert!(state.accounts.read().unwrap().accounts.is_empty());
        assert!(!directory.0.join("accounts.json").exists());
        assert!(!directory.0.join("secrets.bin").exists());
    }

    #[test]
    fn malformed_auth_cannot_fall_through_to_non_auth_success() {
        let directory = Directory::new();
        let state = AppState::new(directory.0.clone());
        for fixture in [
            serde_json::json!({"pending":true,"token":"synthetic-token","user":null}),
            serde_json::json!({"linked":true,"token":null}),
            serde_json::json!({"pending":false}),
            serde_json::json!({"linked":false}),
        ] {
            assert!(serde_json::from_value::<DiscordPollResponse>(fixture).is_err());
        }
        let mut missing_session = authenticated();
        missing_session.as_object_mut().unwrap().remove("yggdrasil");
        assert!(discord_poll_result(&state, "https://example.invalid/panel", serde_json::from_value(missing_session).unwrap()).is_err());
        assert!(state.accounts.read().unwrap().accounts.is_empty());
        assert!(!directory.0.join("accounts.json").exists());
        assert!(!directory.0.join("secrets.bin").exists());
    }

    #[test]
    fn authenticated_adapter_returns_only_persisted_account_not_credentials() {
        let directory = Directory::new();
        let state = AppState::new(directory.0.clone());
        let actual =
            discord_poll_result(&state, "https://example.invalid/panel", serde_json::from_value(authenticated()).unwrap()).unwrap();
        let account = state.accounts.read().unwrap().active().unwrap().clone();
        assert_eq!(actual, serde_json::json!({"account":account}));
        assert_eq!(actual.as_object().unwrap().len(), 1);
        let encoded = actual.to_string();
        for forbidden in [
            "synthetic-panel-token",
            "synthetic-game-token",
            "synthetic-client-token",
            "synthetic-extra-password",
            "InjectedPlayer",
            "yggdrasil",
        ] {
            assert!(!encoded.contains(forbidden));
        }
        assert_eq!(accounts::AccountsFile::load(&directory.0.join("accounts.json")).active(), Some(&account));
        assert_eq!(state.secrets.get(&account.id).panel_token.as_deref(), Some("synthetic-panel-token"));
    }

    #[test]
    fn failed_adapter_persistence_never_returns_signed_in_success() {
        let directory = Directory::new();
        let state = AppState::new(directory.0.clone());
        let first = discord_poll_result(&state, "https://example.invalid/panel", serde_json::from_value(authenticated()).unwrap()).unwrap();
        let account = state.accounts.read().unwrap().active().unwrap().clone();
        let original_credentials = std::fs::read(directory.0.join("secrets.bin")).unwrap();
        let path = directory.0.join("accounts.json");
        std::fs::write(&path, b"synthetic concurrent account corruption").unwrap();
        let mut replacement = authenticated();
        replacement["token"] = serde_json::json!("synthetic-replacement-token");
        replacement["user"]["username"] = serde_json::json!("ChangedPlayer");
        assert!(discord_poll_result(&state, "https://example.invalid/panel", serde_json::from_value(replacement).unwrap()).is_err());
        assert_eq!(state.accounts.read().unwrap().active(), Some(&account));
        assert_eq!(first, serde_json::json!({"account":account}));
        assert_eq!(state.secrets.get(&account.id).panel_token.as_deref(), Some("synthetic-panel-token"));
        assert_eq!(std::fs::read(directory.0.join("secrets.bin")).unwrap(), original_credentials);
        assert_eq!(std::fs::read(path).unwrap(), b"synthetic concurrent account corruption");
    }
}
