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
#[cfg(test)]
pub(crate) mod regression_tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    pub(crate) struct LocalHttp {
        pub(crate) url: String,
        task: tokio::task::JoinHandle<()>,
    }

    impl LocalHttp {
        pub(crate) async fn serve(body: &'static [u8], advertised: usize, stall: bool) -> Self {
            Self::serve_fixture(body, advertised, stall, "200 OK", None).await
        }

        async fn serve_fixture(
            body: &'static [u8],
            advertised: usize,
            stall: bool,
            status: &'static str,
            split: Option<(usize, Arc<tokio::sync::Notify>)>,
        ) -> Self {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let url = format!("http://{}", listener.local_addr().unwrap());
            let task = tokio::spawn(async move {
                loop {
                    let (mut stream, _) = listener.accept().await.unwrap();
                    let mut request = Vec::new();
                    let mut byte = [0];
                    while !request.ends_with(b"\r\n\r\n") {
                        if stream.read(&mut byte).await.unwrap_or(0) == 0 {
                            break;
                        }
                        request.push(byte[0]);
                        assert!(request.len() < 8192);
                    }
                    let header = format!("HTTP/1.1 {status}\r\nContent-Length: {advertised}\r\nConnection: close\r\n\r\n");
                    if stream.write_all(header.as_bytes()).await.is_err() {
                        continue;
                    }
                    if stall {
                        tokio::time::sleep(Duration::from_millis(150)).await;
                    }
                    let remainder = if let Some((prefix_len, signal)) = &split {
                        if stream.write_all(&body[..*prefix_len]).await.is_err() {
                            continue;
                        }
                        signal.notified().await;
                        &body[*prefix_len..]
                    } else {
                        body
                    };
                    if stream.write_all(remainder).await.is_err() {
                        continue;
                    }
                    if stall {
                        std::future::pending::<()>().await;
                    }
                    stream.shutdown().await.ok();
                }
            });
            Self { url, task }
        }
    }

    impl Drop for LocalHttp {
        fn drop(&mut self) {
            self.task.abort();
        }
    }

    pub(crate) fn local_client() -> reqwest::Client {
        reqwest::Client::builder().no_proxy().timeout(Duration::from_secs(2)).build().unwrap()
    }

    #[tokio::test]
    async fn checksum_mismatch_preserves_destination_and_removes_part() {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("managed.jar");
        std::fs::write(&destination, b"previous").unwrap();
        let server = LocalHttp::serve(b"wrong", 5, false).await;
        let download = Download::new(&server.url, destination.clone(), Some(sha1_bytes(b"right")), Some(5));
        let error = download_all(&local_client(), vec![download], 1, Stage::Files, &crate::progress::noop()).await.unwrap_err();
        assert!(format!("{error:#}").contains("checksum mismatch"));
        assert_eq!(std::fs::read(&destination).unwrap(), b"previous");
        assert!(!destination.with_extension("jar.part").exists());
        assert_only_destination_present(&destination);
    }

    #[tokio::test]
    async fn truncated_http_body_preserves_destination() {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("managed.jar");
        std::fs::write(&destination, b"previous").unwrap();
        let server = LocalHttp::serve(b"short", 100, false).await;
        let download = Download::new(&server.url, destination.clone(), None, Some(100));
        let error = download_all(&local_client(), vec![download], 1, Stage::Files, &crate::progress::noop()).await.unwrap_err();
        assert!(format!("{error:#}").contains("reading download body"));
        assert_eq!(std::fs::read(&destination).unwrap(), b"previous");
        assert_only_destination_present(&destination);
    }

    async fn interrupt_download(destination: &std::path::Path) {
        let server = LocalHttp::serve(b"partial", 100, true).await;
        let download = Download::new(&server.url, destination.to_path_buf(), None, Some(100));
        let received = Arc::new(tokio::sync::Notify::new());
        let signal = received.clone();
        let task = tokio::spawn(async move { download_one(&local_client(), &download, &move |_| signal.notify_one()).await });
        tokio::time::timeout(Duration::from_secs(3), received.notified()).await.unwrap();
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
    }

    #[tokio::test]
    async fn interruption_preserves_destination_and_retry_replaces_partial() {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("managed.jar");
        std::fs::write(&destination, b"previous").unwrap();
        interrupt_download(&destination).await;
        assert_eq!(std::fs::read(&destination).unwrap(), b"previous");
        let server = LocalHttp::serve(b"complete", 8, false).await;
        let download = Download::new(&server.url, destination.clone(), Some(sha1_bytes(b"complete")), Some(8));
        download_all(&local_client(), vec![download], 1, Stage::Files, &crate::progress::noop()).await.unwrap();
        assert_eq!(std::fs::read(&destination).unwrap(), b"complete");
        assert!(!destination.with_extension("jar.part").exists());
    }

    #[tokio::test]
    async fn size_mismatch_must_not_publish_even_with_matching_checksum() {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("managed.jar");
        std::fs::write(&destination, b"previous").unwrap();
        let server = LocalHttp::serve(b"valid", 5, false).await;
        let download = Download::new(&server.url, destination.clone(), Some(sha1_bytes(b"valid")), Some(99));
        let result = download_all(&local_client(), vec![download], 1, Stage::Files, &crate::progress::noop()).await;
        assert_eq!(std::fs::read(&destination).unwrap(), b"previous", "invalid size must not replace the old file");
        assert!(result.is_err());
        assert_only_destination_present(&destination);
    }

    fn assert_only_destination_present(destination: &std::path::Path) {
        let entries: Vec<_> = std::fs::read_dir(destination.parent().unwrap()).unwrap().map(|entry| entry.unwrap().path()).collect();
        assert_eq!(entries, vec![destination.to_path_buf()]);
    }

    async fn assert_declared_size_mismatch_preserves_destination(declared_size: u64, with_checksum: bool) {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("managed.jar");
        std::fs::write(&destination, b"previous").unwrap();
        let body = b"complete";
        let server = LocalHttp::serve(body, body.len(), false).await;
        let checksum = with_checksum.then(|| sha1_bytes(body));
        let download = Download::new(&server.url, destination.clone(), checksum, Some(declared_size));
        let result = download_all(&local_client(), vec![download], 1, Stage::Files, &crate::progress::noop()).await;
        assert_eq!(std::fs::read(&destination).unwrap(), b"previous", "invalid size must not replace the old file");
        assert!(result.is_err(), "a complete HTTP response with mismatched declared size must fail");
        assert!(format!("{:#}", result.unwrap_err()).contains("size mismatch"));
        assert_only_destination_present(&destination);
    }

    #[tokio::test]
    async fn body_shorter_than_declared_size_preserves_destination_with_matching_checksum() {
        assert_declared_size_mismatch_preserves_destination(9, true).await;
    }

    #[tokio::test]
    async fn body_longer_than_declared_size_preserves_destination_with_matching_checksum() {
        assert_declared_size_mismatch_preserves_destination(7, true).await;
    }

    #[tokio::test]
    async fn declared_size_mismatch_preserves_destination_without_checksum() {
        assert_declared_size_mismatch_preserves_destination(9, false).await;
        assert_declared_size_mismatch_preserves_destination(7, false).await;
    }

    async fn assert_valid_download_publishes(declared_size: Option<u64>) {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("managed.jar");
        std::fs::write(&destination, b"previous").unwrap();
        let body = b"complete";
        let server = LocalHttp::serve(body, body.len(), false).await;
        let download = Download::new(&server.url, destination.clone(), Some(sha1_bytes(body)), declared_size);
        download_all(&local_client(), vec![download], 1, Stage::Files, &crate::progress::noop()).await.unwrap();
        assert_eq!(std::fs::read(&destination).unwrap(), body);
        assert!(!destination.with_extension("jar.part").exists());
        assert_only_destination_present(&destination);
    }

    #[tokio::test]
    async fn zero_declared_size_remains_unspecified_with_matching_checksum() {
        assert_valid_download_publishes(Some(0)).await;
    }

    #[tokio::test]
    async fn missing_declared_size_remains_unspecified_with_matching_checksum() {
        assert_valid_download_publishes(None).await;
    }

    #[tokio::test]
    async fn matching_checksum_and_declared_size_publish_download() {
        assert_valid_download_publishes(Some(8)).await;
    }

    #[tokio::test]
    async fn interruption_must_remove_partial_artifact() {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("managed.jar");
        interrupt_download(&destination).await;
        assert!(!destination.exists());
        assert!(!destination.with_extension("jar.part").exists());
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
    }

    #[tokio::test]
    async fn fresh_download_rejects_non_full_response_statuses_before_writing() {
        for status in ["206 Partial Content", "204 No Content", "205 Reset Content", "201 Created", "404 Not Found"] {
            let directory = tempfile::tempdir().unwrap();
            let destination = directory.path().join("managed.jar");
            std::fs::write(&destination, b"previous").unwrap();
            let server = LocalHttp::serve_fixture(b"", 0, false, status, None).await;
            let download = Download::new(&server.url, destination.clone(), Some(sha1_bytes(b"")), None);
            let written = AtomicU64::new(0);
            let error = download_one(&local_client(), &download, &|count| {
                written.fetch_add(count, Ordering::Relaxed);
            })
            .await
            .unwrap_err();
            assert!(format!("{error:#}").contains(&format!("returned {status}")));
            assert_eq!(written.load(Ordering::Relaxed), 0);
            assert_eq!(std::fs::read(&destination).unwrap(), b"previous");
            assert_only_destination_present(&destination);
        }
    }

    fn owned_parts(directory: &std::path::Path) -> Vec<std::path::PathBuf> {
        std::fs::read_dir(directory)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.file_name().unwrap().to_string_lossy().starts_with(".velora-download-"))
            .collect()
    }

    #[tokio::test]
    async fn oversized_chunk_is_rejected_before_write_even_with_matching_digest() {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("managed.jar");
        std::fs::write(&destination, b"previous").unwrap();
        let signal = Arc::new(tokio::sync::Notify::new());
        let server = LocalHttp::serve_fixture(b"complete", 8, false, "200 OK", Some((4, signal.clone()))).await;
        let download = Download::new(&server.url, destination.clone(), Some(sha1_bytes(b"complete")), Some(7));
        let written = AtomicU64::new(0);
        let error = download_one(&local_client(), &download, &|count| {
            assert!(count > 0 && count <= 7);
            let parts = owned_parts(directory.path());
            assert_eq!(parts.len(), 1);
            assert!(std::fs::metadata(&parts[0]).unwrap().len() <= 7);
            written.fetch_add(count, Ordering::Relaxed);
            signal.notify_one();
        })
        .await
        .unwrap_err();
        assert!(format!("{error:#}").contains("size mismatch"));
        assert!((1..=28).contains(&written.load(Ordering::Relaxed)));
        assert_eq!(std::fs::read(&destination).unwrap(), b"previous");
        assert_only_destination_present(&destination);
    }

    #[tokio::test]
    async fn concurrent_attempts_do_not_share_or_truncate_temps() {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("managed.jar");
        std::fs::write(&destination, b"previous").unwrap();
        let legacy_part = destination.with_extension("jar.part");
        std::fs::write(&legacy_part, b"not owned").unwrap();
        let stalled_server = LocalHttp::serve(b"partial", 100, true).await;
        let stalled_download = Download::new(&stalled_server.url, destination.clone(), None, Some(100));
        let received = Arc::new(tokio::sync::Notify::new());
        let signal = received.clone();
        let task = tokio::spawn(async move { download_one(&local_client(), &stalled_download, &move |_| signal.notify_one()).await });
        tokio::time::timeout(Duration::from_secs(3), received.notified()).await.unwrap();
        let first_part = owned_parts(directory.path()).pop().unwrap();
        let server = LocalHttp::serve(b"complete", 8, false).await;
        let download = Download::new(&server.url, destination.clone(), Some(sha1_bytes(b"complete")), Some(8));
        download_one(&local_client(), &download, &|_| {}).await.unwrap();
        assert_eq!(std::fs::read(&destination).unwrap(), b"complete");
        assert_eq!(std::fs::read(&first_part).unwrap(), b"partial");
        assert_eq!(std::fs::read(&legacy_part).unwrap(), b"not owned");
        assert_eq!(owned_parts(directory.path()), vec![first_part]);
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
    }

    #[tokio::test]
    async fn publication_failure_removes_owned_temp_and_preserves_destination() {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("managed.jar");
        std::fs::create_dir(&destination).unwrap();
        std::fs::write(destination.join("sentinel"), b"previous").unwrap();
        let server = LocalHttp::serve(b"complete", 8, false).await;
        let download = Download::new(&server.url, destination.clone(), Some(sha1_bytes(b"complete")), Some(8));
        let error = download_one(&local_client(), &download, &|_| {}).await.unwrap_err();
        assert!(format!("{error:#}").contains("publishing verified download"));
        assert_eq!(std::fs::read(destination.join("sentinel")).unwrap(), b"previous");
        assert_only_destination_present(&destination);
    }

    #[tokio::test]
    async fn cleanup_failure_reports_cleanup_and_original_verification_error() {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("managed.jar");
        std::fs::write(&destination, b"previous").unwrap();
        let server = LocalHttp::serve(b"wrong", 5, false).await;
        let download = Download::new(&server.url, destination.clone(), Some(sha1_bytes(b"right")), Some(5));
        let error = download_one(&local_client(), &download, &|_| {
            for part in owned_parts(directory.path()).into_iter().filter(|path| path.is_file()) {
                std::fs::remove_file(&part).unwrap();
                std::fs::create_dir(&part).unwrap();
            }
        })
        .await
        .unwrap_err();
        let message = format!("{error:#}");
        assert!(message.contains("removing owned partial download"));
        assert!(message.contains("checksum mismatch"));
        assert_eq!(std::fs::read(&destination).unwrap(), b"previous");
        assert_eq!(owned_parts(directory.path()).len(), 4);
    }

    #[tokio::test]
    async fn unspecified_size_is_bounded_before_temporary_creation() {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("managed.jar");
        std::fs::write(&destination, b"previous").unwrap();
        let server = LocalHttp::serve(b"complete", 8, false).await;
        let download = Download::new(&server.url, destination.clone(), None, None);
        let error = download_one_with_limit(&local_client(), &download, 7, &|_| panic!("advertised oversized body must fail early")).await.unwrap_err();
        assert!(format!("{error:#}").contains("byte limit"));
        assert_eq!(std::fs::read(&destination).unwrap(), b"previous");
        assert_only_destination_present(&destination);
    }

    #[tokio::test]
    async fn chunked_unspecified_size_is_bounded_even_without_digest_or_length_header() {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("managed.jar");
        std::fs::write(&destination, b"previous").unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            for _attempt in 0..4 {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut request = Vec::new();
                let mut byte = [0];
                while !request.ends_with(b"\r\n\r\n") { stream.read_exact(&mut byte).await.unwrap(); request.push(byte[0]); }
                stream.write_all(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n8\r\ncomplete\r\n0\r\n\r\n").await.unwrap();
            }
        });
        let download = Download::new(&url, destination.clone(), None, None);
        let written = AtomicU64::new(0);
        let error = download_one_with_limit(&local_client(), &download, 7, &|count| { written.fetch_add(count, Ordering::Relaxed); }).await.unwrap_err();
        assert!(format!("{error:#}").contains("byte limit"));
        assert!(written.load(Ordering::Relaxed) <= 28);
        assert_eq!(std::fs::read(&destination).unwrap(), b"previous");
        assert_only_destination_present(&destination);
        server.await.unwrap();
    }

    #[tokio::test]
    async fn explicit_limit_rejects_declared_oversize_without_network() {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("managed.jar");
        let download = Download::new("http://127.0.0.1:1", destination, None, Some(8));
        let error = download_one_with_limit(&local_client(), &download, 7, &|_| {}).await.unwrap_err();
        assert!(format!("{error:#}").contains("declared download size"));
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
    }
}
