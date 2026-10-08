//! Host-local adapters supply presentation and trusted addresses; no Panel or experience imports.
use crate::error::AppError;
use axum::http::{request::Parts, HeaderMap};
use std::{future::Future, path::PathBuf, pin::Pin, sync::Arc};
pub type HostFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
pub trait HostPorts: Send + Sync {
    fn public_base<'a>(&'a self, headers: &'a HeaderMap) -> HostFuture<'a, String>;
    fn server_name(&self) -> HostFuture<'_, Result<String, AppError>>;
    fn client_ip<'a>(&'a self, parts: &'a mut Parts) -> HostFuture<'a, Result<Option<String>, AppError>>;
}
#[derive(Clone)]
pub struct AuthorityState {
    /// Existing token/cape/asset account storage, supplied by the compatibility host.
    pub db: sqlx::SqlitePool,
    /// Existing live group/name lookup storage. A standalone owner can supply the same pool.
    pub identity_db: sqlx::SqlitePool,
    pub ygg: Arc<velora_auth_core::keys::Keys>,
    pub login_guard: Arc<velora_auth_core::login_guard::LoginGuard>,
    pub textures_dir: PathBuf,
    pub host: Arc<dyn HostPorts>,
    pub implementation_version: String,
}
