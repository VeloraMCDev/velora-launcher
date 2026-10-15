use super::*;
use crate::launch::{self, Auth, GcPreset, LaunchOptions};
use crate::progress::Event;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::io::{Cursor, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use zip::write::SimpleFileOptions;

const VERSION: &str = "1.21.1";
const COMPONENT: &str = "synthetic-runtime";
const LIBRARY_COUNT: usize = 128;
const DEADLINE: Duration = Duration::from_secs(30);

fn archive(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in entries {
        writer.start_file(*name, SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored)).unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn artifact(origin: &str, route: &str, bytes: &[u8]) -> Value {
    json!({"url":format!("{origin}{route}"),"sha1":http::sha1_bytes(bytes),"size":bytes.len()})
}

struct Fixture {
    root: tempfile::TempDir,
    layout: Layout,
    client: reqwest::Client,
    origin: String,
    routes: Arc<BTreeMap<String, Vec<u8>>>,
    counts: Arc<Mutex<BTreeMap<String, usize>>>,
    task: Option<tokio::task::JoinHandle<()>>,
    native_name: &'static str,
    java: Option<PathBuf>,
}

impl Fixture {
    async fn new(java: Option<PathBuf>, main_class: Option<&[u8]>) -> Self {
        let root = tempfile::tempdir().unwrap();
        let layout = Layout::new(root.path().join("managed"));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let native_name = if cfg!(windows) {
            "fixture.dll"
        } else if cfg!(target_os = "macos") {
            "fixture.dylib"
        } else {
            "fixture.so"
        };
        let native = archive(&[(native_name, b"synthetic native bytes, never loaded"), ("META-INF/excluded.txt", b"excluded")]);
        let padding = vec![0x5a; 1024 * 1024];
        let client = archive(&[
            ("FixtureMain.class", main_class.unwrap_or(b"synthetic class, never executed")),
            ("synthetic-padding", &padding),
            ("assets/minecraft/textures/item/synthetic.png", b"synthetic texture bytes"),
        ]);
        let library = archive(&[("synthetic-library", &vec![0x31; 16384])]);
        let object = vec![0x17; 16384];
        let object_hash = http::sha1_bytes(&object);
        let object_route = format!("/objects/{}/{object_hash}", &object_hash[..2]);
        let asset_index = serde_json::to_vec(&json!({"objects":{
            "synthetic/first": {"hash":object_hash,"size":object.len()},
            "synthetic/duplicate": {"hash":object_hash,"size":object.len()}
        }}))
        .unwrap();
        let mut libraries: Vec<Value> = (0..LIBRARY_COUNT)
            .map(|index| {
                let route = format!("/libraries/{index}");
                let mut download = artifact(&origin, &route, &library);
                download["path"] = json!(format!("synthetic/library-{index}.jar"));
                json!({"name":format!("synthetic:library-{index}:1"),"downloads":{"artifact":download}})
            })
            .collect();
        let mut native_download = artifact(&origin, "/native", &native);
        native_download["path"] = json!("synthetic/native.jar");
        libraries.push(json!({"name":"synthetic:native:1","natives":{Env::current().os:"synthetic"},
            "extract":{"exclude":["META-INF/"]},"downloads":{"classifiers":{"synthetic":native_download}}}));
        let logging = b"synthetic logging configuration".to_vec();
        let mut logging_file = artifact(&origin, "/logging", &logging);
        logging_file["id"] = json!("synthetic.xml");
        let mut asset_reference = artifact(&origin, "/asset-index", &asset_index);
        asset_reference["id"] = json!("synthetic-assets");
        let version = serde_json::to_vec(&json!({
            "id":VERSION,"mainClass":"FixtureMain","arguments":{"jvm":["-cp","${classpath}"],"game":[]},
            "javaVersion":{"component":COMPONENT,"majorVersion":21},"libraries":libraries,
            "downloads":{"client":artifact(&origin,"/client",&client)},"assetIndex":asset_reference,
            "logging":{"client":{"argument":"-Dvelora.fixture.logging=${path}","file":logging_file}}
        }))
        .unwrap();
        let manifest = serde_json::to_vec(&json!({"latest":{"release":VERSION,"snapshot":VERSION},"versions":[{
            "id":VERSION,"type":"release","url":format!("{origin}/version"),"sha1":http::sha1_bytes(&version)
        }]}))
        .unwrap();
        let runtime = vec![0x4a; 4096];
        let executable = java::java_exe(Path::new(""), false).to_str().unwrap().replace('\\', "/");
        let runtime_manifest = serde_json::to_vec(&json!({"files":{executable:{"type":"file","executable":true,
            "downloads":{"raw":artifact(&origin,"/runtime",&runtime)}}}}))
        .unwrap();
        let runtime_index = serde_json::to_vec(&json!({java::platform_key():{COMPONENT:[{
            "manifest":{"url":format!("{origin}/runtime-manifest"),"sha1":http::sha1_bytes(&runtime_manifest)},
            "version":{"name":"synthetic-21"}
        }]}}))
        .unwrap();
        let mut routes = BTreeMap::from([
            ("/versions".into(), manifest),
            ("/version".into(), version),
            ("/client".into(), client),
            ("/asset-index".into(), asset_index),
            ("/native".into(), native),
            ("/logging".into(), logging),
            (object_route, object),
            ("/runtime-index".into(), runtime_index),
            ("/runtime-manifest".into(), runtime_manifest),
            ("/runtime".into(), runtime),
        ]);
        for index in 0..LIBRARY_COUNT {
            routes.insert(format!("/libraries/{index}"), library.clone());
        }
        let routes = Arc::new(routes);
        let counts = Arc::new(Mutex::new(BTreeMap::<String, usize>::new()));
        let server_routes = Arc::clone(&routes);
        let server_counts = Arc::clone(&counts);
        let task = tokio::spawn(async move {
            loop {
                let (mut socket, _) = listener.accept().await.unwrap();
                tokio::time::timeout(DEADLINE, async {
                    let mut request = Vec::new();
                    let mut byte = [0];
                    while !request.ends_with(b"\r\n\r\n") {
                        assert!(request.len() < 8192);
                        socket.read_exact(&mut byte).await.unwrap();
                        request.push(byte[0]);
                    }
                    let request = String::from_utf8(request).unwrap();
                    let route = request.split_whitespace().nth(1).unwrap();
                    *server_counts.lock().unwrap().entry(route.into()).or_default() += 1;
                    let body = server_routes.get(route).expect("unexpected synthetic route");
                    socket
                        .write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).as_bytes())
                        .await
                        .unwrap();
                    socket.write_all(body).await.unwrap();
                })
                .await
                .unwrap();
            }
        });
        let client = reqwest::Client::builder().no_proxy().redirect(reqwest::redirect::Policy::none()).timeout(DEADLINE).build().unwrap();
        let game = layout.instance_dir("synthetic-profile");
        std::fs::create_dir_all(game.join("saves")).unwrap();
        std::fs::write(game.join("saves/synthetic-save"), b"preserved synthetic save").unwrap();
        Self { root, layout, client, origin, routes, counts, task: Some(task), native_name, java }
    }

    fn spec(&self, deep_verify: bool) -> InstallSpec {
        InstallSpec {
            mc_version: VERSION.into(),
            loader: Loader::Vanilla,
            loader_version: None,
            java_override: self.java.clone(),
            game_dir: self.layout.instance_dir("synthetic-profile"),
            concurrency: 4,
            deep_verify,
        }
    }

    async fn run(&self, deep: bool) -> (Installed, Value) {
        self.run_with(deep, None).await
    }

    async fn run_with(&self, deep: bool, observer: Option<Reporter>) -> (Installed, Value) {
        self.counts.lock().unwrap().clear();
        LIBRARY_CHECKS.with(|count| count.set(0));
        LIBRARY_RESOLUTIONS.with(|count| count.set(0));
        let stages = Arc::new(Mutex::new(Vec::new()));
        let stage_events = Arc::clone(&stages);
        let started = Instant::now();
        let reporter: Reporter = Arc::new(move |event| {
            if let Some(observer) = &observer {
                observer(event.clone());
            }
            if let Event::Stage { stage, .. } = event {
                stage_events.lock().unwrap().push(json!({"stage":stage,"at_ms":started.elapsed().as_secs_f64()*1000.0}));
            }
        });
        let versions = format!("{}/versions", self.origin);
        let java = format!("{}/runtime-index", self.origin);
        let assets = format!("{}/objects", self.origin);
        let sources = Sources { versions: &versions, java: &java, assets: &assets };
        let installed =
            tokio::time::timeout(DEADLINE, install_from(&self.client, &self.layout, &self.spec(deep), &reporter, Some(&sources)))
                .await
                .unwrap()
                .unwrap();
        let elapsed = started.elapsed().as_secs_f64() * 1000.0;
        assert_eq!(LIBRARY_RESOLUTIONS.with(|count| count.get()), 1);
        assert_eq!(LIBRARY_CHECKS.with(|count| count.get()), 2 * (LIBRARY_COUNT + 1));
        self.assert_installed(&installed);
        let counts = self.counts.lock().unwrap().clone();
        assert!(counts.keys().all(|route| self.routes.contains_key(route)));
        let metadata = counts
            .iter()
            .filter(|(route, _)| ["/versions", "/version", "/asset-index", "/runtime-index", "/runtime-manifest"].contains(&route.as_str()))
            .map(|(_, count)| count)
            .sum::<usize>();
        let requests = counts.values().sum::<usize>();
        (
            installed,
            json!({"elapsed_ms":elapsed,"metadata_requests":metadata,"artifact_requests":requests-metadata,"requests":requests,"stages":*stages.lock().unwrap(),
                "library_file_checks":LIBRARY_CHECKS.with(|count|count.get()),"library_resolutions":LIBRARY_RESOLUTIONS.with(|count|count.get())}),
        )
    }

    fn assert_installed(&self, installed: &Installed) {
        assert_eq!(installed.version.id, VERSION);
        assert_eq!(installed.java_major, 21);
        assert_eq!(installed.libraries.len(), LIBRARY_COUNT + 1);
        assert_eq!(installed.classpath(&self.layout).len(), LIBRARY_COUNT + 1);
        assert_eq!(std::fs::read(&installed.client_jar).unwrap(), self.routes["/client"]);
        for index in 0..LIBRARY_COUNT {
            assert_eq!(
                std::fs::read(self.layout.libraries().join(format!("synthetic/library-{index}.jar"))).unwrap(),
                self.routes[&format!("/libraries/{index}")]
            );
        }
        assert_eq!(std::fs::read(installed.natives_dir.join(self.native_name)).unwrap(), b"synthetic native bytes, never loaded");
        assert!(!installed.natives_dir.join("excluded.txt").exists());
        assert_eq!(std::fs::read(self.spec(false).game_dir.join("saves/synthetic-save")).unwrap(), b"preserved synthetic save");
        let object = &self.routes.iter().find(|(route, _)| route.starts_with("/objects/")).unwrap().1;
        assert_eq!(std::fs::read(assets::object_path(&self.layout, &http::sha1_bytes(object))).unwrap(), **object);
        assert_eq!(std::fs::read(self.layout.assets().join("log_configs/synthetic.xml")).unwrap(), self.routes["/logging"]);
        if self.java.is_none() {
            assert_eq!(std::fs::read(java::java_exe(&self.layout.runtimes().join(COMPONENT), false)).unwrap(), self.routes["/runtime"]);
            assert_eq!(
                std::fs::read_to_string(self.layout.runtimes().join(COMPONENT).join(".scopenet-runtime")).unwrap(),
                http::sha1_bytes(&self.routes["/runtime-manifest"])
            );
        }
    }

    async fn flows(&self) -> (Installed, Value) {
        let (_, cold) = self.run(false).await;
        let (_, warm) = self.run(false).await;
        let (_, deep) = self.run(true).await;
        let path = self.layout.libraries().join("synthetic/library-0.jar");
        let mut corrupt = std::fs::read(&path).unwrap();
        corrupt[0] ^= 0xff;
        std::fs::write(path, corrupt).unwrap();
        let (installed, repair) = self.run(true).await;
        assert_eq!(cold["artifact_requests"], if self.java.is_some() { LIBRARY_COUNT + 4 } else { LIBRARY_COUNT + 5 });
        assert_eq!(warm["artifact_requests"], 0);
        assert_eq!(deep["artifact_requests"], 0);
        assert_eq!(repair["artifact_requests"], 1);
        assert_eq!(warm["metadata_requests"], if self.java.is_some() { 1 } else { 3 });
        (installed, json!({"cold":cold,"warm":warm,"deep":deep,"repair":repair}))
    }

    async fn close(mut self) {
        let task = self.task.take().unwrap();
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        assert!(self.root.path().exists());
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if let Some(task) = &self.task {
            task.abort();
        }
    }
}

