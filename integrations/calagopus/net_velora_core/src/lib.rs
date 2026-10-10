//! Velora Core for Calagopus: connect a game server to the Velora Panel and inspect its manually installed Velora Core mod.
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
use std::sync::Arc;

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
                        ("manage", "Allows writing the Velora connection config and setting up Velora sign-in."),
                    ]),
                },
            )
            .add_admin_permission_group(
                "velora-core",
                PermissionGroup {
                    description: "Admin controls for the Velora Core extension.",
                    permissions: IndexMap::from([("manage", "Allows changing the Velora Panel address.")]),
                },
            )
    }

    async fn initialize_router(&mut self, state: State, builder: ExtensionRouteBuilder) -> ExtensionRouteBuilder {
        builder
            .add_admin_api_router(|router| router.nest("/extensions/net.velora.core", routes::admin::router(&state)))
            .add_client_server_api_router(|router| router.nest("/velora-core", routes::server::router(&state)))
    }

    async fn initialize_background_tasks(&mut self, _state: State, builder: BackgroundTaskBuilder) -> BackgroundTaskBuilder {
        // Mod jars are downloaded from the Velora Admin Panel and uploaded by the operator.
        builder
    }
}
