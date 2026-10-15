use super::safety;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const JOURNAL_LIMIT: u64 = 16 * 1024 * 1024;

type Snapshot = Option<Vec<(PathBuf, Option<String>)>>;

#[cfg(test)]
type PublicationHook = fn(&str, &Path, &Path) -> Result<()>;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct MoveRecord {
    source: String,
    destination: String,
    snapshot: Snapshot,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    version: u32,
    original_state: Snapshot,
    next_state: Snapshot,
    moves: Vec<MoveRecord>,
    directories: Vec<String>,
    rollback_complete: bool,
    #[serde(default)]
    cleanup_started: bool,
}

pub(super) struct Ownership {
    _file: File,
}

pub(super) fn claim(game: &Path) -> Result<Ownership> {
    let metadata = safety::inspect(game, ".scopenet", true)?;
    std::fs::create_dir_all(&metadata)?;
    let path = safety::inspect(game, ".scopenet/sync.lock", false)?;
    let file = OpenOptions::new().read(true).write(true).create(true).truncate(false).open(&path)?;
    if !file.metadata()?.is_file() {
        bail!("sync ownership is not a regular file");
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if file.metadata()?.nlink() != 1 {
            bail!("sync ownership file has multiple links");
        }
    }
    file.try_lock().context("unfinished or concurrent sync: ownership is held")?;
    Ok(Ownership { _file: file })
}

