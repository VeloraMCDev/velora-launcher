use super::*;
use serde::Serialize;
use serde_json::json;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const COMPONENT: &str = "java-runtime-synthetic";
const PAYLOAD: &[u8] = &[0x5a; 4096];
const MARKER: &str = ".scopenet-runtime";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
struct Counts {
    index: usize,
    manifest: usize,
    artifact: usize,
    unexpected: usize,
}

struct Responses {
    index: (u16, Vec<u8>),
    manifest: (u16, Vec<u8>),
    artifact: (u16, Vec<u8>),
    manifest_advertised_size: Option<usize>,
    manifest_streamed_size: Option<usize>,
    manifest_chunked: bool,
    counts: Counts,
}

struct Fixture {
    root: tempfile::TempDir,
    client: reqwest::Client,
    origin: String,
    digest: String,
    responses: Arc<Mutex<Responses>>,
    task: tokio::task::JoinHandle<()>,
}

impl Fixture {
    async fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let executable = java_exe(Path::new(""), false).to_str().unwrap().replace('\\', "/");
        let manifest = serde_json::to_vec(&json!({
            "files": { executable: {
                "type": "file", "executable": true,
                "downloads": { "raw": {
                    "sha1": http::sha1_bytes(PAYLOAD), "size": PAYLOAD.len(),
                    "url": format!("{origin}/artifact")
                }}
            }}
        }))
        .unwrap();
        let digest = http::sha1_bytes(&manifest);
        let index = serde_json::to_vec(&json!({
            platform_key(): { COMPONENT: [{
                "manifest": { "sha1": digest, "url": format!("{origin}/manifest") },
                "version": { "name": "synthetic" }
            }]}
        }))
        .unwrap();
        let responses = Arc::new(Mutex::new(Responses {
            index: (200, index),
            manifest: (200, manifest),
            artifact: (200, PAYLOAD.to_vec()),
            manifest_advertised_size: None,
            manifest_streamed_size: None,
            manifest_chunked: false,
            counts: Counts::default(),
        }));
        let server_responses = responses.clone();
        let task = tokio::spawn(async move {
            loop {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut request = Vec::new();
                tokio::time::timeout(Duration::from_secs(5), async {
                    while !request.ends_with(b"\r\n\r\n") {
                        assert!(request.len() < 8192);
                        let mut byte = [0];
                        stream.read_exact(&mut byte).await.unwrap();
                        request.push(byte[0]);
                    }
                })
                .await
                .unwrap();
                let request = String::from_utf8(request).unwrap();
                let route = request.split_whitespace().nth(1).unwrap();
                let (status, body, advertised_size, streamed_size, chunked) = {
                    let mut responses = server_responses.lock().unwrap();
                    let (status, body) = match route {
                        "/index" => {
                            responses.counts.index += 1;
                            responses.index.clone()
                        }
                        "/manifest" => {
                            responses.counts.manifest += 1;
                            responses.manifest.clone()
                        }
                        "/artifact" => {
                            responses.counts.artifact += 1;
                            responses.artifact.clone()
                        }
                        _ => {
                            responses.counts.unexpected += 1;
                            (404, Vec::new())
                        }
                    };
                    let is_manifest = route == "/manifest";
                    (
                        status,
                        body,
                        if is_manifest { responses.manifest_advertised_size } else { None },
                        if is_manifest { responses.manifest_streamed_size } else { None },
                        is_manifest && responses.manifest_chunked,
                    )
                };
                let framing = if chunked {
                    "Transfer-Encoding: chunked\r\n".to_owned()
                } else if streamed_size.is_some() {
                    String::new()
                } else {
                    format!("Content-Length: {}\r\n", advertised_size.unwrap_or(body.len()))
                };
                let header = format!("HTTP/1.1 {status} Fixture\r\n{framing}Connection: close\r\n\r\n");
                let sent: std::io::Result<()> = async {
                    stream.write_all(header.as_bytes()).await?;
                    if let Some(mut remaining) = streamed_size {
                        let padding = [b' '; 16 * 1024];
                        while remaining > 0 {
                            let length = remaining.min(padding.len());
                            if chunked {
                                stream.write_all(format!("{length:x}\r\n").as_bytes()).await?;
                            }
                            stream.write_all(&padding[..length]).await?;
                            if chunked {
                                stream.write_all(b"\r\n").await?;
                            }
                            remaining -= length;
                        }
                        if chunked {
                            stream.write_all(b"0\r\n\r\n").await?;
                        }
                    } else {
                        stream.write_all(&body).await?;
                    }
                    stream.shutdown().await
                }
                .await;
                let _ = sent;
            }
        });
        let client = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();
        Self { root, client, origin, digest, responses, task }
    }

    fn dir(&self) -> PathBuf {
        Layout::new(self.root.path()).runtimes().join(COMPONENT)
    }

    async fn ensure(&self) -> Result<PathBuf> {
        tokio::time::timeout(
            Duration::from_secs(15),
            ensure_runtime_from_index(
                &self.client,
                &Layout::new(self.root.path()),
                COMPONENT,
                4,
                &progress::noop(),
                &format!("{}/index", self.origin),
            ),
        )
        .await
        .expect("bounded runtime ensure")
    }

    fn counts(&self) -> Counts {
        self.responses.lock().unwrap().counts
    }

    fn reference_digest(&self, digest: &str) {
        let mut responses = self.responses.lock().unwrap();
        let mut index: serde_json::Value = serde_json::from_slice(&responses.index.1).unwrap();
        index[platform_key()][COMPONENT][0]["manifest"]["sha1"] = json!(digest);
        responses.index.1 = serde_json::to_vec(&index).unwrap();
    }

    async fn assert_manifest_rejected(&self, message: &str, manifest_requests: usize) {
        self.seed(true, true);
        std::fs::write(self.dir().join(MARKER), b"previous marker").unwrap();
        let error = self.ensure().await.unwrap_err();
        assert!(format!("{error:#}").contains(message));
        assert_eq!(std::fs::read(self.dir().join(MARKER)).unwrap(), b"previous marker");
        assert_eq!(std::fs::read(java_exe(&self.dir(), false)).unwrap(), PAYLOAD);
        assert_eq!(std::fs::read(self.dir().join("unmanaged-sentinel")).unwrap(), b"synthetic retained");
        assert!(!self.dir().join("new-directory").exists());
        self.assert_counts(1, manifest_requests, 0);
    }

    fn assert_counts(&self, index: usize, manifest: usize, artifact: usize) {
        assert_eq!(self.counts(), Counts { index, manifest, artifact, unexpected: 0 });
    }

    fn seed(&self, marker: bool, executable: bool) {
        std::fs::create_dir_all(self.dir()).unwrap();
        if marker {
            std::fs::write(self.dir().join(MARKER), &self.digest).unwrap();
        }
        if executable {
            let path = java_exe(&self.dir(), false);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, PAYLOAD).unwrap();
        }
        std::fs::write(self.dir().join("unmanaged-sentinel"), b"synthetic retained").unwrap();
    }

    fn assert_preserved(&self) {
        assert_eq!(std::fs::read(self.dir().join(MARKER)).unwrap(), self.digest.as_bytes());
        assert_eq!(std::fs::read(java_exe(&self.dir(), false)).unwrap(), PAYLOAD);
        assert_eq!(std::fs::read(self.dir().join("unmanaged-sentinel")).unwrap(), b"synthetic retained");
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.task.abort();
    }
}

