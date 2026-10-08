//! Explicit transport commands; the heartbeat daemon never invokes these.
use crate::{probe_executor::Policy, probe_job::Envelope};
use serde::de::DeserializeOwned;
use std::{fs::File, io::Read, path::Path};

pub fn read<T: DeserializeOwned>(path: &Path) -> Result<T, &'static str> {
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|_| "Probe transport input unavailable")?
        .take(8193)
        .read_to_end(&mut bytes)
        .map_err(|_| "Probe transport input unreadable")?;
    if bytes.len() > 8192 {
        return Err("Probe transport input too large");
    }
    serde_json::from_slice(&bytes).map_err(|_| "Invalid probe transport input")
}

pub fn stage(policy: &Policy, envelope: &Envelope, agent_id: &str, path: &Path, now: u64) -> Result<(), &'static str> {
    if policy.schema != 1 || policy.agent_id != agent_id {
        return Err("Probe transport policy denied");
    }
    crate::probe_job::verify(envelope, &policy.trusted_public_key, agent_id, now)?;
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
        let parent = path.parent().ok_or("Probe job path unavailable")?;
        let meta = std::fs::symlink_metadata(parent).map_err(|_| "Probe job directory unavailable")?;
        if !path.is_absolute()
            || !meta.is_dir()
            || meta.uid() != unsafe { libc::geteuid() }
            || meta.mode() & 0o077 != 0
            || parent.canonicalize().map_err(|_| "Probe job directory unavailable")? != parent
        {
            return Err("Probe job directory must be private, owned and canonical");
        }
        let bytes = serde_json::to_vec(envelope).map_err(|_| "Probe job encoding failed")?;
        match std::fs::OpenOptions::new().write(true).create_new(true).mode(0o600).custom_flags(libc::O_NOFOLLOW).open(path) {
            Ok(mut file) => {
                file.write_all(&bytes).map_err(|_| "Probe job write failed")?;
                file.sync_all().map_err(|_| "Probe job sync failed")?;
                File::open(parent).and_then(|directory| directory.sync_all()).map_err(|_| "Probe job directory sync failed")?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                let mut file = std::fs::OpenOptions::new()
                    .read(true)
                    .custom_flags(libc::O_NOFOLLOW)
                    .open(path)
                    .map_err(|_| "Probe job path conflict")?;
                let meta = file.metadata().map_err(|_| "Probe job path unavailable")?;
                if !meta.is_file() || meta.uid() != unsafe { libc::geteuid() } || meta.mode() & 0o077 != 0 {
                    return Err("Probe job path conflict");
                }
                let mut existing = Vec::new();
                (&mut file).take(8193).read_to_end(&mut existing).map_err(|_| "Probe job unreadable")?;
                if existing != bytes {
                    return Err("A different probe job is still staged");
                }
            }
            Err(_) => return Err("Probe job staging unavailable"),
        }
        Ok(())
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Err("Probe transport staging requires Linux")
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::{symlink, PermissionsExt};
    #[test]
    fn staging_verifies_before_writes_preserves_retries_and_rejects_symlinks() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!("../../deployment/contracts/probe-job.fixture.json")).unwrap();
        let directory = std::env::temp_dir().join(format!("velora-probe-transport-{}", crate::new_nonce().unwrap()));
        std::fs::create_dir(&directory).unwrap();
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).unwrap();
        let policy = Policy {
            schema: 1,
            trusted_public_key: fixture["public_key"].as_str().unwrap().into(),
            agent_id: fixture["agent_id"].as_str().unwrap().into(),
            state_directory: directory.clone(),
        };
        let envelope: Envelope = serde_json::from_value(fixture["envelope"].clone()).unwrap();
        let now = fixture["now"].as_u64().unwrap();
        let job = directory.join("job.json");
        assert!(stage(&policy, &envelope, "wrong-agent", &job, now).is_err());
        assert!(!job.exists());
        stage(&policy, &envelope, &policy.agent_id, &job, now).unwrap();
        stage(&policy, &envelope, &policy.agent_id, &job, now).unwrap();
        let link = directory.join("link.json");
        symlink(&job, &link).unwrap();
        assert!(stage(&policy, &envelope, &policy.agent_id, &link, now).is_err());
        std::fs::write(&job, b"previous staged job").unwrap();
        assert!(stage(&policy, &envelope, &policy.agent_id, &job, now).is_err());
        assert_eq!(std::fs::read(&job).unwrap(), b"previous staged job");
        std::fs::remove_dir_all(directory).unwrap();
    }
}
