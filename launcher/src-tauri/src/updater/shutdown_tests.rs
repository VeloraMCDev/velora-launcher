use super::*;
use crate::shutdown::Shutdown;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::oneshot;

struct Directory(PathBuf);

impl Directory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("velora-installer-shutdown-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[tokio::test]
async fn cancelled_download_closes_and_removes_owned_partial_before_shutdown_drains() {
    let directory = Directory::new();
    let destination = directory.0.join("setup.exe");
    let partial = destination.with_extension("exe.part");
    let body = b"synthetic installer body";
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let (release, released) = oneshot::channel();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut headers = Vec::new();
        let mut byte = [0];
        while !headers.ends_with(b"\r\n\r\n") {
            assert!(headers.len() < 16384);
            socket.read_exact(&mut byte).await.unwrap();
            headers.push(byte[0]);
        }
        socket
            .write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).as_bytes())
            .await
            .unwrap();
        socket.write_all(&body[..5]).await.unwrap();
        let _ = tokio::time::timeout(Duration::from_secs(5), released).await;
    });
    let info = UpdateInfo {
        version: "99.0.0".into(),
        notes: String::new(),
        url,
        size: body.len() as u64,
        sha256: Some(format!("{:x}", Sha256::digest(body))),
        signature: None,
    };
    let shutdown = Arc::new(Shutdown::default());
    let work = shutdown.enter().unwrap();
    let task_destination = destination.clone();
    let task = tokio::spawn(async move {
        let _work = work;
        let client = reqwest::Client::builder().no_proxy().timeout(Duration::from_secs(5)).build().unwrap();
        download_installer(&client, &info, &task_destination).await
    });
    tokio::time::timeout(Duration::from_secs(5), async {
        while std::fs::metadata(&partial).map(|metadata| metadata.len()).unwrap_or(0) != 5 {
            assert!(!task.is_finished());
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(shutdown.begin());
    assert!(!shutdown.mark_ready());
    task.abort();
    assert!(tokio::time::timeout(Duration::from_secs(5), task).await.unwrap().unwrap_err().is_cancelled());
    assert!(!partial.exists());
    assert!(!destination.exists());
    shutdown.wait_idle().await;
    assert!(shutdown.mark_ready());
    release.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(5), server).await.unwrap().unwrap();
}

#[test]
fn exclusive_partial_creation_preserves_preexisting_bytes() {
    let directory = Directory::new();
    let partial = directory.0.join("setup.exe.part");
    std::fs::write(&partial, b"synthetic prior owner").unwrap();
    assert!(InstallerPart::create(partial.clone()).is_err());
    assert_eq!(std::fs::read(&partial).unwrap(), b"synthetic prior owner");
    assert_eq!(std::fs::read_dir(&directory.0).unwrap().count(), 1);
}

#[test]
fn failed_publication_removes_only_owned_partial_and_preserves_destination() {
    let directory = Directory::new();
    let destination = directory.0.join("setup.exe");
    let partial = destination.with_extension("exe.part");
    std::fs::create_dir(&destination).unwrap();
    let sentinel = destination.join("synthetic-sentinel");
    std::fs::write(&sentinel, b"preserved").unwrap();
    let mut part = InstallerPart::create(partial.clone()).unwrap();
    part.file.as_mut().unwrap().write_all(b"synthetic installer").unwrap();
    assert!(part.publish(&destination).is_err());
    assert!(!partial.exists());
    assert_eq!(std::fs::read(sentinel).unwrap(), b"preserved");
}
