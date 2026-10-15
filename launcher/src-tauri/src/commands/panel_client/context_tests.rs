use super::*;
use crate::accounts::Account;
use crate::secrets::Secret;
use std::path::PathBuf;

struct Fixture {
    state: AppState,
    directory: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let directory = std::env::temp_dir().join(format!("velora-account-request-{}", uuid::Uuid::new_v4()));
        let state = AppState::new(directory.clone());
        {
            let mut settings = state.settings.write().unwrap();
            settings.panel_url = Some("https://panel.example.invalid/Tenant".into());
            settings.selected_instance = Some("synthetic-first-instance".into());
        }
        state.accounts.write().unwrap().upsert(Account {
            id: "synthetic-first".into(),
            kind: "panel".into(),
            username: "FixturePlayer".into(),
            uuid: "synthetic-first-uuid".into(),
            panel_url: Some("https://panel.example.invalid/Tenant".into()),
            role: None,
        });
        state.secrets.set("synthetic-first", Secret { panel_token: Some("synthetic-account-token".into()), ..Secret::default() }).unwrap();
        Self { state, directory }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.directory).unwrap();
    }
}

#[tokio::test]
async fn account_request_keeps_its_destination_bearer_and_selected_instance() {
    let fixture = Fixture::new();
    let request = account_api(&fixture.state, reqwest::Method::GET, "/account/stats").await.unwrap();
    {
        let mut settings = fixture.state.settings.write().unwrap();
        settings.panel_url = Some("https://other.example.invalid/Other".into());
        settings.selected_instance = Some("synthetic-other-instance".into());
    }
    fixture.state.accounts.write().unwrap().active = None;
    let request = request.build().unwrap();
    assert_eq!(request.url().as_str(), "https://panel.example.invalid/Tenant/api/v1/account/stats");
    assert_eq!(request.headers()[reqwest::header::AUTHORIZATION], "Bearer synthetic-account-token");
    assert_eq!(request.headers()["X-Velora-Instance"], "synthetic-first-instance");
}

#[tokio::test]
async fn unknown_or_different_account_origin_cannot_build_an_authenticated_request() {
    let fixture = Fixture::new();
    for panel in [None, Some("https://panel.example.invalid/tenant"), Some("https://other.example.invalid/Tenant")] {
        fixture.state.accounts.write().unwrap().accounts[0].panel_url = panel.map(str::to_owned);
        let error = account_api(&fixture.state, reqwest::Method::GET, "/account/stats").await.unwrap_err();
        assert_eq!(error, "sign in with a server account to change your skin");
    }
    assert_eq!(fixture.state.secrets.get("synthetic-first").panel_token.as_deref(), Some("synthetic-account-token"));
}

#[tokio::test]
async fn pending_transaction_cannot_build_an_authenticated_request() {
    let fixture = Fixture::new();
    let journal = fixture.directory.join(crate::account_transaction::JOURNAL_NAME);
    std::fs::write(&journal, b"synthetic pending evidence").unwrap();
    let error = account_api(&fixture.state, reqwest::Method::GET, "/account/stats").await.unwrap_err();
    assert_eq!(error, "sign in with a server account to change your skin");
    assert_eq!(std::fs::read(&journal).unwrap(), b"synthetic pending evidence");
}
