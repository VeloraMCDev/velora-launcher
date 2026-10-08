//! Engine download orchestration; platform HTTP primitives are shared.
use crate::progress::{Event, Reporter, Stage};
use anyhow::Result;
use futures::StreamExt;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
pub use velora_platform_utils::http::*;

/// Download many files in parallel, reporting aggregate byte progress.
pub async fn download_all(
    client: &reqwest::Client,
    downloads: Vec<Download>,
    concurrency: usize,
    stage: Stage,
    reporter: &Reporter,
) -> Result<()> {
    if downloads.is_empty() {
        return Ok(());
    }
    let total: u64 = downloads.iter().map(|d| d.size).sum();
    let files_total = downloads.len() as u32;
    let done = Arc::new(AtomicU64::new(0));
    let files_done = Arc::new(AtomicU32::new(0));
    let last_emit = Arc::new(std::sync::Mutex::new(Instant::now() - Duration::from_secs(1)));

    let emit = {
        let done = done.clone();
        let files_done = files_done.clone();
        let reporter = reporter.clone();
        move |force: bool| {
            let mut last = last_emit.lock().unwrap();
            if force || last.elapsed() >= Duration::from_millis(100) {
                *last = Instant::now();
                let d = done.load(Ordering::Relaxed);
                reporter(Event::Progress {
                    stage,
                    done: d,
                    total: total.max(d),
                    files_done: files_done.load(Ordering::Relaxed),
                    files_total,
                });
            }
        }
    };
    emit(true);

    let results: Vec<Result<()>> = futures::stream::iter(downloads.into_iter().map(|dl| {
        let done = done.clone();
        let files_done = files_done.clone();
        let emit = &emit;
        async move {
            let on_bytes = |n: u64| {
                done.fetch_add(n, Ordering::Relaxed);
                emit(false);
            };
            let r = download_one(client, &dl, &on_bytes).await;
            files_done.fetch_add(1, Ordering::Relaxed);
            emit(false);
            r
        }
    }))
    .buffer_unordered(concurrency.clamp(1, 64))
    .collect()
    .await;
    emit(true);

    let errors: Vec<_> = results.into_iter().filter_map(|r| r.err()).collect();
    if let Some(first) = errors.into_iter().next() {
        return Err(first);
    }
    Ok(())
}
