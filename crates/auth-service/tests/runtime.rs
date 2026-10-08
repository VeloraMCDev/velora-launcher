use base64::Engine;
use reqwest::{Client, StatusCode};
use rsa::{pkcs8::DecodePublicKey, signature::Verifier};
use serde_json::{json, Value};
use std::{path::Path, time::Duration};
use velora_auth_http::config::RuntimeConfig;
use velora_auth_service::Runtime;

fn configuration(directory: &Path) -> RuntimeConfig {
    RuntimeConfig::from_lookup(|name| match name {
        "VELORA_AUTH_PUBLIC_ORIGIN" => Some("https://example.invalid/platform".into()),
        "VELORA_AUTH_DATA_DIR" => Some(directory.display().to_string()),
        "VELORA_AUTH_BOOTSTRAP_USERNAME" => Some("BootAdmin".into()),
        "VELORA_AUTH_BOOTSTRAP_PASSWORD_FILE" => Some(directory.join("bootstrap-password").display().to_string()),
        _ => None,
    })
    .unwrap()
}
async fn start(runtime: Runtime) -> (String, tokio::sync::oneshot::Sender<()>, tokio::task::JoinHandle<anyhow::Result<()>>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(runtime.serve(listener, async {
        let _ = stopped.await;
    }));
    (base, stop, server)
}
async fn stop(sender: tokio::sync::oneshot::Sender<()>, server: tokio::task::JoinHandle<anyhow::Result<()>>) {
    sender.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(5), server).await.unwrap().unwrap().unwrap();
}
#[tokio::test]
async fn real_listener_bootstrap_game_auth_signatures_proxy_policy_restart_and_shutdown() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("bootstrap-password"), "synthetic-admin-password").unwrap();
    let config = configuration(directory.path());
    let runtime = Runtime::prepare(config.clone(), "Synthetic Velora".into()).await.unwrap();
    let pool = runtime.pool().clone();
    let original = velora_auth_core::identity_store::find_by_name(&pool, "BootAdmin").await.unwrap().unwrap();
    assert_eq!(original.role, "admin");
    assert!(velora_auth_core::password::verify_password("synthetic-admin-password", &original.password_hash));
    assert!(Runtime::prepare(config.clone(), "Other".into()).await.err().unwrap().to_string().contains("writer lock"));
    #[cfg(unix)]
    {
        let alias = directory.path().join("alias.sqlite");
        std::os::unix::fs::symlink(&config.database, &alias).unwrap();
        let mut aliased = config.clone();
        aliased.database = alias;
        assert!(Runtime::prepare(aliased, "Other".into()).await.err().unwrap().to_string().contains("writer lock"));
    }
    let rsa_bytes = std::fs::read(&config.rsa_key).unwrap();
    let jwt_bytes = std::fs::read(&config.persisted_jwt).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for path in [&config.database, &config.rsa_key, &config.persisted_jwt] {
            assert_eq!(std::fs::metadata(path).unwrap().permissions().mode() & 0o777, 0o600);
        }
        assert_eq!(std::fs::metadata(&config.textures).unwrap().permissions().mode() & 0o777, 0o700);
    }
    let (base, signal, server) = start(runtime).await;
    let client = Client::builder().timeout(Duration::from_secs(5)).build().unwrap();
    assert_eq!(client.get(format!("{base}/health/ready")).send().await.unwrap().status(), StatusCode::OK);
    let response = client
        .get(format!("{base}/api/yggdrasil"))
        .header("host", "spoofed.invalid")
        .header("x-forwarded-host", "spoofed.invalid")
        .send()
        .await
        .unwrap();
    assert_eq!(response.headers()["x-authlib-injector-api-location"], "https://example.invalid/platform/api/yggdrasil");
    let metadata: Value = response.json().await.unwrap();
    assert_eq!(metadata["meta"]["serverName"], "Synthetic Velora");
    assert_eq!(metadata["skinDomains"], json!(["example.invalid"]));
    let signed: Value = client
        .get(format!("{base}/api/yggdrasil/sessionserver/session/minecraft/profile/{}?unsigned=false", original.uuid.replace('-', "")))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let public = rsa::RsaPublicKey::from_public_key_pem(metadata["signaturePublickey"].as_str().unwrap()).unwrap();
    let verifier = rsa::pkcs1v15::VerifyingKey::<sha1::Sha1>::new(public);
    for property in signed["properties"].as_array().unwrap() {
        let signature = base64::engine::general_purpose::STANDARD.decode(property["signature"].as_str().unwrap()).unwrap();
        verifier
            .verify(property["value"].as_str().unwrap().as_bytes(), &rsa::pkcs1v15::Signature::try_from(signature.as_slice()).unwrap())
            .unwrap();
    }
    let auth: Value = client
        .post(format!("{base}/api/yggdrasil/authserver/authenticate"))
        .json(&json!({"username":"BootAdmin","password":"synthetic-admin-password","clientToken":"synthetic-client"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let token = auth["accessToken"].as_str().unwrap();
    assert_eq!(auth["selectedProfile"]["id"], original.uuid.replace('-', ""));
    let join = client
        .post(format!("{base}/api/yggdrasil/sessionserver/session/minecraft/join"))
        .header("x-forwarded-for", "198.51.100.9")
        .json(&json!({"accessToken":token,"selectedProfile":original.uuid.replace('-',""),"serverId":"synthetic-server"}))
        .send()
        .await
        .unwrap();
    assert_eq!(join.status(), StatusCode::NO_CONTENT);
    assert_eq!(client.get(format!("{base}/api/yggdrasil/sessionserver/session/minecraft/hasJoined?username=BootAdmin&serverId=synthetic-server&ip=198.51.100.9")).send().await.unwrap().status(), StatusCode::NO_CONTENT);
    assert_eq!(
        client
            .get(format!(
                "{base}/api/yggdrasil/sessionserver/session/minecraft/hasJoined?username=BootAdmin&serverId=synthetic-server&ip=127.0.0.1"
            ))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    assert_eq!(client.get(format!("{base}/api/v1/auth/me")).send().await.unwrap().status(), StatusCode::NOT_FOUND);
    stop(signal, server).await;
    assert!(pool.is_closed());
    std::fs::remove_file(&config.bootstrap.as_ref().unwrap().password_file).unwrap();
    let restarted = Runtime::prepare(config.clone(), "Synthetic Velora".into()).await.unwrap();
    let after = velora_auth_core::identity_store::find_by_name(restarted.pool(), "BootAdmin").await.unwrap().unwrap();
    assert_eq!((after.id, after.uuid, after.password_hash), (original.id, original.uuid, original.password_hash));
    assert_eq!(std::fs::read(&config.rsa_key).unwrap(), rsa_bytes);
    assert_eq!(std::fs::read(&config.persisted_jwt).unwrap(), jwt_bytes);
    let (base, signal, server) = start(restarted).await;
    assert_eq!(
        client
            .post(format!("{base}/api/yggdrasil/authserver/validate"))
            .json(&json!({"accessToken":token,"clientToken":"synthetic-client"}))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NO_CONTENT
    );
    stop(signal, server).await;
}

#[tokio::test]
async fn mixed_database_and_damaged_material_fail_without_repair_rotation_or_bootstrap() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("bootstrap-password"), "synthetic-admin-password").unwrap();
    let config = configuration(directory.path());
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(sqlx::sqlite::SqliteConnectOptions::new().filename(&config.database).create_if_missing(true).foreign_keys(true))
        .await
        .unwrap();
    sqlx::query("CREATE TABLE private_synthetic_marker(value TEXT)").execute(&pool).await.unwrap();
    pool.close().await;
    let before = std::fs::read(&config.database).unwrap();
    assert!(Runtime::prepare(config.clone(), "Synthetic".into()).await.err().unwrap().to_string().contains("offline import"));
    assert_eq!(std::fs::read(&config.database).unwrap(), before);
    assert!(!config.rsa_key.exists());
    assert!(!config.persisted_jwt.exists());
    // Replace only this synthetic fixture with an owned store; never operate on a production store.
    std::fs::remove_file(&config.database).unwrap();
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(sqlx::sqlite::SqliteConnectOptions::new().filename(&config.database).create_if_missing(true).foreign_keys(true))
        .await
        .unwrap();
    velora_auth_core::schema::initialize(&pool).await.unwrap();
    pool.close().await;
    std::fs::write(&config.rsa_key, "synthetic damaged key").unwrap();
    std::fs::write(&config.persisted_jwt, b"synthetic preserved JWT key material at least 32 bytes").unwrap();
    let jwt = std::fs::read(&config.persisted_jwt).unwrap();
    assert!(Runtime::prepare(config.clone(), "Synthetic".into()).await.is_err());
    assert_eq!(std::fs::read(&config.rsa_key).unwrap(), b"synthetic damaged key");
    assert_eq!(std::fs::read(&config.persisted_jwt).unwrap(), jwt);
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(sqlx::sqlite::SqliteConnectOptions::new().filename(&config.database).foreign_keys(true))
        .await
        .unwrap();
    assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM users").fetch_one(&pool).await.unwrap(), 0);
    pool.close().await;
}

#[tokio::test]
async fn readiness_and_game_admission_fail_on_live_store_outage_while_liveness_remains_available() {
    let directory = tempfile::tempdir().unwrap();
    let mut config = configuration(directory.path());
    config.bootstrap = None;
    // Avoid an additional slow key generation using a synthetic pre-existing signing key.
    use rsa::pkcs8::{EncodePrivateKey, LineEnding};
    let private = rsa::RsaPrivateKey::new(&mut rsa::rand_core::OsRng, 2048).unwrap();
    std::fs::write(&config.rsa_key, private.to_pkcs8_pem(LineEnding::LF).unwrap().as_bytes()).unwrap();
    let runtime = Runtime::prepare(config, "Synthetic".into()).await.unwrap();
    let pool = runtime.pool().clone();
    let (base, signal, server) = start(runtime).await;
    let client = Client::builder().timeout(Duration::from_secs(5)).build().unwrap();
    pool.close().await;
    let response = client.get(format!("{base}/health/ready")).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(response.json::<Value>().await.unwrap(), json!({"status":"unavailable"}));
    assert_eq!(client.get(format!("{base}/health/live")).send().await.unwrap().status(), StatusCode::OK);
    assert_eq!(
        client
            .post(format!("{base}/api/yggdrasil/authserver/authenticate"))
            .json(&json!({"username":"Synthetic","password":"synthetic-password"}))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::INTERNAL_SERVER_ERROR
    );
    stop(signal, server).await;
}

#[test]
fn binary_help_and_invalid_configuration_fail_without_storage_or_secret_echoes() {
    let directory = tempfile::tempdir().unwrap();
    let binary = env!("CARGO_BIN_EXE_velora-auth-server");
    let help = std::process::Command::new(binary).arg("--help").env_clear().current_dir(directory.path()).output().unwrap();
    assert!(help.status.success());
    assert!(String::from_utf8_lossy(&help.stdout).contains("/health/ready"));
    let missing = std::process::Command::new(binary).env_clear().current_dir(directory.path()).output().unwrap();
    assert!(!missing.status.success());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("VELORA_AUTH_PUBLIC_ORIGIN"));
    let invalid = std::process::Command::new(binary)
        .env_clear()
        .env("VELORA_AUTH_PUBLIC_ORIGIN", "https://synthetic:secret@example.invalid")
        .current_dir(directory.path())
        .output()
        .unwrap();
    assert!(!invalid.status.success());
    assert!(!String::from_utf8_lossy(&invalid.stderr).contains("synthetic:secret"));
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
}

