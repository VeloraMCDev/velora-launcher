//! Per-server Velora Core page: what is installed, connect it to the Velora Panel and explain manual installation.
use super::State;
use utoipa_axum::{router::OpenApiRouter, routes};

mod status {
    use crate::{settings, updater, velora, wings};
    use serde::Serialize;
    use shared::{
        GetState,
        models::{server::GetServer, user::GetPermissionManager},
        response::{ApiResponse, ApiResponseResult},
    };
    use std::time::Duration;
    use utoipa::ToSchema;

    #[derive(ToSchema, Serialize)]
    struct Release {
        version: String,
        minecraft: String,
        notes: String,
    }

    #[derive(ToSchema, Serialize)]
    pub struct Response {
        /// The Velora Panel address is set in the extension settings.
        configured: bool,
        panel_url: String,
        latest: Option<Release>,
        installed_version: Option<String>,
        installed_file: Option<String>,
        /// Installed under the pre-rename `scopenet-fabric-*` name.
        legacy_install: bool,
        fabric_api_present: bool,
        config_present: bool,
        /// `scopenet.properties` exists but `velora-core.properties` does not; the mod copies it on first start.
        legacy_config: bool,
        authlib_present: bool,
        /// Always false: mod installation is manual.
        auto_update: bool,
        /// Always false: no automatic updater is registered.
        auto_update_enabled: bool,
        update_available: bool,
        server_state: Option<String>,
        last_result: Option<String>,
        javaagent_flag: Option<String>,
        error: Option<String>,
    }

    #[utoipa::path(get, path = "/", responses(
        (status = OK, body = inline(Response)),
    ), params(
        ("server" = uuid::Uuid, description = "The server ID"),
    ))]
    pub async fn route(state: GetState, permissions: GetPermissionManager, server: GetServer) -> ApiResponseResult {
        permissions.has_server_permission("velora-core.read")?;
        let state = state.0;
        let server = server.0;
        let config = settings::load(&state).await?;
        let mut error = None;

        let latest = if config.panel_url.is_empty() {
            None
        } else {
            match velora::latest(&config.panel_url, Duration::from_secs(30)).await {
                Ok(latest) => latest,
                Err(err) => {
                    error = Some(format!("The Velora Panel could not be reached: {err}"));
                    None
                }
            }
        };
        let installed = match updater::installed(&state, &server).await {
            Ok(installed) => Some(installed),
            Err(err) => {
                error.get_or_insert(format!("The mods folder could not be read: {err}"));
                None
            }
        };
        let newest = installed.as_ref().and_then(|i| i.newest());
        let config_names = wings::list(&state, &server, updater::CONFIG_DIR).await.unwrap_or_default();
        let root_names = wings::list(&state, &server, "/").await.unwrap_or_default();
        let update_available = match (&latest, newest) {
            (Some(latest), Some(jar)) => velora::newer(&latest.version, &jar.version) || jar.legacy,
            _ => false,
        };

        ApiResponse::new_serialized(Response {
            configured: !config.panel_url.is_empty(),
            panel_url: config.panel_url.to_string(),
            installed_version: newest.map(|j| j.version.clone()),
            installed_file: newest.map(|j| j.file.clone()),
            legacy_install: newest.is_some_and(|j| j.legacy),
            fabric_api_present: installed.as_ref().is_some_and(|i| i.fabric_api),
            config_present: config_names.iter().any(|n| n == "velora-core.properties"),
            legacy_config: config_names.iter().any(|n| n == "scopenet.properties") && !config_names.iter().any(|n| n == "velora-core.properties"),
            authlib_present: root_names.iter().any(|n| n == "authlib-injector.jar"),
            auto_update: false,
            auto_update_enabled: false,
            update_available,
            server_state: wings::power_state(&state, &server).await.ok().flatten(),
            last_result: None,
            javaagent_flag: (!config.panel_url.is_empty()).then(|| velora::javaagent_flag(&config.panel_url)),
            latest: latest.map(|l| Release { version: l.version, minecraft: l.minecraft, notes: l.notes }),
            error,
        })
        .ok()
    }
}