pub(super) fn flush_directory(path: &Path) -> Result<()> {
    #[cfg(unix)]
    File::open(path)?.sync_all()?;
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

#[derive(Default)]
pub(super) struct Publication {
    moves: Vec<(PathBuf, PathBuf, Snapshot)>,
    directories: Vec<PathBuf>,
    root: Option<PathBuf>,
    journal: Option<Journal>,
    #[cfg(test)]
    pub(super) hook: Option<PublicationHook>,
}

impl Publication {
    pub(super) fn start(game: &Path, workspace: &Path) -> Result<Self> {
        let mut publication = Self {
            root: Some(game.to_path_buf()),
            journal: Some(Journal {
                version: 1,
                original_state: safety::fingerprint(&super::state_path(game))?,
                next_state: None,
                moves: Vec::new(),
                directories: Vec::new(),
                rollback_complete: false,
                cleanup_started: false,
            }),
            ..Self::default()
        };
        flush_directory(workspace.parent().context("staging parent missing")?)?;
        publication.persist()?;
        Ok(publication)
    }

    fn relative(&self, path: &Path) -> Result<String> {
        let root = self.root.as_ref().context("publication root missing")?;
        let relative = path.strip_prefix(root)?.to_str().context("non-UTF8 publication path")?.replace('\\', "/");
        safety::normalize(&relative, true)
    }

    fn persist(&mut self) -> Result<()> {
        let (Some(root), Some(journal)) = (&self.root, &self.journal) else { return Ok(()) };
        let workspace = root.join(".scopenet/staged-sync");
        let bytes = serde_json::to_vec(journal)?;
        if bytes.len() as u64 > JOURNAL_LIMIT {
            bail!("sync journal exceeds bounded size");
        }
        let temporary = safety::inspect(root, ".scopenet/staged-sync/journal.tmp", false)?;
        let mut output = OpenOptions::new().write(true).create_new(true).open(&temporary)?;
        output.write_all(&bytes)?;
        output.sync_all()?;
        drop(output);
        std::fs::rename(&temporary, workspace.join("journal.json"))?;
        flush_directory(&workspace)
    }

    pub(super) fn prepare_state(&mut self, path: &Path) -> Result<()> {
        if let Some(journal) = &mut self.journal {
            journal.next_state = safety::fingerprint(path)?;
        }
        self.persist()
    }

    pub(super) fn authorize_cleanup(&mut self) -> Result<()> {
        if let Some(journal) = &mut self.journal {
            journal.cleanup_started = true;
        }
        self.persist()
    }

    pub(super) fn parents(&mut self, path: &Path) -> Result<()> {
        let parent = path.parent().context("sync destination has no parent")?;
        match safety::metadata(parent)? {
            Some(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => return Ok(()),
            Some(_) => bail!("unsafe publication parent: {}", parent.display()),
            None => {}
        }
        self.parents(parent)?;
        self.directories.push(parent.to_path_buf());
        if self.journal.is_some() {
            let relative = self.relative(parent)?;
            self.journal.as_mut().unwrap().directories.push(relative);
            self.persist()?;
        }
        std::fs::create_dir(parent)?;
        flush_directory(parent.parent().context("publication parent missing")?)?;
        Ok(())
    }

    pub(super) fn move_path(&mut self, source: &Path, destination: &Path) -> Result<()> {
        self.move_paths(&[(source.to_path_buf(), destination.to_path_buf())])
    }

    pub(super) fn plan_moves(&mut self, paths: &[(PathBuf, PathBuf)]) -> Result<usize> {
        let start = self.moves.len();
        let mut relative_paths = Vec::new();
        let mut planned = Vec::new();
        for (source, destination) in paths {
            relative_paths.push(self.relative(source)?);
            relative_paths.push(self.relative(destination)?);
            if safety::metadata(destination)?.is_some() {
                bail!("refusing publication overwrite: {}", destination.display());
            }
            let snapshot = safety::fingerprint(source)?;
            if snapshot.is_none() {
                bail!("publication source disappeared");
            }
            planned.push((source.clone(), destination.clone(), snapshot));
        }
        safety::unique(&relative_paths)?;
        for (_, destination, _) in &planned {
            self.parents(destination)?;
        }
        for (source, destination, snapshot) in planned {
            if self.journal.is_some() {
                let source_relative = self.relative(&source)?;
                let destination_relative = self.relative(&destination)?;
                self.journal.as_mut().unwrap().moves.push(MoveRecord {
                    source: source_relative,
                    destination: destination_relative,
                    snapshot: snapshot.clone(),
                });
            }
            self.moves.push((source, destination, snapshot));
        }
        if !paths.is_empty() {
            #[cfg(test)]
            if let Some(hook) = self.hook {
                hook("before_plan_persist", &paths[0].0, &paths[0].1)?;
            }
            self.persist()?;
        }
        Ok(start)
    }

    pub(super) fn move_paths(&mut self, paths: &[(PathBuf, PathBuf)]) -> Result<()> {
        let start = self.plan_moves(paths)?;
        for (source, destination, snapshot) in &self.moves[start..] {
            #[cfg(test)]
            if let Some(hook) = self.hook {
                hook("before_move", source, destination)?;
            }
            if safety::metadata(destination)?.is_some() || safety::fingerprint(source)? != *snapshot {
                bail!("planned publication paths changed; refusing mutation");
            }
            std::fs::rename(source, destination).with_context(|| format!("moving {} to {}", source.display(), destination.display()))?;
            flush_directory(source.parent().context("move source parent missing")?)?;
            flush_directory(destination.parent().context("move destination parent missing")?)?;
        }
        Ok(())
    }

    pub(super) fn rollback(&mut self) -> Result<()> {
        while let Some((source, destination, snapshot)) = self.moves.last() {
            let existing = safety::fingerprint(source)?;
            if existing.is_some() {
                if existing != *snapshot {
                    bail!("rollback destination occupied: {}", source.display());
                }
                if safety::metadata(destination)?.is_some() {
                    bail!("rollback has two occupied paths; retain both for review");
                }
            } else {
                if safety::fingerprint(destination)? != *snapshot {
                    bail!("rollback source changed: {}", destination.display());
                }
                std::fs::rename(destination, source)
                    .with_context(|| format!("restoring {} from {}", source.display(), destination.display()))?;
                flush_directory(source.parent().context("rollback parent missing")?)?;
                flush_directory(destination.parent().context("rollback destination parent missing")?)?;
                #[cfg(test)]
                if let Some(hook) = self.hook {
                    hook("after_reverse", source, destination)?;
                }
            }
            #[cfg(test)]
            let reversed = (source.clone(), destination.clone());
            self.moves.pop();
            if let Some(journal) = &mut self.journal {
                journal.moves.pop();
            }
            self.persist()?;
            #[cfg(test)]
            if let Some(hook) = self.hook {
                hook("after_reverse_persist", &reversed.0, &reversed.1)?;
            }
        }
        while let Some(directory) = self.directories.last() {
            match std::fs::remove_dir(directory) {
                Ok(()) => flush_directory(directory.parent().context("rollback directory parent missing")?)?,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error).with_context(|| format!("removing publication directory {}", directory.display())),
            }
            self.directories.pop();
            if let Some(journal) = &mut self.journal {
                journal.directories.pop();
            }
            self.persist()?;
        }
        if let Some(journal) = &mut self.journal {
            journal.rollback_complete = true;
        }
        self.persist()?;
        Ok(())
    }
}

pub(super) fn write_state(path: &Path, state: &super::SyncState) -> Result<()> {
    use std::io::Write;
    let mut output = std::fs::OpenOptions::new().write(true).create_new(true).open(path)?;
    output.write_all(&serde_json::to_vec_pretty(state)?)?;
    output.sync_all()?;
    Ok(())
}

pub(super) fn recover(game: &Path) -> Result<()> {
    let workspace = safety::inspect(game, ".scopenet/staged-sync", true)?;
    if safety::metadata(&workspace)?.is_none() {
        return Ok(());
    }
    if std::fs::read_dir(&workspace)?.next().is_none() {
        std::fs::remove_dir(&workspace)?;
        return flush_directory(workspace.parent().unwrap());
    }
    let path = safety::inspect(game, ".scopenet/staged-sync/journal.json", false)?;
    let mut bytes = Vec::new();
    File::open(&path)
        .context("unfinished sync lacks recovery journal; manual review required")?
        .take(JOURNAL_LIMIT + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > JOURNAL_LIMIT {
        bail!("sync recovery journal exceeds bounded size");
    }
    let journal: Journal = serde_json::from_slice(&bytes).context("invalid recovery journal; refusing mutation")?;
    validate_journal(game, &journal)?;
    let staged_state = workspace.join("state.json");
    let current = safety::fingerprint(&super::state_path(game))?;
    if journal.rollback_complete {
        if current != journal.original_state || !journal.moves.is_empty() || !journal.directories.is_empty() {
            bail!("completed rollback evidence changed; refusing cleanup");
        }
        return cleanup_workspace(&workspace);
    }
    if journal.next_state.is_some() && safety::metadata(&staged_state)?.is_none() {
        if current != journal.next_state {
            bail!("committed sync state changed; refusing recovery");
        }
        for record in &journal.moves {
            let destination = game.join(&record.destination);
            let snapshot = safety::fingerprint(&destination)?;
            let backup = record.destination.starts_with(".scopenet/staged-sync/rollback/");
            let verified = if backup && journal.cleanup_started {
                match (&snapshot, &record.snapshot) {
                    (None, _) => true,
                    (Some(remaining), Some(expected)) => remaining.iter().all(|entry| expected.contains(entry)),
                    _ => false,
                }
            } else {
                snapshot == record.snapshot
            };
            if !verified {
                bail!("committed sync file changed; retain backup for review");
            }
        }
        if !journal.cleanup_started {
            let mut publication = Publication { root: Some(game.to_path_buf()), journal: Some(journal), ..Publication::default() };
            let temporary = safety::inspect(game, ".scopenet/staged-sync/journal.tmp", false)?;
            if safety::metadata(&temporary)?.is_some() {
                std::fs::remove_file(temporary)?;
                flush_directory(&workspace)?;
            }
            publication.authorize_cleanup()?;
        }
        return cleanup_workspace(&workspace);
    }
    if journal.cleanup_started {
        bail!("cleanup authorization lacks committed state; refusing recovery");
    }
    if current != journal.original_state {
        bail!("original sync state changed; refusing recovery");
    }
    if let Some(expected) = &journal.next_state {
        if safety::fingerprint(&staged_state)?.as_ref() != Some(expected) {
            bail!("staged sync state changed; refusing recovery");
        }
    }
    let mut publication = Publication {
        moves: journal
            .moves
            .iter()
            .map(|record| (game.join(&record.source), game.join(&record.destination), record.snapshot.clone()))
            .collect(),
        directories: journal.directories.iter().map(|relative| game.join(relative)).collect(),
        root: Some(game.to_path_buf()),
        journal: Some(journal),
        #[cfg(test)]
        hook: None,
    };
    let temporary = safety::inspect(game, ".scopenet/staged-sync/journal.tmp", false)?;
    if safety::metadata(&temporary)?.is_some() {
        std::fs::remove_file(temporary)?;
        flush_directory(&workspace)?;
    }
    publication.rollback().context("interrupted sync rollback failed; retain staging for review")?;
    cleanup_workspace(&workspace)
}

fn cleanup_workspace(workspace: &Path) -> Result<()> {
    safety::fingerprint(workspace)?;
    for entry in std::fs::read_dir(workspace)? {
        let entry = entry?;
        if entry.file_name() == "journal.json" {
            continue;
        }
        if entry.file_type()?.is_dir() {
            std::fs::remove_dir_all(entry.path())?;
        } else {
            std::fs::remove_file(entry.path())?;
        }
    }
    flush_directory(workspace)?;
    let journal = workspace.join("journal.json");
    match std::fs::remove_file(journal) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    flush_directory(workspace)?;
    std::fs::remove_dir(workspace)?;
    flush_directory(workspace.parent().unwrap())
}

fn validate_journal(game: &Path, journal: &Journal) -> Result<()> {
    if journal.version != 1 {
        bail!("unsupported sync recovery journal version");
    }
    if journal.cleanup_started && (journal.next_state.is_none() || journal.rollback_complete) {
        bail!("invalid cleanup authorization");
    }
    for record in &journal.moves {
        for relative in [&record.source, &record.destination] {
            if safety::normalize(relative, true)? != *relative {
                bail!("noncanonical recovery path");
            }
            let directory = record.snapshot.as_ref().is_some_and(|snapshot| snapshot.iter().any(|(_, checksum)| checksum.is_none()));
            safety::inspect(game, relative, directory)?;
        }
        let source_staged = record
            .source
            .strip_prefix(".scopenet/staged-sync/downloads/")
            .is_some_and(|index| !index.is_empty() && index.bytes().all(|byte| byte.is_ascii_digit()));
        let destination_backup = record.destination.strip_prefix(".scopenet/staged-sync/rollback/");
        let quarantine = record.destination.strip_prefix(".scopenet/removed/");
        let valid = if source_staged {
            safety::normalize(&record.destination, false).is_ok()
        } else if let Some(relative) = destination_backup {
            relative == record.source && safety::normalize(&record.source, false).is_ok()
        } else if let Some(relative) = quarantine {
            relative.split_once('/').is_some_and(|(epoch, tail)| {
                epoch.parse::<i64>().is_ok_and(|value| value > 0)
                    && tail == record.source
                    && record.source.strip_prefix("mods/").is_some_and(|name| !name.is_empty() && !name.contains('/'))
            })
        } else {
            false
        };
        if !valid || record.snapshot.is_none() {
            bail!("invalid recovery move");
        }
    }
    for relative in &journal.directories {
        if safety::normalize(relative, true)? != *relative || relative == ".scopenet" || relative == ".scopenet/staged-sync" {
            bail!("invalid recovery directory");
        }
        let rollback = relative == ".scopenet/staged-sync/rollback" || relative.starts_with(".scopenet/staged-sync/rollback/");
        let removed = relative == ".scopenet/removed" || relative.starts_with(".scopenet/removed/");
        if relative.starts_with(".scopenet/") && !rollback && !removed {
            bail!("invalid internal recovery directory");
        }
        safety::inspect(game, relative, true)?;
    }
    Ok(())
}

pub(super) fn finish(workspace: &Path, result: Result<super::SyncReport>) -> Result<super::SyncReport> {
    let root = workspace.parent().and_then(Path::parent).context("staging root missing")?;
    match recover(root) {
        Ok(()) => match flush_directory(workspace.parent().context("staging parent missing")?) {
            Ok(()) => result,
            Err(error) => Err(error).context("sync staging cleanup directory flush failed"),
        },
        Err(cleanup) => match result {
            Ok(_) => Err(cleanup).context("sync committed but staging cleanup failed"),
            Err(error) => Err(error).context(format!("staging cleanup failed: {cleanup}")),
        },
    }
}
