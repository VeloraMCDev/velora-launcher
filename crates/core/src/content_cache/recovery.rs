use super::*;

pub(super) const OWNER_NAME: &str = "ownership-v1";
const OWNER_FORMAT: &[u8] = b"velora-content-cache-v1\n";

impl Key {
    pub(super) fn from_filename(name: &str) -> Result<Self, Error> {
        if name.len() > 160 {
            return Err(Error::UnsafePath);
        }
        let mut fields = name.splitn(4, '-');
        let platform = fields.next().ok_or(Error::Integrity)?.parse().map_err(|_| Error::Integrity)?;
        let artifact_type = fields.next().ok_or(Error::Integrity)?.parse().map_err(|_| Error::Integrity)?;
        let trust_domain = fields.next().ok_or(Error::Integrity)?.parse().map_err(|_| Error::Integrity)?;
        let suffix = fields.next().ok_or(Error::Integrity)?;
        let (algorithm, suffix) = if let Some(suffix) = suffix.strip_prefix("sha256-") {
            (Algorithm::Sha256, suffix)
        } else if let Some(suffix) = suffix.strip_prefix("sha1-compat-") {
            (Algorithm::Sha1Compatibility, suffix)
        } else {
            return Err(Error::Integrity);
        };
        let (digest, size) = suffix.split_once('-').ok_or(Error::Integrity)?;
        let size = size.parse().map_err(|_| Error::Integrity)?;
        let key = Self::verified_identity(algorithm, Some(digest), size, Namespace { platform, artifact_type, trust_domain })
            .ok_or(Error::Integrity)?;
        if key.filename() != name {
            return Err(Error::Integrity);
        }
        Ok(key)
    }
}

pub(super) fn same_file(first: &fs::Metadata, second: &fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        first.dev() == second.dev() && first.ino() == second.ino()
    }
    #[cfg(not(unix))]
    {
        let _ = (first, second);
        false
    }
}

fn inventory_file(path: &Path) -> Result<fs::Metadata, Error> {
    let metadata = fs::symlink_metadata(path).map_err(|_| Error::Io)?;
    if !metadata.file_type().is_file() {
        return Err(Error::UnsafePath);
    }
    Ok(metadata)
}

impl ContentCache {
    pub fn reopen(root: PathBuf, limits: Limits) -> Result<Self, Error> {
        Self::validate_limits(limits)?;
        Self::open_owned(root, limits, false)
    }

    pub(super) fn open_owned(root: PathBuf, limits: Limits, create: bool) -> Result<Self, Error> {
        if !cfg!(unix) {
            return Err(Error::UnsupportedPlatform);
        }
        safe_directory(&root)?;
        let root_metadata = fs::symlink_metadata(&root).map_err(|_| Error::Io)?;
        let owner_path = root.join(OWNER_NAME);
        if !create && regular(&owner_path)? != OWNER_FORMAT.len() as u64 {
            return Err(Error::Integrity);
        }
        let mut options = OpenOptions::new();
        options.read(true).write(true).create_new(create);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut ownership = options.open(&owner_path).map_err(|_| Error::Io)?;
        ownership.try_lock().map_err(|error| match error {
            fs::TryLockError::WouldBlock => Error::Collision,
            fs::TryLockError::Error(_) => Error::Io,
        })?;
        if create {
            ownership.write_all(OWNER_FORMAT).map_err(|_| Error::Io)?;
            ownership.sync_all().map_err(|_| Error::Io)?;
        } else {
            let mut marker = [0u8; OWNER_FORMAT.len()];
            ownership.read_exact(&mut marker).map_err(|_| Error::Integrity)?;
            if marker != OWNER_FORMAT {
                return Err(Error::Integrity);
            }
        }
        let mut cache = Self { root, limits, state: Mutex::new(State::default()), ownership, root_metadata };
        cache.check_ownership()?;
        let state = cache.recover()?;
        cache.state = Mutex::new(state);
        cache.sync_directory()?;
        if create {
            File::open(cache.root.parent().ok_or(Error::UnsafePath)?).map_err(|_| Error::Io)?.sync_all().map_err(|_| Error::Io)?;
        }
        Ok(cache)
    }

