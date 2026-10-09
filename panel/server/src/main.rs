use velora_panel::{app, bootstrap_admin, build_state, config::Config, db};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // `velora-panel healthcheck` — used by the Docker HEALTHCHECK so the
    // image doesn't need curl.
    if std::env::args().nth(1).as_deref() == Some("healthcheck") {
        return healthcheck().await;
    }

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,tower_http=warn,sqlx=warn")))
        .compact()
        .init();

    let cfg = Config::from_env();
    let pool = db::connect(&cfg.data_dir).await?;
    if !cfg.web_dir.join("index.html").exists() {
        tracing::warn!("web UI not found at {} (build panel/web or set SCOPENET_WEB_DIR)", cfg.web_dir.display());
    }
    let bind = cfg.bind.clone();
    let state = build_state(cfg, pool).await?;
    bootstrap_admin(&state).await?;
    state.experiences.enable_workers(&state).await.map_err(|e| anyhow::anyhow!(e.message))?;
    tokio::spawn(velora_panel::routes::mc_assets::ensure_on_startup(state.clone()));

    let listener = tokio::net::TcpListener::bind(&bind).await?;
    tracing::info!("Velora panel v{} listening on http://{bind}", env!("CARGO_PKG_VERSION"));
    axum::serve(listener, app(state).into_make_service_with_connect_info::<std::net::SocketAddr>())
        .with_graceful_shutdown(shutdown())
        .await?;
    Ok(())
}

async fn shutdown() {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.ok();
    };
    #[cfg(unix)]
    let term = async {
        if let Ok(mut s) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            s.recv().await;
        }
    };
    #[cfg(not(unix))]
    let term = std::future::pending::<()>();
    tokio::select! { _ = ctrl_c => {}, _ = term => {} }
    tracing::info!("shutting down");
}

async fn healthcheck() -> anyhow::Result<()> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let bind = Config::from_env().bind;
    let port = bind.rsplit(':').next().unwrap_or("8080");
    let mut s = tokio::net::TcpStream::connect(format!("127.0.0.1:{port}")).await?;
    s.write_all(b"GET /healthz HTTP/1.0\r\nHost: localhost\r\n\r\n").await?;
    let mut buf = String::new();
    s.read_to_string(&mut buf).await?;
    if buf.starts_with("HTTP/1.1 200") || buf.starts_with("HTTP/1.0 200") {
        Ok(())
    } else {
        anyhow::bail!("unhealthy: {}", buf.lines().next().unwrap_or(""))
    }
}