#[cfg(unix)]
#[tokio::test]
async fn actual_binary_serves_readiness_and_sigterm_releases_listener_and_writer_lock() {
    use rsa::pkcs8::{EncodePrivateKey, LineEnding};
    struct Child(std::process::Child);
    impl Drop for Child {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let directory = tempfile::tempdir().unwrap();
    let mut config = configuration(directory.path());
    config.bootstrap = None;
    let private = rsa::RsaPrivateKey::new(&mut rsa::rand_core::OsRng, 2048).unwrap();
    std::fs::write(&config.rsa_key, private.to_pkcs8_pem(LineEnding::LF).unwrap().as_bytes()).unwrap();
    let reservation = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = reservation.local_addr().unwrap();
    drop(reservation);
    let mut child = Child(
        std::process::Command::new(env!("CARGO_BIN_EXE_velora-auth-server"))
            .env_clear()
            .env("VELORA_AUTH_PUBLIC_ORIGIN", "https://example.invalid/platform")
            .env("VELORA_AUTH_DATA_DIR", directory.path())
            .env("VELORA_AUTH_BIND", address.to_string())
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let client = Client::builder().timeout(Duration::from_secs(1)).build().unwrap();
    let url = format!("http://{address}/health/ready");
    tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            if client.get(&url).send().await.is_ok_and(|response| response.status() == StatusCode::OK) {
                break;
            }
            assert!(child.0.try_wait().unwrap().is_none(), "service exited before readiness");
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .unwrap();
    assert!(std::process::Command::new("kill").args(["-TERM", &child.0.id().to_string()]).status().unwrap().success());
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(status) = child.0.try_wait().unwrap() {
                assert!(status.success());
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .unwrap();
    assert!(client.get(&url).send().await.is_err());
    let restarted = Runtime::prepare(config, "Synthetic".into()).await.unwrap();
    restarted.close().await;
}
