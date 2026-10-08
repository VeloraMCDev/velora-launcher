//! Account-attributed activity only; no passwords, game logs, chat, or local paths.
use crate::{accounts, state::AppState};
use scopenet_shared::LaunchEvent;
use std::time::Duration;

/// Capture the identity when work starts, so switching accounts cannot
/// attribute a running game's exit to the newly selected player.
#[derive(Clone)]
pub struct Recorder {
    http: reqwest::Client,
    panel: String,
    token: Option<String>,
    username: Option<String>,
}

pub fn capture(state: &AppState) -> Option<Recorder> {
    // We need at least a panel URL to report to. Token is optional – offline
    // and Microsoft accounts won't have a panel token, but we still want to
    // fire events where possible (the endpoint will just record them as
    // unauthenticated). If there is no panel configured at all, skip silently.
    let panel = state.panel_url()?;
    let token = accounts::panel_token(state);
    let accounts = state.accounts.read().unwrap();
    let active = accounts.active()?;

    Some(Recorder { http: state.http.clone(), panel, token, username: Some(active.username.clone()) })
}

impl Recorder {
    pub async fn send(&self, kind: &str, instance: &str) -> anyhow::Result<()> {
        let mut req = self.http.post(format!("{}/api/v1/launcher/events", self.panel)).timeout(Duration::from_secs(5));
        if let Some(token) = &self.token {
            req = req.bearer_auth(token);
        }
        req.json(&LaunchEvent { kind: kind.into(), instance_id: instance.into(), username: self.username.clone() })
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    pub fn background(&self, kind: &str, instance: &str) {
        let recorder = self.clone();
        let kind = kind.to_owned();
        let instance = instance.to_owned();
        tauri::async_runtime::spawn(async move {
            if let Err(e) = recorder.send(&kind, &instance).await {
                tracing::warn!("activity report failed: {e}");
            }
        });
    }
}

pub async fn send(state: &AppState, kind: &str, instance: &str) -> anyhow::Result<()> {
    if let Some(recorder) = capture(state) {
        recorder.send(kind, instance).await?;
    }
    Ok(())
}

pub fn background(state: &AppState, kind: &str, instance: &str) {
    if let Some(recorder) = capture(state) {
        recorder.background(kind, instance);
    }
}
