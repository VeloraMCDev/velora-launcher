use super::*;
use std::process::{Child, Command, Stdio};

#[test]
fn reopen_preserves_both_algorithms_namespaces_and_exact_empty() {
    let (directory, cache) = fixture(limits());
    let root = cache.root.clone();
    let keys: Vec<_> = [Algorithm::Sha256, Algorithm::Sha1Compatibility]
        .into_iter()
        .flat_map(|algorithm| [b"abc".as_slice(), b"".as_slice()].into_iter().map(move |body| (identity(body, algorithm), body)))
        .collect();
    for (key, body) in &keys {
        cache.populate(Some(key), &mut Cursor::new(body), || false).unwrap();
    }
    let owner_inode = fs::metadata(root.join(OWNER_NAME)).unwrap();
    assert!(matches!(ContentCache::reopen(root.clone(), limits()), Err(Error::Collision)));
    drop(cache);
    for _iteration in 0..3 {
        let reopened = ContentCache::reopen(root.clone(), limits()).unwrap();
        assert!(super::super::recovery::same_file(&owner_inode, &fs::metadata(root.join(OWNER_NAME)).unwrap()));
        for (key, body) in &keys {
            assert_eq!(reopened.get(Some(key), || false).unwrap().unwrap(), *body);
        }
        assert_eq!(reopened.state.lock().unwrap().bytes, 6);
        assert_eq!(reopened.state.lock().unwrap().entries.len(), 4);
    }
    drop(directory);
}

#[test]
fn reopening_verifies_final_corruption_before_removing_scratch() {
    for replacement in [b"bad".as_slice(), b"too long".as_slice()] {
        let (_directory, cache) = fixture(limits());
        let key = identity(b"abc", Algorithm::Sha256);
        cache.populate(Some(&key), &mut Cursor::new(b"abc"), || false).unwrap();
        let root = cache.root.clone();
        drop(cache);
        fs::write(root.join(key.filename()), replacement).unwrap();
        fs::write(root.join("staging-9"), b"partial").unwrap();
        assert!(matches!(ContentCache::reopen(root.clone(), limits()), Err(Error::Integrity)));
        assert_eq!(fs::read(root.join("staging-9")).unwrap(), b"partial");
        assert_eq!(fs::read(root.join(key.filename())).unwrap(), replacement);
    }
}

#[test]
fn recovery_rejects_unbounded_ambiguous_and_noncanonical_inventory() {
    for filename in ["unknown", "staging-0", "staging-01", "staging-18446744073709551616"] {
        let (_directory, cache) = fixture(limits());
        let root = cache.root.clone();
        drop(cache);
        fs::write(root.join(filename), b"preserve").unwrap();
        assert!(ContentCache::reopen(root.clone(), limits()).is_err());
        assert_eq!(fs::read(root.join(filename)).unwrap(), b"preserve");
    }
    let (_directory, cache) = fixture(limits());
    let root = cache.root.clone();
    drop(cache);
    fs::write(root.join("staging-1"), b"first").unwrap();
    fs::write(root.join("staging-2"), b"second").unwrap();
    assert!(matches!(ContentCache::reopen(root.clone(), limits()), Err(Error::Integrity)));
    assert_eq!(fs::read(root.join("staging-1")).unwrap(), b"first");
    assert_eq!(fs::read(root.join("staging-2")).unwrap(), b"second");
    let key = identity(b"abc", Algorithm::Sha256);
    let (prefix, size) = key.filename().rsplit_once('-').map(|(prefix, size)| (prefix.to_owned(), size.to_owned())).unwrap();
    for filename in [format!("0{}", key.filename()), key.filename().to_uppercase(), format!("{prefix}-0{size}")] {
        assert!(Key::from_filename(&filename).is_err());
    }
    assert_eq!(Key::from_filename(&key.filename()).unwrap(), key);
}

