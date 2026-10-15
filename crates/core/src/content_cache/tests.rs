use super::*;
use std::cell::Cell;
use std::io::Cursor;
use std::sync::Arc;
use std::time::{Duration, Instant};

const NAMESPACE: Namespace = Namespace { platform: 1, artifact_type: 2, trust_domain: 3 };

fn fixture(limits: Limits) -> (tempfile::TempDir, ContentCache) {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap().join("cache");
    let cache = ContentCache::create(root, limits).unwrap();
    (directory, cache)
}

fn limits() -> Limits {
    Limits { max_object_bytes: 1024 * 1024, max_disk_bytes: 4 * 1024 * 1024, max_entries: 8 }
}

fn identity(body: &[u8], algorithm: Algorithm) -> Key {
    let digest = match algorithm {
        Algorithm::Sha256 => {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("synthetic");
            fs::write(&path, body).unwrap();
            velora_platform_utils::authlib::sha256_file(&path).unwrap()
        }
        Algorithm::Sha1Compatibility => velora_platform_utils::http::sha1_bytes(body),
    };
    Key::verified_identity(algorithm, Some(&digest), body.len() as u64, NAMESPACE).unwrap()
}

fn names(cache: &ContentCache) -> Vec<String> {
    fs::read_dir(&cache.root)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .filter(|name| name != OWNER_NAME)
        .collect()
}

#[test]
fn sha256_known_vector_and_warm_reverification() {
    let (_directory, cache) = fixture(limits());
    let key =
        Key::verified_identity(Algorithm::Sha256, Some("BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD"), 3, NAMESPACE)
            .unwrap();
    assert_eq!(cache.get(Some(&key), || false).unwrap(), None);
    assert_eq!(cache.populate(Some(&key), &mut Cursor::new(b"abc"), || false).unwrap(), Population::Published);
    assert_eq!(cache.get(Some(&key), || false).unwrap().unwrap(), b"abc");
    assert_eq!(cache.populate(Some(&key), &mut Cursor::new(b"not consumed"), || false).unwrap(), Population::Existing);
    fs::write(cache.root.join(key.filename()), b"bad").unwrap();
    assert_eq!(cache.get(Some(&key), || false), Err(Error::Integrity));
    assert_eq!(cache.populate(Some(&key), &mut Cursor::new(b"abc"), || false), Err(Error::Integrity));
    assert_eq!(fs::read(cache.root.join(key.filename())).unwrap(), b"bad");
}

#[test]
fn sha1_compatibility_and_exact_empty_size() {
    for body in [b"legacy".as_slice(), b"".as_slice()] {
        for algorithm in [Algorithm::Sha256, Algorithm::Sha1Compatibility] {
            let (_directory, cache) = fixture(limits());
            let key = identity(body, algorithm);
            cache.populate(Some(&key), &mut Cursor::new(body), || false).unwrap();
            assert_eq!(cache.get(Some(&key), || false).unwrap().unwrap(), body);
        }
    }
}

#[test]
fn unknown_and_malformed_digest_decline_without_reading() {
    let (_directory, cache) = fixture(limits());
    let mut reader = FailingReader;
    for digest in [None, Some(""), Some("../secret"), Some("https://synthetic.invalid/?token=secret")] {
        assert!(Key::verified_identity(Algorithm::Sha256, digest, 3, NAMESPACE).is_none());
    }
    assert!(Key::verified_identity(Algorithm::Sha1Compatibility, Some(&"g".repeat(40)), 3, NAMESPACE).is_none());
    assert_eq!(cache.populate(None, &mut reader, || false).unwrap(), Population::Declined);
    assert_eq!(cache.get(None, || false).unwrap(), None);
    assert!(names(&cache).is_empty());
}

#[test]
fn mismatch_truncation_and_excess_never_publish() {
    for body in [b"abc".as_slice(), b"ab".as_slice(), b"abcd".as_slice()] {
        let (_directory, cache) = fixture(limits());
        let key = identity(b"xyz", Algorithm::Sha256);
        assert_eq!(cache.populate(Some(&key), &mut Cursor::new(body), || false), Err(Error::Integrity));
        assert!(names(&cache).is_empty());
    }
    let (_directory, cache) = fixture(limits());
    let key = identity(b"", Algorithm::Sha256);
    assert_eq!(cache.populate(Some(&key), &mut Cursor::new(b"a"), || false), Err(Error::Integrity));
    assert!(names(&cache).is_empty());
}