#[tokio::test]
async fn complete_install_profile_fixture_checks_actual_pipeline_and_preserves_save() {
    let fixture = Fixture::new(None, None).await;
    fixture.flows().await;
    fixture.close().await;
}

#[test]
fn child_loader_still_merges_and_resolves_final_libraries_in_order() {
    let vanilla: VersionJson = serde_json::from_value(json!({"id":"synthetic-vanilla","libraries":[
        {"name":"synthetic:shared:1","url":"https://example.invalid/vanilla/"},
        {"name":"synthetic:base:1","url":"https://example.invalid/vanilla/"}
    ]}))
    .unwrap();
    let child: VersionJson = serde_json::from_value(json!({"id":"synthetic-loader","libraries":[
        {"name":"synthetic:shared:2","url":"https://example.invalid/loader/"},
        {"name":"synthetic:extra:1","url":"https://example.invalid/loader/"}
    ]}))
    .unwrap();
    let environment = Env::current();
    let original_libraries = libraries::resolve(&vanilla.libraries, &environment);
    let expected = version::merge(vanilla.clone(), child.clone());
    let expected_libraries = libraries::resolve(&expected.libraries, &environment);
    LIBRARY_RESOLUTIONS.with(|count| count.set(0));
    let (actual_version, actual_libraries) = resolved_version(vanilla, Some(child), original_libraries, &environment);
    assert_eq!(actual_version.id, expected.id);
    assert_eq!(actual_libraries, expected_libraries);
    assert_eq!(
        actual_libraries.iter().map(|library| library.name.as_str()).collect::<Vec<_>>(),
        vec!["synthetic:shared:2", "synthetic:extra:1", "synthetic:base:1"]
    );
    assert_eq!(LIBRARY_RESOLUTIONS.with(|count| count.get()), 1);
}

