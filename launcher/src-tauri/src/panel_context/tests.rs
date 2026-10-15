use super::*;
use crate::accounts::{Account, AccountsFile};
use crate::secrets::Secret;
use base64::Engine;
use std::path::PathBuf;

const PANEL: &str = "https://panel.example.invalid/Tenant";
const TOKEN: &str = "synthetic-context-token";

struct Fixture {
    state: AppState,
    directory: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let directory = std::env::temp_dir().join(format!("velora-panel-context-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&directory).unwrap();
        std::fs::write(directory.join("secrets.key"), base64::engine::general_purpose::STANDARD.encode([41u8; 32])).unwrap();
        let state = AppState::new(directory.clone());
        state
            .update_settings(|settings| {
                settings.panel_url = Some(PANEL.into());
                settings.selected_instance = Some("synthetic-before-instance".into());
            })
            .unwrap();
        let account = Account {
            id: "synthetic-context-account".into(),
            kind: "panel".into(),
            username: "ContextFixture".into(),
            uuid: "00000000-0000-4000-8000-000000000041".into(),
            panel_url: Some(PANEL.into()),
            role: None,
        };
        *state.accounts.write().unwrap() = AccountsFile { active: Some(account.id.clone()), accounts: vec![account.clone()] };
        state.accounts.read().unwrap().save(&directory.join("accounts.json")).unwrap();
        state.secrets.set(&account.id, Secret { panel_token: Some(TOKEN.into()), ..Secret::default() }).unwrap();
        Self { state, directory }
    }

    fn account(&self) -> Account {
        self.state.accounts.read().unwrap().active().unwrap().clone()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

#[test]
fn matching_context_auth_preserves_the_panel_prefix() {
    let fixture = Fixture::new();
    for origin in [PANEL, "https://PANEL.example.invalid:443/Tenant///"] {
        fixture.state.accounts.write().unwrap().accounts[0].panel_url = Some(origin.into());
        let context = current(&fixture.state).unwrap();
        assert_eq!(context.panel, PANEL);
        assert_eq!(context.token.as_deref(), Some(TOKEN));
        assert_eq!(context.selected_instance.as_deref(), Some("synthetic-before-instance"));
        assert_eq!(token_for_panel(&fixture.state, PANEL).as_deref(), Some(TOKEN));
        assert_eq!(token_for_account(&fixture.state, &fixture.account(), PANEL).as_deref(), Some(TOKEN));
    }
}

#[test]
fn absent_legacy_origin_denies_without_migrating_or_erasing_credentials() {
    let fixture = Fixture::new();
    fixture.state.accounts.write().unwrap().accounts[0].panel_url = None;
    fixture.state.accounts.read().unwrap().save(&fixture.directory.join("accounts.json")).unwrap();
    let names = ["accounts.json", "secrets.bin", "secrets.key", "settings.json"];
    let before = names.map(|name| std::fs::read(fixture.directory.join(name)).unwrap());

    assert!(current(&fixture.state).unwrap().token.is_none());
    assert!(token_for_panel(&fixture.state, PANEL).is_none());
    assert!(token_for_account(&fixture.state, &fixture.account(), PANEL).is_none());
    assert!(fixture.account().panel_url.is_none());
    assert_eq!(fixture.state.secrets.get(&fixture.account().id).panel_token.as_deref(), Some(TOKEN));
    for (name, bytes) in names.into_iter().zip(before) {
        assert_eq!(std::fs::read(fixture.directory.join(name)).unwrap(), bytes, "lookup modified {name}");
    }
}

#[test]
fn tenant_case_host_port_scheme_and_invalid_origins_deny_bearer() {
    let fixture = Fixture::new();
    for destination in [
        "https://panel.example.invalid/tenant",
        "https://panel.example.invalid/Tenant/other",
        "https://other.example.invalid/Tenant",
        "https://panel.example.invalid:8443/Tenant",
        "http://panel.example.invalid/Tenant",
        "https://user:pass@panel.example.invalid/Tenant",
        "https://panel.example.invalid/Tenant?scope=other",
        "https://panel.example.invalid/Tenant#other",
        "not a URL",
        "",
    ] {
        assert!(token_for_panel(&fixture.state, destination).is_none(), "destination: {destination}");
        assert!(token_for_account(&fixture.state, &fixture.account(), destination).is_none(), "destination: {destination}");
        fixture.state.accounts.write().unwrap().accounts[0].panel_url = Some(destination.into());
        assert!(current(&fixture.state).unwrap().token.is_none(), "origin: {destination}");
        assert!(token_for_account(&fixture.state, &fixture.account(), PANEL).is_none(), "origin: {destination}");
        fixture.state.accounts.write().unwrap().accounts[0].panel_url = Some(PANEL.into());
    }
    let invalid = "https://user:pass@panel.example.invalid/Tenant";
    fixture.state.settings.write().unwrap().panel_url = Some(invalid.into());
    fixture.state.accounts.write().unwrap().accounts[0].panel_url = Some(invalid.into());
    assert!(current(&fixture.state).unwrap().token.is_none());
    assert!(token_for_panel(&fixture.state, invalid).is_none());
    assert_eq!(fixture.state.secrets.get(&fixture.account().id).panel_token.as_deref(), Some(TOKEN));
}

#[test]
fn offline_missing_active_and_missing_secret_never_borrow_bearer() {
    let fixture = Fixture::new();
    fixture.state.accounts.write().unwrap().accounts[0].kind = "offline".into();
    assert!(current(&fixture.state).unwrap().token.is_none());
    assert!(token_for_panel(&fixture.state, PANEL).is_none());
    assert!(token_for_account(&fixture.state, &fixture.account(), PANEL).is_none());
    fixture.state.accounts.write().unwrap().accounts[0].kind = "panel".into();
    fixture.state.accounts.write().unwrap().active = None;
    assert!(current(&fixture.state).unwrap().token.is_none());
    assert!(token_for_panel(&fixture.state, PANEL).is_none());
    assert_eq!(token_for_account(&fixture.state, &fixture.state.accounts.read().unwrap().accounts[0], PANEL).as_deref(), Some(TOKEN));
    fixture.state.accounts.write().unwrap().active = Some("missing-account".into());
    assert!(token_for_panel(&fixture.state, PANEL).is_none());
    fixture.state.secrets.remove("synthetic-context-account").unwrap();
    fixture.state.accounts.write().unwrap().active = Some("synthetic-context-account".into());
    assert!(current(&fixture.state).unwrap().token.is_none());
    assert!(token_for_account(&fixture.state, &fixture.account(), PANEL).is_none());
}

#[test]
fn captured_context_keeps_selected_instance_and_auth_after_mutation() {
    let fixture = Fixture::new();
    let before = current(&fixture.state).unwrap();
    let next_panel = "https://next.example.invalid/OtherTenant";
    {
        let mut settings = fixture.state.settings.write().unwrap();
        let mut accounts = fixture.state.accounts.write().unwrap();
        settings.panel_url = Some(next_panel.into());
        settings.selected_instance = Some("synthetic-after-instance".into());
        accounts.accounts[0].panel_url = Some(next_panel.into());
        fixture
            .state
            .secrets
            .set("synthetic-context-account", Secret { panel_token: Some("synthetic-after-token".into()), ..Secret::default() })
            .unwrap();
    }
    let after = current(&fixture.state).unwrap();
    assert_eq!(before.panel, PANEL);
    assert_eq!(before.token.as_deref(), Some(TOKEN));
    assert_eq!(before.selected_instance.as_deref(), Some("synthetic-before-instance"));
    assert_eq!(after.panel, next_panel);
    assert_eq!(after.token.as_deref(), Some("synthetic-after-token"));
    assert_eq!(after.selected_instance.as_deref(), Some("synthetic-after-instance"));
    assert!(token_for_panel(&fixture.state, PANEL).is_none());
}