    pub(super) fn check_ownership(&self) -> Result<(), Error> {
        safe_directory(&self.root)?;
        if !same_file(&self.root_metadata, &fs::symlink_metadata(&self.root).map_err(|_| Error::Io)?) {
            return Err(Error::UnsafePath);
        }
        let path = self.root.join(OWNER_NAME);
        if regular(&path)? != OWNER_FORMAT.len() as u64
            || !same_file(&self.ownership.metadata().map_err(|_| Error::Io)?, &fs::symlink_metadata(path).map_err(|_| Error::Io)?)
        {
            return Err(Error::UnsafePath);
        }
        Ok(())
    }

    pub(super) fn sync_directory(&self) -> Result<(), Error> {
        self.check_ownership()?;
        File::open(&self.root).map_err(|_| Error::Io)?.sync_all().map_err(|_| Error::Io)
    }

    fn recover(&self) -> Result<State, Error> {
        let mut state = State::default();
        let mut scratch = None;
        let mut committed = Vec::new();
        for entry in fs::read_dir(&self.root).map_err(|_| Error::Io)? {
            let entry = entry.map_err(|_| Error::Io)?;
            let name = entry.file_name().into_string().map_err(|_| Error::UnsafePath)?;
            if name == OWNER_NAME {
                continue;
            }
            let metadata = inventory_file(&entry.path())?;
            if let Some(sequence) = name.strip_prefix("staging-") {
                let sequence: u64 = sequence.parse().map_err(|_| Error::Integrity)?;
                if sequence == 0 || name != format!("staging-{sequence}") || scratch.is_some() {
                    return Err(Error::Integrity);
                }
                self.capacity(&State::default(), metadata.len())?;
                state.sequence = sequence;
                scratch = Some((entry.path(), metadata));
            } else {
                if committed.len() >= self.limits.max_entries {
                    return Err(Error::Capacity);
                }
                let key = Key::from_filename(&name)?;
                self.capacity(&state, key.size)?;
                if metadata.len() != key.size {
                    return Err(Error::Integrity);
                }
                state.bytes = state.bytes.checked_add(key.size).ok_or(Error::Capacity)?;
                state.entries.insert(name, key.size);
                committed.push((entry.path(), metadata, key));
            }
        }
        let linked = scratch
            .as_ref()
            .and_then(|(_, scratch_metadata)| committed.iter().find(|(_, metadata, _)| same_file(metadata, scratch_metadata)));
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            for (_, metadata, _) in &committed {
                let expected_links = if linked.is_some_and(|(_, linked_metadata, _)| same_file(metadata, linked_metadata)) { 2 } else { 1 };
                if metadata.nlink() != expected_links {
                    return Err(Error::UnsafePath);
                }
            }
            if let Some((_, metadata)) = &scratch {
                if metadata.nlink() != if linked.is_some() { 2 } else { 1 } {
                    return Err(Error::UnsafePath);
                }
            }
        }
        if let Some((_, metadata)) = &scratch {
            if linked.is_none() && state.bytes.checked_add(metadata.len()).is_none_or(|total| total > self.limits.max_disk_bytes) {
                return Err(Error::Capacity);
            }
        }
        for (path, _, key) in &committed {
            let digest = match key.algorithm {
                Algorithm::Sha256 => velora_platform_utils::authlib::sha256_file(path),
                Algorithm::Sha1Compatibility => velora_platform_utils::http::sha1_file(path),
            }
            .map_err(|_| Error::Io)?;
            if digest != key.digest {
                return Err(Error::Integrity);
            }
        }
        self.check_ownership()?;
        if let Some((path, _)) = scratch {
            fs::remove_file(path).map_err(|_| Error::Io)?;
            self.sync_directory()?;
        }
        Ok(state)
    }
}