struct FailingReader;

impl Read for FailingReader {
    fn read(&mut self, _buffer: &mut [u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("https://synthetic.invalid/?token=secret"))
    }
}

#[test]
fn errors_do_not_leak_reader_details_or_urls() {
    let (_directory, cache) = fixture(limits());
    let key = identity(b"abc", Algorithm::Sha256);
    let error = cache.populate(Some(&key), &mut FailingReader, || false).unwrap_err();
    assert_eq!(error, Error::Io);
    assert_eq!(error.to_string(), "content cache: Io");
    assert!(!format!("{key:?}").contains("secret"));
    assert!(names(&cache).is_empty());
}

#[test]
fn bounds_include_staging_and_entry_limit() {
    let key = identity(b"abc", Algorithm::Sha256);
    let (_directory, cache) = fixture(Limits { max_object_bytes: 2, max_disk_bytes: 10, max_entries: 1 });
    assert_eq!(cache.populate(Some(&key), &mut FailingReader, || false), Err(Error::Capacity));
    let (_directory, cache) = fixture(Limits { max_object_bytes: 3, max_disk_bytes: 5, max_entries: 1 });
    cache.populate(Some(&key), &mut Cursor::new(b"abc"), || false).unwrap();
    assert_eq!(cache.get(Some(&key), || false), Err(Error::Capacity));
    let (_directory, cache) = fixture(Limits { max_object_bytes: 3, max_disk_bytes: 6, max_entries: 1 });
    cache.populate(Some(&key), &mut Cursor::new(b"abc"), || false).unwrap();
    assert_eq!(cache.get(Some(&key), || false).unwrap().unwrap(), b"abc");
    let other = identity(b"def", Algorithm::Sha256);
    assert_eq!(cache.populate(Some(&other), &mut FailingReader, || false), Err(Error::Capacity));
}

#[test]
fn invalid_limits_and_root_reuse_are_rejected() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap().join("cache");
    assert!(matches!(ContentCache::create(root.clone(), Limits { max_object_bytes: 0, ..limits() }), Err(Error::InvalidLimits)));
    assert!(matches!(ContentCache::create(root.clone(), Limits { max_entries: 0, ..limits() }), Err(Error::InvalidLimits)));
    assert!(matches!(ContentCache::create(root.clone(), Limits { max_disk_bytes: 1, ..limits() }), Err(Error::InvalidLimits)));
    assert!(matches!(ContentCache::create(PathBuf::from("relative"), limits()), Err(Error::UnsafePath)));
    let _cache = ContentCache::create(root.clone(), limits()).unwrap();
    assert!(matches!(ContentCache::create(root, limits()), Err(Error::Collision)));
}

#[test]
fn interruption_cleans_only_request_scratch_then_full_retry() {
    let (_directory, cache) = fixture(limits());
    let retained = identity(b"retained", Algorithm::Sha256);
    cache.populate(Some(&retained), &mut Cursor::new(b"retained"), || false).unwrap();
    let body = vec![37; 200_000];
    let key = identity(&body, Algorithm::Sha256);
    let calls = Cell::new(0);
    let cancelled = || {
        calls.set(calls.get() + 1);
        calls.get() >= 3
    };
    assert_eq!(cache.populate(Some(&key), &mut Cursor::new(&body), cancelled), Err(Error::Cancelled));
    assert_eq!(names(&cache), vec![retained.filename()]);
    assert_eq!(cache.get(Some(&key), || false).unwrap(), None);
    cache.populate(Some(&key), &mut Cursor::new(&body), || false).unwrap();
    assert_eq!(cache.get(Some(&retained), || false).unwrap().unwrap(), b"retained");
    assert_eq!(cache.get(Some(&key), || false).unwrap().unwrap(), body);
}

