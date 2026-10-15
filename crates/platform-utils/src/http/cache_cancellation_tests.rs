use super::{get_json_cached, get_json_cached_with, write_cached_json};
use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

const TEST_TIMEOUT: Duration = Duration::from_secs(5);
const SYNTHETIC_JSON: &[u8] = br#"{"synthetic":true,"revision":7}"#;

async fn bounded<Output>(future: impl Future<Output = Output>) -> Output {
    tokio::time::timeout(TEST_TIMEOUT, future).await.expect("synthetic cache test exceeded its deadline")
}

fn local_client() -> reqwest::Client {
    reqwest::Client::builder().no_proxy().timeout(Duration::from_secs(2)).pool_max_idle_per_host(0).build().unwrap()
}

async fn serve_json_once() -> (String, JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/synthetic.json", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        bounded(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            let mut byte = [0];
            while !request.ends_with(b"\r\n\r\n") {
                assert!(request.len() < 8192, "synthetic request header too large");
                stream.read_exact(&mut byte).await.unwrap();
                request.push(byte[0]);
            }
            assert!(request.starts_with(b"GET /synthetic.json HTTP/1.1\r\n"));
            let headers = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                SYNTHETIC_JSON.len()
            );
            stream.write_all(headers.as_bytes()).await.unwrap();
            stream.write_all(SYNTHETIC_JSON).await.unwrap();
            stream.shutdown().await.unwrap();
        })
        .await;
    });
    (url, server)
}

struct Owner {
    dropped: Arc<AtomicBool>,
}

impl Drop for Owner {
    fn drop(&mut self) {
        self.dropped.store(true, Ordering::SeqCst);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn abort_during_inline_cache_write_keeps_owner_until_write_completes() {
    let directory = tempfile::tempdir().unwrap();
    let cache = directory.path().join("nested/synthetic.json");
    let task_cache = cache.clone();
    let (url, server) = serve_json_once().await;
    let client = local_client();
    let dropped = Arc::new(AtomicBool::new(false));
    let owner = Owner { dropped: Arc::clone(&dropped) };
    let (entered_sender, entered) = oneshot::channel();
    let (release_sender, release) = mpsc::channel();
    let task = tokio::spawn(async move {
        let _owner = owner;
        get_json_cached_with::<serde_json::Value>(&client, &url, &task_cache, move |path, bytes| {
            entered_sender.send(()).unwrap();
            release.recv_timeout(TEST_TIMEOUT).expect("test must release the synchronous cache writer");
            write_cached_json(path, bytes);
        })
        .await
    });

    bounded(entered).await.unwrap();
    task.abort();
    assert!(!task.is_finished());
    assert!(!dropped.load(Ordering::SeqCst));
    assert!(!cache.exists());

    release_sender.send(()).unwrap();
    match bounded(task).await {
        Ok(result) => assert_eq!(result.unwrap(), serde_json::from_slice::<serde_json::Value>(SYNTHETIC_JSON).unwrap()),
        Err(error) => assert!(error.is_cancelled(), "unexpected task failure: {error}"),
    }
    bounded(server).await.unwrap();
    assert_eq!(std::fs::read(&cache).unwrap(), SYNTHETIC_JSON);
    assert!(dropped.load(Ordering::SeqCst));
}

#[tokio::test]
async fn public_cached_json_falls_back_to_exact_online_copy_when_offline() {
    let directory = tempfile::tempdir().unwrap();
    let cache = directory.path().join("nested/synthetic.json");
    let (url, server) = serve_json_once().await;
    let client = local_client();
    let expected = serde_json::from_slice::<serde_json::Value>(SYNTHETIC_JSON).unwrap();

    let online = bounded(get_json_cached::<serde_json::Value>(&client, &url, &cache)).await.unwrap();
    assert_eq!(online, expected);
    assert_eq!(std::fs::read(&cache).unwrap(), SYNTHETIC_JSON);
    bounded(server).await.unwrap();

    let offline = bounded(get_json_cached::<serde_json::Value>(&client, &url, &cache)).await.unwrap();
    assert_eq!(offline, expected);
    assert_eq!(std::fs::read(&cache).unwrap(), SYNTHETIC_JSON);
}

#[tokio::test]
async fn public_cached_json_returns_online_value_when_cache_parent_is_a_file() {
    let directory = tempfile::tempdir().unwrap();
    let blocked_parent = directory.path().join("synthetic-parent-file");
    std::fs::write(&blocked_parent, b"synthetic sentinel").unwrap();
    let cache = blocked_parent.join("synthetic.json");
    let (url, server) = serve_json_once().await;
    let client = local_client();

    let online = bounded(get_json_cached::<serde_json::Value>(&client, &url, &cache)).await.unwrap();
    bounded(server).await.unwrap();
    assert_eq!(online, serde_json::from_slice::<serde_json::Value>(SYNTHETIC_JSON).unwrap());
    assert_eq!(std::fs::read(&blocked_parent).unwrap(), b"synthetic sentinel");
    assert!(!cache.exists());
}
