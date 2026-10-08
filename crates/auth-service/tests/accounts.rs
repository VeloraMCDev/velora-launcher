use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use sqlx::{Sqlite, SqlitePool, Transaction};
use std::sync::{
    atomic::{AtomicBool, AtomicU8, Ordering},
    Arc, Mutex,
};
use velora_auth_core::{identity::IdentityRecord, identity_store, launcher_sessions};
use velora_auth_http::{
    account::MutationHost,
    config::RuntimeConfig,
    error::{AppError, AppResult},
    reset::ResetHost,
    state::HostFuture,
    web::{AccountHost, AccountPolicy},
    web_routes::AccountPorts,
};
use velora_auth_service::Runtime;
use velora_platform_contracts::RegistrationMode;

struct SyntheticHost {
    audit: SqlitePool,
    policy: AtomicU8,
    online: AtomicBool,
    reject_rename: AtomicBool,
    mail: Mutex<Vec<String>>,
}
impl AccountHost for SyntheticHost {
    fn policy(&self) -> HostFuture<'_, AppResult<AccountPolicy>> {
        Box::pin(async move {
            Ok(AccountPolicy {
                panel_accounts: true,
                registration: match self.policy.load(Ordering::SeqCst) {
                    1 => RegistrationMode::Open,
                    2 => RegistrationMode::Approval,
                    _ => RegistrationMode::Closed,
                },
            })
        })
    }
    fn check_username<'a>(&'a self, name: &'a str) -> HostFuture<'a, AppResult<()>> {
        Box::pin(async move {
            if name.eq_ignore_ascii_case("Blocked") {
                Err(AppError::bad_request("synthetic username policy"))
            } else {
                Ok(())
            }
        })
    }
    fn record_login<'a>(&'a self, user: &'a IdentityRecord) -> HostFuture<'a, AppResult<()>> {
        Box::pin(async move {
            sqlx::query("INSERT INTO audit(user_id, username) VALUES(?,?)").bind(user.id).bind(&user.username).execute(&self.audit).await?;
            Ok(())
        })
    }
}
impl MutationHost for SyntheticHost {
    fn online<'a>(&'a self, _: &'a str) -> HostFuture<'a, AppResult<bool>> {
        Box::pin(async move { Ok(self.online.load(Ordering::SeqCst)) })
    }
    fn renamed<'a, 'c>(
        &'a self,
        tx: &'a mut Transaction<'c, Sqlite>,
        user: &'a IdentityRecord,
        _: &'a str,
    ) -> HostFuture<'a, AppResult<()>> {
        Box::pin(async move {
            launcher_sessions::revoke(tx, user.id).await?;
            if self.reject_rename.load(Ordering::SeqCst) {
                Err(AppError::bad_request("synthetic transactional side effect failed"))
            } else {
                Ok(())
            }
        })
    }
}
impl ResetHost for SyntheticHost {
    fn email_base(&self) -> HostFuture<'_, AppResult<String>> {
        Box::pin(async { Ok("https://example.invalid/platform".into()) })
    }
    fn send_email<'a>(&'a self, _: &'a str, _: &'a str, body: &'a str) -> HostFuture<'a, AppResult<()>> {
        Box::pin(async move {
            self.mail.lock().unwrap().push(body.to_owned());
            Ok(())
        })
    }
}
fn ports(host: &Arc<SyntheticHost>) -> AccountPorts {
    AccountPorts { policy: host.clone(), mutations: host.clone(), reset: host.clone() }
}
struct Running {
    base: String,
    stop: tokio::sync::oneshot::Sender<()>,
    task: tokio::task::JoinHandle<anyhow::Result<()>>,
}
async fn start(runtime: Runtime, host: &Arc<SyntheticHost>) -> Running {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let ports = ports(host);
    let task = tokio::spawn(runtime.serve_with_accounts(listener, ports, async {
        let _ = stopped.await;
    }));
    Running { base, stop, task }
}
async fn stop(running: Running) {
    running.stop.send(()).unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), running.task).await.unwrap().unwrap().unwrap();
}
fn configuration(dir: &std::path::Path) -> RuntimeConfig {
    RuntimeConfig::from_lookup(|name| match name {
        "VELORA_AUTH_DATA_DIR" => Some(dir.display().to_string()),
        "VELORA_AUTH_PUBLIC_ORIGIN" => Some("https://example.invalid/platform".into()),
        _ => None,
    })
    .unwrap()
}
async fn host() -> Arc<SyntheticHost> {
    let audit = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::query("CREATE TABLE audit(user_id INTEGER,username TEXT)").execute(&audit).await.unwrap();
    Arc::new(SyntheticHost {
        audit,
        policy: AtomicU8::new(0),
        online: AtomicBool::new(false),
        reject_rename: AtomicBool::new(false),
        mail: Mutex::new(vec![]),
    })
}
async fn post(client: &Client, base: &str, path: &str, body: Value) -> (StatusCode, Value) {
    let response = client.post(format!("{base}/api/v1/{path}")).json(&body).send().await.unwrap();
    (response.status(), response.json().await.unwrap())
}

