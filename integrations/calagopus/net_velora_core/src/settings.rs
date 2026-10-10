use serde::{Deserialize, Serialize};
use shared::extensions::settings::{
    ExtensionSettings, SettingsDeserializeExt, SettingsDeserializer, SettingsSerializeExt, SettingsSerializer,
};

/// What the operator configures once: where the Velora Panel lives and whether linked servers update themselves.
/// The Velora server token is never stored here; it is written straight into the server's own config file.
#[derive(Clone, Serialize, Deserialize)]
pub struct ExtensionSettingsData {
    /// Public address of the Velora Panel, such as `https://velora.example.com`. Empty until configured.
    pub panel_url: compact_str::CompactString,
    /// Install new approved releases on linked servers while they are stopped.
    pub auto_update: bool,
    /// Servers that opted in to automatic updates.
    pub linked: Vec<uuid::Uuid>,
}

impl Default for ExtensionSettingsData {
    fn default() -> Self {
        Self { panel_url: Default::default(), auto_update: true, linked: Vec::new() }
    }
}

#[async_trait::async_trait]
impl SettingsSerializeExt for ExtensionSettingsData {
    async fn serialize(&self, serializer: SettingsSerializer) -> Result<SettingsSerializer, anyhow::Error> {
        Ok(serializer
            .write_raw_setting("panel_url", self.panel_url.clone())
            .write_raw_setting("auto_update", if self.auto_update { "true" } else { "false" })
            .write_serde_setting("linked", &self.linked)?)
    }
}

pub struct ExtensionSettingsDataDeserializer;

#[async_trait::async_trait]
impl SettingsDeserializeExt for ExtensionSettingsDataDeserializer {
    async fn deserialize_boxed(&self, mut deserializer: SettingsDeserializer<'_>) -> Result<ExtensionSettings, anyhow::Error> {
        Ok(Box::new(ExtensionSettingsData {
            panel_url: deserializer.take_raw_setting("panel_url").unwrap_or_default(),
            auto_update: deserializer.take_raw_setting("auto_update").map(|v| v != "false").unwrap_or(true),
            linked: deserializer.read_serde_setting("linked").unwrap_or_default(),
        }))
    }
}

/// Reads the current settings. A fresh install has none, so defaults apply.
pub async fn load(state: &shared::State) -> Result<ExtensionSettingsData, anyhow::Error> {
    let settings = state.settings.get().await?;
    Ok(settings.find_extension_settings::<ExtensionSettingsData>()?.clone())
}

/// Applies `change` to the stored settings and saves them.
pub async fn update(state: &shared::State, change: impl FnOnce(&mut ExtensionSettingsData)) -> Result<(), anyhow::Error> {
    let mut settings = state.settings.get_mut().await?;
    change(settings.find_mut_extension_settings::<ExtensionSettingsData>()?);
    settings.save().await?;
    Ok(())
}
