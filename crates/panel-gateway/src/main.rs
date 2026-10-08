use anyhow::{Context, Result};
use velora_panel_gateway::{config::Configuration, Gateway};
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
        println!("Velora HTTP gateway\nRequired: VELORA_GATEWAY_PUBLIC_ORIGIN, VELORA_GATEWAY_AUTH_UPSTREAM.\nOptional: VELORA_GATEWAY_PANEL_UPSTREAM, VELORA_GATEWAY_BIND, VELORA_GATEWAY_TRUSTED_PROXIES, VELORA_GATEWAY_HEADER_TIMEOUT_SECS.\nGame authentication and textures have a fixed owner; remaining routes require a Panel upstream.\nSee GATEWAY.md for TLS, trusted proxy, lifecycle and migration limits.");
        return Ok(());
    }
    if arguments == ["--version"] {
        println!("velora-gateway {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    anyhow::ensure!(arguments.is_empty(), "unexpected arguments; use --help");
    let config = Configuration::from_env()?;
    let listener = tokio::net::TcpListener::bind(config.bind).await.context("binding gateway listener")?;
    eprintln!("Velora gateway listening on {}", listener.local_addr()?);
    Gateway::new(config).serve(listener, shutdown()).await
}
