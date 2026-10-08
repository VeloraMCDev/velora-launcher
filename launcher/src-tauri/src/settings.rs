//! Player settings, stored as JSON in the app data directory.

use scopenet_core::launch::GcPreset;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct CompanionSettings {
    pub enabled: bool,
    pub notifications: bool,
    #[serde(rename = "claimBorders")]
    pub claim_borders: bool,
    pub scale: f32,
    pub opacity: f32,
    pub widgets: Vec<CompanionWidget>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompanionWidget {
    pub id: String,
    pub enabled: bool,
    pub x: f64,
    pub y: f64,
    /// This module's own size on top of the global scale (set per module in the launcher or in the game's HUD manager).
    #[serde(default = "one")]
    pub scale: f64,
}
fn one() -> f64 {
    1.0
}
impl Default for CompanionSettings {
    fn default() -> Self { Self { enabled: true, notifications: true, claim_borders: false, scale: 1.0, opacity: 0.88,
        widgets: [("level", 0.02, 0.04), ("balance", 0.76, 0.04), ("guild", 0.02, 0.8), ("claim", 0.4, 0.04), ("quests", 0.76, 0.3), ("clock", 0.76, 0.8)]
            .into_iter().map(|(id,x,y)| CompanionWidget { id: id.into(), enabled: id != "clock", x, y, scale: 1.0 }).collect() } }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(default)]
pub struct InstanceOverride {
    /// 0 = use the global / instance default.
    pub memory_max_mb: u32,
    pub memory_min_mb: u32,
    pub jvm_args: String,
    pub java_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Settings {
    /// Panel URL chosen by the player (ignored when one is baked in and locked).
    pub panel_url: Option<String>,
    pub selected_instance: Option<String>,

    // ---- Game ----
    /// 0 = use the value the admin set for the instance.
    pub memory_max_mb: u32,
    pub memory_min_mb: u32,
    pub custom_resolution: bool,
    pub width: u32,
    pub height: u32,
    pub fullscreen: bool,
    /// "minimize" | "hide" | "keep" | "close"
    pub after_launch: String,
    pub show_console: bool,

    // ---- Java ----
    pub java_path: Option<String>,
    pub gc: GcPreset,
    pub jvm_args: String,

    // ---- Appearance ----
    /// "server" (admin's theme) | "custom"
    pub theme_mode: String,
    pub accent: Option<String>,
    pub background_video: bool,
    pub reduce_motion: bool,
    /// None = follow the admin's setting.
    pub glass: Option<bool>,
    pub ui_scale: u32,

    // ---- Launcher ----
    pub concurrent_downloads: u32,
    pub check_updates: bool,
    pub send_stats: bool,

    // ---- Controls ----
    pub keybinds_enabled: bool,
    pub keybinds: BTreeMap<String, String>,
    pub vanilla_controls_enabled: bool,
    pub auto_jump: bool,
    pub sensitivity: f64,
    pub show_news: bool,
    pub show_social_sidebar: bool,
    pub companion: CompanionSettings,
    /// Vanilla options captured from each instance after the game exits.
    pub instance_game_options: HashMap<String, BTreeMap<String, String>>,

    pub instance_overrides: HashMap<String, InstanceOverride>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            panel_url: None,
            selected_instance: None,
            memory_max_mb: 0,
            memory_min_mb: 0,
            custom_resolution: false,
            width: 1280,
            height: 720,
            fullscreen: false,
            after_launch: "minimize".into(),
            show_console: false,
            java_path: None,
            gc: GcPreset::G1,
            jvm_args: String::new(),
            theme_mode: "server".into(),
            accent: None,
            background_video: true,
            reduce_motion: false,
            glass: None,
            ui_scale: 100,
            concurrent_downloads: 12,
            check_updates: true,
            send_stats: true,
            keybinds_enabled: false,
            keybinds: BTreeMap::new(),
            vanilla_controls_enabled: false,
            auto_jump: true,
            sensitivity: 0.5,
            show_news: true,
            show_social_sidebar: false,
            companion: CompanionSettings::default(),
            instance_game_options: HashMap::new(),
            instance_overrides: HashMap::new(),
        }
    }
}

impl Settings {
    pub fn load(path: &Path) -> Self {
        std::fs::read(path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(self)?)?;
        std::fs::rename(tmp, path)?;
        Ok(())
    }
}
