use recovery::OWNER_NAME;
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Algorithm {
    Sha256,
    Sha1Compatibility,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Namespace {
    pub platform: u64,
    pub artifact_type: u64,
    pub trust_domain: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Key {
    algorithm: Algorithm,
    digest: String,
    size: u64,
    namespace: Namespace,
}

impl Key {
    pub fn verified_identity(algorithm: Algorithm, digest: Option<&str>, size: u64, namespace: Namespace) -> Option<Self> {
        let digest = digest?;
        let length = match algorithm {
            Algorithm::Sha256 => 64,
            Algorithm::Sha1Compatibility => 40,
        };
        if digest.len() != length || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return None;
        }
        Some(Self { algorithm, digest: digest.to_ascii_lowercase(), size, namespace })
    }

    fn filename(&self) -> String {
        let algorithm = match self.algorithm {
            Algorithm::Sha256 => "sha256",
            Algorithm::Sha1Compatibility => "sha1-compat",
        };
        format!(
            "{}-{}-{}-{}-{}-{}",
            self.namespace.platform, self.namespace.artifact_type, self.namespace.trust_domain, algorithm, self.digest, self.size
        )
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub max_object_bytes: u64,
    pub max_disk_bytes: u64,
    pub max_entries: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidLimits,
    UnsafePath,
    Io,
    Integrity,
    Cancelled,
    Capacity,
    Collision,
    Poisoned,
    UnsupportedPlatform,
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "content cache: {self:?}")
    }
}

