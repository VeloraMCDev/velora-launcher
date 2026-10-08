use anyhow::{Context, Result};
use velora_auth_http::config::RuntimeConfig;

async fn shutdown() {
    let interrupt = async {
        tokio::signal::ctrl_c().await.expect("installing interrupt handler");
    };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).expect("installing termination handler").recv().await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! { () = interrupt => {}, () = terminate => {} }
}

#[tokio::main]
async fn main() -> Result<()> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if arguments == ["--help"] {
        println!("Velora game-authentication service\nConfiguration: VELORA_AUTH_PUBLIC_ORIGIN (required), VELORA_AUTH_BIND, VELORA_AUTH_DATA_DIR, VELORA_AUTH_SERVER_NAME.\nSee SERVICE.md for storage, signing, bootstrap and trusted-proxy configuration.\nRoutes: /api/yggdrasil, /textures, /health/live, /health/ready.\nWeb account/gateway cutover remains incomplete.");
        return Ok(());
    }
    if arguments == ["--version"] {
        println!("velora-auth-server {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    anyhow::ensure!(arguments.is_empty(), "unexpected arguments; use --help (credentials belong in protected configuration files)");
    let configuration = RuntimeConfig::from_env().context("invalid Authentication configuration")?;
    let bind = configuration.bind;
    let name = std::env::var("VELORA_AUTH_SERVER_NAME").unwrap_or_else(|_| "Velora".into());
    let runtime = velora_auth_service::Runtime::prepare(configuration, name).await?;
    let listener = match tokio::net::TcpListener::bind(bind).await {
        Ok(listener) => listener,
        Err(error) => {
            runtime.close().await;
            return Err(error).context("binding Authentication listener");
        }
    };
    eprintln!("Velora game authentication listening on {}", listener.local_addr()?);
    runtime.serve(listener, shutdown()).await
}