#[tokio::test]
async fn retained_second_pass_repairs_library_removed_after_initial_downloads() {
    let fixture = Fixture::new(None, None).await;
    let path = fixture.layout.libraries().join("synthetic/library-0.jar");
    let removed = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let observer_removed = Arc::clone(&removed);
    let observer: Reporter = Arc::new(move |event| {
        if let Event::Progress { stage: Stage::Game, files_done, files_total, .. } = event {
            if files_total > 0 && files_done == files_total && !observer_removed.swap(true, std::sync::atomic::Ordering::SeqCst) {
                std::fs::remove_file(&path).unwrap();
            }
        }
    });
    fixture.run_with(false, Some(observer)).await;
    assert!(removed.load(std::sync::atomic::Ordering::SeqCst));
    assert_eq!(fixture.counts.lock().unwrap()["/libraries/0"], 2);
    fixture.close().await;
}

#[tokio::test]
async fn retained_presence_guard_rejects_library_removed_after_second_pass() {
    let fixture = Fixture::new(None, None).await;
    let path = fixture.layout.libraries().join("synthetic/library-0.jar");
    let removed = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let observer_removed = Arc::clone(&removed);
    let reporter: Reporter = Arc::new(move |event| {
        if let Event::Progress { stage, files_done, files_total, .. } = event {
            let bit = match stage {
                Stage::Game => 1,
                Stage::Loader => 2,
                _ => 0,
            };
            if bit != 0
                && files_total > 0
                && files_done == files_total
                && observer_removed.fetch_or(bit, std::sync::atomic::Ordering::SeqCst) & bit == 0
            {
                std::fs::remove_file(&path).unwrap();
            }
        }
    });
    let versions = format!("{}/versions", fixture.origin);
    let java = format!("{}/runtime-index", fixture.origin);
    let assets = format!("{}/objects", fixture.origin);
    let sources = Sources { versions: &versions, java: &java, assets: &assets };
    let result =
        tokio::time::timeout(DEADLINE, install_from(&fixture.client, &fixture.layout, &fixture.spec(false), &reporter, Some(&sources)))
            .await
            .unwrap();
    assert!(result.unwrap_err().to_string().contains("library synthetic:library-0:1 is missing"));
    assert_eq!(removed.load(std::sync::atomic::Ordering::SeqCst), 3);
    assert_eq!(fixture.counts.lock().unwrap()["/libraries/0"], 2);
    assert_eq!(std::fs::read(fixture.spec(false).game_dir.join("saves/synthetic-save")).unwrap(), b"preserved synthetic save");
    fixture.close().await;
}

