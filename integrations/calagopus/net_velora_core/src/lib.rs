//! Velora Core for Calagopus: connect a game server to the Velora Panel, install the Velora Core mod from it, and keep it current.
use indexmap::IndexMap;
use shared::{
    State,
    extensions::{
        Extension, ExtensionPermissionsBuilder, ExtensionRouteBuilder,
        background_tasks::BackgroundTaskBuilder,
        settings::ExtensionSettingsDeserializer,
    },
    permissions::PermissionGroup,
};
use std::{sync::Arc, time::Duration};

mod routes;
mod settings;
mod updater;
mod velora;
mod wings;

#[derive(Default)]
pub struct ExtensionStruct;

#[async_trait::async_trait]
impl Extension for ExtensionStruct {
    async fn settings_deserializer(&self, _state: State) -> ExtensionSettingsDeserializer {
        Arc::new(settings::ExtensionSettingsDataDeserializer)
    }

    async fn initialize_permissions(&mut self, _state: State, builder: ExtensionPermissionsBuilder) -> ExtensionPermissionsBuilder {
        builder
            .add_server_permission_group(
                "velora-core",
                PermissionGroup {
                    description: "Controls access to the Velora Core mod on a server.",
                    permissions: IndexMap::from([
                        ("read", "Allows viewing which Velora Core version is installed and whether an update is available."),
                        ("manage", "Allows connecting the server to the Velora Panel and installing or updating Velora Core."),
                    ]),
                },
            )
            .add_admin_permission_group(
                "velora-core",
                PermissionGroup {
                    description: "Admin controls for the Velora Core extension.",
                    permissions: IndexMap::from([("manage", "Allows changing the Velora Panel address and automatic update settings.")]),
                },
            )
    }

    async fn initialize_router(&mut self, state: State, builder: ExtensionRouteBuilder) -> ExtensionRouteBuilder {
        builder
            .add_admin_api_router(|router| router.nest("/extensions/net.velora.core", routes::admin::router(&state)))
            .add_client_server_api_router(|router| router.nest("/velora-core", routes::server::router(&state)))
    }

    async fn initialize_background_tasks(&mut self, _state: State, builder: BackgroundTaskBuilder) -> BackgroundTaskBuilder {
        // A restart takes a server through "offline" for only a few seconds, so poll quickly while a server is waiting
        // for that moment and slowly otherwise. Updates only ever happen while a server is stopped.
        builder
            .add_task("net-velora-core-auto-update", |state| async move {
                // A failure (Panel unreachable, node down) must not become a busy loop, so it just waits for the next pass.
                let waiting = match updater::tick(&state).await {
                    Ok(waiting) => waiting,
                    Err(err) => {
                        tracing::debug!(%err, "Velora Core auto-update pass failed");
                        false
                    }
                };
                tokio::time::sleep(Duration::from_secs(if waiting { 5 } else { 300 })).await;
                Ok(())
            })
            .await;
        builder
    }
}
