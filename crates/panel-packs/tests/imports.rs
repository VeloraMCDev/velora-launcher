use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    response::IntoResponse,
    routing::any,
    Router,
};
use serde_json::json;
use sqlx::SqlitePool;
use std::{
    collections::HashMap,
    io::{Cursor, Write},
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
};
use velora_panel_packs::*;
struct Host {
    http: reqwest::Client,
    dir: PathBuf,
    pool: SqlitePool,
    base: String,
    keys: AtomicUsize,
}
impl PackHost for Host {
    fn http(&self) -> &reqwest::Client {
        &self.http
    }
    fn files_dir(&self) -> PathBuf {
        self.dir.clone()
    }
    fn write_pool(&self) -> &SqlitePool {
        &self.pool
    }
    fn platform_pool(&self) -> &SqlitePool {
        &self.pool
    }
    async fn curseforge_key(&self) -> AppResult<String> {
        self.keys.fetch_add(1, Ordering::SeqCst);
        Ok("synthetic-provider-key".into())
    }
    fn now(&self) -> String {
        "synthetic-time".into()
    }
    fn modrinth_api(&self) -> &str {
        &self.base
    }
    fn curseforge_api(&self) -> &str {
        &self.base
    }
}
async fn host(dir: PathBuf) -> Host {
    Host {
        http: reqwest::Client::new(),
        dir,
        pool: SqlitePool::connect("sqlite::memory:").await.unwrap(),
        base: "http://127.0.0.1:1".into(),
        keys: AtomicUsize::new(0),
    }
}
fn archive(files: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let mut out = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (path, data) in files {
        out.start_file(*path, zip::write::SimpleFileOptions::default()).unwrap();
        out.write_all(data).unwrap();
    }
    out.finish().unwrap().into_inner()
}
type Calls = Arc<Mutex<Vec<(String, String, Vec<u8>)>>>;
async fn serve(responses: HashMap<String, (u16, Vec<u8>)>) -> (String, Calls, tokio::task::JoinHandle<()>) {
    let calls: Calls = Default::default();
    let stored = calls.clone();
    let responses = Arc::new(responses);
    let app = Router::new().fallback(any(move |req: Request<Body>| {
        let calls = stored.clone();
        let responses = responses.clone();
        async move {
            let path = req.uri().path().to_string();
            let key = req.headers().get("x-api-key").and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
            let body = to_bytes(req.into_body(), 1_000_000).await.unwrap();
            calls.lock().unwrap().push((path.clone(), key, body.to_vec()));
            let (status, body) = responses.get(&path).cloned().unwrap_or((404, Vec::new()));
            (StatusCode::from_u16(status).unwrap(), [("content-type", "application/json")], body).into_response()
        }
    }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (base, calls, task)
}
#[tokio::test]
async fn plain_zip_keeps_fallback_paths_junk_rules_and_partial_failure_behavior() {
    let temp = tempfile::tempdir().unwrap();
    let host = host(temp.path().into()).await;
    let bytes = archive(&[
        ("Pack/mods/a.jar", b"mod".to_vec()),
        ("Pack/logs/latest.log", b"private log".to_vec()),
        ("Pack/config/../../escape", b"no".to_vec()),
    ]);
    let (info, kind) = import_zip(
        &host,
        "test",
        bytes.clone(),
        Fallback { mc_version: Some("1.21".into()), loader: Some(Loader::Fabric), loader_version: Some("pinned".into()) },
    )
    .await
    .unwrap();
    assert_eq!(kind, "zip");
    assert_eq!(info.files.len(), 1);
    assert_eq!(info.files[0].path, "mods/a.jar");
    assert_eq!(std::fs::read(temp.path().join("test/mods/a.jar")).unwrap(), b"mod");
    assert!(!temp.path().join("escape").exists());
    assert_eq!(host.keys.load(Ordering::SeqCst), 0);
    assert_eq!(info.loader, Loader::Fabric);
    assert_eq!(info.loader_version.as_deref(), Some("pinned"));
    let err = import_zip(&host, "missing", bytes, Fallback::default()).await.unwrap_err();
    assert_eq!(err.status, 400);
    assert!(err.message.starts_with("this zip isn't a Modrinth"));
    assert!(temp.path().join("missing/mods/a.jar").exists());
}
#[tokio::test]
async fn mrpack_preserves_client_filtering_safe_paths_and_override_precedence() {
    let temp = tempfile::tempdir().unwrap();
    let host = host(temp.path().into()).await;
    let index = json!({"name":"Synthetic pack","versionId":"one","dependencies":{"minecraft":"1.20.1","forge":"47.2.0"},"files":[{"path":"mods/shared.jar","hashes":{"sha1":"cdn"},"downloads":["https://example.invalid/mod"],"fileSize":7},{"path":"mods/server.jar","hashes":{"sha1":"x"},"downloads":["https://example.invalid/server"],"env":{"client":"unsupported"}},{"path":"../escape.jar","hashes":{"sha1":"x"},"downloads":["https://example.invalid/escape"]}]});
    let bytes = archive(&[
        ("modrinth.index.json", serde_json::to_vec(&index).unwrap()),
        ("overrides/mods/shared.jar", b"old".to_vec()),
        ("client-overrides/mods/shared.jar", b"client".to_vec()),
    ]);
    let (info, kind) = import_zip(&host, "one", bytes, Fallback::default()).await.unwrap();
    assert_eq!(kind, "modrinth");
    assert_eq!(info.loader_version.as_deref(), Some("1.20.1-47.2.0"));
    assert_eq!(info.files.len(), 1);
    assert_eq!(info.files[0].origin, "override");
    assert_eq!(info.files[0].url, "/files/one/mods/shared.jar");
    assert_eq!(std::fs::read(temp.path().join("one/mods/shared.jar")).unwrap(), b"client");
    assert_eq!(host.keys.load(Ordering::SeqCst), 0);
}
#[tokio::test]
async fn curseforge_uses_required_ids_key_and_preserves_missing_download_notes() {
    let temp = tempfile::tempdir().unwrap();
    let mut host = host(temp.path().into()).await;
    let responses = HashMap::from([
        (
            "/mods/files".into(),
            (
                200,
                serde_json::to_vec(
                    &json!({"data":[{"id":22,"modId":11,"fileName":"a.jar","fileLength":8,"hashes":[{"algo":1,"value":"sha"}]}]}),
                )
                .unwrap(),
            ),
        ),
        (
            "/mods".into(),
            (
                200,
                serde_json::to_vec(
                    &json!({"data":[{"id":11,"name":"Synthetic","classId":6,"links":{"websiteUrl":"https://example.invalid/synthetic"}}]}),
                )
                .unwrap(),
            ),
        ),
    ]);
    let (base, calls, task) = serve(responses).await;
    host.base = base;
    let manifest = json!({"name":"Synthetic","version":"one","minecraft":{"version":"1.21","modLoaders":[{"id":"fabric-0.16","primary":true}]},"files":[{"projectID":11,"fileID":22},{"projectID":99,"fileID":88,"required":false}],"overrides":"overrides"});
    let bytes = archive(&[("manifest.json", serde_json::to_vec(&manifest).unwrap()), ("overrides/config/x", b"override".to_vec())]);
    let (info, kind) = import_zip(&host, "one", bytes, Fallback::default()).await.unwrap();
    assert_eq!(kind, "curseforge");
    assert_eq!(info.loader, Loader::Fabric);
    assert_eq!(info.files.len(), 2);
    let missing = info.files.iter().find(|f| f.path == "mods/a.jar").unwrap();
    assert_eq!(missing.url, "");
    assert!(missing.note.as_deref().unwrap().contains("/files/22"));
    assert_eq!(host.keys.load(Ordering::SeqCst), 1);
    let calls = calls.lock().unwrap();
    assert_eq!(calls.len(), 2);
    assert!(calls.iter().all(|c| c.1 == "synthetic-provider-key"));
    assert_eq!(serde_json::from_slice::<serde_json::Value>(&calls[0].2).unwrap(), json!({"fileIds":[22]}));
    task.abort();
}
#[tokio::test]
async fn provider_errors_keep_original_status_and_key_lookup_order() {
    let temp = tempfile::tempdir().unwrap();
    let mut host = host(temp.path().into()).await;
    let (base, _, task) = serve(HashMap::from([("/mods/files".into(), (403, b"{}".to_vec()))])).await;
    host.base = base;
    let manifest = json!({"name":"Synthetic","version":"one","minecraft":{"version":"1.21","modLoaders":[]},"files":[{"projectID":11,"fileID":22}],"overrides":"overrides"});
    let error = import_zip(&host, "one", archive(&[("manifest.json", serde_json::to_vec(&manifest).unwrap())]), Fallback::default())
        .await
        .unwrap_err();
    assert_eq!((error.status, error.message.as_str()), (400, "CurseForge rejected the API key"));
    task.abort();
}
#[tokio::test]
async fn corrupt_archives_and_mrpack_missing_dependencies_keep_errors() {
    let temp = tempfile::tempdir().unwrap();
    let host = host(temp.path().into()).await;
    let err = import_zip(&host, "one", b"invalid".to_vec(), Fallback::default()).await.unwrap_err();
    assert_eq!(err.status, 400);
    assert!(err.message.starts_with("not a valid zip:"));
    let index = json!({"files":[],"dependencies":{}});
    let err = import_zip(&host, "one", archive(&[("modrinth.index.json", serde_json::to_vec(&index).unwrap())]), Fallback::default())
        .await
        .unwrap_err();
    assert_eq!((err.status, err.message.as_str()), (400, "mrpack has no minecraft dependency"));
}
#[tokio::test]
async fn garbage_collection_uses_scoped_metadata_and_leaves_referenced_local_files() {
    let temp = tempfile::tempdir().unwrap();
    let host = host(temp.path().into()).await;
    sqlx::raw_sql("CREATE TABLE instance_files(instance_id TEXT,path TEXT,url TEXT,sha1 TEXT,size INTEGER,origin TEXT,note TEXT); INSERT INTO instance_files VALUES('one','keep','/files/one/keep','sha',1,'upload',NULL);").execute(&host.pool).await.unwrap();
    std::fs::create_dir_all(temp.path().join("one/sub")).unwrap();
    std::fs::create_dir_all(temp.path().join("other")).unwrap();
    for name in ["one/keep", "one/stale", "one/sub/stale", "other/untouched"] {
        std::fs::write(temp.path().join(name), b"data").unwrap();
    }
    gc_files(&host, "one").await.unwrap();
    assert!(temp.path().join("one/keep").exists());
    assert!(!temp.path().join("one/stale").exists());
    assert!(!temp.path().join("one/sub").exists());
    assert!(temp.path().join("other/untouched").exists());
}
#[tokio::test]
async fn modrinth_download_selection_labels_and_checksums_match_original_behavior() {
    let temp = tempfile::tempdir().unwrap();
    let (download, _, download_task) = serve(HashMap::from([("/archive".into(), (200, b"pack-bytes".to_vec()))])).await;
    for valid in [true, false] {
        let mut host = host(temp.path().into()).await;
        let sha = if valid { velora_platform_utils::http::sha1_bytes(b"pack-bytes").to_uppercase() } else { "wrong".into() };
        let version = json!({"id":"v","project_id":"p","name":"Synthetic","version_number":"v1","files":[{"filename":"skip.jar","url":"http://127.0.0.1:1","primary":true},{"filename":"test.mrpack","url":format!("{download}/archive"),"hashes":{"sha1":sha}}]});
        let (base, calls, task) = serve(HashMap::from([
            ("/version/v".into(), (200, serde_json::to_vec(&version).unwrap())),
            ("/project/p".into(), (200, serde_json::to_vec(&json!({"title":"Synthetic","icon_url":"/supplied-icon"})).unwrap())),
        ]))
        .await;
        host.base = base;
        let result = fetch_modrinth(&host, "v").await;
        if valid {
            let (bytes, label, icon) = result.unwrap();
            assert_eq!(bytes, b"pack-bytes");
            assert_eq!(label, "Modrinth · Synthetic v1");
            assert_eq!(icon.as_deref(), Some("/supplied-icon"));
        } else {
            let error = result.unwrap_err();
            assert_eq!((error.status, error.message.as_str()), (400, "downloaded pack failed its checksum"));
        }
        assert_eq!(host.keys.load(Ordering::SeqCst), 0);
        assert_eq!(calls.lock().unwrap().len(), 2);
        task.abort();
    }
    download_task.abort();
}
#[tokio::test]
async fn curseforge_downloads_keep_key_scope_labels_and_missing_download_error() {
    let temp = tempfile::tempdir().unwrap();
    let (download, _, download_task) = serve(HashMap::from([("/archive".into(), (200, b"cf-bytes".to_vec()))])).await;
    for available in [true, false] {
        let mut host = host(temp.path().into()).await;
        let info = json!({"data":{"displayName":"v1","downloadUrl":if available{Some(format!("{download}/archive"))}else{None}}});
        let (base, calls, task) = serve(HashMap::from([
            ("/mods/11/files/22".into(), (200, serde_json::to_vec(&info).unwrap())),
            ("/mods/11".into(), (200, serde_json::to_vec(&json!({"data":{"name":"Synthetic","logo":{"thumbnailUrl":"/icon"}}})).unwrap())),
        ]))
        .await;
        host.base = base;
        let result = fetch_curseforge(&host, 11, 22).await;
        if available {
            let (bytes, label, icon) = result.unwrap();
            assert_eq!(bytes, b"cf-bytes");
            assert_eq!(label, "CurseForge · Synthetic (v1)");
            assert_eq!(icon.as_deref(), Some("/icon"));
        } else {
            let error = result.unwrap_err();
            assert_eq!(error.status, 400);
            assert!(error.message.starts_with("this modpack can't be downloaded through the API"));
        }
        assert_eq!(host.keys.load(Ordering::SeqCst), 1);
        assert!(calls.lock().unwrap().iter().all(|c| c.1 == "synthetic-provider-key"));
        task.abort();
    }
    download_task.abort();
}
