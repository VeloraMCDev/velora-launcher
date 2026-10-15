use crate::panel_client::{FailureKind, PanelFailure};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use velora_shared::InstanceManifest;

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct Authority {
    panel: String,
    account: Option<(String, String, String)>,
    authenticated: bool,
}

impl Authority {
    pub(super) fn new(panel: &str, account: Option<&crate::accounts::Account>, authenticated: bool) -> Result<Self> {
        let mut origin = reqwest::Url::parse(panel)?;
        anyhow::ensure!(
            matches!(origin.scheme(), "http" | "https")
                && origin.host_str().is_some()
                && origin.username().is_empty()
                && origin.password().is_none()
                && origin.query().is_none()
                && origin.fragment().is_none(),
            "invalid panel configuration"
        );
        origin.set_path(&format!("{}/", origin.path().trim_end_matches('/')));
        Ok(Self {
            panel: origin.to_string(),
            account: account.map(|account| (account.id.clone(), account.kind.clone(), account.uuid.clone())),
            authenticated,
        })
    }

    pub(super) fn same_panel(&self, panel: &str) -> bool {
        Self::new(panel, None, false).is_ok_and(|authority| authority.panel == self.panel)
    }
}

#[derive(Serialize, Deserialize)]
struct CachedManifest {
    #[serde(default)]
    cache_version: u32,
    #[serde(default)]
    cache_authority: Option<Authority>,
    #[serde(flatten)]
    manifest: InstanceManifest,
}