#[test]
fn retrieval_cancellation_and_panic_cleanup() {
    let (_directory, cache) = fixture(limits());
    let body = vec![12; 200_000];
    let key = identity(&body, Algorithm::Sha256);
    cache.populate(Some(&key), &mut Cursor::new(&body), || false).unwrap();
    let calls = Cell::new(0);
    assert_eq!(
        cache.get(Some(&key), || {
            calls.set(calls.get() + 1);
            calls.get() >= 3
        }),
        Err(Error::Cancelled)
    );
    assert_eq!(names(&cache), vec![key.filename()]);
    let other = identity(b"other", Algorithm::Sha256);
    let calls = Cell::new(0);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        cache.populate(Some(&other), &mut Cursor::new(b"other"), || {
            calls.set(calls.get() + 1);
            assert!(calls.get() < 3, "synthetic panic");
            false
        })
    }));
    assert!(result.is_err());
    assert_eq!(names(&cache), vec![key.filename()]);
    assert_eq!(cache.get(Some(&key), || false), Err(Error::Poisoned));
}

#[test]
fn namespaces_and_instances_do_not_share_entries() {
    let (_first_directory, first) = fixture(limits());
    let (_second_directory, second) = fixture(limits());
    let key = identity(b"abc", Algorithm::Sha256);
    first.populate(Some(&key), &mut Cursor::new(b"abc"), || false).unwrap();
    assert_eq!(second.get(Some(&key), || false).unwrap(), None);
    for namespace in
        [Namespace { platform: 9, ..NAMESPACE }, Namespace { artifact_type: 9, ..NAMESPACE }, Namespace { trust_domain: 9, ..NAMESPACE }]
    {
        let other = Key { namespace, ..key.clone() };
        assert_eq!(first.get(Some(&other), || false).unwrap(), None);
        first.populate(Some(&other), &mut Cursor::new(b"abc"), || false).unwrap();
    }
    assert_eq!(names(&first).len(), 4);
    assert!(names(&second).is_empty());
}

#[test]
fn concurrent_requests_publish_once() {
    let (_directory, cache) = fixture(limits());
    let cache = Arc::new(cache);
    let key = identity(b"abc", Algorithm::Sha256);
    let workers: Vec<_> = (0..4)
        .map(|_| {
            let cache = cache.clone();
            let key = key.clone();
            std::thread::spawn(move || cache.populate(Some(&key), &mut Cursor::new(b"abc"), || false).unwrap())
        })
        .collect();
    let outcomes: Vec<_> = workers.into_iter().map(|worker| worker.join().unwrap()).collect();
    assert_eq!(outcomes.iter().filter(|outcome| **outcome == Population::Published).count(), 1);
    assert_eq!(outcomes.iter().filter(|outcome| **outcome == Population::Existing).count(), 3);
    assert_eq!(names(&cache), vec![key.filename()]);
}

#[test]
fn nonregular_unknown_missing_and_size_corruption_fail_closed() {
    for mutation in 0..4 {
        let (_directory, cache) = fixture(limits());
        let key = identity(b"abc", Algorithm::Sha256);
        cache.populate(Some(&key), &mut Cursor::new(b"abc"), || false).unwrap();
        let path = cache.root.join(key.filename());
        match mutation {
            0 => {
                fs::remove_file(&path).unwrap();
                fs::create_dir(&path).unwrap();
            }
            1 => {
                fs::write(cache.root.join("unknown"), b"preserve").unwrap();
            }
            2 => {
                fs::remove_file(&path).unwrap();
            }
            _ => {
                fs::write(&path, b"truncated").unwrap();
            }
        }
        assert!(cache.get(Some(&key), || false).is_err());
        assert!(cache.populate(Some(&key), &mut Cursor::new(b"abc"), || false).is_err());
    }
}

#[test]
fn publication_collision_preserves_existing_bytes() {
    let (_directory, cache) = fixture(limits());
    let key = identity(b"abc", Algorithm::Sha256);
    let calls = Cell::new(0);
    let result = cache.populate(Some(&key), &mut Cursor::new(b"abc"), || {
        calls.set(calls.get() + 1);
        if calls.get() == 4 {
            fs::write(cache.root.join(key.filename()), b"preserve").unwrap();
        }
        false
    });
    assert_eq!(result, Err(Error::Collision));
    assert_eq!(fs::read(cache.root.join(key.filename())).unwrap(), b"preserve");
    assert_eq!(names(&cache), vec![key.filename()]);
}