#[tokio::test]
async fn real_account_http_registration_admission_rename_reset_and_restart() {
    let dir = tempfile::tempdir().unwrap();
    let config = configuration(dir.path());
    let runtime = Runtime::prepare(config.clone(), "Synthetic".into()).await.unwrap();
    let pool = runtime.pool().clone();
    let host = host().await;
    let running = start(runtime, &host).await;
    let base = &running.base;
    let client = Client::builder().timeout(std::time::Duration::from_secs(10)).build().unwrap();
    let input = json!({"username":"Example","password":"synthetic-password","email":"example@example.invalid"});
    assert_eq!(post(&client, base, "auth/register", input.clone()).await.0, StatusCode::FORBIDDEN);
    host.policy.store(2, Ordering::SeqCst);
    let pending = post(&client, base, "auth/register", json!({"username":"Pending","password":"synthetic-password"})).await;
    assert_eq!(pending.0, StatusCode::OK);
    assert_eq!(pending.1["pending"], true);
    assert_eq!(pending.1["token"], "");
    assert!(pending.1["yggdrasil"].is_null());
    assert_eq!(
        post(&client, base, "auth/login", json!({"username":"Pending","password":"synthetic-password"})).await.0,
        StatusCode::FORBIDDEN
    );
    host.policy.store(1, Ordering::SeqCst);
    assert_eq!(
        post(&client, base, "auth/register", json!({"username":"Blocked","password":"synthetic-password"})).await.1["error"],
        "synthetic username policy"
    );
    let registered = post(&client, base, "auth/register", input).await;
    assert_eq!(registered.0, StatusCode::OK);
    let id = registered.1["user"]["id"].as_i64().unwrap();
    let uuid = registered.1["user"]["uuid"].as_str().unwrap().to_owned();
    let login = post(&client, base, "auth/login", json!({"username":"Example","password":"synthetic-password"})).await;
    assert_eq!(login.0, StatusCode::OK);
    let token = login.1["token"].as_str().unwrap().to_owned();
    assert_eq!(sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM audit").fetch_one(&host.audit).await.unwrap(), 1);
    let me = client.get(format!("{base}/api/v1/auth/me")).bearer_auth(&token).send().await.unwrap();
    assert_eq!(me.status(), StatusCode::OK);
    assert_eq!(me.json::<Value>().await.unwrap()["uuid"], uuid);
    assert_eq!(
        client.get(format!("{base}/api/v1/auth/me")).send().await.unwrap().json::<Value>().await.unwrap()["error"],
        "sign in required"
    );
    sqlx::query("UPDATE users SET status='pending',auth_version=99 WHERE id=?").bind(id).execute(&pool).await.unwrap();
    assert_eq!(client.get(format!("{base}/api/v1/auth/me")).bearer_auth(&token).send().await.unwrap().status(), StatusCode::FORBIDDEN);
    sqlx::query("UPDATE users SET status='active',auth_version=0 WHERE id=?").bind(id).execute(&pool).await.unwrap();
    host.online.store(true, Ordering::SeqCst);
    let rename = || {
        client
            .put(format!("{base}/api/v1/account/username"))
            .bearer_auth(&token)
            .json(&json!({"username":"Renamed","password":"synthetic-password"}))
    };
    assert_eq!(rename().send().await.unwrap().status(), StatusCode::CONFLICT);
    host.online.store(false, Ordering::SeqCst);
    launcher_sessions::record(&pool, id, "127.0.0.1", "Example", "2026-01-01T00:00:00Z").await.unwrap();
    host.reject_rename.store(true, Ordering::SeqCst);
    assert_eq!(rename().send().await.unwrap().status(), StatusCode::BAD_REQUEST);
    assert_eq!(identity_store::find_by_id(&pool, id).await.unwrap().unwrap().username, "Example");
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM launcher_sessions WHERE user_id=?").bind(id).fetch_one(&pool).await.unwrap(),
        1
    );
    host.reject_rename.store(false, Ordering::SeqCst);
    let renamed = rename().send().await.unwrap().json::<Value>().await.unwrap();
    assert_eq!(renamed["user"]["uuid"], uuid);
    let renamed_token = renamed["token"].as_str().unwrap().to_owned();
    assert_eq!(client.get(format!("{base}/api/v1/auth/me")).bearer_auth(&token).send().await.unwrap().status(), StatusCode::UNAUTHORIZED);
    assert_eq!(post(&client, base, "auth/forgot-password", json!({"email":"example@example.invalid"})).await.0, StatusCode::OK);
    let mail = host.mail.lock().unwrap()[0].clone();
    let reset_token = mail.split("token=").nth(1).unwrap().lines().next().unwrap();
    let reset = json!({"token":reset_token,"password":"new-synthetic-password"});
    assert_eq!(post(&client, base, "auth/reset-password", reset.clone()).await.0, StatusCode::OK);
    assert_eq!(post(&client, base, "auth/reset-password", reset).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(
        client.get(format!("{base}/api/v1/auth/me")).bearer_auth(&renamed_token).send().await.unwrap().status(),
        StatusCode::UNAUTHORIZED
    );
    let final_login = post(&client, base, "auth/login", json!({"username":"Renamed","password":"new-synthetic-password"})).await;
    assert_eq!(final_login.0, StatusCode::OK);
    let final_token = final_login.1["token"].as_str().unwrap().to_owned();
    stop(running).await;
    let restarted = Runtime::prepare(config, "Synthetic".into()).await.unwrap();
    let restarted = start(restarted, &host).await;
    let me = client.get(format!("{}/api/v1/auth/me", restarted.base)).bearer_auth(final_token).send().await.unwrap();
    assert_eq!(me.status(), StatusCode::OK);
    assert_eq!(me.json::<Value>().await.unwrap()["uuid"], uuid);
    stop(restarted).await;
    host.audit.close().await;
}

#[tokio::test]
async fn required_audit_and_authority_outages_do_not_mint_tokens_or_cache_admission() {
    let dir = tempfile::tempdir().unwrap();
    let runtime = Runtime::prepare(configuration(dir.path()), "Synthetic".into()).await.unwrap();
    let pool = runtime.pool().clone();
    let host = host().await;
    host.policy.store(1, Ordering::SeqCst);
    let running = start(runtime, &host).await;
    let client = Client::new();
    let user = json!({"username":"Example","password":"synthetic-password"});
    let registered = post(&client, &running.base, "auth/register", user.clone()).await;
    let token = registered.1["token"].as_str().unwrap();
    let before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM ygg_tokens").fetch_one(&pool).await.unwrap();
    host.audit.close().await;
    assert_eq!(post(&client, &running.base, "auth/login", user).await.0, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM ygg_tokens").fetch_one(&pool).await.unwrap(), before);
    pool.close().await;
    assert_eq!(
        client.get(format!("{}/api/v1/auth/me", running.base)).bearer_auth(token).send().await.unwrap().status(),
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(client.get(format!("{}/health/ready", running.base)).send().await.unwrap().status(), StatusCode::SERVICE_UNAVAILABLE);
    stop(running).await;
}

#[tokio::test]
async fn actual_multipart_profile_cape_and_avatar_routes_keep_bytes_origins_and_errors() {
    let dir = tempfile::tempdir().unwrap();
    let runtime = Runtime::prepare(configuration(dir.path()), "Synthetic".into()).await.unwrap();
    let host = host().await;
    host.policy.store(1, Ordering::SeqCst);
    let running = start(runtime, &host).await;
    let client = Client::new();
    let registered = post(&client, &running.base, "auth/register", json!({"username":"Example","password":"synthetic-password"})).await;
    let token = registered.1["token"].as_str().unwrap();
    let uuid = registered.1["user"]["uuid"].as_str().unwrap();
    let profile_url = format!("{}/api/v1/account/profile", running.base);
    let profile: Value = client.get(&profile_url).bearer_auth(token).send().await.unwrap().json().await.unwrap();
    assert_eq!(profile["uuid"], uuid);
    assert!(profile["skin_url"].is_null());
    let skin_url = format!("{}/api/v1/account/skin", running.base);
    let missing =
        client.post(&skin_url).bearer_auth(token).multipart(reqwest::multipart::Form::new().text("model", "slim")).send().await.unwrap();
    assert_eq!(missing.status(), StatusCode::BAD_REQUEST);
    assert_eq!(missing.json::<Value>().await.unwrap()["error"], "no file uploaded");
    let image = image::RgbaImage::from_pixel(64, 64, image::Rgba([12, 34, 56, 255]));
    let mut buffer = std::io::Cursor::new(Vec::new());
    image.write_to(&mut buffer, image::ImageFormat::Png).unwrap();
    let png = buffer.into_inner();
    let form = reqwest::multipart::Form::new()
        .text("model", "slim")
        .part("file", reqwest::multipart::Part::bytes(png.clone()).file_name("synthetic.png"));
    let uploaded: Value = client.post(&skin_url).bearer_auth(token).multipart(form).send().await.unwrap().json().await.unwrap();
    assert_eq!(uploaded["skin_model"], "slim");
    let public_path = uploaded["skin_url"].as_str().unwrap().strip_prefix("https://example.invalid/platform/textures/").unwrap();
    let stored = client.get(format!("{}/textures/{public_path}", running.base)).send().await.unwrap();
    assert_eq!(stored.status(), StatusCode::OK);
    assert_eq!(stored.bytes().await.unwrap().as_ref(), png);
    let avatar = client.get(format!("{}/api/v1/avatar/{uuid}?size=32", running.base)).send().await.unwrap();
    assert_eq!(avatar.status(), StatusCode::OK);
    assert_eq!(avatar.headers()["content-type"], "image/png");
    assert_eq!(avatar.headers()["cache-control"], "public, max-age=300");
    let head = image::load_from_memory(&avatar.bytes().await.unwrap()).unwrap();
    assert_eq!((head.width(), head.height()), (32, 32));
    let model: Value = client
        .put(format!("{skin_url}/model"))
        .bearer_auth(token)
        .json(&json!({"model":"legacy-other"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(model["skin_model"], "classic");
    let cape_url = format!("{}/api/v1/account/cape", running.base);
    assert_eq!(
        client.put(&cape_url).bearer_auth(token).json(&json!({"cape_id":777})).send().await.unwrap().status(),
        StatusCode::FORBIDDEN
    );
    let cape: Value = client.put(cape_url).bearer_auth(token).json(&json!({"cape_id":null})).send().await.unwrap().json().await.unwrap();
    assert!(cape["cape"].is_null());
    let removed: Value = client.delete(skin_url).bearer_auth(token).send().await.unwrap().json().await.unwrap();
    assert!(removed["skin_url"].is_null());
    assert_eq!(client.get(format!("{}/api/v1/avatar/{uuid}", running.base)).send().await.unwrap().status(), StatusCode::NOT_FOUND);
    stop(running).await;
    host.audit.close().await;
}