#[test]
fn reopening_rebuilds_capacity_and_rejects_oversized_scratch() {
    let (_directory, cache) = fixture(limits());
    let first = identity(b"abc", Algorithm::Sha256);
    let second = identity(b"def", Algorithm::Sha256);
    for (key, body) in [(&first, b"abc"), (&second, b"def")] {
        cache.populate(Some(key), &mut Cursor::new(body), || false).unwrap();
    }
    let root = cache.root.clone();
    drop(cache);
    assert!(matches!(ContentCache::reopen(root.clone(), Limits { max_entries: 1, ..limits() }), Err(Error::Capacity)));
    assert!(matches!(
        ContentCache::reopen(root.clone(), Limits { max_object_bytes: 3, max_disk_bytes: 5, ..limits() }),
        Err(Error::Capacity)
    ));
    fs::write(root.join("staging-1"), b"xxxx").unwrap();
    assert!(matches!(ContentCache::reopen(root.clone(), Limits { max_object_bytes: 3, ..limits() }), Err(Error::Capacity)));
    assert_eq!(fs::read(root.join("staging-1")).unwrap(), b"xxxx");
    fs::remove_file(root.join("staging-1")).unwrap();
    let reopened = ContentCache::reopen(root, Limits { max_object_bytes: 3, max_disk_bytes: 8, ..limits() }).unwrap();
    assert_eq!(reopened.get(Some(&first), || false), Err(Error::Capacity));
    assert_eq!(reopened.state.lock().unwrap().bytes, 6);
}

#[test]
fn hostile_owner_root_and_scratch_substitutions_fail_closed() {
    use std::os::unix::fs::symlink;
    for hostile_name in [OWNER_NAME.to_owned(), "staging-1".to_owned(), identity(b"abc", Algorithm::Sha256).filename()] {
        for hardlink in [false, true] {
            let (directory, cache) = fixture(limits());
            let root = cache.root.clone();
            drop(cache);
            let outside = directory.path().join("outside");
            let bytes = if hostile_name == OWNER_NAME { b"velora-content-cache-v1\n".as_slice() } else { b"abc".as_slice() };
            fs::write(&outside, bytes).unwrap();
            let hostile = root.join(&hostile_name);
            if hostile_name == OWNER_NAME {
                fs::remove_file(&hostile).unwrap();
            }
            if hardlink {
                fs::hard_link(&outside, &hostile).unwrap();
            } else {
                symlink(&outside, &hostile).unwrap();
            }
            assert!(matches!(ContentCache::reopen(root, limits()), Err(Error::UnsafePath)));
            assert_eq!(fs::read(outside).unwrap(), bytes);
        }
    }
    let (_directory, cache) = fixture(limits());
    let key = identity(b"abc", Algorithm::Sha256);
    fs::rename(&cache.root, cache.root.with_file_name("old-cache")).unwrap();
    let replacement = ContentCache::create(cache.root.clone(), limits()).unwrap();
    assert_eq!(cache.populate(Some(&key), &mut Cursor::new(b"abc"), || false), Err(Error::UnsafePath));
    assert!(names(&replacement).is_empty());
}

#[test]
fn malformed_or_missing_ownership_marker_is_not_adopted() {
    for marker in [None, Some(b"bad".as_slice()), Some(b"xxxxxxxxxxxxxxxxxxxxxx\n".as_slice())] {
        let (_directory, cache) = fixture(limits());
        let root = cache.root.clone();
        drop(cache);
        fs::remove_file(root.join(OWNER_NAME)).unwrap();
        if let Some(marker) = marker {
            fs::write(root.join(OWNER_NAME), marker).unwrap();
        }
        assert!(ContentCache::reopen(root, limits()).is_err());
    }
}

#[test]
fn canonical_size_is_parsed_but_reopen_rejects_mismatched_bytes() {
    let (_directory, cache) = fixture(limits());
    let key = identity(b"abc", Algorithm::Sha256);
    let filename = format!("{}0", key.filename());
    assert_eq!(Key::from_filename(&filename).unwrap().size, 30);
    let root = cache.root.clone();
    drop(cache);
    fs::write(root.join(&filename), b"abc").unwrap();
    assert!(matches!(ContentCache::reopen(root.clone(), limits()), Err(Error::Integrity)));
    assert_eq!(fs::read(root.join(filename)).unwrap(), b"abc");
}

#[test]
fn sha1_compatibility_corruption_is_rejected_before_scratch_cleanup() {
    let (_directory, cache) = fixture(limits());
    let key = identity(b"abc", Algorithm::Sha1Compatibility);
    cache.populate(Some(&key), &mut Cursor::new(b"abc"), || false).unwrap();
    let root = cache.root.clone();
    drop(cache);
    fs::write(root.join(key.filename()), b"bad").unwrap();
    fs::write(root.join("staging-1"), b"preserved scratch").unwrap();
    assert!(matches!(ContentCache::reopen(root.clone(), limits()), Err(Error::Integrity)));
    assert_eq!(fs::read(root.join("staging-1")).unwrap(), b"preserved scratch");
    assert_eq!(fs::read(root.join(key.filename())).unwrap(), b"bad");
}

struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn start_worker(root: &Path, mode: &str) -> ChildGuard {
    let ready = root.with_file_name("worker-ready");
    let _ = fs::remove_file(&ready);
    let child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "content_cache::tests::recovery_tests::process_worker", "--test-threads=1"])
        .env("VELORA_CACHE_TEST_ROOT", root)
        .env("VELORA_CACHE_TEST_MODE", mode)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut guard = ChildGuard(child);
    let deadline = Instant::now() + Duration::from_secs(10);
    while !ready.exists() {
        assert!(guard.0.try_wait().unwrap().is_none(), "child exited before fault boundary");
        assert!(Instant::now() < deadline, "child fault boundary timed out");
        std::thread::sleep(Duration::from_millis(10));
    }
    guard
}

fn park_worker(root: &Path) -> ! {
    fs::write(root.with_file_name("worker-ready"), b"ready").unwrap();
    loop {
        std::thread::park_timeout(Duration::from_secs(60));
    }
}

struct InterruptedReader {
    root: PathBuf,
    first: bool,
}

impl Read for InterruptedReader {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if self.first {
            self.first = false;
            buffer[0] = b'a';
            Ok(1)
        } else {
            park_worker(&self.root)
        }
    }
}

#[test]
fn process_worker() {
    let Some(root) = std::env::var_os("VELORA_CACHE_TEST_ROOT") else { return };
    let root = PathBuf::from(root);
    let mode = std::env::var("VELORA_CACHE_TEST_MODE").unwrap();
    let cache = ContentCache::reopen(root.clone(), limits()).unwrap();
    let key = identity(b"abc", Algorithm::Sha256);
    match mode.as_str() {
        "partial" => {
            cache.populate(Some(&key), &mut InterruptedReader { root, first: true }, || false).unwrap();
        }
        "verified" | "linked" | "linked-unsynced" => {
            let mut state = cache.state.lock().unwrap();
            let mut scratch = cache.scratch(&mut state).unwrap();
            cache.stage(&mut scratch, &mut Cursor::new(b"abc"), &key, &|| false).unwrap();
            if mode.starts_with("linked") {
                fs::hard_link(&scratch.path, cache.root.join(key.filename())).unwrap();
                if mode == "linked" {
                    cache.sync_directory().unwrap();
                }
            }
            park_worker(&root);
        }
        "owned" => park_worker(&root),
        "published" => {
            cache.populate(Some(&key), &mut Cursor::new(b"abc"), || false).unwrap();
            park_worker(&root);
        }
        _ => panic!("unexpected synthetic mode"),
    }
}

#[test]
fn killed_process_releases_ownership_and_recovers_interrupted_boundaries() {
    for mode in ["owned", "partial", "verified", "linked-unsynced", "linked", "published"] {
        let (_directory, cache) = fixture(limits());
        let retained = identity(b"retained", Algorithm::Sha256);
        cache.populate(Some(&retained), &mut Cursor::new(b"retained"), || false).unwrap();
        let root = cache.root.clone();
        drop(cache);
        let mut worker = start_worker(&root, mode);
        assert!(matches!(ContentCache::reopen(root.clone(), limits()), Err(Error::Collision)));
        worker.0.kill().unwrap();
        assert!(!worker.0.wait().unwrap().success());
        let reopened = ContentCache::reopen(root.clone(), limits()).unwrap();
        let key = identity(b"abc", Algorithm::Sha256);
        assert_eq!(reopened.get(Some(&retained), || false).unwrap().unwrap(), b"retained");
        if mode.starts_with("linked") || mode == "published" {
            assert_eq!(reopened.get(Some(&key), || false).unwrap().unwrap(), b"abc");
        } else {
            assert_eq!(reopened.get(Some(&key), || false).unwrap(), None);
            assert_eq!(reopened.populate(Some(&key), &mut Cursor::new(b"abc"), || false).unwrap(), Population::Published);
        }
        assert!(!names(&reopened).iter().any(|name| name.starts_with("staging-")));
        drop(reopened);
        ContentCache::reopen(root, limits()).unwrap();
    }
}