mod connect {
    use crate::{settings, updater, velora, wings};
    use axum::http::StatusCode;
    use serde::{Deserialize, Serialize};
    use shared::{
        GetState,
        models::{
            server::{GetServer, GetServerActivityLogger},
            user::GetPermissionManager,
        },
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Deserialize)]
    pub struct Payload {
        /// The server token from the Velora Panel's Servers page (`sn_` and 40 characters).
        token: String,
        /// Replace an existing `velora-core.properties`, which resets every option in it.
        #[serde(default)]
        overwrite: bool,
    }

    #[derive(ToSchema, Serialize)]
    struct Response {
        message: String,
    }

    #[utoipa::path(post, path = "/connect", responses(
        (status = OK, body = inline(Response)),
        (status = BAD_REQUEST, body = shared::ApiError),
        (status = CONFLICT, body = shared::ApiError),
    ), params(
        ("server" = uuid::Uuid, description = "The server ID"),
    ), request_body = inline(Payload))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        server: GetServer,
        activity_logger: GetServerActivityLogger,
        shared::Payload(data): shared::Payload<Payload>,
    ) -> ApiResponseResult {
        permissions.has_server_permission("velora-core.manage")?;
        let state = state.0;
        let server = server.0;
        let token = data.token.trim().to_owned();
        if !velora::token_ok(&token) {
            return ApiResponse::error("That is not a Velora server token. Copy it from the Velora Panel's Servers page; it starts with sn_.").with_status(StatusCode::BAD_REQUEST).ok();
        }
        let config = settings::load(&state).await?;
        if config.panel_url.is_empty() {
            return ApiResponse::error("Ask an administrator to set the Velora Panel address in the Velora Core extension settings first.").with_status(StatusCode::CONFLICT).ok();
        }
        let names = wings::list(&state, &server, updater::CONFIG_DIR).await?;
        if names.iter().any(|n| n == "velora-core.properties") && !data.overwrite {
            return ApiResponse::error("This server already has a Velora Core config. Replacing it resets every option in it; confirm to continue.").with_status(StatusCode::CONFLICT).ok();
        }
        wings::create_directory(&state, &server, "/", "config").await;
        wings::write(&state, &server, updater::CONFIG_FILE, velora::config_file(&config.panel_url, &token)).await?;
        // The token is a credential: it goes to the server's own file and is never logged or kept by the Panel.
        activity_logger.log("server:velora-core.connect", serde_json::json!({ "panel_url": config.panel_url.to_string() })).await;
        ApiResponse::new_serialized(Response { message: "Connected. Restart the server so Velora Core reads the new config.".into() }).ok()
    }
}

mod authlib {
    use crate::{settings, wings};
    use axum::http::StatusCode;
    use serde::{Deserialize, Serialize};
    use shared::{
        GetState,
        models::{
            server::{GetServer, GetServerActivityLogger},
            user::GetPermissionManager,
        },
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Deserialize)]
    pub struct Payload {}

    #[derive(ToSchema, Serialize)]
    struct Response {
        message: String,
    }

    #[utoipa::path(post, path = "/authlib", responses(
        (status = OK, body = inline(Response)),
        (status = CONFLICT, body = shared::ApiError),
    ), params(
        ("server" = uuid::Uuid, description = "The server ID"),
    ), request_body = inline(Payload))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        server: GetServer,
        activity_logger: GetServerActivityLogger,
        shared::Payload(_data): shared::Payload<Payload>,
    ) -> ApiResponseResult {
        permissions.has_server_permission("velora-core.manage")?;
        let state = state.0;
        let server = server.0;
        let config = settings::load(&state).await?;
        if config.panel_url.is_empty() {
            return ApiResponse::error("Ask an administrator to set the Velora Panel address in the Velora Core extension settings first.").with_status(StatusCode::CONFLICT).ok();
        }
        if wings::list(&state, &server, "/").await?.iter().any(|n| n == "authlib-injector.jar") {
            return ApiResponse::new_serialized(Response { message: "authlib-injector.jar is already in the server folder.".into() }).ok();
        }
        let url = format!("{}/api/v1/launcher/authlib-injector.jar", config.panel_url);
        wings::pull(&state, &server, "/", &url, "authlib-injector.jar").await?;
        activity_logger.log("server:velora-core.authlib", serde_json::json!({})).await;
        ApiResponse::new_serialized(Response { message: "Downloaded authlib-injector.jar. Add the start flag shown on this page to the startup command.".into() }).ok()
    }
}

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(status::route))
        .routes(routes!(connect::route))
        .routes(routes!(authlib::route))
        .with_state(state.clone())
}
