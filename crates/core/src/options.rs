//! Writes player keybinds / preferences into Minecraft's `options.txt`.

use anyhow::Result;
use std::collections::BTreeMap;
use std::path::Path;

/// Player-owned vanilla preferences. Pack-managed options.txt may be replaced
/// during sync, so these are captured separately and merged back afterward.
const VANILLA_KEYS: &[&str] = &[
    "fov",
    "gamma",
    "graphicsMode",
    "fancyGraphics",
    "renderDistance",
    "simulationDistance",
    "maxFps",
    "enableVsync",
    "fullscreen",
    "fullscreenResolution",
    "guiScale",
    "particles",
    "clouds",
    "renderClouds",
    "entityShadows",
    "entityDistanceScaling",
    "biomeBlendRadius",
    "mipmapLevels",
    "ao",
    "ambientOcclusion",
    "bobView",
    "attackIndicator",
    "autoJump",
    "mouseSensitivity",
    "invertYMouse",
    "discrete_mouse_scroll",
    "mouseWheelSensitivity",
    "rawMouseInput",
    "toggleCrouch",
    "toggleSprint",
    "sneakToggled",
    "sprintToggled",
    "screenEffectScale",
    "fovEffectScale",
    "darknessEffectScale",
    "narrator",
    "subtitles",
    "chatVisibility",
    "chatColors",
    "chatLinks",
    "chatLinksPrompt",
    "chatOpacity",
    "textBackgroundOpacity",
    "backgroundForChatOnly",
    "chatScale",
    "chatWidth",
    "chatHeightFocused",
    "chatHeightUnfocused",
    "mainHand",
    "lang",
    "forceUnicodeFont",
    "tutorialStep",
    "hideBundleTutorial",
    "soundDevice",
    "directionalAudio",
    "musicFrequency",
    "notificationDisplayTime",
    "joinedFirstServer",
    "pauseOnLostFocus",
    "reducedDebugInfo",
    "showSubtitles",
];

pub fn read_vanilla_preferences(game_dir: &Path) -> Result<BTreeMap<String, String>> {
    let path = game_dir.join("options.txt");
    let contents = match std::fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(e) => return Err(e.into()),
    };
    Ok(contents
        .lines()
        .filter_map(|line| {
            let (key, value) = line.split_once(':')?;
            (VANILLA_KEYS.contains(&key) || key.starts_with("soundCategory_")).then(|| (key.to_owned(), value.to_owned()))
        })
        .collect())
}

/// Merge `key_<binding>:<key>` lines into options.txt, keeping everything
/// else the game has written. Only for 1.13+ (named key codes).
pub fn apply_keybinds(game_dir: &Path, binds: &BTreeMap<String, String>) -> Result<()> {
    let mut values: BTreeMap<String, String> = binds.iter().map(|(k, v)| (format!("key_{k}"), v.clone())).collect();
    apply(game_dir, &mut values)
}

/// Merge arbitrary `name:value` options.
pub fn apply(game_dir: &Path, values: &mut BTreeMap<String, String>) -> Result<()> {
    if values.is_empty() {
        return Ok(());
    }
    let path = game_dir.join("options.txt");
    let existing = std::fs::read_to_string(&path).unwrap_or_default();
    let mut lines: Vec<String> = Vec::new();
    for line in existing.lines() {
        match line.split_once(':') {
            Some((k, _)) if values.contains_key(k) => {
                let v = values.remove(k).unwrap();
                lines.push(format!("{k}:{v}"));
            }
            _ => lines.push(line.to_string()),
        }
    }
    for (k, v) in values.iter() {
        lines.push(format!("{k}:{v}"));
    }
    std::fs::create_dir_all(game_dir)?;
    std::fs::write(path, lines.join("\n") + "\n")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merges_keybinds() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("options.txt"), "version:3465\nkey_key.forward:key.keyboard.w\nfov:0.0\n").unwrap();
        let mut binds = BTreeMap::new();
        binds.insert("key.forward".to_string(), "key.keyboard.up".to_string());
        binds.insert("key.sprint".to_string(), "key.keyboard.left.control".to_string());
        apply_keybinds(dir.path(), &binds).unwrap();
        let out = std::fs::read_to_string(dir.path().join("options.txt")).unwrap();
        assert_eq!(out, "version:3465\nkey_key.forward:key.keyboard.up\nfov:0.0\nkey_key.sprint:key.keyboard.left.control\n");
    }

    #[test]
    fn captures_only_vanilla_preferences() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("options.txt"),
            "fov:0.8\ngamma:1.0\nautoJump:false\nsoundCategory_music:0.25\nkey_key.forward:key.keyboard.w\nmodded:yes\n",
        )
        .unwrap();
        let prefs = read_vanilla_preferences(dir.path()).unwrap();
        assert_eq!(prefs.get("fov").map(String::as_str), Some("0.8"));
        assert_eq!(prefs.get("autoJump").map(String::as_str), Some("false"));
        assert_eq!(prefs.get("soundCategory_music").map(String::as_str), Some("0.25"));
        assert!(!prefs.contains_key("modded"));
        assert!(!prefs.contains_key("key_key.forward"));
    }
}
