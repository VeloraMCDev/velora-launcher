//! Persisted signing material must never be silently replaced during startup.
use anyhow::{bail, Context, Result};
use rand::RngCore;
use std::fs::OpenOptions;
use std::io::{ErrorKind, Write};
use std::path::Path;

pub(crate) fn write_new(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent().filter(|parent| !parent.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

fn existing_jwt(path: &Path) -> Result<Vec<u8>> {
    let bytes = std::fs::read(path).context("reading existing JWT signing secret; restore it rather than replacing it")?;
    if bytes.len() < 32 {
        bail!("existing JWT signing secret is shorter than 32 bytes; inspect or restore it explicitly; automatic rotation is refused");
    }
    Ok(bytes)
}

/// Preserve configured legacy bytes exactly; load persisted bytes or create once.
/// Never replace an unreadable, short or concurrently created persisted secret.
pub fn jwt_secret(configured: Option<&[u8]>, path: &Path) -> Result<Vec<u8>> {
    if let Some(bytes) = configured {
        return Ok(bytes.to_vec());
    }
    match std::fs::read(path) {
        Ok(bytes) => {
            if bytes.len() < 32 {
                return existing_jwt(path);
            }
            return Ok(bytes);
        }
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Err(error) => return Err(error).context("reading existing JWT signing secret; automatic replacement is refused"),
    }
    let mut bytes = vec![0; 64];
    rand::thread_rng().fill_bytes(&mut bytes);
    match write_new(path, &bytes) {
        Ok(()) => Ok(bytes),
        Err(error) if error.kind() == ErrorKind::AlreadyExists => existing_jwt(path),
        Err(error) => Err(error).context("creating JWT signing secret; inspect any incomplete file before retrying"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn jwt_bytes_survive_restarts_and_configured_legacy_values_are_preserved() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("jwt.secret");
        let first = jwt_secret(None, &path).unwrap();
        assert_eq!(first.len(), 64);
        assert_eq!(jwt_secret(None, &path).unwrap(), first);
        assert_eq!(jwt_secret(Some(b"synthetic-legacy"), &path).unwrap(), b"synthetic-legacy");
        assert_eq!(std::fs::read(&path).unwrap(), first);
        assert!(write_new(&path, b"replacement").is_err());
        assert_eq!(std::fs::read(&path).unwrap(), first);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(std::fs::metadata(path).unwrap().permissions().mode() & 0o777, 0o600);
        }
    }
    #[test]
    fn invalid_or_unreadable_existing_jwt_material_is_never_rotated() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("jwt.secret");
        std::fs::write(&path, b"synthetic-short").unwrap();
        assert!(jwt_secret(None, &path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"synthetic-short");
        assert!(jwt_secret(None, directory.path()).is_err());
        assert!(directory.path().is_dir());
    }
}