#[test]
fn final_verification_cancellation_does_not_publish() {
    let (_directory, cache) = fixture(limits());
    let key = identity(b"abc", Algorithm::Sha256);
    let calls = Cell::new(0);
    assert_eq!(
        cache.populate(Some(&key), &mut Cursor::new(b"abc"), || {
            calls.set(calls.get() + 1);
            calls.get() == 4
        }),
        Err(Error::Cancelled)
    );
    assert!(names(&cache).is_empty());
    assert_eq!(cache.populate(Some(&key), &mut Cursor::new(b"abc"), || true), Err(Error::Cancelled));
    assert!(names(&cache).is_empty());
}

#[test]
fn aggregate_payload_bound_declines_without_consuming_input() {
    let (_directory, cache) = fixture(Limits { max_object_bytes: 3, max_disk_bytes: 5, max_entries: 8 });
    let key = identity(b"abc", Algorithm::Sha256);
    cache.populate(Some(&key), &mut Cursor::new(b"abc"), || false).unwrap();
    let other = identity(b"def", Algorithm::Sha256);
    assert_eq!(cache.populate(Some(&other), &mut FailingReader, || false), Err(Error::Capacity));
    assert_eq!(names(&cache), vec![key.filename()]);
}

#[test]
fn retrieval_accounts_committed_payload_once_and_scratch_once() {
    for budget in [8, 9] {
        let (_directory, cache) = fixture(Limits { max_object_bytes: 3, max_disk_bytes: budget, max_entries: 8 });
        let first = identity(b"abc", Algorithm::Sha256);
        let second = identity(b"def", Algorithm::Sha256);
        cache.populate(Some(&first), &mut Cursor::new(b"abc"), || false).unwrap();
        cache.populate(Some(&second), &mut Cursor::new(b"def"), || false).unwrap();
        assert_eq!(cache.state.lock().unwrap().bytes, 6);
        let result = cache.get(Some(&first), || false);
        if budget == 8 {
            assert_eq!(result, Err(Error::Capacity));
        } else {
            assert_eq!(result.unwrap().unwrap(), b"abc");
        }
        assert_eq!(cache.state.lock().unwrap().bytes, 6);
        assert_eq!(names(&cache).len(), 2);
    }
}

#[test]
fn scratch_cleanup_closes_handle_before_unlink_and_on_unlink_failure() {
    let (_directory, cache) = fixture(limits());
    let mut state = cache.state.lock().unwrap();
    let mut scratch = cache.scratch(&mut state).unwrap();
    assert!(scratch.file.is_some());
    scratch.cleanup().unwrap();
    assert!(scratch.file.is_none());
    assert!(!scratch.path.exists());
    let cleaned_path = scratch.path.clone();
    fs::write(&cleaned_path, b"replacement sentinel").unwrap();
    scratch.cleanup().unwrap();
    drop(scratch);
    assert_eq!(fs::read(&cleaned_path).unwrap(), b"replacement sentinel");
    fs::remove_file(cleaned_path).unwrap();
    let mut failed = cache.scratch(&mut state).unwrap();
    drop(failed.file.take());
    fs::remove_file(&failed.path).unwrap();
    fs::create_dir(&failed.path).unwrap();
    let sentinel = cache.root.join("open-sentinel");
    fs::write(&sentinel, b"preserve").unwrap();
    failed.file = Some(File::open(&sentinel).unwrap());
    let path = failed.path.clone();
    assert_eq!(failed.cleanup(), Err(Error::Io));
    assert!(failed.file.is_none());
    drop(failed);
    assert!(path.is_dir());
    assert_eq!(fs::read(&sentinel).unwrap(), b"preserve");
    fs::remove_file(sentinel).unwrap();
    fs::remove_dir(path).unwrap();
}

#[cfg(unix)]
#[test]
fn sockets_and_replaced_root_are_rejected() {
    use std::os::unix::fs::symlink;
    use std::os::unix::net::UnixListener;
    let (directory, cache) = fixture(limits());
    let key = identity(b"abc", Algorithm::Sha256);
    let _listener = UnixListener::bind(cache.root.join("socket")).unwrap();
    assert!(cache.populate(Some(&key), &mut Cursor::new(b"abc"), || false).is_err());
    fs::remove_file(cache.root.join("socket")).unwrap();
    fs::rename(&cache.root, cache.root.with_file_name("displaced-cache")).unwrap();
    let outside = directory.path().canonicalize().unwrap().join("outside");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("sentinel"), b"preserve").unwrap();
    symlink(&outside, &cache.root).unwrap();
    assert_eq!(cache.populate(Some(&key), &mut Cursor::new(b"abc"), || false), Err(Error::UnsafePath));
    assert_eq!(fs::read(outside.join("sentinel")).unwrap(), b"preserve");
}