impl std::error::Error for Error {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Population {
    Published,
    Existing,
    Declined,
}

#[derive(Default)]
struct State {
    entries: BTreeMap<String, u64>,
    bytes: u64,
    sequence: u64,
}

pub struct ContentCache {
    root: PathBuf,
    limits: Limits,
    state: Mutex<State>,
    ownership: File,
    root_metadata: fs::Metadata,
}

struct Scratch {
    path: PathBuf,
    file: Option<File>,
    removed: bool,
}

impl Scratch {
    fn cleanup(&mut self) -> Result<(), Error> {
        drop(self.file.take());
        if !self.removed {
            fs::remove_file(&self.path).map_err(|_| Error::Io)?;
            self.removed = true;
        }
        Ok(())
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

fn safe_directory(path: &Path) -> Result<(), Error> {
    if !path.is_absolute()
        || path.components().any(|component| !matches!(component, Component::RootDir | Component::Prefix(_) | Component::Normal(_)))
    {
        return Err(Error::UnsafePath);
    }
    for ancestor in path.ancestors() {
        if !fs::symlink_metadata(ancestor).map_err(|_| Error::UnsafePath)?.file_type().is_dir() {
            return Err(Error::UnsafePath);
        }
    }
    Ok(())
}

fn regular(path: &Path) -> Result<u64, Error> {
    let metadata = fs::symlink_metadata(path).map_err(|_| Error::Io)?;
    if !metadata.file_type().is_file() {
        return Err(Error::UnsafePath);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.nlink() != 1 {
            return Err(Error::UnsafePath);
        }
    }
    Ok(metadata.len())
}

fn check_cancel(cancelled: &impl Fn() -> bool) -> Result<(), Error> {
    if cancelled() {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}

impl ContentCache {
    pub fn create(root: PathBuf, limits: Limits) -> Result<Self, Error> {
        Self::validate_limits(limits)?;
        if !cfg!(unix) {
            return Err(Error::UnsupportedPlatform);
        }
        let parent = root.parent().ok_or(Error::UnsafePath)?;
        safe_directory(parent)?;
        if root.file_name().is_none() || root.components().any(|component| matches!(component, Component::ParentDir | Component::CurDir)) {
            return Err(Error::UnsafePath);
        }
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder
            .create(&root)
            .map_err(|error| if error.kind() == std::io::ErrorKind::AlreadyExists { Error::Collision } else { Error::Io })?;
        Self::open_owned(root, limits, true)
    }

    fn validate_limits(limits: Limits) -> Result<(), Error> {
        if limits.max_entries == 0
            || limits.max_object_bytes == 0
            || limits.max_object_bytes > limits.max_disk_bytes
            || usize::try_from(limits.max_object_bytes).is_err()
        {
            return Err(Error::InvalidLimits);
        }
        Ok(())
    }

    fn inspect(&self, state: &State) -> Result<(), Error> {
        self.check_ownership()?;
        let mut count = 0;
        for entry in fs::read_dir(&self.root).map_err(|_| Error::Io)? {
            let entry = entry.map_err(|_| Error::Io)?;
            let name = entry.file_name().into_string().map_err(|_| Error::UnsafePath)?;
            if name == OWNER_NAME {
                continue;
            }
            let expected = state.entries.get(&name).ok_or(Error::Collision)?;
            if regular(&entry.path())? != *expected {
                return Err(Error::Integrity);
            }
            count += 1;
        }
        if count != state.entries.len() {
            return Err(Error::Integrity);
        }
        Ok(())
    }

    fn scratch(&self, state: &mut State) -> Result<Scratch, Error> {
        state.sequence = state.sequence.checked_add(1).ok_or(Error::Capacity)?;
        let path = self.root.join(format!("staging-{}", state.sequence));
        let mut options = OpenOptions::new();
        options.read(true).write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let file =
            options
                .open(&path)
                .map_err(|error| if error.kind() == std::io::ErrorKind::AlreadyExists { Error::Collision } else { Error::Io })?;
        Ok(Scratch { path, file: Some(file), removed: false })
    }

    fn capacity(&self, state: &State, size: u64) -> Result<(), Error> {
        if size > self.limits.max_object_bytes || state.bytes.checked_add(size).is_none_or(|total| total > self.limits.max_disk_bytes) {
            return Err(Error::Capacity);
        }
        Ok(())
    }

    fn stage(&self, scratch: &mut Scratch, reader: &mut impl Read, key: &Key, cancelled: &impl Fn() -> bool) -> Result<(), Error> {
        let mut remaining = key.size;
        let mut buffer = [0u8; 65536];
        let staged_file = scratch.file.as_mut().ok_or(Error::Io)?;
        while remaining > 0 {
            check_cancel(cancelled)?;
            let requested = remaining.min(buffer.len() as u64) as usize;
            let count = reader.read(&mut buffer[..requested]).map_err(|_| Error::Io)?;
            if count == 0 {
                return Err(Error::Integrity);
            }
            staged_file.write_all(&buffer[..count]).map_err(|_| Error::Io)?;
            remaining -= count as u64;
        }
        check_cancel(cancelled)?;
        if reader.read(&mut buffer[..1]).map_err(|_| Error::Io)? != 0 {
            return Err(Error::Integrity);
        }
        staged_file.flush().map_err(|_| Error::Io)?;
        staged_file.sync_all().map_err(|_| Error::Io)?;
        if regular(&scratch.path)? != key.size {
            return Err(Error::Integrity);
        }
        let digest = match key.algorithm {
            Algorithm::Sha256 => velora_platform_utils::authlib::sha256_file(&scratch.path),
            Algorithm::Sha1Compatibility => velora_platform_utils::http::sha1_file(&scratch.path),
        }
        .map_err(|_| Error::Io)?;
        if digest != key.digest {
            return Err(Error::Integrity);
        }
        check_cancel(cancelled)
    }

    fn retrieve(&self, state: &mut State, key: &Key, cancelled: &impl Fn() -> bool) -> Result<Option<Vec<u8>>, Error> {
        let name = key.filename();
        if !state.entries.contains_key(&name) {
            return Ok(None);
        }
        self.capacity(state, key.size)?;
        let source = self.root.join(name);
        if regular(&source)? != key.size {
            return Err(Error::Integrity);
        }
        let mut reader = File::open(source).map_err(|_| Error::Io)?;
        let mut scratch = self.scratch(state)?;
        self.stage(&mut scratch, &mut reader, key, cancelled)?;
        let bytes = fs::read(&scratch.path).map_err(|_| Error::Io)?;
        check_cancel(cancelled)?;
        scratch.cleanup()?;
        Ok(Some(bytes))
    }

    pub fn get(&self, key: Option<&Key>, cancelled: impl Fn() -> bool) -> Result<Option<Vec<u8>>, Error> {
        let Some(key) = key else { return Ok(None) };
        check_cancel(&cancelled)?;
        let mut state = self.state.lock().map_err(|_| Error::Poisoned)?;
        self.inspect(&state)?;
        self.retrieve(&mut state, key, &cancelled)
    }

    pub fn populate(&self, key: Option<&Key>, reader: &mut impl Read, cancelled: impl Fn() -> bool) -> Result<Population, Error> {
        let Some(key) = key else { return Ok(Population::Declined) };
        check_cancel(&cancelled)?;
        let mut state = self.state.lock().map_err(|_| Error::Poisoned)?;
        self.inspect(&state)?;
        if self.retrieve(&mut state, key, &cancelled)?.is_some() {
            return Ok(Population::Existing);
        }
        self.capacity(&state, key.size)?;
        if state.entries.len() >= self.limits.max_entries {
            return Err(Error::Capacity);
        }
        let mut scratch = self.scratch(&mut state)?;
        self.stage(&mut scratch, reader, key, &cancelled)?;
        let name = key.filename();
        fs::hard_link(&scratch.path, self.root.join(&name)).map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                Error::Collision
            } else {
                Error::Io
            }
        })?;
        state.entries.insert(name, key.size);
        state.bytes += key.size;
        self.sync_directory()?;
        scratch.cleanup()?;
        self.sync_directory()?;
        Ok(Population::Published)
    }
}

mod recovery;

#[cfg(all(test, unix))]
mod tests;

#[cfg(all(test, not(unix)))]
mod unsupported_tests {
    use super::*;

    #[test]
    fn unsupported_platform_does_not_create_or_adopt_a_cache() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("cache");
        let limits = Limits { max_object_bytes: 1, max_disk_bytes: 1, max_entries: 1 };
        assert!(matches!(ContentCache::create(root.clone(), limits), Err(Error::UnsupportedPlatform)));
        assert!(!root.exists());
        assert!(matches!(ContentCache::reopen(root, limits), Err(Error::UnsupportedPlatform)));
    }
}
