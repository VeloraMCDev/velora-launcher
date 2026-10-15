//! Account-attributed activity only; no passwords, game logs, chat, or local paths.
use crate::state::AppState;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use velora_shared::LaunchEvent;

/// Capture the identity when work starts, so switching accounts cannot
/// attribute a running game's exit to the newly selected player.
#[derive(Clone)]
pub struct Recorder {
    http: reqwest::Client,
    panel: String,
    token: Option<String>,
    username: Option<String>,
    settings: Arc<RwLock<crate::settings::Settings>>,
    shutdown: Arc<crate::shutdown::Shutdown>,
}

pub fn capture(state: &AppState) -> Option<Recorder> {
    let settings = state.settings.read().unwrap();
    if !settings.send_stats {
        return None;
    }
    let panel = AppState::panel_url_from_settings(&settings)?;
    let accounts = state.accounts.read().unwrap();
    let active = accounts.active()?;
    let token = crate::panel_context::token_for_account(state, active, &panel);

    Some(Recorder {
        http: state.http.clone(),
        panel,
        token,
        username: Some(active.username.clone()),
        settings: Arc::clone(&state.settings),
        shutdown: state.shutdown.clone(),
    })
}

impl Recorder {
    pub async fn send(&self, kind: &str, instance: &str) -> anyhow::Result<()> {
        if !self.settings.read().unwrap().send_stats {
            return Ok(());
        }
        let client = crate::panel_client::PanelClient::new(self.http.clone(), &self.panel, self.token.as_deref())?;
        let request = client.request(reqwest::Method::POST, "/launcher/events")?.timeout(Duration::from_secs(5)).json(&LaunchEvent {
            kind: kind.into(),
            instance_id: instance.into(),
            username: self.username.clone(),
        });
        let response = crate::panel_client::PanelClient::send(request).await?;
        crate::panel_client::PanelClient::empty(response, crate::panel_client::ErrorPolicy::Generic).await?;
        Ok(())
    }

    pub fn background(&self, kind: &str, instance: &str) {
        if !self.settings.read().unwrap().send_stats {
            return;
        }
        let Ok(work) = self.shutdown.enter() else { return };
        let recorder = self.clone();
        let kind = kind.to_owned();
        let instance = instance.to_owned();
        tauri::async_runtime::spawn(async move {
            let _work = work;
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

#[cfg(test)]
#[path = "telemetry/snapshot_tests.rs"]
mod snapshot_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accounts;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::path::PathBuf;

    struct Fixture {
        state: AppState,
        listener: TcpListener,
        directory: PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            let directory = std::env::temp_dir().join(format!("velora-telemetry-{}", uuid::Uuid::new_v4()));
            let state = AppState::new(directory.clone());
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            state.settings.write().unwrap().panel_url = Some(format!("http://{}", listener.local_addr().unwrap()));
            state.accounts.write().unwrap().upsert(accounts::Account {
                id: "synthetic-account".into(),
                kind: "offline".into(),
                username: "FixturePlayer".into(),
                uuid: "00000000-0000-0000-0000-000000000017".into(),
                panel_url: None,
                role: None,
            });
            Self { state, listener, directory }
        }

        fn assert_no_request(&self) {
            self.listener.set_nonblocking(true).unwrap();
            assert_eq!(self.listener.accept().unwrap_err().kind(), std::io::ErrorKind::WouldBlock);
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.directory).unwrap();
        }
    }

    #[tokio::test]
    async fn disabled_statistics_skip_capture_and_public_send() {
        let fixture = Fixture::new();
        fixture.state.settings.write().unwrap().send_stats = false;
        assert!(capture(&fixture.state).is_none());
        send(&fixture.state, "launch_failed", "synthetic-instance").await.unwrap();
        background(&fixture.state, "launch_failed", "synthetic-instance");
        fixture.assert_no_request();
    }

    #[tokio::test]
    async fn captured_recorder_observes_persisted_opt_out() {
        let fixture = Fixture::new();
        let recorder = capture(&fixture.state).unwrap();
        fixture.state.update_settings(|settings| settings.send_stats = false).unwrap();
        assert!(!crate::settings::Settings::load(&fixture.directory.join("settings.json")).send_stats);
        recorder.send("game_exit", "synthetic-instance").await.unwrap();
        recorder.background("game_exit", "synthetic-instance");
        fixture.assert_no_request();
    }

    #[tokio::test]
    async fn persisted_opt_out_survives_state_restart() {
        let fixture = Fixture::new();
        fixture.state.update_settings(|settings| settings.send_stats = false).unwrap();
        let restarted = AppState::new(fixture.directory.clone());
        assert!(!restarted.settings.read().unwrap().send_stats);
        assert!(capture(&restarted).is_none());
        send(&restarted, "launcher_open", "synthetic-instance").await.unwrap();
        fixture.assert_no_request();
    }

    #[test]
    fn failed_opt_out_save_preserves_enabled_preference() {
        let fixture = Fixture::new();
        std::fs::create_dir(fixture.directory.join("settings.json")).unwrap();
        assert!(fixture.state.update_settings(|settings| settings.send_stats = false).is_err());
        assert!(fixture.state.settings.read().unwrap().send_stats);
        assert!(capture(&fixture.state).is_some());
        fixture.assert_no_request();
    }

    #[tokio::test]
    async fn enabled_statistics_keep_captured_identity_and_request_contract() {
        let fixture = Fixture::new();
        let recorder = capture(&fixture.state).unwrap();
        fixture.state.accounts.write().unwrap().upsert(accounts::Account {
            id: "different-synthetic-account".into(),
            kind: "offline".into(),
            username: "DifferentPlayer".into(),
            uuid: "00000000-0000-0000-0000-000000000018".into(),
            panel_url: None,
            role: None,
        });
        let server = std::thread::spawn(move || {
            fixture.listener.set_nonblocking(true).unwrap();
            let deadline = std::time::Instant::now() + Duration::from_secs(8);
            let mut stream = loop {
                match fixture.listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(std::time::Instant::now() < deadline, "activity request did not arrive");
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("fixture accept failed: {error}"),
                }
            };
            stream.set_nonblocking(false).unwrap();
            stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
            stream.set_write_timeout(Some(Duration::from_secs(5))).unwrap();
            let mut bytes = Vec::new();
            loop {
                let mut buffer = [0; 1024];
                let count = stream.read(&mut buffer).unwrap();
                assert_ne!(count, 0);
                bytes.extend_from_slice(&buffer[..count]);
                if let Some(header_end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                    let headers = std::str::from_utf8(&bytes[..header_end]).unwrap();
                    let length: usize = headers
                        .lines()
                        .find_map(|line| {
                            let (name, value) = line.split_once(':')?;
                            name.eq_ignore_ascii_case("content-length").then(|| value.trim().parse().unwrap())
                        })
                        .unwrap();
                    if bytes.len() >= header_end + 4 + length {
                        assert!(headers.starts_with("POST /api/v1/launcher/events HTTP/1.1\r\n"));
                        assert!(!headers.to_ascii_lowercase().contains("authorization:"));
                        let body: serde_json::Value = serde_json::from_slice(&bytes[header_end + 4..]).unwrap();
                        assert_eq!(
                            body,
                            serde_json::json!({"kind":"game_exit","instance_id":"synthetic-instance","username":"FixturePlayer"})
                        );
                        break;
                    }
                }
            }
            stream.write_all(b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
        });
        recorder.send("game_exit", "synthetic-instance").await.unwrap();
        server.join().unwrap();
    }
}
