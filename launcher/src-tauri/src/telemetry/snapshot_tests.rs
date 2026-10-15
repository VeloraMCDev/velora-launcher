use super::*;
use crate::accounts::Account;
use crate::secrets::Secret;
use std::path::PathBuf;
use std::sync::{Barrier, TryLockError};
use std::time::Instant;

struct Fixture {
    state: Arc<AppState>,
    directory: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let directory = std::env::temp_dir().join(format!("velora-telemetry-snapshot-{}", uuid::Uuid::new_v4()));
        let mut state = AppState::new(directory.clone());
        state.http = reqwest::Client::builder().no_proxy().build().unwrap();
        state.settings.write().unwrap().panel_url = Some("https://panel.example.invalid/Tenant".into());
        let fixture = Self { state: Arc::new(state), directory };
        fixture.account("first", "FirstPlayer", Some("https://panel.example.invalid/Tenant"));
        fixture
            .state
            .secrets
            .set("first", Secret { panel_token: Some("synthetic-first-panel-token".into()), ..Secret::default() })
            .unwrap();
        fixture
    }

    fn account(&self, id: &str, username: &str, panel: Option<&str>) {
        let mut accounts = self.state.accounts.write().unwrap();
        accounts.accounts.retain(|account| account.id != id);
        accounts.upsert(Account {
            id: id.into(),
            username: username.into(),
            uuid: format!("synthetic-{id}-uuid"),
            kind: "panel".into(),
            panel_url: panel.map(str::to_owned),
            role: None,
        });
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.directory).unwrap();
    }
}

#[test]
fn credential_scope_uses_the_validated_request_base() {
    let fixture = Fixture::new();
    for (account_panel, expected) in [
        (Some("https://panel.example.invalid/Tenant"), true),
        (Some("https://PANEL.example.invalid:443/Tenant///"), true),
        (Some("https://panel.example.invalid/tenant"), false),
        (Some("https://panel.example.invalid/Tenant/other"), false),
        (Some("https://other.example.invalid/Tenant"), false),
        (Some("http://panel.example.invalid/Tenant"), false),
        (Some("https://panel.example.invalid:8443/Tenant"), false),
        (Some("https://user:pass@panel.example.invalid/Tenant"), false),
        (Some("https://panel.example.invalid/Tenant?scope=other"), false),
        (Some("https://panel.example.invalid/Tenant#other"), false),
        (Some("not a URL"), false),
        (None, false),
    ] {
        fixture.account("first", "FirstPlayer", account_panel);
        let recorder = capture(&fixture.state).unwrap();
        assert_eq!(recorder.token.is_some(), expected, "account Panel scope did not match expectation");
        assert_eq!(recorder.username.as_deref(), Some("FirstPlayer"));
        if expected {
            assert_eq!(recorder.token.as_deref(), Some("synthetic-first-panel-token"));
        }
    }
    assert_eq!(fixture.state.secrets.get("first").panel_token.as_deref(), Some("synthetic-first-panel-token"));
}

#[test]
fn invalid_destination_never_captures_a_bearer() {
    let fixture = Fixture::new();
    fixture.state.settings.write().unwrap().panel_url = Some("https://user:pass@panel.example.invalid/Tenant".into());
    fixture.account("first", "FirstPlayer", Some("https://user:pass@panel.example.invalid/Tenant"));
    assert!(capture(&fixture.state).unwrap().token.is_none());
}

#[test]
fn offline_identity_cannot_borrow_a_stored_panel_token() {
    let fixture = Fixture::new();
    fixture.state.accounts.write().unwrap().accounts[0].kind = "offline".into();
    assert!(capture(&fixture.state).unwrap().token.is_none());
}

#[test]
fn pending_transaction_still_fences_snapshot_credentials() {
    let fixture = Fixture::new();
    let journal = fixture.directory.join(crate::account_transaction::JOURNAL_NAME);
    let evidence = b"synthetic pending journal evidence";
    std::fs::write(&journal, evidence).unwrap();
    assert!(capture(&fixture.state).unwrap().token.is_none());
    assert_eq!(std::fs::read(&journal).unwrap(), evidence);
}