#[derive(Serialize)]
struct Sample {
    cold_us: u128,
    warm_us: u128,
    cold_requests: Counts,
    warm_requests: Counts,
}

async fn sample() -> Sample {
    let fixture = Fixture::new().await;
    fixture.seed(false, false);
    let started = Instant::now();
    let cold = fixture.ensure().await.unwrap();
    let cold_us = started.elapsed().as_micros();
    assert_eq!(cold, fixture.dir());
    fixture.assert_counts(1, 1, 1);
    fixture.assert_preserved();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(std::fs::metadata(java_exe(&cold, false)).unwrap().permissions().mode() & 0o777, 0o755);
    }
    let cold_requests = fixture.counts();
    let marker = fixture.dir().join(MARKER);
    let marker_time = std::time::SystemTime::UNIX_EPOCH + Duration::from_secs(10_000);
    std::fs::File::options().write(true).open(&marker).unwrap().set_times(std::fs::FileTimes::new().set_modified(marker_time)).unwrap();
    let marker_time = std::fs::metadata(&marker).unwrap().modified().unwrap();
    let started = Instant::now();
    let warm = fixture.ensure().await.unwrap();
    let warm_us = started.elapsed().as_micros();
    assert_eq!(warm, cold);
    fixture.assert_counts(2, 2, 1);
    fixture.assert_preserved();
    assert_eq!(std::fs::metadata(&marker).unwrap().modified().unwrap(), marker_time);
    Sample { cold_us, warm_us, cold_requests, warm_requests: Counts { index: 1, manifest: 1, ..Counts::default() } }
}

#[tokio::test]
async fn cold_warm_contract_preserves_artifacts_and_marker() {
    sample().await;
}

