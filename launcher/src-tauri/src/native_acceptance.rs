#[cfg(not(debug_assertions))]
compile_error!("native-acceptance is a debug-only synthetic fixture feature; never package it");

use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Deserialize;
use std::fs::{self, File, OpenOptions};
use std::io::Read;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const MARKER: &str = ".velora-native-acceptance.json";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Marker {
    version: u32,
    id: Uuid,
    purpose: String,
}

pub(crate) struct FixtureRoot {
    pub(crate) root: PathBuf,
    _lock: File,
}

fn regular(path: &Path, limit: u64) -> Result<Vec<u8>> {
    let metadata = fs::symlink_metadata(path)?;
    anyhow::ensure!(metadata.file_type().is_file() && metadata.len() <= limit, "invalid synthetic fixture file");
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        anyhow::ensure!(metadata.nlink() == 1 && metadata.mode() & 0o077 == 0, "synthetic fixture files must be private and unlinked");
    }
    let mut bytes = Vec::new();
    File::open(path)?.take(limit + 1).read_to_end(&mut bytes)?;
    anyhow::ensure!(bytes.len() as u64 <= limit, "synthetic fixture file exceeds limit");
    Ok(bytes)
}

pub(crate) fn fallback_key(root: &Path) -> Result<[u8; 32]> {
    let encoded = regular(&root.join("secrets.key"), 128)?;
    let key: [u8; 32] =
        STANDARD.decode(&encoded)?.try_into().map_err(|_| anyhow::anyhow!("synthetic fixture requires an existing valid fallback key"))?;
    match fs::symlink_metadata(root.join("secrets.bin")) {
        Ok(_) => crate::secrets::validate_fixture_store(&key, &regular(&root.join("secrets.bin"), 8 * 1024 * 1024)?)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    Ok(key)
}

fn validate_tree(root: &Path) -> Result<()> {
    let mut pending = vec![(root.to_owned(), 0usize)];
    let mut entries = 0usize;
    while let Some((directory, depth)) = pending.pop() {
        anyhow::ensure!(depth <= 16, "synthetic fixture tree exceeds depth limit");
        for entry in fs::read_dir(directory)? {
            entries += 1;
            anyhow::ensure!(entries <= 10_000, "synthetic fixture tree exceeds entry limit");
            let path = entry?.path();
            let metadata = fs::symlink_metadata(&path)?;
            anyhow::ensure!(
                metadata.file_type().is_dir() || metadata.file_type().is_file(),
                "synthetic fixture contains a link or special file"
            );
            if metadata.file_type().is_dir() {
                pending.push((path, depth + 1));
            } else {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::MetadataExt;
                    anyhow::ensure!(metadata.nlink() == 1, "synthetic fixture contains a multiply-linked file");
                }
            }
        }
    }
    Ok(())
}

impl FixtureRoot {
    pub(crate) fn isolate_windows(&self, windows: &mut [tauri::utils::config::WindowConfig]) {
        for window in windows {
            window.incognito = true;
            window.data_directory = Some(self.root.join("webview"));
            window.title = "Velora Synthetic Acceptance".into();
        }
    }

    pub(crate) fn from_environment() -> Result<Self> {
        anyhow::ensure!(
            crate::state::build::PANEL_URL.is_none() && crate::state::build::REPO.is_none(),
            "synthetic build cannot contain a remote Panel/repository"
        );
        let root = std::env::var_os("VELORA_NATIVE_ACCEPTANCE_ROOT")
            .context("synthetic fixture root is required; installed-data fallback is disabled")?;
        Self::open(Path::new(&root))
    }

