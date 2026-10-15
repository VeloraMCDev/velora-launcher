use super::tests::{entry, write_fixture};
use super::*;
use crate::http::regression_tests::{local_client, LocalHttp};
use std::time::Instant;

const PAYLOAD: &[u8] = &[0x5a; 16 * 1024];

#[derive(Debug, Serialize)]
struct Sample {
    cold_us: u128,
    warm_us: u128,
    deep_us: u128,
    repair_us: u128,
    metadata_checks_us: u128,
    snapshot_fingerprints_us: u128,
    cold_downloads: usize,
    warm_downloads: usize,
    deep_downloads: usize,
    repair_downloads: usize,
}

fn assert_preserved(game: &Path, count: usize) {
    assert_eq!(load_state(game).revision, 9);
    assert_eq!(load_state(game).managed.len(), count);
    assert_eq!(std::fs::read(game.join("saves/synthetic-world/player.txt")).unwrap(), b"synthetic player file");
    assert!(!game.join(".scopenet/staged-sync").exists());
}

async fn sample(count: usize) -> Sample {
    assert!(count > 0);
    let directory = tempfile::tempdir().unwrap();
    let game = directory.path();
    write_fixture(game, "saves/synthetic-world/player.txt", b"synthetic player file");
    let server = LocalHttp::serve(PAYLOAD, PAYLOAD.len(), false).await;
    let client = local_client();
    let files: Vec<_> = (0..count).map(|index| entry(&format!("mods/synthetic-{index:04}.jar"), &server.url, PAYLOAD)).collect();
    let reporter = progress::noop();

    let started = Instant::now();
    let cold = sync(&client, &server.url, game, 9, 0, &files, 4, false, &reporter).await.unwrap();
    let cold_us = started.elapsed().as_micros();
    assert_eq!(cold.downloaded, count);
    assert_eq!((cold.removed, cold.cleaned), (0, 0));
    assert_preserved(game, count);

    let started = Instant::now();
    let warm = sync(&client, &server.url, game, 9, 0, &files, 4, false, &reporter).await.unwrap();
    let warm_us = started.elapsed().as_micros();
    assert_eq!((warm.downloaded, warm.removed, warm.cleaned), (0, 0, 0));
    assert_preserved(game, count);

    let started = Instant::now();
    let deep = sync(&client, &server.url, game, 9, 0, &files, 4, true, &reporter).await.unwrap();
    let deep_us = started.elapsed().as_micros();
    assert_eq!((deep.downloaded, deep.removed, deep.cleaned), (0, 0, 0));
    assert_preserved(game, count);

    write_fixture(game, &files[0].path, &[0x33; PAYLOAD.len()]);
    let started = Instant::now();
    let repair = sync(&client, &server.url, game, 9, 0, &files, 4, true, &reporter).await.unwrap();
    let repair_us = started.elapsed().as_micros();
    assert_eq!((repair.downloaded, repair.removed, repair.cleaned), (1, 0, 0));
    assert_eq!(std::fs::read(game.join(&files[0].path)).unwrap(), PAYLOAD);
    assert_preserved(game, count);

    let started = Instant::now();
    for file in &files {
        assert!(http::file_ok(&game.join(&file.path), Some(&file.sha1), Some(file.size), false));
    }
    let metadata_checks_us = started.elapsed().as_micros();

    let started = Instant::now();
    for file in &files {
        assert!(safety::fingerprint(&game.join(&file.path)).unwrap().is_some());
    }
    let snapshot_fingerprints_us = started.elapsed().as_micros();

    for file in &files {
        assert!(http::file_ok(&game.join(&file.path), Some(&file.sha1), Some(file.size), true));
    }

    Sample {
        cold_us,
        warm_us,
        deep_us,
        repair_us,
        metadata_checks_us,
        snapshot_fingerprints_us,
        cold_downloads: cold.downloaded,
        warm_downloads: warm.downloaded,
        deep_downloads: deep.downloaded,
        repair_downloads: repair.downloaded,
    }
}

#[tokio::test]
async fn profile_fixture_checks_real_sync_and_preserves_unmanaged_content() {
    let measured = sample(3).await;
    assert_eq!(measured.cold_downloads, 3);
    assert_eq!(measured.warm_downloads, 0);
    assert_eq!(measured.deep_downloads, 0);
    assert_eq!(measured.repair_downloads, 1);
}

#[tokio::test]
#[ignore = "explicit synthetic sync profile, not native preparation acceptance"]
async fn controlled_preparation_profile() {
    let mut samples = Vec::new();
    for _iteration in 0..5 {
        samples.push(sample(64).await);
    }
    println!(
        "SYNC_PROFILE {}",
        serde_json::json!({
            "schema_version": 1,
            "files": 64,
            "bytes_per_file": PAYLOAD.len(),
            "payload_bytes": 64 * PAYLOAD.len(),
            "concurrency": 4,
            "samples": samples,
            "scope": "fresh temporary instance; loopback HTTP; OS cache uncontrolled; debug test build"
        })
    );
}