fn cache_parent(path: &Path) -> Result<&Path> {
    let parent = path.parent().ok_or_else(|| anyhow::anyhow!("invalid manifest cache path"))?;
    match std::fs::symlink_metadata(parent) {
        Ok(metadata) => anyhow::ensure!(metadata.file_type().is_dir(), "manifest cache parent must be a directory"),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    Ok(parent)
}

pub(super) fn publish(path: &Path, authority: &Authority, id: &str, manifest: &InstanceManifest) -> Result<()> {
    anyhow::ensure!(manifest.instance.id == id, "instance manifest identity mismatch");
    std::fs::create_dir_all(cache_parent(path)?)?;
    cache_parent(path)?;
    crate::persistence::atomic_write_json(
        path,
        &CachedManifest { cache_version: 1, cache_authority: Some(authority.clone()), manifest: manifest.clone() },
    )
}

pub(super) fn read(path: &Path, authority: &Authority, id: &str) -> Result<InstanceManifest> {
    cache_parent(path)?;
    let cached = crate::persistence::read_json::<CachedManifest>(path)?.ok_or_else(|| anyhow::anyhow!("manifest cache missing"))?;
    anyhow::ensure!(
        cached.cache_version == 1 && cached.cache_authority.as_ref() == Some(authority) && cached.manifest.instance.id == id,
        "offline manifest authority unavailable or mismatched"
    );
    Ok(cached.manifest)
}

pub(super) fn invalidate(path: &Path, authority: &Authority) -> Result<()> {
    cache_parent(path)?;
    let Some(mut cached) = crate::persistence::read_json::<CachedManifest>(path)? else { return Ok(()) };
    if cached.cache_authority.as_ref() == Some(authority) {
        cached.cache_authority = None;
        crate::persistence::atomic_write_json(path, &cached)?;
    }
    Ok(())
}

pub(super) fn recover_with<Manifest>(fetched: Result<Manifest>, cached: impl FnOnce() -> Result<Manifest>) -> Result<Manifest> {
    match fetched {
        Ok(manifest) => Ok(manifest),
        Err(error) => {
            let network = error.downcast_ref::<PanelFailure>().is_some_and(|failure| {
                failure.kind == FailureKind::Network && failure.status.is_none_or(|status| (200..300).contains(&status))
            });
            if !network {
                return Err(error);
            }
            cached().map_err(|_| error.context("offline instance manifest unavailable or invalid"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::panel_client::{ErrorPolicy, PanelClient};
    use std::cell::Cell;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    struct Fixture {
        root: std::path::PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!("velora-manifest-authority-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(&root).unwrap();
            Self { root }
        }

        fn path(&self) -> std::path::PathBuf {
            self.root.join("manifest.json")
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.root).unwrap();
        }
    }

    fn account(id: &str) -> crate::accounts::Account {
        crate::accounts::Account {
            id: id.into(),
            kind: "panel".into(),
            username: "Synthetic".into(),
            uuid: format!("synthetic-{id}"),
            panel_url: Some("https://panel.invalid/tenant".into()),
            role: None,
        }
    }

    fn manifest() -> InstanceManifest {
        let mut manifest = InstanceManifest::default();
        manifest.instance.id = "synthetic-instance".into();
        manifest
    }

    #[test]
    fn scoped_cache_survives_restart_and_preserves_legacy_manifest_shape() {
        let fixture = Fixture::new();
        let authority = Authority::new("https://panel.invalid/tenant/", Some(&account("first")), true).unwrap();
        let expected = manifest();
        publish(&fixture.path(), &authority, &expected.instance.id, &expected).unwrap();
        let restarted = Authority::new("https://PANEL.invalid:443/tenant", Some(&account("first")), true).unwrap();
        assert_eq!(read(&fixture.path(), &restarted, &expected.instance.id).unwrap(), expected);
        let legacy: InstanceManifest = serde_json::from_slice(&std::fs::read(fixture.path()).unwrap()).unwrap();
        assert_eq!(legacy, expected);
        assert_eq!(std::fs::read_dir(&fixture.root).unwrap().count(), 1);
    }

    #[test]
    fn panel_account_authentication_and_instance_changes_reject_cached_authority() {
        let fixture = Fixture::new();
        let first = account("first");
        let original = Authority::new("https://panel.invalid/tenant", Some(&first), true).unwrap();
        publish(&fixture.path(), &original, "synthetic-instance", &manifest()).unwrap();
        let mut changed_uuid = first.clone();
        changed_uuid.uuid = "changed-identity".into();
        let mut changed_kind = first.clone();
        changed_kind.kind = "offline".into();
        for authority in [
            Authority::new("https://other.invalid/tenant", Some(&first), true).unwrap(),
            Authority::new("http://panel.invalid/tenant", Some(&first), true).unwrap(),
            Authority::new("https://panel.invalid/Tenant", Some(&first), true).unwrap(),
            Authority::new("https://panel.invalid/tenant", Some(&account("second")), true).unwrap(),
            Authority::new("https://panel.invalid/tenant", Some(&changed_uuid), true).unwrap(),
            Authority::new("https://panel.invalid/tenant", Some(&changed_kind), true).unwrap(),
            Authority::new("https://panel.invalid/tenant", None, true).unwrap(),
            Authority::new("https://panel.invalid/tenant", Some(&first), false).unwrap(),
        ] {
            assert!(read(&fixture.path(), &authority, "synthetic-instance").is_err());
        }
        assert!(read(&fixture.path(), &original, "different-instance").is_err());
        assert!(publish(&fixture.path(), &original, "different-instance", &manifest()).is_err());
        assert_eq!(read(&fixture.path(), &original, "synthetic-instance").unwrap(), manifest());
    }

    #[test]
    fn legacy_cache_requires_online_refresh_without_changing_destination() {
        let fixture = Fixture::new();
        let authority = Authority::new("https://panel.invalid", None, false).unwrap();
        let expected = manifest();
        std::fs::write(fixture.path(), serde_json::to_vec(&expected).unwrap()).unwrap();
        assert!(read(&fixture.path(), &authority, "synthetic-instance").is_err());
        publish(&fixture.path(), &authority, "synthetic-instance", &expected).unwrap();
        assert_eq!(read(&fixture.path(), &authority, "synthetic-instance").unwrap(), expected);
    }

    #[test]
    fn corrupt_cache_is_preserved_when_publication_is_rejected() {
        let fixture = Fixture::new();
        let authority = Authority::new("https://panel.invalid", None, false).unwrap();
        std::fs::write(fixture.path(), b"corrupt synthetic evidence").unwrap();
        assert!(read(&fixture.path(), &authority, "synthetic-instance").is_err());
        assert!(publish(&fixture.path(), &authority, "synthetic-instance", &manifest()).is_err());
        assert_eq!(std::fs::read(fixture.path()).unwrap(), b"corrupt synthetic evidence");
    }

    #[test]
    fn known_denial_invalidates_only_matching_authority_until_online_refresh() {
        let fixture = Fixture::new();
        let authority = Authority::new("https://panel.invalid", Some(&account("first")), true).unwrap();
        let other = Authority::new("https://panel.invalid", Some(&account("second")), true).unwrap();
        let expected = manifest();
        publish(&fixture.path(), &authority, "synthetic-instance", &expected).unwrap();
        invalidate(&fixture.path(), &other).unwrap();
        assert_eq!(read(&fixture.path(), &authority, "synthetic-instance").unwrap(), expected);
        invalidate(&fixture.path(), &authority).unwrap();
        assert!(read(&fixture.path(), &authority, "synthetic-instance").is_err());
        let legacy: InstanceManifest = serde_json::from_slice(&std::fs::read(fixture.path()).unwrap()).unwrap();
        assert_eq!(legacy, expected);
        publish(&fixture.path(), &authority, "synthetic-instance", &expected).unwrap();
        assert_eq!(read(&fixture.path(), &authority, "synthetic-instance").unwrap(), expected);
    }

    #[test]
    fn invalid_panel_authorities_are_rejected_without_secret_material() {
        for panel in [
            "invalid",
            "file:///tmp/fixture",
            "https://user:secret@panel.invalid",
            "https://panel.invalid?token=secret",
            "https://panel.invalid#secret",
        ] {
            assert!(Authority::new(panel, None, false).is_err());
        }
        let authority = Authority::new("https://panel.invalid", Some(&account("first")), true).unwrap();
        let serialized = serde_json::to_string(&authority).unwrap();
        assert!(!serialized.contains("panel_token"));
        assert!(!serialized.contains("username"));
    }

    #[cfg(unix)]
    #[test]
    fn symlink_cache_is_not_followed_or_replaced() {
        let fixture = Fixture::new();
        let target = fixture.root.join("unrelated.json");
        let expected = manifest();
        std::fs::write(&target, serde_json::to_vec(&expected).unwrap()).unwrap();
        std::os::unix::fs::symlink(&target, fixture.path()).unwrap();
        let authority = Authority::new("https://panel.invalid", None, false).unwrap();
        assert!(read(&fixture.path(), &authority, "synthetic-instance").is_err());
        assert!(publish(&fixture.path(), &authority, "synthetic-instance", &expected).is_err());
        assert!(std::fs::symlink_metadata(fixture.path()).unwrap().file_type().is_symlink());
        assert_eq!(serde_json::from_slice::<InstanceManifest>(&std::fs::read(target).unwrap()).unwrap(), expected);
    }

    #[cfg(unix)]
    #[test]
    fn symlink_cache_parent_is_not_followed() {
        let fixture = Fixture::new();
        let directory = fixture.root.join("unrelated");
        std::fs::create_dir(&directory).unwrap();
        let link = fixture.root.join(".scopenet");
        std::os::unix::fs::symlink(&directory, &link).unwrap();
        let path = link.join("manifest.json");
        let authority = Authority::new("https://panel.invalid", None, false).unwrap();
        assert!(read(&path, &authority, "synthetic-instance").is_err());
        assert!(publish(&path, &authority, "synthetic-instance", &manifest()).is_err());
        assert!(invalidate(&path, &authority).is_err());
        assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 0);
    }

    async fn fetched(status: u16, body: &str) -> Result<serde_json::Value> {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let body = body.to_owned();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            let mut buffer = [0u8; 1024];
            while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                let count = socket.read(&mut buffer).await.unwrap();
                assert_ne!(count, 0);
                request.extend_from_slice(&buffer[..count]);
            }
            assert!(String::from_utf8(request).unwrap().to_ascii_lowercase().contains("authorization: bearer synthetic-account"));
            socket
                .write_all(
                    format!("HTTP/1.1 {status} Fixture\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes(),
                )
                .await
                .unwrap();
        });
        let client = PanelClient::new(
            reqwest::Client::builder().no_proxy().build().unwrap(),
            &format!("http://{address}"),
            Some("synthetic-account"),
        )
        .unwrap();
        let response = PanelClient::send(client.request(reqwest::Method::GET, "/launcher/instances/synthetic").unwrap()).await.unwrap();
        let result = PanelClient::json(response, ErrorPolicy::Generic).await.map_err(Into::into);
        server.await.unwrap();
        result
    }

    async fn state_with_responses(fixture: &Fixture, statuses: Vec<u16>) -> (crate::state::AppState, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let panel = format!("http://{}", listener.local_addr().unwrap());
        let mut state = crate::state::AppState::new(fixture.root.clone());
        state.http = reqwest::Client::builder().no_proxy().timeout(std::time::Duration::from_secs(2)).build().unwrap();
        state.settings.write().unwrap().panel_url = Some(panel.clone());
        assert_eq!(state.panel_url().as_deref(), Some(panel.as_str()));
        let mut first = account("first");
        first.panel_url = Some(panel);
        state.accounts.write().unwrap().accounts.push(first);
        state.accounts.write().unwrap().active = Some("first".into());
        state.secrets.set("first", crate::secrets::Secret { panel_token: Some("synthetic-account".into()), ..Default::default() }).unwrap();
        let server = tokio::spawn(async move {
            for status in statuses {
                let (mut socket, _) = tokio::time::timeout(std::time::Duration::from_secs(5), listener.accept()).await.unwrap().unwrap();
                let mut request = Vec::new();
                let mut buffer = [0u8; 1024];
                while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                    let count = socket.read(&mut buffer).await.unwrap();
                    assert_ne!(count, 0);
                    request.extend_from_slice(&buffer[..count]);
                }
                assert!(String::from_utf8(request).unwrap().to_ascii_lowercase().contains("authorization: bearer synthetic-account"));
                let body = serde_json::to_string(&manifest()).unwrap();
                socket
                    .write_all(
                        format!("HTTP/1.1 {status} Fixture\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len())
                            .as_bytes(),
                    )
                    .await
                    .unwrap();
            }
        });
        (state, server)
    }

    #[tokio::test]
    async fn actual_instance_fetch_without_account_origin_never_sends_a_bearer() {
        let fixture = Fixture::new();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let panel = format!("http://{}", listener.local_addr().unwrap());
        let mut state = crate::state::AppState::new(fixture.root.clone());
        state.http = reqwest::Client::builder().no_proxy().timeout(std::time::Duration::from_secs(2)).build().unwrap();
        state.settings.write().unwrap().panel_url = Some(panel);
        let mut legacy = account("legacy");
        legacy.panel_url = None;
        state.accounts.write().unwrap().upsert(legacy);
        state
            .secrets
            .set("legacy", crate::secrets::Secret { panel_token: Some("synthetic-account".into()), ..Default::default() })
            .unwrap();
        let server = tokio::spawn(async move {
            tokio::time::timeout(std::time::Duration::from_secs(3), async move {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = Vec::new();
                while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                    let mut buffer = [0; 1024];
                    let count = socket.read(&mut buffer).await.unwrap();
                    assert_ne!(count, 0);
                    request.extend_from_slice(&buffer[..count]);
                    assert!(request.len() <= 8192);
                }
                let request = String::from_utf8(request).unwrap();
                assert!(request.starts_with("GET /api/v1/launcher/instances/synthetic-instance HTTP/1.1\r\n"));
                assert!(!request.to_ascii_lowercase().contains("authorization:"));
                let body = serde_json::to_string(&manifest()).unwrap();
                socket
                    .write_all(
                        format!("HTTP/1.1 200 Fixture\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes(),
                    )
                    .await
                    .unwrap();
            })
            .await
            .unwrap();
        });
        let result = super::super::instance_manifest(&state, "synthetic-instance").await;
        server.await.unwrap();
        assert_eq!(result.unwrap(), manifest());
        let cache = state.layout.instance_dir("synthetic-instance").join(".scopenet/manifest.json");
        let cached: CachedManifest = serde_json::from_slice(&std::fs::read(cache).unwrap()).unwrap();
        assert!(!cached.cache_authority.unwrap().authenticated);
        assert_eq!(state.secrets.get("legacy").panel_token.as_deref(), Some("synthetic-account"));
    }

    #[tokio::test]
    async fn actual_instance_fetch_rejects_cross_account_and_panel_offline_reuse() {
        let fixture = Fixture::new();
        let (state, server) = state_with_responses(&fixture, vec![200]).await;
        assert_eq!(super::super::instance_manifest(&state, "synthetic-instance").await.unwrap(), manifest());
        server.await.unwrap();
        assert_eq!(super::super::instance_manifest(&state, "synthetic-instance").await.unwrap(), manifest());
        let panel = state.panel_url().unwrap();
        let mut second = account("second");
        second.panel_url = Some(panel.clone());
        state.accounts.write().unwrap().accounts.push(second);
        state.accounts.write().unwrap().active = Some("second".into());
        state
            .secrets
            .set("second", crate::secrets::Secret { panel_token: Some("synthetic-account".into()), ..Default::default() })
            .unwrap();
        assert!(super::super::instance_manifest(&state, "synthetic-instance").await.is_err());
        state.accounts.write().unwrap().active = Some("first".into());
        assert_eq!(super::super::instance_manifest(&state, "synthetic-instance").await.unwrap(), manifest());
        state.settings.write().unwrap().panel_url = Some(format!("{panel}/other-tenant"));
        assert!(super::super::instance_manifest(&state, "synthetic-instance").await.is_err());
        state.settings.write().unwrap().panel_url = Some(panel);
        state.secrets.remove("first").unwrap();
        assert!(super::super::instance_manifest(&state, "synthetic-instance").await.is_err());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn actual_denial_fences_restored_cache_after_invalidation_write_failure() {
        let fixture = Fixture::new();
        let (state, server) = state_with_responses(&fixture, vec![200, 403]).await;
        super::super::instance_manifest(&state, "synthetic-instance").await.unwrap();
        let cache = state.layout.instance_dir("synthetic-instance").join(".scopenet/manifest.json");
        let retained = cache.with_file_name("synthetic-retained.json");
        std::fs::rename(&cache, &retained).unwrap();
        std::os::unix::fs::symlink(&retained, &cache).unwrap();
        let denial = super::super::instance_manifest(&state, "synthetic-instance").await.unwrap_err();
        assert_eq!(denial.downcast_ref::<PanelFailure>().unwrap().status, Some(403));
        server.await.unwrap();
        std::fs::remove_file(&cache).unwrap();
        std::fs::rename(&retained, &cache).unwrap();
        let cached: CachedManifest = serde_json::from_slice(&std::fs::read(&cache).unwrap()).unwrap();
        assert!(cached.cache_authority.is_some());
        let offline = super::super::instance_manifest(&state, "synthetic-instance").await.unwrap_err();
        assert_eq!(offline.downcast_ref::<PanelFailure>().unwrap().kind, FailureKind::Network);
        assert!(state.manifest_denials.observe(&cache).is_some());
    }

    #[tokio::test]
    async fn actual_successful_refresh_after_denial_restores_offline_access() {
        let fixture = Fixture::new();
        let (state, server) = state_with_responses(&fixture, vec![200, 401, 200]).await;
        super::super::instance_manifest(&state, "synthetic-instance").await.unwrap();
        assert!(super::super::instance_manifest(&state, "synthetic-instance").await.is_err());
        let cache = state.layout.instance_dir("synthetic-instance").join(".scopenet/manifest.json");
        assert!(state.manifest_denials.observe(&cache).is_some());
        assert_eq!(super::super::instance_manifest(&state, "synthetic-instance").await.unwrap(), manifest());
        server.await.unwrap();
        assert!(state.manifest_denials.observe(&cache).is_none());
        assert_eq!(super::super::instance_manifest(&state, "synthetic-instance").await.unwrap(), manifest());
    }

    #[tokio::test]
    async fn actual_instance_denial_revokes_future_offline_fallback() {
        for status in [401, 403, 404] {
            let fixture = Fixture::new();
            let (state, server) = state_with_responses(&fixture, vec![200, status]).await;
            super::super::instance_manifest(&state, "synthetic-instance").await.unwrap();
            let denial = super::super::instance_manifest(&state, "synthetic-instance").await.unwrap_err();
            assert_eq!(denial.downcast_ref::<PanelFailure>().unwrap().status, Some(status));
            server.await.unwrap();
            let offline = super::super::instance_manifest(&state, "synthetic-instance").await.unwrap_err();
            assert_eq!(offline.downcast_ref::<PanelFailure>().unwrap().kind, FailureKind::Network);
        }
    }

    #[tokio::test]
    async fn authorization_and_status_failures_never_read_cached_authority() {
        for status in [401, 403, 404, 429, 500, 503] {
            let cache_calls = Cell::new(0);
            let result = recover_with(fetched(status, "{\"error\":\"synthetic private response\"}").await, || {
                cache_calls.set(cache_calls.get() + 1);
                Ok(serde_json::json!({"cached": true}))
            });
            let error = result.unwrap_err();
            assert_eq!(cache_calls.get(), 0, "{status}");
            assert_eq!(error.downcast_ref::<PanelFailure>().unwrap().status, Some(status));
            assert!(!error.to_string().contains("synthetic private response"));
        }
    }

    #[tokio::test]
    async fn malformed_success_is_not_an_offline_cache_signal() {
        let cache_calls = Cell::new(0);
        let result = recover_with(fetched(200, "malformed").await, || {
            cache_calls.set(cache_calls.get() + 1);
            Ok(serde_json::json!({"cached": true}))
        });
        assert_eq!(cache_calls.get(), 0);
        assert_eq!(result.unwrap_err().downcast_ref::<PanelFailure>().unwrap().kind, FailureKind::Decode);
    }

    #[test]
    fn invalid_configuration_and_untyped_denial_do_not_read_cache() {
        let invalid = PanelClient::new(reqwest::Client::new(), "invalid panel", None).unwrap_err();
        for error in [anyhow::Error::from(invalid), anyhow::anyhow!("this instance no longer exists or you don't have access to it")] {
            let cache_calls = Cell::new(0);
            let result = recover_with::<u64>(Err(error), || {
                cache_calls.set(cache_calls.get() + 1);
                Ok(42)
            });
            assert!(result.is_err());
            assert_eq!(cache_calls.get(), 0);
        }
    }

    #[tokio::test]
    async fn successful_online_response_does_not_read_old_cache() {
        let cache_calls = Cell::new(0);
        let result = recover_with(fetched(200, "{\"online\":true}").await, || {
            cache_calls.set(cache_calls.get() + 1);
            Ok(serde_json::json!({"cached": true}))
        })
        .unwrap();
        assert_eq!(result, serde_json::json!({"online": true}));
        assert_eq!(cache_calls.get(), 0);
    }

    #[tokio::test]
    async fn transport_failure_allows_valid_cache_and_keeps_network_error_on_cache_failure() {
        for valid_cache in [true, false] {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let address = listener.local_addr().unwrap();
            drop(listener);
            let client =
                PanelClient::new(reqwest::Client::builder().no_proxy().build().unwrap(), &format!("http://{address}"), None).unwrap();
            let error =
                PanelClient::send(client.request(reqwest::Method::GET, "/launcher/instances/synthetic").unwrap()).await.unwrap_err();
            assert_eq!(error.kind, FailureKind::Network);
            let cache_calls = Cell::new(0);
            let result = recover_with::<u64>(Err(error.into()), || {
                cache_calls.set(cache_calls.get() + 1);
                if valid_cache {
                    Ok(42)
                } else {
                    anyhow::bail!("synthetic unreadable cache")
                }
            });
            assert_eq!(cache_calls.get(), 1);
            if valid_cache {
                assert_eq!(result.unwrap(), 42);
            } else {
                assert_eq!(result.unwrap_err().downcast_ref::<PanelFailure>().unwrap().kind, FailureKind::Network);
            }
        }
    }
}
