//! Standalone Yggdrasil authority. Web mutation/audit/presence cutover remains separate.
use anyhow::{Context, Result};
use axum::{
    extract::DefaultBodyLimit,
    http::{request::Parts, HeaderMap, HeaderValue, StatusCode},
    routing::get,
    Json, Router,
};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    SqlitePool,
};
use std::{
    future::Future,
    net::IpAddr,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use velora_auth_core::{identity_store, keys, login_guard::LoginGuard, password, schema, tokens};
use velora_auth_http::{
    config::RuntimeConfig,
    error::AppError,
    state::{AuthorityState, HostFuture, HostPorts},
};

struct Presentation {
    origin: String,
    name: String,
    proxies: Vec<IpAddr>,
}
impl HostPorts for Presentation {
    fn public_base<'a>(&'a self, _: &'a HeaderMap) -> HostFuture<'a, String> {
        Box::pin(async { self.origin.clone() })
    }
    fn server_name(&self) -> HostFuture<'_, Result<String, AppError>> {
        Box::pin(async { Ok(self.name.clone()) })
    }
    fn client_ip<'a>(&'a self, parts: &'a mut Parts) -> HostFuture<'a, Result<Option<String>, AppError>> {
        Box::pin(async move { Ok(velora_auth_http::net::client_ip(parts, &self.proxies)) })
    }
}

fn protected_directory(path: &Path) -> Result<()> {
    if path.exists() {
        anyhow::ensure!(path.is_dir(), "Authentication storage directory must be a directory");
        return Ok(());
    }
    let mut builder = std::fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path).context("creating protected Authentication storage directory")
}
fn prepare_database(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        protected_directory(parent)?;
    }
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    match options.open(path) {
        Ok(file) => file.sync_all().context("creating Authentication database file"),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            anyhow::ensure!(path.is_file(), "Authentication database must be a regular file");
            Ok(())
        }
        Err(error) => Err(error).context("opening protected Authentication database"),
    }
}

