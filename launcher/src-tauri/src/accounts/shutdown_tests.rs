use super::*;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::oneshot;

struct Directory(PathBuf);

impl Directory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("velora-rename-shutdown-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn authenticated(username: &str, token: &str) -> AuthResponse {
    serde_json::from_value(json!({
        "token":token,
        "user":{"id":17,"username":username,"uuid":"00000000-0000-0000-0000-000000000017","role":"player","groups":[]},
        "yggdrasil":{"access_token":"synthetic-game-token","client_token":"synthetic-client-token"}
    }))
    .unwrap()
}

#[tokio::test]
async fn admitted_rename_commits_remote_success_after_shutdown_closes_fresh_admission() {
    let directory = Directory::new();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let panel = format!("http://{}", listener.local_addr().unwrap());
    let mut state = AppState::new(directory.0.clone());
    state.http = reqwest::Client::builder().no_proxy().timeout(Duration::from_secs(5)).build().unwrap();
    state.update_settings(|settings| settings.panel_url = Some(panel.clone())).unwrap();
    let original = save_panel_account(&state, &panel, authenticated("FixturePlayer", "synthetic-initial-panel-token")).unwrap();
    let response = serde_json::to_vec(&authenticated("RenamedPlayer", "synthetic-renamed-panel-token")).unwrap();
    let (headers_sent, headers_received) = oneshot::channel();
    let (release, released) = oneshot::channel();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut headers = Vec::new();
        let mut byte = [0u8];
        while !headers.ends_with(b"\r\n\r\n") {
            stream.read_exact(&mut byte).await.unwrap();
            headers.push(byte[0]);
            assert!(headers.len() < 16384);
        }
        let headers = String::from_utf8(headers).unwrap();
        assert!(headers.starts_with("PUT /api/v1/account/username HTTP/1.1\r\n"));
        let length = headers
            .lines()
            .find_map(|line| {
                line.split_once(':')
                    .filter(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                    .map(|(_, value)| value.trim().parse::<usize>().unwrap())
            })
            .unwrap();
        assert!(length < 16384);
        let mut request = vec![0; length];
        stream.read_exact(&mut request).await.unwrap();
        let request: serde_json::Value = serde_json::from_slice(&request).unwrap();
        assert_eq!(request["username"], "RenamedPlayer");
        stream
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    response.len()
                )
                .as_bytes(),
            )
            .await
            .unwrap();
        headers_sent.send(()).unwrap();
        released.await.unwrap();
        stream.write_all(&response).await.unwrap();
    });
    let state = Arc::new(state);
    let worker_state = Arc::clone(&state);
    let rename = tokio::spawn(async move { rename_panel(&worker_state, "RenamedPlayer", "synthetic-password").await });
    tokio::time::timeout(Duration::from_secs(5), headers_received).await.unwrap().unwrap();
    assert!(state.shutdown.begin());
    assert!(!state.shutdown.mark_ready());
    assert!(state.mutation_lease().is_err());
    release.send(()).unwrap();
    let updated = tokio::time::timeout(Duration::from_secs(5), rename).await.unwrap().unwrap().unwrap();
    tokio::time::timeout(Duration::from_secs(5), server).await.unwrap().unwrap();
    assert_eq!(updated.id, original.id);
    assert_eq!(updated.username, "RenamedPlayer");
    assert_eq!(state.accounts.read().unwrap().active(), Some(&updated));
    assert_eq!(AccountsFile::load(&directory.0.join("accounts.json")).active(), Some(&updated));
    assert_eq!(state.secrets.get(&updated.id).panel_token.as_deref(), Some("synthetic-renamed-panel-token"));
    state.shutdown.wait_idle().await;
    assert!(state.shutdown.mark_ready());
}

#[test]
fn standalone_commit_after_closing_preserves_exact_saved_accounts_and_credentials() {
    let directory = Directory::new();
    let state = AppState::new(directory.0.clone());
    let panel = "https://example.invalid/panel";
    let account = save_panel_account(&state, panel, authenticated("FixturePlayer", "synthetic-initial-panel-token")).unwrap();
    let accounts = std::fs::read(directory.0.join("accounts.json")).unwrap();
    let credentials = std::fs::read(directory.0.join("secrets.bin")).unwrap();
    assert!(state.shutdown.begin());
    let error = save_panel_account(&state, panel, authenticated("LatePlayer", "synthetic-late-panel-token")).unwrap_err();
    assert_eq!(error.to_string(), "launcher is shutting down");
    assert_eq!(std::fs::read(directory.0.join("accounts.json")).unwrap(), accounts);
    assert_eq!(std::fs::read(directory.0.join("secrets.bin")).unwrap(), credentials);
    assert_eq!(state.accounts.read().unwrap().active(), Some(&account));
    assert_eq!(state.secrets.get(&account.id).panel_token.as_deref(), Some("synthetic-initial-panel-token"));
    assert!(state.shutdown.mark_ready());
}