#[test]
fn settings_stay_stable_while_capture_waits_for_accounts() {
    let fixture = Fixture::new();
    let accounts = fixture.state.accounts.write().unwrap();
    let state = Arc::clone(&fixture.state);
    let started = Arc::new(Barrier::new(2));
    let capture_started = Arc::clone(&started);
    let task = std::thread::spawn(move || {
        capture_started.wait();
        capture(&state)
    });
    started.wait();
    let deadline = Instant::now() + Duration::from_secs(3);
    let observed_read_guard = loop {
        match fixture.state.settings.try_write() {
            Err(TryLockError::WouldBlock) => break true,
            Err(TryLockError::Poisoned(_)) => panic!("synthetic settings lock poisoned"),
            Ok(_) => {}
        }
        if Instant::now() >= deadline {
            break false;
        }
        std::thread::sleep(Duration::from_millis(1));
    };
    drop(accounts);
    let recorder = task.join().unwrap().unwrap();
    assert!(observed_read_guard, "capture released its destination snapshot before account acquisition");
    assert_eq!(recorder.token.as_deref(), Some("synthetic-first-panel-token"));
}

#[test]
fn concurrent_identity_changes_never_mix_snapshot_fields() {
    let fixture = Fixture::new();
    fixture.account("second", "SecondPlayer", Some("https://other.example.invalid/Other"));
    fixture.state.secrets.set("second", Secret { panel_token: Some("synthetic-second-panel-token".into()), ..Secret::default() }).unwrap();
    fixture.state.accounts.write().unwrap().active = Some("first".into());
    let state = Arc::clone(&fixture.state);
    let started = Arc::new(Barrier::new(2));
    let writer_started = Arc::clone(&started);
    let writer = std::thread::spawn(move || {
        writer_started.wait();
        for iteration in 0..2000 {
            let mut settings = state.settings.write().unwrap();
            let mut accounts = state.accounts.write().unwrap();
            let (id, panel) = if iteration % 2 == 0 {
                ("second", "https://other.example.invalid/Other")
            } else {
                ("first", "https://panel.example.invalid/Tenant")
            };
            settings.panel_url = Some(panel.into());
            accounts.active = Some(id.into());
        }
    });
    started.wait();
    let snapshots: Vec<_> = (0..2000).map(|_| capture(&fixture.state).unwrap()).collect();
    writer.join().unwrap();
    for recorder in snapshots {
        match recorder.panel.as_str() {
            "https://panel.example.invalid/Tenant" => {
                assert_eq!(recorder.username.as_deref(), Some("FirstPlayer"));
                assert_eq!(recorder.token.as_deref(), Some("synthetic-first-panel-token"));
            }
            "https://other.example.invalid/Other" => {
                assert_eq!(recorder.username.as_deref(), Some("SecondPlayer"));
                assert_eq!(recorder.token.as_deref(), Some("synthetic-second-panel-token"));
            }
            _ => panic!("unexpected synthetic Panel"),
        }
    }
}

#[tokio::test]
async fn retained_authenticated_event_uses_only_its_captured_destination() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let fixture = Fixture::new();
    let first = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let second = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    second.set_nonblocking(true).unwrap();
    let first_panel = format!("http://{}/Tenant", first.local_addr().unwrap());
    let second_panel = format!("http://{}/Other", second.local_addr().unwrap());
    fixture.state.settings.write().unwrap().panel_url = Some(first_panel.clone());
    fixture.account("first", "FirstPlayer", Some(&first_panel));
    let recorder = capture(&fixture.state).unwrap();
    fixture.state.settings.write().unwrap().panel_url = Some(second_panel.clone());
    fixture.account("second", "SecondPlayer", Some(&second_panel));
    fixture.state.secrets.set("second", Secret { panel_token: Some("synthetic-second-panel-token".into()), ..Secret::default() }).unwrap();
    let server = tokio::spawn(async move {
        tokio::time::timeout(Duration::from_secs(3), async move {
            let (mut stream, _) = first.accept().await.unwrap();
            let mut bytes = Vec::new();
            loop {
                let mut buffer = [0; 1024];
                let count = stream.read(&mut buffer).await.unwrap();
                assert_ne!(count, 0);
                bytes.extend_from_slice(&buffer[..count]);
                assert!(bytes.len() <= 8192, "synthetic request exceeded fixture bound");
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
                        assert!(headers.starts_with("POST /Tenant/api/v1/launcher/events HTTP/1.1\r\n"));
                        assert!(headers.lines().any(|line| line.eq_ignore_ascii_case("authorization: Bearer synthetic-first-panel-token")));
                        assert!(!headers.contains("synthetic-second-panel-token"));
                        let body: serde_json::Value = serde_json::from_slice(&bytes[header_end + 4..]).unwrap();
                        assert_eq!(body["username"], "FirstPlayer");
                        assert_eq!(body["kind"], "game_exit");
                        break;
                    }
                }
            }
            stream.write_all(b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await.unwrap();
        })
        .await
        .unwrap();
    });
    let result = recorder.send("game_exit", "synthetic-instance").await;
    server.await.unwrap();
    result.unwrap();
    assert_eq!(second.accept().unwrap_err().kind(), std::io::ErrorKind::WouldBlock);
}
