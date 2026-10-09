//! Player accounts: panel accounts (authenticated by the panel's Yggdrasil
//! server through authlib-injector) and local offline accounts.

use crate::secrets::Secret;
use crate::state::AppState;
use anyhow::{anyhow, bail, Context, Result};
use velora_shared::{offline_uuid, valid_username, AuthResponse, LoginRequest, RegisterRequest};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::Path;

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
        let mut file: Self = std::fs::read(path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        // Microsoft accounts are no longer supported.
        file.accounts.retain(|a| a.kind == "panel" || a.kind == "offline");
        if file.active.as_ref().is_some_and(|id| !file.accounts.iter().any(|a| &a.id == id)) {
            file.active = file.accounts.first().map(|a| a.id.clone());
        }
        file
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        std::fs::write(path, serde_json::to_vec_pretty(self)?)?;
        Ok(())
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

pub(crate) async fn panel_error(resp: reqwest::Response) -> anyhow::Error {
    let status = resp.status();
    let body: serde_json::Value = resp.json().await.unwrap_or_default();
    anyhow!(
        "{}",
        body.get("error")
            .and_then(|e| e.as_str())
            .or_else(|| body.get("errorMessage").and_then(|e| e.as_str()))
            .map(String::from)
            .unwrap_or_else(|| format!("panel returned {status}"))
    )
}

pub async fn login_panel(state: &AppState, username: &str, password: &str) -> Result<Account> {
    let panel = state.panel_url().ok_or_else(|| anyhow!("no panel configured"))?;
    let resp = state
        .http
        .post(format!("{panel}/api/v1/auth/login"))
        .json(&LoginRequest { username: username.trim().into(), password: password.into() })
        .send()
        .await
        .context("couldn't reach the server")?;
    if !resp.status().is_success() {
        return Err(panel_error(resp).await);
    }
    let auth: AuthResponse = resp.json().await?;
    save_panel_account(state, &panel, auth)
}

pub async fn register_panel(state: &AppState, username: &str, password: &str, email: Option<String>) -> Result<(Option<Account>, bool)> {
    let panel = state.panel_url().ok_or_else(|| anyhow!("no panel configured"))?;
    let resp = state
        .http
        .post(format!("{panel}/api/v1/auth/register"))
        .json(&RegisterRequest { username: username.trim().into(), password: password.into(), email })
        .send()
        .await
        .context("couldn't reach the server")?;
    if !resp.status().is_success() {
        return Err(panel_error(resp).await);
    }
    let auth: AuthResponse = resp.json().await?;
    if auth.pending {
        return Ok((None, true));
    }
    Ok((Some(save_panel_account(state, &panel, auth)?), false))
}

pub(crate) fn save_panel_account(state: &AppState, panel: &str, auth: AuthResponse) -> Result<Account> {
    let account = Account {
        id: uuid::Uuid::new_v4().to_string(),
        kind: "panel".into(),
        username: auth.user.username.clone(),
        uuid: auth.user.uuid.clone(),
        panel_url: Some(panel.to_string()),
        role: Some(auth.user.role.clone()),
    };
    let account = state.accounts.write().unwrap().upsert(account);
    let ygg = auth.yggdrasil.ok_or_else(|| anyhow!("the server didn't start a game session — is the panel up to date?"))?;
    state.secrets.set(
        &account.id,
        Secret { panel_token: Some(auth.token), ygg_access_token: Some(ygg.access_token), ygg_client_token: Some(ygg.client_token) },
    )?;
    state.save_accounts()?;
    Ok(account)
}

pub async fn rename_panel(state: &AppState, username: &str, password: &str) -> Result<Account> {
    if !state.game.lock().unwrap().is_empty() || state.task.lock().unwrap().as_ref().is_some_and(|t| !t.is_finished()) {
        bail!("close Minecraft and wait for any installation to finish before changing your username");
    }
    let panel = state.panel_url().ok_or_else(|| anyhow!("no panel configured"))?;
    let token = panel_token(state).ok_or_else(|| anyhow!("sign in to your server account first"))?;
    let resp = state
        .http
        .put(format!("{panel}/api/v1/account/username"))
        .bearer_auth(token)
        .json(&json!({"username": username, "password": password}))
        .send()
        .await?;
    if !resp.status().is_success() {
        return Err(panel_error(resp).await);
    }
    save_panel_account(state, &panel, resp.json().await?)
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
    let account = state.accounts.write().unwrap().upsert(account);
    state.save_accounts()?;
    Ok(account)
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

/// Panel token for requests, when the active account belongs to this panel.
pub fn panel_token(state: &AppState) -> Option<String> {
    let panel = state.panel_url()?;
    let accounts = state.accounts.read().unwrap();
    let active = accounts.active()?;
    if active.kind != "panel" {
        return None;
    }
    if let Some(active_url) = &active.panel_url {
        let a = active_url.trim_end_matches('/');
        let p = panel.trim_end_matches('/');
        if !a.eq_ignore_ascii_case(p) {
            return None;
        }
    }
    state.secrets.get(&active.id).panel_token
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
