//! Mojang version JSON (also used by Fabric/Quilt/Forge profiles).

use crate::rules::Rule;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VersionJson {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inherits_from: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jar: Option<String>,
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub main_class: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minecraft_arguments: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Arguments>,
    #[serde(default)]
    pub libraries: Vec<Library>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset_index: Option<AssetIndexRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assets: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub downloads: Option<VersionDownloads>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub java_version: Option<JavaVersion>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logging: Option<Logging>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_time: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Arguments {
    #[serde(default)]
    pub game: Vec<Arg>,
    #[serde(default)]
    pub jvm: Vec<Arg>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum Arg {
    Plain(String),
    Ruled { rules: Vec<Rule>, value: ArgValue },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ArgValue {
    One(String),
    Many(Vec<String>),
}

impl ArgValue {
    pub fn values(&self) -> Vec<String> {
        match self {
            ArgValue::One(s) => vec![s.clone()],
            ArgValue::Many(v) => v.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Library {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub downloads: Option<LibraryDownloads>,
    /// Maven repository base (Fabric/Quilt/legacy Forge style).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rules: Option<Vec<Rule>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub natives: Option<HashMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extract: Option<Extract>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha1: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LibraryDownloads {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact: Option<Artifact>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub classifiers: Option<HashMap<String, Artifact>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Artifact {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha1: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    #[serde(default)]
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Extract {
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AssetIndexRef {
    pub id: String,
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub total_size: Option<u64>,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VersionDownloads {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client: Option<Artifact>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct JavaVersion {
    #[serde(default)]
    pub component: String,
    #[serde(default)]
    pub major_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Logging {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client: Option<LoggingClient>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LoggingClient {
    pub argument: String,
    pub file: LogFile,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LogFile {
    pub id: String,
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
    pub url: String,
}

impl VersionJson {
    /// Java major version this version wants (8 for anything old).
    pub fn java_major(&self) -> u32 {
        self.java_version.as_ref().map(|j| j.major_version).filter(|m| *m > 0).unwrap_or(8)
    }

    pub fn java_component(&self) -> String {
        self.java_version.as_ref().map(|j| j.component.clone()).filter(|c| !c.is_empty()).unwrap_or_else(|| "jre-legacy".into())
    }

    /// True for 1.13+ style argument lists.
    pub fn is_modern(&self) -> bool {
        self.arguments.is_some()
    }

    /// Does this version understand `--quickPlayMultiplayer` (1.20+)?
    pub fn supports_quick_play(&self) -> bool {
        self.arguments
            .as_ref()
            .map(|a| {
                a.game.iter().any(|arg| match arg {
                    Arg::Plain(s) => s.contains("quickPlayMultiplayer"),
                    Arg::Ruled { value, .. } => value.values().iter().any(|v| v.contains("quickPlayMultiplayer")),
                })
            })
            .unwrap_or(false)
    }
}

fn lib_key(name: &str) -> String {
    crate::maven::Coord::parse(name).map(|c| c.key()).unwrap_or_else(|| name.to_string())
}

/// Merge a child profile (Fabric/Forge/...) onto its parent (vanilla).
/// Child values win; argument lists are concatenated; libraries are
/// de-duplicated with the child's copy taking precedence.
pub fn merge(parent: VersionJson, child: VersionJson) -> VersionJson {
    let mut seen = HashSet::new();
    let mut libraries = Vec::new();
    for lib in child.libraries.into_iter().chain(parent.libraries) {
        // Natives-map libraries (LWJGL 2) share a key with their jar entry.
        let key = if lib.natives.is_some() { format!("{}#natives", lib_key(&lib.name)) } else { lib_key(&lib.name) };
        if seen.insert(key) {
            libraries.push(lib);
        }
    }

    let arguments = match (parent.arguments, child.arguments) {
        (Some(mut p), Some(c)) => {
            p.game.extend(c.game);
            p.jvm.extend(c.jvm);
            Some(p)
        }
        (p, c) => c.or(p),
    };

    VersionJson {
        id: child.id,
        inherits_from: None,
        jar: child.jar.or(parent.jar),
        kind: child.kind.or(parent.kind),
        main_class: child.main_class.or(parent.main_class),
        minecraft_arguments: child.minecraft_arguments.or(parent.minecraft_arguments),
        arguments,
        libraries,
        asset_index: child.asset_index.or(parent.asset_index),
        assets: child.assets.or(parent.assets),
        downloads: child.downloads.or(parent.downloads),
        java_version: child.java_version.or(parent.java_version),
        logging: child.logging.or(parent.logging),
        release_time: parent.release_time.or(child.release_time),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub const VANILLA: &str = r#"{
      "id": "1.20.1", "type": "release", "mainClass": "net.minecraft.client.main.Main",
      "javaVersion": {"component": "java-runtime-gamma", "majorVersion": 17},
      "assetIndex": {"id": "5", "sha1": "abc", "size": 1, "totalSize": 2, "url": "https://x/5.json"},
      "assets": "5",
      "downloads": {"client": {"sha1": "c", "size": 3, "url": "https://x/client.jar"}},
      "arguments": {
        "game": ["--username", "${auth_player_name}",
          {"rules": [{"action": "allow", "features": {"has_custom_resolution": true}}], "value": ["--width", "${resolution_width}", "--height", "${resolution_height}"]},
          {"rules": [{"action": "allow", "features": {"is_quick_play_multiplayer": true}}], "value": ["--quickPlayMultiplayer", "${quickPlayMultiplayer}"]}],
        "jvm": [{"rules": [{"action": "allow", "os": {"name": "osx"}}], "value": ["-XstartOnFirstThread"]}, "-cp", "${classpath}"]
      },
      "libraries": [
        {"name": "org.ow2.asm:asm:9.3", "downloads": {"artifact": {"path": "org/ow2/asm/asm/9.3/asm-9.3.jar", "sha1": "a", "size": 1, "url": "https://libraries.minecraft.net/org/ow2/asm/asm/9.3/asm-9.3.jar"}}},
        {"name": "org.lwjgl:lwjgl:3.3.1:natives-windows", "downloads": {"artifact": {"path": "org/lwjgl/lwjgl/3.3.1/lwjgl-3.3.1-natives-windows.jar", "sha1": "n", "size": 1, "url": "https://libraries.minecraft.net/n.jar"}}, "rules": [{"action": "allow", "os": {"name": "windows"}}]}
      ]
    }"#;

    const FABRIC: &str = r#"{
      "id": "fabric-loader-0.16.9-1.20.1", "inheritsFrom": "1.20.1", "mainClass": "net.fabricmc.loader.impl.launch.knot.KnotClient",
      "arguments": {"game": [], "jvm": ["-DFabricMcEmu= net.minecraft.client.main.Main "]},
      "libraries": [
        {"name": "org.ow2.asm:asm:9.7.1", "url": "https://maven.fabricmc.net/", "sha1": "f"},
        {"name": "net.fabricmc:fabric-loader:0.16.9", "url": "https://maven.fabricmc.net/"}
      ]
    }"#;

    #[test]
    fn merges_fabric_onto_vanilla() {
        let parent: VersionJson = serde_json::from_str(VANILLA).unwrap();
        let child: VersionJson = serde_json::from_str(FABRIC).unwrap();
        assert!(parent.supports_quick_play());
        let merged = merge(parent, child);
        assert_eq!(merged.id, "fabric-loader-0.16.9-1.20.1");
        assert_eq!(merged.main_class.as_deref(), Some("net.fabricmc.loader.impl.launch.knot.KnotClient"));
        assert_eq!(merged.java_major(), 17);
        // asm de-duplicated, Fabric's newer copy wins
        let asm: Vec<_> = merged.libraries.iter().filter(|l| l.name.starts_with("org.ow2.asm:asm:")).collect();
        assert_eq!(asm.len(), 1);
        assert_eq!(asm[0].name, "org.ow2.asm:asm:9.7.1");
        assert_eq!(merged.libraries.len(), 3);
        let args = merged.arguments.unwrap();
        assert_eq!(args.jvm.len(), 4);
        assert!(merged.asset_index.is_some());
    }
}
