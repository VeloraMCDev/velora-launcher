use super::*;
use crate::accounts::AccountsFile;
use crate::secrets::Secret;
use base64::Engine;
use std::path::PathBuf;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::time::timeout;

const LIMIT: Duration = Duration::from_secs(5);
const TOKEN: &str = "synthetic-bootstrap-token";

struct Fixture {
    state: AppState,
    directory: PathBuf,
}

impl Fixture {
    fn new(panel: &str) -> Self {
        let directory = std::env::temp_dir().join(format!("velora-bootstrap-credentials-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&directory).unwrap();
        std::fs::write(directory.join("secrets.key"), base64::engine::general_purpose::STANDARD.encode([42u8; 32])).unwrap();
        let mut state = AppState::new(directory.clone());
        state.http = reqwest::Client::builder().no_proxy().connect_timeout(LIMIT).timeout(LIMIT).build().unwrap();
        state
            .update_settings(|settings| {
                settings.panel_url = Some(panel.into());
                settings.selected_instance = Some("synthetic-original-instance".into());
            })
            .unwrap();
        let account = Account {
            id: "synthetic-bootstrap-account".into(),
            kind: "panel".into(),
            username: "BootstrapFixture".into(),
            uuid: "00000000-0000-4000-8000-000000000042".into(),
            panel_url: Some(panel.into()),
            role: None,
        };
        *state.accounts.write().unwrap() = AccountsFile { active: Some(account.id.clone()), accounts: vec![account.clone()] };
        state.accounts.read().unwrap().save(&directory.join("accounts.json")).unwrap();
        state.secrets.set(&account.id, Secret { panel_token: Some(TOKEN.into()), ..Secret::default() }).unwrap();
        Self { state, directory }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

struct Server(tokio::task::JoinHandle<String>);

impl Server {
    fn start(listener: TcpListener) -> Self {
        Self(tokio::spawn(async move {
            timeout(LIMIT, async move {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut headers = Vec::new();
                loop {
                    assert!(headers.len() < 8192, "fixture request headers exceeded limit");
                    headers.push(stream.read_u8().await.unwrap());
                    if headers.ends_with(b"\r\n\r\n") {
                        break;
                    }
                }
                let manifest = LauncherManifest {
                    api_version: 1,
                    panel_version: "synthetic-bootstrap-credentials".into(),
                    ..LauncherManifest::default()
                };
                let body = serde_json::to_vec(&manifest).unwrap();
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                stream.write_all(response.as_bytes()).await.unwrap();
                stream.write_all(&body).await.unwrap();
                stream.shutdown().await.unwrap();
                String::from_utf8(headers).unwrap()
            })
            .await
            .expect("loopback manifest server exceeded deadline")
        }))
    }

    async fn request(&mut self) -> String {
        timeout(LIMIT, &mut self.0).await.expect("fixture task exceeded deadline").unwrap()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.0.abort();
    }
}

async fn listener() -> (TcpListener, String) {
    let listener = timeout(LIMIT, TcpListener::bind("127.0.0.1:0")).await.unwrap().unwrap();
    let panel = format!("http://{}/Tenant", listener.local_addr().unwrap());
    (listener, panel)
}

fn authorization(headers: &str) -> Vec<&str> {
    headers
        .lines()
        .filter_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("authorization").then(|| value.trim())
        })
        .collect()
}

#[tokio::test]
async fn prospective_panel_fetch_is_anonymous_and_does_not_commit_old_context() {
    let (original_listener, original_panel) = listener().await;
    let (prospective_listener, prospective_panel) = listener().await;
    let fixture = Fixture::new(&original_panel);
    let settings = fixture.state.settings.read().unwrap().clone();
    let names = ["settings.json", "accounts.json", "secrets.bin", "secrets.key"];
    let before = names.map(|name| std::fs::read(fixture.directory.join(name)).unwrap());
    let mut prospective = Server::start(prospective_listener);

    let manifest = timeout(LIMIT, fetch_manifest(&fixture.state, &prospective_panel)).await.unwrap().unwrap();
    assert_eq!(manifest.panel_version, "synthetic-bootstrap-credentials");
    let request = prospective.request().await;
    assert!(request.starts_with("GET /Tenant/api/v1/launcher/manifest HTTP/1.1\r\n"));
    assert!(authorization(&request).is_empty(), "prospective Panel received old credentials");
    assert!(timeout(Duration::from_millis(100), original_listener.accept()).await.is_err(), "original Panel received a request");
    assert_eq!(*fixture.state.settings.read().unwrap(), settings);
    assert_eq!(fixture.state.accounts.read().unwrap().active().unwrap().panel_url.as_deref(), Some(original_panel.as_str()));
    assert_eq!(fixture.state.secrets.get("synthetic-bootstrap-account").panel_token.as_deref(), Some(TOKEN));
    assert!(fixture.state.manifest.read().unwrap().is_none());
    for (name, bytes) in names.into_iter().zip(before) {
        assert_eq!(std::fs::read(fixture.directory.join(name)).unwrap(), bytes, "fetch committed {name}");
    }
}

#[tokio::test]
async fn explicitly_matching_destination_fetch_receives_bearer() {
    let (destination_listener, destination) = listener().await;
    let fixture = Fixture::new(&destination);
    fixture.state.settings.write().unwrap().panel_url = Some("https://unrelated.example.invalid/OtherTenant".into());
    let mut server = Server::start(destination_listener);

    let manifest = timeout(LIMIT, fetch_manifest(&fixture.state, &format!("{destination}///"))).await.unwrap().unwrap();
    assert_eq!(manifest.api_version, 1);
    let request = server.request().await;
    assert!(request.starts_with("GET /Tenant/api/v1/launcher/manifest HTTP/1.1\r\n"));
    assert_eq!(authorization(&request), vec![format!("Bearer {TOKEN}")]);
    assert_eq!(fixture.state.settings.read().unwrap().panel_url.as_deref(), Some("https://unrelated.example.invalid/OtherTenant"));
}
