//! Operator settings: where the Velora Panel is and whether linked servers update themselves.
use super::State;
use utoipa_axum::{router::OpenApiRouter, routes};

mod get {
    use crate::{settings, velora};
    use serde::Serialize;
    use shared::{
        GetState,
        models::user::GetPermissionManager,
        response::{ApiResponse, ApiResponseResult},
    };
    use std::time::Duration;
    use utoipa::ToSchema;

    #[derive(ToSchema, Serialize)]
    struct Release {
        version: String,
        minecraft: String,
        server_jar: String,
    }

    #[derive(ToSchema, Serialize)]
    struct Response {
        panel_url: String,
        auto_update: bool,
        linked_servers: usize,
        /// The release the Velora Panel serves, if the Panel is reachable and has approved one.
        latest: Option<Release>,
        /// Why the Velora Panel could not be asked, if it could not.
        error: Option<String>,
    }

    #[utoipa::path(get, path = "/", responses(
        (status = OK, body = inline(Response)),
    ))]
    pub async fn route(state: GetState, permissions: GetPermissionManager) -> ApiResponseResult {
        permissions.has_admin_permission("velora-core.manage")?;
        let state = state.0;
        let config = settings::load(&state).await?;
        let (latest, error) = if config.panel_url.is_empty() {
            (None, None)
        } else {
            match velora::latest(&config.panel_url, Duration::from_secs(5)).await {
                Ok(latest) => (latest, None),
                Err(err) => (None, Some(err.to_string())),
            }
        };
        ApiResponse::new_serialized(Response {
            panel_url: config.panel_url.to_string(),
            auto_update: config.auto_update,
            linked_servers: config.linked.len(),
            latest: latest.map(|l| Release { version: l.version, minecraft: l.minecraft, server_jar: l.server.name }),
            error,
        })
        .ok()
    }
}

mod put {
    use crate::{settings, velora};
    use axum::http::StatusCode;
    use serde::{Deserialize, Serialize};
    use shared::{
        GetState,
        models::user::GetPermissionManager,
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Deserialize)]
    pub struct Payload {
        panel_url: String,
        auto_update: bool,
    }

    #[derive(ToSchema, Serialize)]
    struct Response {}

    #[utoipa::path(put, path = "/", responses(
        (status = OK, body = inline(Response)),
        (status = BAD_REQUEST, body = shared::ApiError),
    ), request_body = inline(Payload))]
    pub async fn route(state: GetState, permissions: GetPermissionManager, shared::Payload(data): shared::Payload<Payload>) -> ApiResponseResult {
        permissions.has_admin_permission("velora-core.manage")?;
        let panel_url = if data.panel_url.trim().is_empty() {
            String::new()
        } else {
            match velora::normalize_panel_url(&data.panel_url) {
                Ok(url) => url,
                Err(message) => return ApiResponse::error(&message).with_status(StatusCode::BAD_REQUEST).ok(),
            }
        };
        settings::update(&state.0, |s| {
            s.panel_url = panel_url.into();
            s.auto_update = data.auto_update;
        })
        .await?;
        ApiResponse::new_serialized(Response {}).ok()
    }
}

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new().routes(routes!(get::route, put::route)).with_state(state.clone())
}
