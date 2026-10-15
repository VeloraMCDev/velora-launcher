use serde::{de::DeserializeOwned, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub(crate) fn read_json<Value: DeserializeOwned>(path: &Path) -> anyhow::Result<Option<Value>> {
    read_bytes(path)?.map(|bytes| serde_json::from_slice(&bytes).map_err(Into::into)).transpose()
}

pub(crate) fn read_bytes(path: &Path) -> anyhow::Result<Option<Vec<u8>>> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if !metadata.file_type().is_file() => {
            anyhow::bail!("JSON destination must be a regular file")
        }
        Ok(_) => Ok(Some(fs::read(path)?)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

pub(crate) fn atomic_write_json<Value: Serialize + DeserializeOwned>(path: &Path, value: &Value) -> anyhow::Result<()> {
    write_json_with(
        path,
        &temporary_path(path)?,
        value,
        |file, bytes| file.write_all(bytes),
        |source, destination| fs::rename(source, destination),
    )
}

pub(crate) fn atomic_write_bytes(path: &Path, bytes: &[u8], validate: impl Fn(&[u8]) -> anyhow::Result<()>) -> anyhow::Result<()> {
    let verify = || -> anyhow::Result<()> {
        match fs::symlink_metadata(path) {
            Ok(metadata) if !metadata.file_type().is_file() => anyhow::bail!("persistence destination must be a regular file"),
            Ok(_) => validate(&fs::read(path)?),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
    };
    write_bytes_with(
        path,
        &temporary_path(path)?,
        bytes,
        verify,
        |file, bytes| file.write_all(bytes),
        |source, destination| fs::rename(source, destination),
    )
}

fn temporary_path(path: &Path) -> anyhow::Result<PathBuf> {
    let mut name = std::ffi::OsString::from(".");
    name.push(path.file_name().ok_or_else(|| anyhow::anyhow!("JSON destination has no filename"))?);
    name.push(format!(".{}.tmp", Uuid::new_v4()));
    Ok(path.with_file_name(name))
}

fn write_json_with<Value, WriteBytes, Rename>(
    path: &Path,
    temporary: &Path,
    value: &Value,
    write_bytes: WriteBytes,
    rename: Rename,
) -> anyhow::Result<()>
where
    Value: Serialize + DeserializeOwned,
    WriteBytes: FnOnce(&mut File, &[u8]) -> io::Result<()>,
    Rename: FnOnce(&Path, &Path) -> io::Result<()>,
{
    let bytes = serde_json::to_vec_pretty(value)?;
    write_bytes_with(
        path,
        temporary,
        &bytes,
        || {
            read_json::<Value>(path)?;
            Ok(())
        },
        write_bytes,
        rename,
    )
}

fn write_bytes_with(
    path: &Path,
    temporary: &Path,
    bytes: &[u8],
    verify: impl Fn() -> anyhow::Result<()>,
    write_bytes: impl FnOnce(&mut File, &[u8]) -> io::Result<()>,
    rename: impl FnOnce(&Path, &Path) -> io::Result<()>,
) -> anyhow::Result<()> {
    if temporary.parent() != path.parent() || temporary.file_name().is_none() || temporary == path {
        anyhow::bail!("JSON temporary must be a distinct sibling file");
    }
    verify()?;
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(temporary)?;
    let result = (|| -> anyhow::Result<()> {
        let metadata = file.metadata()?;
        if !metadata.file_type().is_file() {
            anyhow::bail!("JSON temporary must be a regular file");
        }
        write_bytes(&mut file, bytes)?;
        file.sync_all()?;
        drop(file);
        let current = fs::symlink_metadata(temporary)?;
        if !current.file_type().is_file() {
            anyhow::bail!("JSON temporary must remain a regular file");
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if current.dev() != metadata.dev() || current.ino() != metadata.ino() {
                anyhow::bail!("JSON temporary identity changed");
            }
        }
        verify()?;
        rename(temporary, path)?;
        Ok(())
    })();
    if result.is_err() && fs::remove_file(temporary).is_err() {
        tracing::warn!("Failed to clean up JSON temporary file after persistence failure");
    }
    result
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) struct TestDirectory(pub(crate) PathBuf);

    impl TestDirectory {
        pub(crate) fn new() -> Self {
            let directory = std::env::temp_dir().join(format!("velora-persistence-test-{}", Uuid::new_v4()));
            fs::create_dir(&directory).unwrap();
            Self(directory)
        }

        pub(crate) fn settings_path(&self) -> PathBuf {
            self.0.join("settings.json")
        }

        pub(crate) fn assert_only_settings(&self) {
            let entries: Vec<_> = fs::read_dir(&self.0).unwrap().map(|entry| entry.unwrap().path()).collect();
            assert_eq!(entries, vec![self.settings_path()]);
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    #[test]
    fn temporary_names_are_unique_uuid_siblings() {
        let directory = TestDirectory::new();
        let path = directory.settings_path();
        let first = temporary_path(&path).unwrap();
        let second = temporary_path(&path).unwrap();
        assert_ne!(first, second);
        assert_eq!(first.parent(), path.parent());
        let name = first.file_name().unwrap().to_str().unwrap();
        let identifier = name.strip_prefix(".settings.json.").unwrap().strip_suffix(".tmp").unwrap();
        assert_eq!(Uuid::parse_str(identifier).unwrap().get_version_num(), 4);
    }

    #[test]
    fn cleanup_failure_does_not_mask_original_error() {
        let directory = TestDirectory::new();
        let path = directory.settings_path();
        let temporary = temporary_path(&path).unwrap();
        fs::write(&path, b"[1]").unwrap();
        let error = write_json_with(
            &path,
            &temporary,
            &vec![2],
            |file, bytes| file.write_all(bytes),
            |source, _| {
                fs::remove_file(source)?;
                fs::create_dir(source)?;
                Err(io::Error::new(io::ErrorKind::PermissionDenied, "synthetic original failure"))
            },
        )
        .unwrap_err();
        let original = error.downcast_ref::<io::Error>().unwrap();
        assert_eq!(original.kind(), io::ErrorKind::PermissionDenied);
        assert_eq!(original.to_string(), "synthetic original failure");
        assert_eq!(fs::read(&path).unwrap(), b"[1]");
        assert!(temporary.is_dir());
    }

    #[test]
    fn existing_temporary_directory_is_not_removed() {
        let directory = TestDirectory::new();
        let path = directory.settings_path();
        let temporary = temporary_path(&path).unwrap();
        fs::write(&path, b"[1]").unwrap();
        fs::create_dir(&temporary).unwrap();
        assert!(write_json_with(
            &path,
            &temporary,
            &vec![2],
            |file, bytes| file.write_all(bytes),
            |source, destination| fs::rename(source, destination)
        )
        .is_err());
        assert!(temporary.is_dir());
        assert_eq!(fs::read(&path).unwrap(), b"[1]");
    }

    #[test]
    fn failed_partial_write_preserves_destination_and_original_error() {
        let directory = TestDirectory::new();
        let path = directory.settings_path();
        fs::write(&path, b"[1]").unwrap();
        let error = write_json_with(
            &path,
            &temporary_path(&path).unwrap(),
            &vec![2],
            |file, _| {
                file.write_all(b"[")?;
                Err(io::Error::new(io::ErrorKind::WriteZero, "synthetic write failure"))
            },
            |_, _| panic!("rename must not run after write failure"),
        )
        .unwrap_err();
        assert_eq!(error.downcast_ref::<io::Error>().unwrap().kind(), io::ErrorKind::WriteZero);
        assert_eq!(fs::read(&path).unwrap(), b"[1]");
        directory.assert_only_settings();
    }

    #[test]
    fn failed_rename_preserves_destination_and_original_error() {
        let directory = TestDirectory::new();
        let path = directory.settings_path();
        fs::write(&path, b"[1]").unwrap();
        let error = write_json_with(
            &path,
            &temporary_path(&path).unwrap(),
            &vec![2],
            |file, bytes| file.write_all(bytes),
            |_, _| Err(io::Error::new(io::ErrorKind::PermissionDenied, "synthetic rename failure")),
        )
        .unwrap_err();
        assert_eq!(error.downcast_ref::<io::Error>().unwrap().kind(), io::ErrorKind::PermissionDenied);
        assert_eq!(fs::read(&path).unwrap(), b"[1]");
        directory.assert_only_settings();
    }

    #[test]
    fn collision_is_not_truncated_or_removed() {
        let directory = TestDirectory::new();
        let path = directory.settings_path();
        let temporary = temporary_path(&path).unwrap();
        fs::write(&path, b"[1]").unwrap();
        fs::write(&temporary, b"synthetic collision").unwrap();
        let error = write_json_with(
            &path,
            &temporary,
            &vec![2],
            |file, bytes| file.write_all(bytes),
            |source, destination| fs::rename(source, destination),
        )
        .unwrap_err();
        assert_eq!(error.downcast_ref::<io::Error>().unwrap().kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read(&temporary).unwrap(), b"synthetic collision");
        assert_eq!(fs::read(&path).unwrap(), b"[1]");
    }

    #[test]
    fn unsafe_temporary_location_is_rejected() {
        let directory = TestDirectory::new();
        let path = directory.settings_path();
        fs::write(&path, b"[1]").unwrap();
        for temporary in [path.clone(), directory.0.join("absent").join("temporary")] {
            assert!(write_json_with(
                &path,
                &temporary,
                &vec![2],
                |file, bytes| file.write_all(bytes),
                |source, destination| fs::rename(source, destination)
            )
            .is_err());
        }
        assert_eq!(fs::read(&path).unwrap(), b"[1]");
        directory.assert_only_settings();
    }

    #[test]
    fn destination_corrupted_during_write_is_preserved() {
        let directory = TestDirectory::new();
        let path = directory.settings_path();
        fs::write(&path, b"[1]").unwrap();
        assert!(write_json_with(
            &path,
            &temporary_path(&path).unwrap(),
            &vec![2],
            |file, bytes| {
                file.write_all(bytes)?;
                fs::write(&path, b"synthetic corrupt replacement")
            },
            |_, _| panic!("rename must not run after failed revalidation"),
        )
        .is_err());
        assert_eq!(fs::read(&path).unwrap(), b"synthetic corrupt replacement");
        directory.assert_only_settings();
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_and_nonregular_files_are_rejected() {
        use std::os::unix::fs::symlink;
        let directory = TestDirectory::new();
        let path = directory.settings_path();
        let target = directory.0.join("synthetic-target.json");
        fs::write(&target, b"[1]").unwrap();
        symlink(&target, &path).unwrap();
        assert!(atomic_write_json(&path, &vec![2]).is_err());
        assert!(fs::symlink_metadata(&path).unwrap().file_type().is_symlink());
        fs::remove_file(&path).unwrap();
        fs::create_dir(&path).unwrap();
        assert!(atomic_write_json(&path, &vec![2]).is_err());
        fs::remove_dir(&path).unwrap();
        let temporary = temporary_path(&path).unwrap();
        symlink(&target, &temporary).unwrap();
        assert!(write_json_with(
            &path,
            &temporary,
            &vec![2],
            |file, bytes| file.write_all(bytes),
            |source, destination| fs::rename(source, destination)
        )
        .is_err());
        assert!(fs::symlink_metadata(&temporary).unwrap().file_type().is_symlink());
        assert_eq!(fs::read(&target).unwrap(), b"[1]");
    }

    #[cfg(unix)]
    #[test]
    fn new_file_has_private_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let directory = TestDirectory::new();
        let path = directory.settings_path();
        atomic_write_json(&path, &vec![1]).unwrap();
        assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
        directory.assert_only_settings();
    }
}