pub struct Runtime {
    // OS lock survives as an empty file; the held handle, not file existence, excludes a second writer.
    _lock: std::fs::File,
    authority: AuthorityState,
    // Load/preserve JWT material at boot, even while web account routes remain unmounted.
    jwt: Arc<tokens::Keys>,
    origin: String,
    serving: Arc<AtomicBool>,
}
impl Runtime {
    /// Validate/initialize only an owned store, preserving all existing signing material.
    pub async fn prepare(configuration: RuntimeConfig, server_name: String) -> Result<Self> {
        anyhow::ensure!(!server_name.trim().is_empty(), "Authentication server name must not be empty");
        let api_location = HeaderValue::from_str(&format!("{}/api/yggdrasil", configuration.public_origin))
            .context("Authentication public origin cannot be used in the API-location header")?;
        drop(api_location);
        protected_directory(&configuration.data_dir)?;
        if let Some(parent) = configuration.database.parent().filter(|p| !p.as_os_str().is_empty()) {
            protected_directory(parent)?;
        }
        let database_identity = if configuration.database.exists() {
            configuration.database.canonicalize().context("resolving Authentication database identity")?
        } else {
            let absolute = if configuration.database.is_absolute() {
                configuration.database.clone()
            } else {
                std::env::current_dir()?.join(&configuration.database)
            };
            absolute
                .parent()
                .context("Authentication database needs a parent directory")?
                .canonicalize()?
                .join(absolute.file_name().context("Authentication database needs a file name")?)
        };
        let mut lock_path = database_identity.as_os_str().to_os_string();
        lock_path.push(".service.lock");
        let lock_path = std::path::PathBuf::from(lock_path);
        if let Ok(metadata) = std::fs::symlink_metadata(&lock_path) {
            anyhow::ensure!(metadata.file_type().is_file(), "Authentication writer lock must be a regular file");
        }
        let mut lock_options = std::fs::OpenOptions::new();
        lock_options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            lock_options.mode(0o600);
        }
        let lock = lock_options.open(&lock_path).context("opening Authentication writer lock")?;
        lock.try_lock().map_err(|error| {
            anyhow::anyhow!("Authentication writer lock unavailable: {error}; run only one service for this owned database")
        })?;
        // Refuse a mixed/damaged store before opening it for writes or generating keys.
        if configuration.database.is_file() && std::fs::metadata(&configuration.database)?.len() != 0 {
            let probe = SqlitePoolOptions::new()
                .max_connections(1)
                .connect_with(
                    SqliteConnectOptions::new()
                        .filename(&configuration.database)
                        .read_only(true)
                        .foreign_keys(true)
                        .pragma("query_only", "ON"),
                )
                .await
                .context("opening existing Authentication store for validation")?;
            let valid = schema::validate(&probe).await;
            probe.close().await;
            valid.context("existing database is not an intact owned store; use explicit offline import, never a mixed legacy database")?;
        }
        prepare_database(&configuration.database)?;
        let options = SqliteConnectOptions::new()
            .filename(&configuration.database)
            .foreign_keys(true)
            .busy_timeout(std::time::Duration::from_secs(5));
        let pool =
            SqlitePoolOptions::new().max_connections(4).connect_with(options).await.context("connecting Authentication owned store")?;
        let prepared = async {
            schema::initialize(&pool)
                .await
                .context("validating Authentication owned schema; mixed stores require explicit offline import")?;
            let has_admin = identity_store::has_admin(&pool).await?;
            let credentials = configuration.bootstrap_credentials(has_admin)?;
            protected_directory(&configuration.textures)?;
            for path in [&configuration.rsa_key, &configuration.persisted_jwt] {
                if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
                    protected_directory(parent)?;
                }
            }
            let signing_path = configuration.rsa_key.clone();
            let secret = configuration.jwt_secret()?;
            let ygg = tokio::task::spawn_blocking(move || keys::Keys::load_or_create(&signing_path)).await??;
            if let Some(credentials) = credentials {
                let hash = password::hash_password(&credentials.password).map_err(anyhow::Error::msg)?;
                let created = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
                let uuid = uuid::Uuid::new_v4().to_string();
                identity_store::insert_first_admin(
                    &pool,
                    identity_store::NewAccount {
                        username: &credentials.username,
                        password_hash: &hash,
                        email: None,
                        role: "admin",
                        status: "active",
                        created_at: &created,
                        uuid: &uuid,
                    },
                )
                .await
                .context("creating first administrator without replacing existing accounts")?;
            }
            Ok::<_, anyhow::Error>((ygg, secret))
        }
        .await;
        let (ygg, secret) = match prepared {
            Ok(value) => value,
            Err(error) => {
                pool.close().await;
                return Err(error);
            }
        };
        let authority = AuthorityState {
            db: pool.clone(),
            identity_db: pool,
            ygg: Arc::new(ygg),
            login_guard: Arc::new(LoginGuard::default()),
            textures_dir: configuration.textures,
            host: Arc::new(Presentation {
                origin: configuration.public_origin.clone(),
                name: server_name,
                proxies: configuration.trusted_proxies,
            }),
            implementation_version: env!("CARGO_PKG_VERSION").into(),
        };
        Ok(Self {
            _lock: lock,
            authority,
            jwt: Arc::new(tokens::Keys::new(&secret)),
            origin: configuration.public_origin,
            serving: Arc::new(AtomicBool::new(true)),
        })
    }
    pub fn pool(&self) -> &SqlitePool {
        &self.authority.db
    }
    /// Explicit account composition; the embedder supplies every platform side effect.
    /// The CLI keeps game-only routing until real policy/audit/presence/email ports are configured.
    pub fn router_with_accounts(&self, ports: velora_auth_http::web_routes::AccountPorts) -> Router {
        let accounts =
            velora_auth_http::web::AccountState { authority: self.authority.clone(), tokens: self.jwt.clone(), host: ports.policy };
        self.router().merge(velora_auth_http::web_routes::routes(velora_auth_http::web_routes::AccountHttpState {
            accounts,
            mutations: ports.mutations,
            reset: ports.reset,
        }))
    }
    pub fn router(&self) -> Router {
        let pool = self.authority.db.clone();
        let serving = self.serving.clone();
        let origin = self.origin.clone();
        velora_auth_http::routes()
            .with_state(self.authority.clone())
            .route("/health/live", get(|| async { Json(serde_json::json!({"status":"alive"})) }))
            .route(
                "/health/ready",
                get(move || {
                    let pool = pool.clone();
                    let serving = serving.clone();
                    async move {
                        if serving.load(Ordering::Acquire) && schema::validate(&pool).await.is_ok() {
                            (StatusCode::OK, Json(serde_json::json!({"status":"ready"})))
                        } else {
                            (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"status":"unavailable"})))
                        }
                    }
                }),
            )
            .layer(DefaultBodyLimit::max(4 * 1024 * 1024))
            .layer(axum::middleware::map_response(move |mut response: axum::response::Response| {
                let origin = origin.clone();
                async move {
                    response.headers_mut().insert(
                        "x-authlib-injector-api-location",
                        HeaderValue::from_str(&format!("{origin}/api/yggdrasil")).expect("validated at startup"),
                    );
                    response
                }
            }))
    }
    /// Stop admission readiness before draining requests; close SQLite after the server stops.
    pub async fn serve<F>(self, listener: tokio::net::TcpListener, shutdown: F) -> Result<()>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let router = self.router();
        self.serve_router(listener, router, shutdown).await
    }
    pub async fn serve_with_accounts<F>(
        self,
        listener: tokio::net::TcpListener,
        ports: velora_auth_http::web_routes::AccountPorts,
        shutdown: F,
    ) -> Result<()>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let router = self.router_with_accounts(ports);
        self.serve_router(listener, router, shutdown).await
    }
    async fn serve_router<F>(self, listener: tokio::net::TcpListener, router: Router, shutdown: F) -> Result<()>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let serving = self.serving.clone();
        let result = axum::serve(listener, router.into_make_service_with_connect_info::<std::net::SocketAddr>())
            .with_graceful_shutdown(async move {
                shutdown.await;
                serving.store(false, Ordering::Release);
            })
            .await;
        self.serving.store(false, Ordering::Release);
        self.authority.db.close().await;
        result.context("serving Authentication requests")
    }
    pub async fn close(self) {
        self.serving.store(false, Ordering::Release);
        self.authority.db.close().await;
    }
}