async fn compile_main(java: &Path, directory: &Path) -> Vec<u8> {
    let source = directory.join("FixtureMain.java");
    std::fs::write(
        &source,
        b"public final class FixtureMain { public static void main(String[] args) { System.out.println(\"VELORA_SYNTHETIC_JVM_OK\"); } }",
    )
    .unwrap();
    let javac = java.parent().unwrap().join(if cfg!(windows) { "javac.exe" } else { "javac" });
    let output = tokio::time::timeout(
        DEADLINE,
        tokio::process::Command::new(javac).kill_on_drop(true).arg("-d").arg(directory).arg(&source).output(),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(output.status.success(), "synthetic JVM fixture compilation failed");
    std::fs::read(directory.join("FixtureMain.class")).unwrap()
}

async fn jvm_sample(fixture: &Fixture, installed: &Installed) -> Value {
    let detection = Instant::now();
    assert_eq!(tokio::time::timeout(DEADLINE, java::detect_major(fixture.java.as_ref().unwrap())).await.unwrap(), Some(21));
    let detection_ms = detection.elapsed().as_secs_f64() * 1000.0;
    let opts = LaunchOptions {
        auth: Auth {
            username: "SyntheticPlayer".into(),
            uuid: "00000000-0000-0000-0000-000000000001".into(),
            access_token: "synthetic-not-a-token".into(),
            user_type: "legacy".into(),
        },
        agent_args: vec![],
        game_dir: fixture.spec(false).game_dir,
        memory_min_mb: 16,
        memory_max_mb: 64,
        gc: GcPreset::Default,
        extra_jvm_args: vec![],
        resolution: None,
        fullscreen: false,
        join_server: None,
        version_label: "synthetic".into(),
        launcher_name: "synthetic-fixture".into(),
        launcher_version: "0".into(),
    };
    let build = Instant::now();
    let command = launch::build(installed, &fixture.layout, &opts);
    let command_ms = build.elapsed().as_secs_f64() * 1000.0;
    let started = Instant::now();
    let mut child = launch::spawn(&command, installed.java_major, true).unwrap();
    let status = match tokio::time::timeout(DEADLINE, child.wait()).await {
        Ok(status) => status.unwrap(),
        Err(_) => {
            child.kill().await.unwrap();
            child.wait().await.unwrap();
            panic!("synthetic JVM deadline exceeded");
        }
    };
    let spawn_to_exit_ms = started.elapsed().as_secs_f64() * 1000.0;
    let mut stdout = Vec::new();
    child.stdout.take().unwrap().read_to_end(&mut stdout).await.unwrap();
    assert!(status.success());
    assert_eq!(String::from_utf8(stdout).unwrap().trim(), "VELORA_SYNTHETIC_JVM_OK");
    json!({"java_detection_ms":detection_ms,"command_ms":command_ms,"spawn_to_exit_ms":spawn_to_exit_ms,"exit_success":true})
}

#[tokio::test]
#[ignore = "explicit comparable install/Java profile; native harmless JVM requires VELORA_PROFILE_JAVA pointing to JDK 21"]
async fn controlled_complete_install_java_profile() {
    let java = PathBuf::from(std::env::var_os("VELORA_PROFILE_JAVA").expect("explicit JDK 21 fixture path required"));
    let compiler_root = tempfile::tempdir().unwrap();
    let main = compile_main(&java, compiler_root.path()).await;
    let mut automatic = Vec::new();
    let mut native = Vec::new();
    for _ in 0..5 {
        let fixture = Fixture::new(None, None).await;
        let (_, flows) = fixture.flows().await;
        automatic.push(flows);
        fixture.close().await;
        let fixture = Fixture::new(Some(java.clone()), Some(&main)).await;
        let (installed, flows) = fixture.flows().await;
        let extraction = Instant::now();
        libraries::extract_natives(&installed.libraries, &fixture.layout.libraries(), &installed.natives_dir).unwrap();
        let native_extraction_ms = extraction.elapsed().as_secs_f64() * 1000.0;
        let parsing = Instant::now();
        let parsed: VersionJson = serde_json::from_slice(&fixture.routes["/version"]).unwrap();
        let metadata_parse_ms = parsing.elapsed().as_secs_f64() * 1000.0;
        let resolution = Instant::now();
        assert_eq!(libraries::resolve(&parsed.libraries, &Env::current()), installed.libraries);
        let library_resolution_ms = resolution.elapsed().as_secs_f64() * 1000.0;
        let verification = Instant::now();
        assert!(installed.libraries.iter().all(|library| http::file_ok(
            &library.file(&fixture.layout.libraries()),
            library.sha1.as_deref(),
            library.size,
            true
        )));
        let hash_verification_ms = verification.elapsed().as_secs_f64() * 1000.0;
        let textures = Instant::now();
        assert!(crate::textures::ensure(&fixture.layout).unwrap());
        let texture_extraction_ms = textures.elapsed().as_secs_f64() * 1000.0;
        let jvm = jvm_sample(&fixture, &installed).await;
        native.push(json!({"flows":flows,"jvm":jvm,"helpers":{"native_extraction_ms":native_extraction_ms,
            "metadata_parse_ms":metadata_parse_ms,"library_resolution_ms":library_resolution_ms,
            "hash_verification_ms":hash_verification_ms,"texture_extraction_ms":texture_extraction_ms}}));
        fixture.close().await;
    }
    println!(
        "COMPLETE_INSTALL_JAVA_PROFILE {}",
        json!({"schema":1,"samples":5,"library_count":LIBRARY_COUNT,
        "configured_concurrency":4,"server":"serial-loopback","os_cache":"uncontrolled","build":"debug-test-no-debug-info",
        "automatic_runtime":"synthetic executable never executed","native_java_major":21,"game_launch":false,
        "automatic":automatic,"native":native})
    );
}