#[cfg(unix)]
mod recovery_tests;

#[cfg(unix)]
#[test]
fn symlink_parents_entries_and_hardlink_aliases_are_rejected() {
    use std::os::unix::fs::symlink;
    let directory = tempfile::tempdir().unwrap();
    let parent = directory.path().canonicalize().unwrap();
    fs::create_dir(parent.join("actual")).unwrap();
    symlink(parent.join("actual"), parent.join("alias")).unwrap();
    assert!(matches!(ContentCache::create(parent.join("alias/cache"), limits()), Err(Error::UnsafePath)));
    let cache = ContentCache::create(parent.join("cache"), limits()).unwrap();
    let key = identity(b"abc", Algorithm::Sha256);
    cache.populate(Some(&key), &mut Cursor::new(b"abc"), || false).unwrap();
    let outside = parent.join("outside");
    fs::write(&outside, b"sentinel").unwrap();
    fs::remove_file(cache.root.join(key.filename())).unwrap();
    symlink(&outside, cache.root.join(key.filename())).unwrap();
    assert_eq!(cache.get(Some(&key), || false), Err(Error::UnsafePath));
    assert_eq!(fs::read(&outside).unwrap(), b"sentinel");
    fs::remove_file(cache.root.join(key.filename())).unwrap();
    fs::hard_link(&outside, cache.root.join(key.filename())).unwrap();
    assert_eq!(cache.get(Some(&key), || false), Err(Error::UnsafePath));
    assert_eq!(fs::read(&outside).unwrap(), b"sentinel");
}

struct ControlledOrigin {
    bytes: Cursor<Vec<u8>>,
    calls: usize,
    fail_after: Option<u64>,
}

impl Read for ControlledOrigin {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if self.fail_after.is_some_and(|limit| self.bytes.position() >= limit) {
            return Err(std::io::Error::new(std::io::ErrorKind::ConnectionReset, "synthetic interruption"));
        }
        let requested = buffer.len().min(65536);
        std::thread::sleep(Duration::from_micros(250));
        self.calls += 1;
        self.bytes.read(&mut buffer[..requested])
    }
}

#[test]
#[ignore = "explicit local benchmark; not an install benchmark"]
fn controlled_local_benchmark() {
    let body: Vec<u8> = (0..1024 * 1024).map(|index| (index % 251) as u8).collect();
    let key = identity(&body, Algorithm::Sha256);
    for iteration in 1..=5 {
        let (_directory, cache) = fixture(limits());
        let mut origin = ControlledOrigin { bytes: Cursor::new(body.clone()), calls: 0, fail_after: None };
        let started = Instant::now();
        assert_eq!(cache.populate(Some(&key), &mut origin, || false).unwrap(), Population::Published);
        let cold = started.elapsed();
        let started = Instant::now();
        assert_eq!(cache.get(Some(&key), || false).unwrap().unwrap(), body);
        let warm = started.elapsed();
        let (_interrupted_directory, interrupted) = fixture(limits());
        let mut broken = ControlledOrigin { bytes: Cursor::new(body.clone()), calls: 0, fail_after: Some(262144) };
        let started = Instant::now();
        assert_eq!(interrupted.populate(Some(&key), &mut broken, || false), Err(Error::Io));
        let interruption = started.elapsed();
        assert!(names(&interrupted).is_empty());
        let mut retry = ControlledOrigin { bytes: Cursor::new(body.clone()), calls: 0, fail_after: None };
        let started = Instant::now();
        interrupted.populate(Some(&key), &mut retry, || false).unwrap();
        let recovery = started.elapsed();
        assert_eq!(interrupted.get(Some(&key), || false).unwrap().unwrap(), body);
        println!(
            "local iteration={iteration} bytes={} cold_us={} warm_us={} interruption_us={} full_retry_us={} cold_reads={} interrupted_bytes={} retry_bytes={} retained_bytes={} scratch_after_interrupt=0",
            body.len(), cold.as_micros(), warm.as_micros(), interruption.as_micros(), recovery.as_micros(), origin.calls,
            broken.bytes.position(), retry.bytes.position(), fs::metadata(cache.root.join(key.filename())).unwrap().len()
        );
    }
}