#[tokio::test]
async fn index_failure_fallback_requires_marker_and_executable() {
    for (marker, executable) in [(false, false), (true, false), (false, true), (true, true)] {
        let fixture = Fixture::new().await;
        fixture.seed(marker, executable);
        fixture.responses.lock().unwrap().index = (503, Vec::new());
        let result = fixture.ensure().await;
        if marker && executable {
            assert_eq!(result.unwrap(), fixture.dir());
            fixture.assert_preserved();
        } else {
            assert!(format!("{:#}", result.unwrap_err()).contains("fetching Java runtime index"));
            assert_eq!(fixture.dir().join(MARKER).exists(), marker);
            assert_eq!(java_exe(&fixture.dir(), false).exists(), executable);
            if marker {
                assert_eq!(std::fs::read(fixture.dir().join(MARKER)).unwrap(), fixture.digest.as_bytes());
            }
            if executable {
                assert_eq!(std::fs::read(java_exe(&fixture.dir(), false)).unwrap(), PAYLOAD);
            }
            assert_eq!(std::fs::read(fixture.dir().join("unmanaged-sentinel")).unwrap(), b"synthetic retained");
        }
        fixture.assert_counts(1, 0, 0);
    }
}

#[tokio::test]
async fn malformed_index_also_uses_installed_fallback() {
    let fixture = Fixture::new().await;
    fixture.seed(true, true);
    fixture.responses.lock().unwrap().index = (200, b"invalid json".to_vec());
    assert_eq!(fixture.ensure().await.unwrap(), fixture.dir());
    fixture.assert_preserved();
    fixture.assert_counts(1, 0, 0);
}

#[tokio::test]
async fn missing_component_is_not_index_failure_fallback() {
    let fixture = Fixture::new().await;
    fixture.seed(true, true);
    fixture.responses.lock().unwrap().index = (200, b"{}".to_vec());
    assert!(fixture.ensure().await.unwrap_err().to_string().contains("Mojang has no"));
    fixture.assert_preserved();
    fixture.assert_counts(1, 0, 0);
}

#[tokio::test]
async fn matching_marker_without_executable_reinstalls() {
    let fixture = Fixture::new().await;
    fixture.seed(true, false);
    assert_eq!(fixture.ensure().await.unwrap(), fixture.dir());
    fixture.assert_preserved();
    fixture.assert_counts(1, 1, 1);
}

#[tokio::test]
async fn changed_marker_rechecks_manifest_but_reuses_verified_artifact() {
    let fixture = Fixture::new().await;
    fixture.seed(true, true);
    std::fs::write(fixture.dir().join(MARKER), b"previous marker").unwrap();
    assert_eq!(fixture.ensure().await.unwrap(), fixture.dir());
    fixture.assert_preserved();
    fixture.assert_counts(1, 1, 0);
}

#[tokio::test]
async fn manifest_failure_preserves_previous_marker_and_artifact() {
    let fixture = Fixture::new().await;
    fixture.seed(true, true);
    std::fs::write(fixture.dir().join(MARKER), b"previous marker").unwrap();
    fixture.responses.lock().unwrap().manifest = (503, Vec::new());
    assert!(fixture.ensure().await.is_err());
    assert_eq!(std::fs::read(fixture.dir().join(MARKER)).unwrap(), b"previous marker");
    assert_eq!(std::fs::read(java_exe(&fixture.dir(), false)).unwrap(), PAYLOAD);
    assert_eq!(std::fs::read(fixture.dir().join("unmanaged-sentinel")).unwrap(), b"synthetic retained");
    fixture.assert_counts(1, 1, 0);
}

#[tokio::test]
async fn failed_artifact_verification_preserves_previous_bytes_and_marker() {
    let fixture = Fixture::new().await;
    fixture.seed(true, true);
    std::fs::write(fixture.dir().join(MARKER), b"previous marker").unwrap();
    std::fs::write(java_exe(&fixture.dir(), false), b"previous artifact").unwrap();
    fixture.responses.lock().unwrap().artifact = (200, vec![0x33; PAYLOAD.len()]);
    assert!(format!("{:#}", fixture.ensure().await.unwrap_err()).contains("checksum mismatch"));
    assert_eq!(std::fs::read(fixture.dir().join(MARKER)).unwrap(), b"previous marker");
    assert_eq!(std::fs::read(java_exe(&fixture.dir(), false)).unwrap(), b"previous artifact");
    assert_eq!(std::fs::read(fixture.dir().join("unmanaged-sentinel")).unwrap(), b"synthetic retained");
    fixture.assert_counts(1, 1, 4);
}

