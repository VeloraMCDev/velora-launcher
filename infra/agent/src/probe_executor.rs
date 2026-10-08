//! Host-local typed executor. No shell, user-supplied commands, mounts or ports.
use crate::probe_job::Envelope;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub schema: u32,
    pub trusted_public_key: String,
    pub agent_id: String,
    pub state_directory: PathBuf,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    pub schema: u32,
    pub job_id: String,
    pub payload_sha256: String,
    pub status: String,
    pub events: Vec<String>,
}
#[cfg(unix)]
mod native {
    use super::*;
    use crate::probe_job::{verify, ProbeJob};
    use sha2::{Digest, Sha256};
    use std::{
        fs,
        io::{Read, Write},
        os::unix::{
            fs::{MetadataExt, OpenOptionsExt},
            io::AsRawFd,
        },
        process::{Command, Stdio},
        time::{Duration, Instant},
    };

    fn private_file(path: &Path, create: bool) -> Result<fs::File, &'static str> {
        let file = fs::OpenOptions::new()
            .read(true)
            .write(create)
            .create(create)
            .truncate(false)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(path)
            .map_err(|_| "Executor state unavailable")?;
        let meta = file.metadata().map_err(|_| "Executor state unavailable")?;
        if !meta.is_file() || meta.uid() != unsafe { libc::geteuid() } || meta.mode() & 0o077 != 0 {
            return Err("Executor state must be privately owned");
        }
        Ok(file)
    }
    fn read<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, &'static str> {
        let mut bytes = Vec::new();
        private_file(path, false)?.take(8193).read_to_end(&mut bytes).map_err(|_| "Executor state unreadable")?;
        if bytes.len() > 8192 {
            return Err("Executor state too large");
        }
        serde_json::from_slice(&bytes).map_err(|_| "Executor state invalid")
    }
    fn write<T: Serialize>(path: &Path, value: &T) -> Result<(), &'static str> {
        let temporary = path.with_extension(format!("{}.tmp", crate::new_nonce()?));
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&temporary)
            .map_err(|_| "Executor state cannot be staged")?;
        file.write_all(&serde_json::to_vec(value).map_err(|_| "Executor state encoding failed")?)
            .map_err(|_| "Executor state write failed")?;
        file.sync_all().map_err(|_| "Executor state sync failed")?;
        fs::rename(&temporary, path).map_err(|_| "Executor state activation failed")?;
        fs::File::open(path.parent().ok_or("Invalid state parent")?)
            .and_then(|directory| directory.sync_all())
            .map_err(|_| "Executor directory sync failed")?;
        Ok(())
    }
    fn docker(args: &[&str], capture: bool) -> Result<Option<String>, &'static str> {
        let mut command = Command::new("/usr/bin/docker");
        command.args(args).env_clear().env("PATH", "/usr/bin:/bin").stdin(Stdio::null()).stderr(Stdio::null()).stdout(if capture {
            Stdio::piped()
        } else {
            Stdio::null()
        });
        // Docker may use the host's registry credential helper; credentials never enter jobs.
        if let Some(home) = std::env::var_os("HOME") {
            command.env("HOME", home);
        }
        let mut child = command.spawn().map_err(|_| "Host-local Docker unavailable")?;
        let started = Instant::now();
        loop {
            if let Some(status) = child.try_wait().map_err(|_| "Docker status unavailable")? {
                if !status.success() {
                    return Ok(None);
                }
                if capture {
                    let mut bytes = Vec::new();
                    child
                        .stdout
                        .take()
                        .ok_or("Docker output unavailable")?
                        .take(257)
                        .read_to_end(&mut bytes)
                        .map_err(|_| "Docker output unavailable")?;
                    if bytes.len() > 256 {
                        return Err("Docker output exceeds policy");
                    }
                    return String::from_utf8(bytes).map(Some).map_err(|_| "Docker output invalid");
                }
                return Ok(Some(String::new()));
            }
            if started.elapsed() > Duration::from_secs(120) {
                let _ = child.kill();
                let _ = child.wait();
                return Err("Docker operation exceeded deadline");
            }
            std::thread::sleep(Duration::from_millis(200));
        }
    }
    trait Runtime {
        fn docker(&self, args: &[&str], capture: bool) -> Result<Option<String>, &'static str>;
        fn health(&self, job: &ProbeJob) -> Result<(), &'static str>;
    }
    struct HostRuntime;
    impl Runtime for HostRuntime {
        fn docker(&self, args: &[&str], capture: bool) -> Result<Option<String>, &'static str> {
            docker(args, capture)
        }
        fn health(&self, job: &ProbeJob) -> Result<(), &'static str> {
            health(job)
        }
    }
    fn activate(runtime: &dyn Runtime, compose: &Path, job: &ProbeJob) -> Result<(), &'static str> {
        write(compose, &job.compose())?;
        let path = compose.to_str().ok_or("Invalid Compose state path")?;
        runtime
            .docker(&["compose", "--project-name", "velora-guide-development-probe", "--file", path, "pull", "probe"], false)?
            .ok_or("Exact probe image pull failed")?;
        runtime
            .docker(
                &[
                    "compose",
                    "--project-name",
                    "velora-guide-development-probe",
                    "--file",
                    path,
                    "up",
                    "--detach",
                    "--no-build",
                    "--wait",
                    "--wait-timeout",
                    "60",
                    "probe",
                ],
                false,
            )?
            .ok_or("Probe activation failed")?;
        Ok(())
    }
    fn health(job: &ProbeJob) -> Result<(), &'static str> {
        let client = reqwest::blocking::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(3))
            .build()
            .map_err(|_| "Health client unavailable")?;
        for _ in 0..15 {
            let response = client.get("http://127.0.0.1:18985/health").send().map_err(|_| "Probe health unavailable")?;
            if !response.status().is_success() {
                return Err("Probe health failed");
            }
            let mut bytes = Vec::new();
            response.take(4097).read_to_end(&mut bytes).map_err(|_| "Probe health unreadable")?;
            if bytes.len() > 4096 {
                return Err("Probe health too large");
            }
            let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|_| "Probe health invalid")?;
            if value["status"] != "ok" || value["service"] != "deployment-probe" || value["commit"] != job.expected_commit {
                return Err("Probe health identity mismatch");
            }
            std::thread::sleep(Duration::from_secs(2));
        }
        Ok(())
    }
    pub fn execute(policy: &Policy, envelope: &Envelope, now: u64) -> Result<Receipt, &'static str> {
        execute_with_runtime(policy, envelope, now, &HostRuntime)
    }
    fn execute_with_runtime(policy: &Policy, envelope: &Envelope, now: u64, runtime: &dyn Runtime) -> Result<Receipt, &'static str> {
        let job = verify(envelope, &policy.trusted_public_key, &policy.agent_id, now)?;
        let root = &policy.state_directory;
        if policy.schema != 1 || !root.is_absolute() || root.components().any(|part| matches!(part, std::path::Component::ParentDir)) {
            return Err("Invalid fixed probe policy");
        }
        let meta = fs::symlink_metadata(root).map_err(|_| "Create the private executor state directory first")?;
        if !meta.is_dir() || meta.uid() != unsafe { libc::geteuid() } || meta.mode() & 0o077 != 0 {
            return Err("Executor state directory must be private and owned");
        }
        if root.canonicalize().map_err(|_| "Executor state unavailable")? != *root {
            return Err("Executor state cannot use symlinks");
        }
        let lock = private_file(&root.join("deployment.lock"), true)?;
        // The lock is released by the OS on process exit, including crashes.
        if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err("Another probe deployment owns the lock");
        }
        let ledger = root.join(format!("{}.json", job.id));
        let hash = hex::encode(Sha256::digest(envelope.payload.as_bytes()));
        if ledger.exists() {
            let mut receipt: Receipt = read(&ledger)?;
            if receipt.schema != 1 || receipt.job_id != job.id || receipt.payload_sha256 != hash {
                return Err("Probe job idempotency conflict");
            }
            if !["SUCCEEDED", "FAILED", "ROLLED_BACK", "ROLLBACK_FAILED"].contains(&receipt.status.as_str()) {
                receipt.status = "BLOCKED".into();
                if receipt.events.last().map(String::as_str) != Some("BLOCKED") {
                    receipt.events.push("BLOCKED".into());
                    write(&ledger, &receipt)?;
                }
            }
            return Ok(receipt);
        }
        let mut receipt = Receipt {
            schema: 1,
            job_id: job.id.clone(),
            payload_sha256: hash,
            status: "PREFLIGHT".into(),
            events: vec!["PREFLIGHT".into()],
        };
        write(&ledger, &receipt)?;
        let known_path = root.join("last-known-good.json");
        let known: Option<ProbeJob> = if known_path.exists() { Some(read(&known_path)?) } else { None };
        if let Some(previous) = &known {
            let mut historical = previous.clone();
            historical.issued_at = now;
            historical.expires_at = now + 300;
            historical.validate(&policy.agent_id, now)?;
        }
        runtime.docker(&["version", "--format", "{{.Server.Version}}"], true)?.ok_or("Docker preflight failed")?;
        if let Some(label) = runtime
            .docker(&["inspect", "--format", "{{index .Config.Labels \"io.velora.managed\"}}", "velora-guide-development-probe"], true)?
        {
            if label.trim() != "development-probe-v1" {
                return Err("Existing container is not owned by this execution policy");
            }
        }
        let compose = root.join("compose.json");
        receipt.status = "PULLING_OR_STAGING".into();
        receipt.events.push(receipt.status.clone());
        write(&ledger, &receipt)?;
        let result = activate(runtime, &compose, &job).and_then(|()| {
            receipt.status = "HEALTH_CHECK".into();
            receipt.events.push(receipt.status.clone());
            write(&ledger, &receipt)?;
            runtime.health(&job)
        });
        if result.is_ok() {
            write(&known_path, &job)?;
            receipt.status = "SUCCEEDED".into();
        } else if let Some(previous) = known {
            receipt.status = "ROLLING_BACK".into();
            receipt.events.push(receipt.status.clone());
            write(&ledger, &receipt)?;
            receipt.status = if activate(runtime, &compose, &previous).and_then(|()| runtime.health(&previous)).is_ok() {
                "ROLLED_BACK"
            } else {
                "ROLLBACK_FAILED"
            }
            .into();
        } else {
            let path = compose.to_str().ok_or("Invalid Compose state path")?;
            let stopped =
                runtime.docker(&["compose", "--project-name", "velora-guide-development-probe", "--file", path, "stop", "probe"], false)?;
            receipt.status = if stopped.is_some() { "FAILED" } else { "ROLLBACK_FAILED" }.into();
        }
        receipt.events.push(receipt.status.clone());
        write(&ledger, &receipt)?;
        Ok(receipt)
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
        use ed25519_dalek::{Signer, SigningKey};
        use std::{
            cell::RefCell,
            os::unix::fs::{symlink, PermissionsExt},
        };
        struct FixtureRuntime {
            calls: RefCell<Vec<Vec<String>>>,
        }
        impl Runtime for FixtureRuntime {
            fn docker(&self, args: &[&str], _: bool) -> Result<Option<String>, &'static str> {
                self.calls.borrow_mut().push(args.iter().map(|value| value.to_string()).collect());
                Ok(if args[0] == "inspect" { None } else { Some(String::new()) })
            }
            fn health(&self, job: &ProbeJob) -> Result<(), &'static str> {
                if job.expected_commit == "c".repeat(40) {
                    Ok(())
                } else {
                    Err("Synthetic bad health")
                }
            }
        }
        fn setup() -> (Policy, Envelope, u64) {
            let fixture: serde_json::Value =
                serde_json::from_str(include_str!("../../deployment/contracts/probe-job.fixture.json")).unwrap();
            let root = std::env::temp_dir().join(format!("velora-probe-executor-{}", crate::new_nonce().unwrap()));
            fs::create_dir(&root).unwrap();
            fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
            (
                Policy {
                    schema: 1,
                    trusted_public_key: fixture["public_key"].as_str().unwrap().into(),
                    agent_id: fixture["agent_id"].as_str().unwrap().into(),
                    state_directory: root,
                },
                serde_json::from_value(fixture["envelope"].clone()).unwrap(),
                fixture["now"].as_u64().unwrap(),
            )
        }
        fn sign(job: &ProbeJob) -> Envelope {
            let key = SigningKey::from_bytes(&[9u8; 32]);
            let payload = serde_json::to_vec(job).unwrap();
            let mut message = b"velora-probe-job-v1\n".to_vec();
            message.extend_from_slice(&payload);
            Envelope {
                schema: 1,
                key_id: hex::encode(Sha256::digest(key.verifying_key().to_bytes())),
                payload: URL_SAFE_NO_PAD.encode(payload),
                signature: URL_SAFE_NO_PAD.encode(key.sign(&message).to_bytes()),
            }
        }
        #[test]
        fn receipts_are_idempotent_and_bad_health_restores_only_the_previous_digest() {
            let (policy, envelope, now) = setup();
            let runtime = FixtureRuntime { calls: RefCell::new(vec![]) };
            assert_eq!(execute_with_runtime(&policy, &envelope, now, &runtime).unwrap().status, "SUCCEEDED");
            let count = runtime.calls.borrow().len();
            assert_eq!(execute_with_runtime(&policy, &envelope, now, &runtime).unwrap().status, "SUCCEEDED");
            assert_eq!(runtime.calls.borrow().len(), count);
            let mut bad = verify(&envelope, &policy.trusted_public_key, &policy.agent_id, now).unwrap();
            bad.expected_commit = "d".repeat(40);
            assert!(execute_with_runtime(&policy, &sign(&bad), now, &runtime).is_err());
            assert_eq!(runtime.calls.borrow().len(), count);
            bad.id = "33333333-3333-4333-8333-333333333333".into();
            bad.image = "ghcr.io/veloramcdev/deployment-probe@sha256:".to_owned() + &"e".repeat(64);
            let receipt = execute_with_runtime(&policy, &sign(&bad), now, &runtime).unwrap();
            assert_eq!(receipt.status, "ROLLED_BACK");
            assert!(receipt.events.contains(&"ROLLING_BACK".into()));
            let known: ProbeJob = read(&policy.state_directory.join("last-known-good.json")).unwrap();
            assert_eq!(known.expected_commit, "c".repeat(40));
            assert_eq!(known.image, "ghcr.io/veloramcdev/deployment-probe@sha256:".to_owned() + &"b".repeat(64));
            fs::remove_dir_all(policy.state_directory).unwrap();
        }
        #[test]
        fn held_lock_and_unowned_state_fail_before_runtime_operations() {
            let (policy, envelope, now) = setup();
            let runtime = FixtureRuntime { calls: RefCell::new(vec![]) };
            let lock = private_file(&policy.state_directory.join("deployment.lock"), true).unwrap();
            assert_eq!(unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) }, 0);
            assert!(execute_with_runtime(&policy, &envelope, now, &runtime).is_err());
            drop(lock);
            fs::set_permissions(&policy.state_directory, fs::Permissions::from_mode(0o755)).unwrap();
            assert!(execute_with_runtime(&policy, &envelope, now, &runtime).is_err());
            assert!(runtime.calls.borrow().is_empty());
            fs::remove_dir_all(policy.state_directory).unwrap();
        }
        #[test]
        fn symlinked_lock_and_state_are_rejected() {
            let (policy, envelope, now) = setup();
            let runtime = FixtureRuntime { calls: RefCell::new(vec![]) };
            let destination = policy.state_directory.join("synthetic-target");
            fs::write(&destination, b"untouched").unwrap();
            symlink(&destination, policy.state_directory.join("deployment.lock")).unwrap();
            assert!(execute_with_runtime(&policy, &envelope, now, &runtime).is_err());
            assert_eq!(fs::read(&destination).unwrap(), b"untouched");
            assert!(runtime.calls.borrow().is_empty());
            fs::remove_dir_all(policy.state_directory).unwrap();
        }
    }
}
pub fn execute(policy: &Policy, envelope: &Envelope, now: u64) -> Result<Receipt, &'static str> {
    #[cfg(unix)]
    {
        native::execute(policy, envelope, now)
    }
    #[cfg(not(unix))]
    {
        let _ = (policy, envelope, now);
        Err("Probe execution requires the Linux Docker host")
    }
}
pub fn from_files(policy: &Path, job: &Path, now: u64) -> Result<Receipt, &'static str> {
    use std::{fs::File, io::Read};
    fn bounded<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, &'static str> {
        let mut bytes = Vec::new();
        File::open(path)
            .map_err(|_| "Probe input unavailable")?
            .take(8193)
            .read_to_end(&mut bytes)
            .map_err(|_| "Probe input unreadable")?;
        if bytes.len() > 8192 {
            return Err("Probe input too large");
        }
        serde_json::from_slice(&bytes).map_err(|_| "Probe input invalid")
    }
    execute(&bounded(policy)?, &bounded(job)?, now)
}
