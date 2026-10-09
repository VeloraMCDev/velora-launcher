//! Velora admin panel.
// SQLx maps several multi-column queries directly to tuples; aliases would
// obscure the selected column order without simplifying the query boundary.
#![allow(clippy::type_complexity)]

pub mod auth;
pub mod bbmodel;
pub mod casino;
pub mod config;
pub mod content_import;
pub mod db;
pub mod embeds;
pub mod error;
pub mod experience;
pub mod icons;
pub mod model_view;
pub mod net;
pub mod pack_import;
pub mod pack_safety;
pub mod packs;
pub mod progression;
pub mod purge;
pub mod rank_glyphs;
pub mod rewards;
pub mod routes;
pub mod scheduler;
pub mod seed;
pub mod state;
pub mod store;
pub mod textures;
pub mod worldmap;
pub mod yggdrasil;

use axum::http::{header, HeaderValue};
use axum::Router;
use rand::RngCore;
use state::AppState;
use std::sync::Arc;
use tower_http::compression::CompressionLayer;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;

/// Load (or create) the JWT signing secret.
pub fn jwt_secret(cfg: &config::Config) -> anyhow::Result<Vec<u8>> {
    velora_auth_core::secrets::jwt_secret(cfg.jwt_secret.as_deref().map(str::as_bytes), &cfg.data_dir.join("jwt.secret"))
}

pub async fn build_state(cfg: config::Config, db: sqlx::SqlitePool) -> anyhow::Result<AppState> {
    let path = cfg.signing_key_path();
    let ygg = tokio::task::spawn_blocking(move || yggdrasil::keys::Keys::load_or_create(&path)).await??;
    build_state_with_keys(cfg, db, Arc::new(ygg)).await
}

/// Like [`build_state`] with a given auth-server key (tests reuse one key).
pub async fn build_state_with_keys(cfg: config::Config, db: sqlx::SqlitePool, ygg: Arc<yggdrasil::keys::Keys>) -> anyhow::Result<AppState> {
    let secret = jwt_secret(&cfg)?;
    Ok(AppState {
        platform_db: db.clone(),
        instance_id: None,
        experiences: Arc::new(experience::ExperienceStores::default()),
        db,
        ygg,
        keys: Arc::new(auth::Keys::new(&secret)),
        http: velora_platform_utils::http::client(),
        login_guard: Arc::new(auth::LoginGuard::default()),
        worldmap: Arc::new(worldmap::WorldMap::new(&cfg.data_dir)),
        cfg: Arc::new(cfg),
    })
}

/// Create the first admin account if none exists.
pub async fn bootstrap_admin(state: &AppState) -> anyhow::Result<()> {
    if velora_auth_core::identity_store::has_admin(&state.platform_db).await? {
        return Ok(());
    }
    let (password, generated) = match &state.cfg.admin_password {
        Some(p) => (p.clone(), false),
        None => {
            let mut bytes = [0u8; 12];
            rand::thread_rng().fill_bytes(&mut bytes);
            (hex::encode(bytes), true)
        }
    };
    let created = velora_auth_http::web::bootstrap_admin(&auth::account::context(state), &state.cfg.admin_username, &password)
        .await
        .map_err(|e| anyhow::anyhow!(e.message))?;
    if created.is_none() {
        return Ok(());
    }
    if generated {
        tracing::warn!("============================================================");
        tracing::warn!(" Created admin account '{}' with password: {password}", state.cfg.admin_username);
        tracing::warn!(" Set ADMIN_PASSWORD to choose your own. Change it after login!");
        tracing::warn!("============================================================");
    } else {
        tracing::info!("created admin account '{}'", state.cfg.admin_username);
    }
    Ok(())
}

pub fn app(state: AppState) -> Router {
    let web = &state.cfg.web_dir;
    let spa = ServeDir::new(web).fallback(ServeFile::new(web.join("index.html")));
    let long_cache = SetResponseHeaderLayer::overriding(header::CACHE_CONTROL, HeaderValue::from_static("public, max-age=86400"));

    Router::new()
        .route("/healthz", axum::routing::get(routes::public::health))
        .route("/health", axum::routing::get(routes::public::health_detail))
        .route("/download/launcher/{platform}", axum::routing::get(routes::landing::download_launcher))
        .merge(routes::api(&state))
        .nest_service("/files", ServeDir::new(state.cfg.files_dir()))
        .nest_service("/uploads", tower::ServiceBuilder::new().layer(long_cache).service(ServeDir::new(state.cfg.uploads_dir())))
        .nest_service("/downloads", ServeDir::new(state.cfg.downloads_dir()))
        .fallback_service(spa)
        // Lets authlib-injector users enter just the panel URL (API Location Indication).
        .layer(SetResponseHeaderLayer::if_not_present(
            header::HeaderName::from_static("x-authlib-injector-api-location"),
            HeaderValue::from_static("/api/yggdrasil/"),
        ))
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .layer(axum::middleware::from_fn_with_state(state.clone(), experience::scope_request))
        .with_state(state)
}
