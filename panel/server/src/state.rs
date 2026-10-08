use crate::auth::{Keys, LoginGuard};
use crate::config::Config;
use sqlx::SqlitePool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub platform_db: SqlitePool,
    pub instance_id: Option<String>,
    pub experiences: Arc<crate::experience::ExperienceStores>,
    pub cfg: Arc<Config>,
    /// Panel session tokens (JWT).
    pub keys: Arc<Keys>,
    /// Auth server signing key (textures, chat certificates).
    pub ygg: Arc<crate::yggdrasil::keys::Keys>,
    pub http: reqwest::Client,
    pub login_guard: Arc<LoginGuard>,
    pub worldmap: Arc<crate::worldmap::WorldMap>,
}

/// Handlers receive the request's experience state; authentication still uses platform state.
pub struct RequestState<T = AppState>(pub T);
impl axum::extract::FromRequestParts<AppState> for RequestState {
    type Rejection = std::convert::Infallible;
    async fn from_request_parts(parts: &mut axum::http::request::Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        Ok(Self(parts.extensions.get::<AppState>().unwrap_or(state).clone()))
    }
}