    fn open(root: &Path) -> Result<Self> {
        anyhow::ensure!(root.is_absolute(), "synthetic fixture root must be absolute");
        let metadata = fs::symlink_metadata(root)?;
        anyhow::ensure!(metadata.file_type().is_dir(), "synthetic fixture root must be a real directory");
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            anyhow::ensure!(metadata.mode() & 0o077 == 0, "synthetic fixture root must be private");
        }
        let temporary = fs::canonicalize(std::env::temp_dir())?;
        anyhow::ensure!(
            root.parent() == Some(temporary.as_path()) && fs::canonicalize(root)? == root,
            "synthetic fixture must be a canonical direct temporary child"
        );
        let name = root.file_name().and_then(|name| name.to_str()).context("invalid synthetic fixture name")?;
        let id = Uuid::parse_str(name.strip_prefix("velora-native-").context("synthetic fixture prefix is required")?)?;
        let marker: Marker = serde_json::from_slice(&regular(&root.join(MARKER), 2048)?)?;
        anyhow::ensure!(
            marker.version == 1 && marker.id == id && marker.purpose == "synthetic-native-acceptance",
            "synthetic fixture ownership mismatch"
        );
        validate_tree(root)?;
        fallback_key(root)?;
        let _: crate::accounts::AccountsFile = serde_json::from_slice(&regular(&root.join("accounts.json"), 1024 * 1024)?)?;
        let settings: crate::settings::Settings = serde_json::from_slice(&regular(&root.join("settings.json"), 1024 * 1024)?)?;
        let panel = reqwest::Url::parse(settings.panel_url.as_deref().context("synthetic loopback Panel is required")?)?;
        anyhow::ensure!(
            panel.scheme() == "http"
                && panel.host_str() == Some("127.0.0.1")
                && panel.port().is_some()
                && panel.username().is_empty()
                && panel.password().is_none()
                && panel.query().is_none()
                && panel.fragment().is_none(),
            "synthetic Panel must be an uncredentialed loopback HTTP endpoint"
        );
        let _: velora_shared::LauncherManifest = serde_json::from_slice(&regular(&root.join("manifest.json"), 1024 * 1024)?)?;
        for name in ["secrets.bin", crate::account_transaction::JOURNAL_NAME] {
            match fs::symlink_metadata(root.join(name)) {
                Ok(_) => {
                    regular(&root.join(name), 48 * 1024 * 1024)?;
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
        let lock_path = root.join(".native-acceptance.lock");
        match fs::symlink_metadata(&lock_path) {
            Ok(_) => {
                regular(&lock_path, 0)?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let lock = options.open(lock_path)?;
        lock.try_lock().context("synthetic fixture is already open or cannot be locked")?;
        Ok(Self { root: root.to_owned(), _lock: lock })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            let id = Uuid::new_v4();
            let root = fs::canonicalize(std::env::temp_dir()).unwrap().join(format!("velora-native-{id}"));
            let mut builder = fs::DirBuilder::new();
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                builder.mode(0o700);
            }
            builder.create(&root).unwrap();
            let fixture = Self(root);
            fixture.write(
                MARKER,
                serde_json::to_vec(&serde_json::json!({"version":1,"id":id,"purpose":"synthetic-native-acceptance"})).unwrap(),
            );
            fixture.write("secrets.key", STANDARD.encode([7u8; 32]).into_bytes());
            fixture.write("accounts.json", b"{\"accounts\":[],\"active\":null}".to_vec());
            fixture.write("settings.json", b"{\"panel_url\":\"http://127.0.0.1:12345\"}".to_vec());
            fixture.write("manifest.json", b"{}".to_vec());
            fixture
        }

        fn write(&self, name: &str, bytes: Vec<u8>) {
            fs::write(self.0.join(name), bytes).unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(self.0.join(name), fs::Permissions::from_mode(0o600)).unwrap();
            }
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    #[test]
    fn valid_fixture_excludes_other_hosts_until_guard_is_dropped() {
        let fixture = Fixture::new();
        let guard = FixtureRoot::open(&fixture.0).unwrap();
        assert_eq!(guard.root, fixture.0);
        assert!(FixtureRoot::open(&fixture.0).is_err());
        drop(guard);
        FixtureRoot::open(&fixture.0).unwrap();
    }

    #[test]
    fn missing_invalid_or_remote_fixture_never_creates_a_key_or_lock() {
        for (name, bytes) in
            [(MARKER, b"{}".as_slice()), ("secrets.key", b"invalid"), ("settings.json", b"{\"panel_url\":\"https://synthetic.invalid\"}")]
        {
            let fixture = Fixture::new();
            fixture.write(name, bytes.to_vec());
            assert!(FixtureRoot::open(&fixture.0).is_err());
            assert!(!fixture.0.join(".native-acceptance.lock").exists());
            assert_eq!(fs::read(fixture.0.join(name)).unwrap(), bytes);
        }
        let fixture = Fixture::new();
        fs::remove_file(fixture.0.join("secrets.key")).unwrap();
        assert!(FixtureRoot::open(&fixture.0).is_err());
        assert!(!fixture.0.join("secrets.key").exists());
        assert!(!fixture.0.join(".native-acceptance.lock").exists());
        assert!(FixtureRoot::open(Path::new("relative-fixture")).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn symlink_and_linked_fixture_files_are_rejected_before_startup() {
        for name in [MARKER, "secrets.key", "accounts.json", "settings.json", "manifest.json", ".native-acceptance.lock"] {
            let fixture = Fixture::new();
            if name == ".native-acceptance.lock" {
                fixture.write(name, vec![]);
            }
            let target = fixture.0.join("synthetic-target");
            fs::rename(fixture.0.join(name), &target).unwrap();
            std::os::unix::fs::symlink(&target, fixture.0.join(name)).unwrap();
            let before = fs::read(&target).unwrap();
            assert!(FixtureRoot::open(&fixture.0).is_err());
            assert_eq!(fs::read(&target).unwrap(), before);
        }
        let fixture = Fixture::new();
        fs::hard_link(fixture.0.join("secrets.key"), fixture.0.join("linked-key")).unwrap();
        assert!(FixtureRoot::open(&fixture.0).is_err());
    }

    #[test]
    fn corrupt_or_wrong_key_encrypted_store_fails_before_native_key_access() {
        let fixture = Fixture::new();
        fixture.write("secrets.bin", b"synthetic corrupt ciphertext".to_vec());
        assert!(fallback_key(&fixture.0).is_err());
        assert!(FixtureRoot::open(&fixture.0).is_err());
        assert!(!fixture.0.join(".native-acceptance.lock").exists());
        fs::remove_file(fixture.0.join("secrets.bin")).unwrap();
        let secrets = crate::secrets::Secrets::try_open(&fixture.0).unwrap();
        secrets
            .set("synthetic", crate::secrets::Secret { panel_token: Some("synthetic-native-token".into()), ..Default::default() })
            .unwrap();
        fallback_key(&fixture.0).unwrap();
        FixtureRoot::open(&fixture.0).unwrap();
        fixture.write("secrets.key", STANDARD.encode([8u8; 32]).into_bytes());
        assert!(fallback_key(&fixture.0).is_err());
        assert!(FixtureRoot::open(&fixture.0).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn nested_startup_and_other_store_links_are_rejected_without_target_changes() {
        for name in ["minecraft", "minecraft/textures", "minecraft/instances/synthetic/textures", "unrelated/nested/file"] {
            let fixture = Fixture::new();
            let target = Fixture::new();
            let path = fixture.0.join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::os::unix::fs::symlink(&target.0, &path).unwrap();
            let before = fs::read(target.0.join("secrets.key")).unwrap();
            assert!(FixtureRoot::open(&fixture.0).is_err());
            assert!(!fixture.0.join(".native-acceptance.lock").exists());
            assert_eq!(fs::read(target.0.join("secrets.key")).unwrap(), before);
            assert_eq!(fs::read_dir(&target.0).unwrap().count(), 5);
        }
    }

    #[test]
    fn automatic_fixture_windows_are_configured_as_nonpersistent_before_build() {
        let fixture = Fixture::new();
        let guard = FixtureRoot::open(&fixture.0).unwrap();
        let mut config: tauri::utils::config::Config = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        assert!(!config.app.windows.is_empty());
        guard.isolate_windows(&mut config.app.windows);
        for window in &config.app.windows {
            assert!(window.incognito);
            assert_eq!(window.data_directory.as_deref(), Some(fixture.0.join("webview").as_path()));
            assert_eq!(window.title, "Velora Synthetic Acceptance");
        }
    }

    #[test]
    fn node_generated_fixture_passes_actual_rust_preflight() {
        use std::process::{Command, Stdio};
        use std::time::{Duration, Instant};
        let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../scripts/native-fixture.mjs");
        let mut child = Command::new("node")
            .arg(script)
            .arg("--selftest")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(20);
        while child.try_wait().unwrap().is_none() {
            if Instant::now() >= deadline {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("synthetic Node fixture exceeded deadline");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success());
        let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(result["selftest"], "passed");
        let root = PathBuf::from(result["root"].as_str().unwrap());
        let guard = FixtureRoot::open(&root).unwrap();
        let fixture = Fixture(root);
        assert!(crate::state::AppState::try_new(fixture.0.clone()).is_ok());
        drop(guard);
    }
}
