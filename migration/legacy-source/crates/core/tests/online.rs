//! Real-network installs against Mojang / Fabric / Forge / NeoForge.
//! Ignored by default; CI runs them with `cargo test -p scopenet-core --test online -- --ignored`.

use scopenet_core::install::{install, InstallSpec};
use scopenet_core::launch::{self, Auth, GcPreset, LaunchOptions};
use scopenet_core::{progress, Layout};
use scopenet_shared::Loader;

async fn run(mc: &str, loader: Loader) {
    let root =
        std::env::var("SCOPENET_TEST_ROOT").map(std::path::PathBuf::from).unwrap_or_else(|_| std::env::temp_dir().join("scopenet-online"));
    let layout = Layout::new(&root);
    let client = scopenet_core::http::client();
    let game_dir = layout.instance_dir(&format!("{mc}-{}", loader.as_str()));
    let spec = InstallSpec {
        mc_version: mc.into(),
        loader,
        loader_version: None,
        java_override: None,
        game_dir: game_dir.clone(),
        concurrency: 16,
        deep_verify: false,
    };
    let installed = install(&client, &layout, &spec, &progress::noop()).await.unwrap_or_else(|e| panic!("{mc} {loader:?}: {e:#}"));

    for path in installed.classpath(&layout) {
        assert!(path.exists(), "missing classpath entry {}", path.display());
    }
    assert!(installed.java.exists(), "java missing at {}", installed.java.display());

    let opts = LaunchOptions {
        auth: Auth {
            username: "CiBot".into(),
            uuid: scopenet_shared::offline_uuid("CiBot"),
            access_token: "0".into(),
            user_type: "legacy".into(),
        },
        agent_args: vec![],
        game_dir,
        memory_min_mb: 512,
        memory_max_mb: 2048,
        gc: GcPreset::G1,
        extra_jvm_args: vec![],
        resolution: None,
        fullscreen: false,
        join_server: Some(("example.org".into(), 25565)),
        version_label: "ci".into(),
        launcher_name: "scopenet".into(),
        launcher_version: "ci".into(),
    };
    let cmd = launch::build(&installed, &layout, &opts);
    let joined = cmd.args.join(" ");
    assert!(!joined.contains("${"), "unsubstituted placeholder in: {joined}");

    // The downloaded runtime must actually run.
    let out = std::process::Command::new(scopenet_core::java::sibling_exe(&installed.java, false)).arg("-version").output().unwrap();
    assert!(out.status.success());
    println!("{mc} {loader:?}: OK ({} classpath entries, java {})", installed.classpath(&layout).len(), installed.java_major);
}

#[tokio::test]
#[ignore]
async fn vanilla_latest_release() {
    let m = scopenet_core::meta::mojang_manifest(&scopenet_core::http::client()).await.unwrap();
    run(&m.latest.release, Loader::Vanilla).await;
}

#[tokio::test]
#[ignore]
async fn vanilla_legacy_1_8_9() {
    run("1.8.9", Loader::Vanilla).await;
}

#[tokio::test]
#[ignore]
async fn fabric_1_21_1() {
    run("1.21.1", Loader::Fabric).await;
}

#[tokio::test]
#[ignore]
async fn quilt_1_20_1() {
    run("1.20.1", Loader::Quilt).await;
}

#[tokio::test]
#[ignore]
async fn forge_1_20_1() {
    run("1.20.1", Loader::Forge).await;
}

#[tokio::test]
#[ignore]
async fn neoforge_1_21_1() {
    run("1.21.1", Loader::NeoForge).await;
}

#[tokio::test]
#[ignore]
async fn authlib_injector_official_download() {
    let dir = tempfile::tempdir().unwrap();
    let layout = Layout::new(dir.path());
    let jar = scopenet_core::authlib::ensure(&scopenet_core::http::client(), &layout, None).await.unwrap();
    let zip = zip::ZipArchive::new(std::fs::File::open(&jar).unwrap()).unwrap();
    assert!(zip.file_names().any(|n| n == "META-INF/MANIFEST.MF"), "not a jar: {}", jar.display());
    println!("authlib-injector: {}", jar.display());
}