#[tokio::test]
async fn matching_marker_warm_path_repairs_corrupted_artifact() {
    let fixture = Fixture::new().await;
    fixture.seed(true, true);
    let changed = vec![0x33; PAYLOAD.len()];
    std::fs::write(java_exe(&fixture.dir(), false), &changed).unwrap();
    assert_eq!(fixture.ensure().await.unwrap(), fixture.dir());
    fixture.assert_preserved();
    fixture.assert_counts(1, 1, 1);
}

#[tokio::test]
async fn manifest_digest_mismatch_precedes_install_mutation_and_decode() {
    for body in [b"{\"files\":{\"new-directory\":{\"type\":\"directory\"}}}".to_vec(), b"not json".to_vec()] {
        let fixture = Fixture::new().await;
        fixture.responses.lock().unwrap().manifest.1 = body;
        fixture.assert_manifest_rejected("manifest SHA-1 mismatch", 1).await;
    }
}

#[tokio::test]
async fn malformed_manifest_reference_is_rejected_before_manifest_request() {
    for digest in [String::new(), "a".repeat(39), "a".repeat(41), "g".repeat(40), format!("{} ", "a".repeat(39)), "é".repeat(20)] {
        let fixture = Fixture::new().await;
        fixture.reference_digest(&digest);
        fixture.assert_manifest_rejected("exactly 40 ASCII hexadecimal", 0).await;
    }
}

#[tokio::test]
async fn uppercase_manifest_digest_verifies_and_preserves_reference_marker_identity() {
    let fixture = Fixture::new().await;
    let digest = fixture.digest.to_ascii_uppercase();
    fixture.reference_digest(&digest);
    fixture.seed(false, false);
    assert_eq!(fixture.ensure().await.unwrap(), fixture.dir());
    assert_eq!(std::fs::read(fixture.dir().join(MARKER)).unwrap(), digest.as_bytes());
    assert_eq!(std::fs::read(java_exe(&fixture.dir(), false)).unwrap(), PAYLOAD);
    fixture.assert_counts(1, 1, 1);
}

#[tokio::test]
async fn digest_verified_invalid_json_is_rejected_before_install_mutation() {
    let fixture = Fixture::new().await;
    let body = b"not json";
    fixture.responses.lock().unwrap().manifest.1 = body.to_vec();
    fixture.reference_digest(&http::sha1_bytes(body));
    fixture.assert_manifest_rejected("decoding verified Java runtime manifest", 1).await;
}

#[tokio::test]
async fn manifest_at_exact_body_limit_is_verified_and_decoded() {
    let fixture = Fixture::new().await;
    fixture.seed(true, true);
    let digest = {
        let mut responses = fixture.responses.lock().unwrap();
        responses.manifest.1.resize(MAX_RUNTIME_MANIFEST_BYTES, b' ');
        http::sha1_bytes(&responses.manifest.1)
    };
    fixture.reference_digest(&digest);
    assert_eq!(fixture.ensure().await.unwrap(), fixture.dir());
    assert_eq!(std::fs::read(fixture.dir().join(MARKER)).unwrap(), digest.as_bytes());
    assert_eq!(std::fs::read(java_exe(&fixture.dir(), false)).unwrap(), PAYLOAD);
    assert_eq!(std::fs::read(fixture.dir().join("unmanaged-sentinel")).unwrap(), b"synthetic retained");
    fixture.assert_counts(1, 1, 0);
}

#[tokio::test]
async fn oversized_manifest_content_length_rejected_before_install_mutation() {
    let fixture = Fixture::new().await;
    fixture.responses.lock().unwrap().manifest_advertised_size = Some(MAX_RUNTIME_MANIFEST_BYTES + 1);
    fixture.assert_manifest_rejected("byte limit (Content-Length)", 1).await;
}

#[tokio::test]
async fn oversized_manifest_stream_rejected_before_install_mutation() {
    for chunked in [false, true] {
        let fixture = Fixture::new().await;
        {
            let mut responses = fixture.responses.lock().unwrap();
            responses.manifest_streamed_size = Some(MAX_RUNTIME_MANIFEST_BYTES + 1);
            responses.manifest_chunked = chunked;
        }
        fixture.assert_manifest_rejected("byte limit (streamed body)", 1).await;
    }
}

#[tokio::test]
#[ignore = "explicit synthetic multi-sample runtime profile; no native or JVM timing"]
async fn controlled_java_runtime_profile() {
    let mut samples = Vec::new();
    for _sample in 0..5 {
        samples.push(sample().await);
    }
    println!(
        "JAVA_RUNTIME_PROFILE {}",
        json!({
            "schema_version": 1, "sample_count": samples.len(),
            "payload_bytes": PAYLOAD.len(), "artifact_count": 1,
            "concurrency": 4, "samples": samples,
            "java_spawned": false, "physical_cold_cache_controlled": false
        })
    );
}
