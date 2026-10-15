//! Player settings, stored as JSON in the app data directory.

use velora_launcher_core::launch::GcPreset;
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
        match crate::persistence::read_json(path) {
            Ok(Some(settings)) => settings,
            Ok(None) => Self::default(),
            Err(_) => {
                tracing::warn!("Settings could not be loaded; using defaults without changing the source file");
                Self::default()
            }
        }
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        crate::persistence::atomic_write_json(path, self)
    }

    pub fn update(current: &mut Self, path: &Path, change: impl FnOnce(&mut Self)) -> anyhow::Result<()> {
        let mut next = current.clone();
        change(&mut next);
        next.save(path)?;
        *current = next;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::tests::TestDirectory;
    use std::fs;

    #[test]
    fn legacy_partial_settings_keep_all_defaults() {
        let directory = TestDirectory::new();
        let path = directory.settings_path();
        fs::write(&path, br#"{"width":1600,"companion":{"opacity":0.5,"widgets":[{"id":"synthetic","enabled":true,"x":0.1,"y":0.2}]},"instance_overrides":{"synthetic":{"memory_max_mb":2048}}}"#).unwrap();
        let mut expected = Settings::default();
        expected.width = 1600;
        expected.companion.opacity = 0.5;
        expected.companion.widgets = vec![CompanionWidget { id: "synthetic".into(), enabled: true, x: 0.1, y: 0.2, scale: 1.0 }];
        expected.instance_overrides.insert("synthetic".into(), InstanceOverride { memory_max_mb: 2048, ..InstanceOverride::default() });
        assert_eq!(Settings::load(&path), expected);
        expected.save(&path).unwrap();
        assert_eq!(Settings::load(&path), expected);
        fs::write(&path, b"{}").unwrap();
        assert_eq!(Settings::load(&path), Settings::default());
    }

    #[test]
    fn missing_settings_can_be_saved_and_valid_settings_replaced() {
        let directory = TestDirectory::new();
        let path = directory.settings_path();
        assert_eq!(Settings::load(&path), Settings::default());
        let mut settings = Settings::default();
        settings.save(&path).unwrap();
        settings.width = 1920;
        settings.save(&path).unwrap();
        assert_eq!(Settings::load(&path), settings);
        directory.assert_only_settings();
    }

    #[test]
    fn corrupt_or_wrong_typed_settings_survive_default_load_and_save() {
        let directory = TestDirectory::new();
        let path = directory.settings_path();
        for bytes in [b"{synthetic corrupt".as_slice(), br#"{"width":"synthetic-invalid"}"#, b"null", b""] {
            fs::write(&path, bytes).unwrap();
            let settings = Settings::load(&path);
            assert_eq!(settings, Settings::default());
            assert!(settings.save(&path).is_err());
            assert_eq!(fs::read(&path).unwrap(), bytes);
            directory.assert_only_settings();
        }
    }

    #[test]
    fn failed_update_preserves_memory_and_successful_update_commits_both() {
        let directory = TestDirectory::new();
        let path = directory.settings_path();
        let mut current = Settings::default();
        let previous = current.clone();
        fs::write(&path, b"synthetic corrupt JSON").unwrap();
        assert!(Settings::update(&mut current, &path, |settings| settings.width = 1920).is_err());
        assert_eq!(current, previous);
        assert_eq!(fs::read(&path).unwrap(), b"synthetic corrupt JSON");
        fs::write(&path, b"{}").unwrap();
        Settings::update(&mut current, &path, |settings| settings.width = 1920).unwrap();
        assert_eq!(current.width, 1920);
        assert_eq!(Settings::load(&path), current);
    }

    #[cfg(unix)]
    #[test]
    fn unreadable_settings_are_not_replaced() {
        use std::os::unix::fs::PermissionsExt;
        let directory = TestDirectory::new();
        let path = directory.settings_path();
        fs::write(&path, b"{}").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).unwrap();
        let read_result = fs::read(&path);
        if read_result.is_err() {
            let settings = Settings::load(&path);
            assert_eq!(settings, Settings::default());
            assert!(settings.save(&path).is_err());
            directory.assert_only_settings();
        }
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"{}");
    }
}
