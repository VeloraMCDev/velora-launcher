//! Player accounts: panel accounts (authenticated by the panel's Yggdrasil
//! server through authlib-injector) and local offline accounts.

use crate::panel_client::{ErrorPolicy, PanelClient};
use crate::secrets::Secret;
use crate::state::AppState;
use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::Path;
use velora_shared::{offline_uuid, valid_username, AuthResponse, LoginRequest, RegisterRequest};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Account {
    pub id: String,
    /// "panel" | "offline"
    pub kind: String,
    pub username: String,
    pub uuid: String,
    /// Panel accounts: which panel they belong to.
    #[serde(default)]
    pub panel_url: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AccountsFile {
    pub accounts: Vec<Account>,
    pub active: Option<String>,
}

impl AccountsFile {
    pub fn load(path: &Path) -> Self {
        let mut file: Self = crate::persistence::read_json(path).ok().flatten().unwrap_or_default();
        // Microsoft accounts are no longer supported.
        file.accounts.retain(|a| a.kind == "panel" || a.kind == "offline");
        if file.active.as_ref().is_some_and(|id| !file.accounts.iter().any(|a| &a.id == id)) {
            file.active = file.accounts.first().map(|a| a.id.clone());
        }
        file
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        crate::persistence::atomic_write_json(path, self)
    }

    pub fn update<Value>(current: &mut Self, path: &Path, change: impl FnOnce(&mut Self) -> Result<Value>) -> Result<Value> {
        let mut candidate = current.clone();
        let value = change(&mut candidate)?;
        candidate.save(path)?;
        *current = candidate;
        Ok(value)
    }

    pub fn active(&self) -> Option<&Account> {
        let id = self.active.as_ref()?;
        self.accounts.iter().find(|a| &a.id == id)
    }

    /// Add or replace (same kind + uuid + panel) and make it active.
    pub fn upsert(&mut self, account: Account) -> Account {
        if let Some(existing) =
            self.accounts.iter_mut().find(|a| a.kind == account.kind && a.uuid == account.uuid && a.panel_url == account.panel_url)
        {
            let id = existing.id.clone();
            *existing = Account { id: id.clone(), ..account };
            self.active = Some(id);
            return existing.clone();
        }
        self.active = Some(account.id.clone());
        self.accounts.push(account.clone());
        account
    }
}

pub fn select(state: &AppState, id: String) -> Result<()> {
    state.update_accounts(|accounts| {
        if !accounts.accounts.iter().any(|account| account.id == id) {
            bail!("account not found");
        }
        accounts.active = Some(id);
        Ok(())
    })
}

pub fn remove(state: &AppState, id: &str) -> Result<()> {
    let _work = state.shutdown.enter().map_err(anyhow::Error::msg)?;
    let mut current = state.accounts.write().unwrap();
    let mut candidate = current.clone();
    candidate.accounts.retain(|account| account.id != id);
    if candidate.active.as_deref() == Some(id) {
        candidate.active = candidate.accounts.first().map(|account| account.id.clone());
    }
    state.secrets.remove_with_accounts(id, &candidate)?;
    *current = candidate;
    Ok(())
}

pub async fn login_panel(state: &AppState, username: &str, password: &str) -> Result<Account> {
    let panel = state.panel_url().ok_or_else(|| anyhow!("no panel configured"))?;
    let client = PanelClient::new(state.http.clone(), &panel, None)?;
    let resp = PanelClient::send(
        client
            .request(reqwest::Method::POST, "/auth/login")?
            .json(&LoginRequest { username: username.trim().into(), password: password.into() }),
    )
    .await?;
    let auth: AuthResponse = PanelClient::json(resp, ErrorPolicy::Account).await?;
    save_panel_account(state, &panel, auth)
}

pub async fn register_panel(state: &AppState, username: &str, password: &str, email: Option<String>) -> Result<(Option<Account>, bool)> {
    let panel = state.panel_url().ok_or_else(|| anyhow!("no panel configured"))?;
    let client = PanelClient::new(state.http.clone(), &panel, None)?;
    let resp = PanelClient::send(client.request(reqwest::Method::POST, "/auth/register")?.json(&RegisterRequest {
        username: username.trim().into(),
        password: password.into(),
        email,
    }))
    .await?;
    let auth: AuthResponse = PanelClient::json(resp, ErrorPolicy::Account).await?;
    if auth.pending {
        return Ok((None, true));
    }
    Ok((Some(save_panel_account(state, &panel, auth)?), false))
}

pub(crate) fn save_panel_account(state: &AppState, panel: &str, auth: AuthResponse) -> Result<Account> {
    let _work = state.shutdown.enter().map_err(anyhow::Error::msg)?;
    commit_panel_account(state, panel, auth)
}

fn commit_panel_account(state: &AppState, panel: &str, auth: AuthResponse) -> Result<Account> {
    let account = Account {
        id: uuid::Uuid::new_v4().to_string(),
        kind: "panel".into(),
        username: auth.user.username.clone(),
        uuid: auth.user.uuid.clone(),
        panel_url: Some(panel.to_string()),
        role: Some(auth.user.role.clone()),
    };
    let ygg = auth.yggdrasil.ok_or_else(|| anyhow!("the server didn't start a game session — is the panel up to date?"))?;
    let mut current = state.accounts.write().unwrap();
    let mut candidate = current.clone();
    let account = candidate.upsert(account);
    state.secrets.set_with_accounts(
        &account.id,
        Secret { panel_token: Some(auth.token), ygg_access_token: Some(ygg.access_token), ygg_client_token: Some(ygg.client_token) },
        &candidate,
    )?;
    *current = candidate;
    Ok(account)
}

pub async fn rename_panel(state: &AppState, username: &str, password: &str) -> Result<Account> {
    let _lease = state
        .mutation_lease()
        .map_err(|_| anyhow!("close Minecraft and wait for any installation to finish before changing your username"))?;
    if !state.game.lock().unwrap().is_empty() {
        bail!("close Minecraft and wait for any installation to finish before changing your username");
    }
    let context = crate::panel_context::current(state).ok_or_else(|| anyhow!("no panel configured"))?;
    let panel = context.panel;
    let token = context.token.ok_or_else(|| anyhow!("sign in to your server account first"))?;
    let client = PanelClient::new(state.http.clone(), &panel, Some(&token))?;
    let resp = PanelClient::send(
        client.request(reqwest::Method::PUT, "/account/username")?.json(&json!({"username": username, "password": password})),
    )
    .await?;
    commit_panel_account(state, &panel, PanelClient::json(resp, ErrorPolicy::Account).await?)
}

pub fn add_offline(state: &AppState, username: &str) -> Result<Account> {
    let allowed = state.manifest.read().unwrap().as_ref().map(|m| m.auth.offline_local).unwrap_or(true);
    if !allowed {
        bail!("offline accounts are disabled on this server");
    }
    let name = username.trim();
    if !valid_username(name) {
        bail!("usernames are 3–16 letters, numbers or underscores");
    }
    let account = Account {
        id: uuid::Uuid::new_v4().to_string(),
        kind: "offline".into(),
        username: name.into(),
        uuid: offline_uuid(name),
        panel_url: None,
        role: None,
    };
    state.update_accounts(|accounts| Ok(accounts.upsert(account)))
}

/// The panel's Yggdrasil API root for an account.
pub fn yggdrasil_url(state: &AppState, account: &Account) -> Option<String> {
    let panel = account.panel_url.clone()?;
    let from_manifest = state
        .manifest
        .read()
        .unwrap()
        .as_ref()
        .filter(|_| state.panel_url().as_deref() == Some(panel.as_str()))
        .and_then(|m| m.auth.yggdrasil_url.clone());
    Some(from_manifest.unwrap_or_else(|| format!("{panel}/api/yggdrasil")))
}

/// Make sure the account's game session is valid, refreshing it if needed.
/// Returns the access token.
async fn ensure_session(state: &AppState, account: &Account, api: &str) -> Result<String> {
    let secret = state.secrets.get(&account.id);
    let (Some(access), Some(client)) = (secret.ygg_access_token.clone(), secret.ygg_client_token.clone()) else {
        bail!("please sign in to {} again", account.username);
    };
    let validate = state
        .http
        .post(format!("{api}/authserver/validate"))
        .json(&json!({ "accessToken": access, "clientToken": client }))
        .send()
        .await
        .context("couldn't reach the auth server")?;
    if validate.status().is_success() {
        return Ok(access);
    }
    let resp = state
        .http
        .post(format!("{api}/authserver/refresh"))
        .json(&json!({ "accessToken": access, "clientToken": client }))
        .send()
        .await
        .context("couldn't reach the auth server")?;
    if !resp.status().is_success() {
        bail!("your session for {} expired — please sign in again", account.username);
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Refreshed {
        access_token: String,
        client_token: String,
    }
    let r: Refreshed = resp.json().await?;
    let _work = state.shutdown.enter().map_err(anyhow::Error::msg)?;
    state
        .secrets
        .set(&account.id, Secret { ygg_access_token: Some(r.access_token.clone()), ygg_client_token: Some(r.client_token), ..secret })?;
    Ok(r.access_token)
}

/// Credentials passed to the game, plus the auth server URL for
/// authlib-injector (panel accounts only).
pub async fn game_auth(state: &AppState, account: &Account) -> Result<(velora_launcher_core::launch::Auth, Option<String>)> {
    use velora_launcher_core::launch::Auth;
    match account.kind.as_str() {
        "panel" => {
            let api = yggdrasil_url(state, account).ok_or_else(|| anyhow!("account has no server"))?;
            let access_token = ensure_session(state, account, &api).await?;
            Ok((
                Auth { username: account.username.clone(), uuid: account.uuid.clone(), access_token, user_type: "mojang".into() },
                Some(api),
            ))
        }
        _ => Ok((
            Auth {
                username: account.username.clone(),
                uuid: account.uuid.clone(),
                // Offline mode: any token works; offline servers ignore it.
                access_token: "0".into(),
                user_type: "legacy".into(),
            },
            None,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upsert_replaces_same_identity() {
        let mut f = AccountsFile::default();
        let a = Account { id: "1".into(), kind: "offline".into(), username: "Steve".into(), uuid: "u".into(), panel_url: None, role: None };
        f.upsert(a.clone());
        f.upsert(Account { id: "2".into(), ..a.clone() });
        assert_eq!(f.accounts.len(), 1);
        assert_eq!(f.active.as_deref(), Some("1"));
    }

    #[test]
    fn drops_microsoft_accounts_on_load() {
        let dir = std::env::temp_dir().join(format!("velora-acc-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("accounts.json");
        std::fs::write(
            &path,
            r#"{"accounts":[{"id":"m","kind":"microsoft","username":"A","uuid":"x"},{"id":"o","kind":"offline","username":"B","uuid":"y"}],"active":"m"}"#,
        )
        .unwrap();
        let f = AccountsFile::load(&path);
        assert_eq!(f.accounts.len(), 1);
        assert_eq!(f.active.as_deref(), Some("o"));
        std::fs::remove_dir_all(dir).ok();
    }
}
#[cfg(test)]
#[path = "accounts/persistence_tests.rs"]
mod persistence_tests;

#[cfg(test)]
#[path = "accounts/shutdown_tests.rs"]
mod shutdown_tests;

#[cfg(test)]
#[path = "accounts/restart_tests.rs"]
mod restart_tests;
