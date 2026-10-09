//! Neutral landing/download records and release naming helpers. Host composition supplies private blocks and lifecycle.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct HostedDownload {
    pub platform: String, // "windows", "mac", "linux"
    pub label: String,
    pub filename: String,
    pub file_url: String,
    pub size: i64,
    pub uploaded_at: String,
    /// Release version, when known ("1.0.1").
    pub version: String,
    /// What the website shows and the browser saves the file as: short and tidy, whatever the file on disk is called.
    pub display_name: String,
}

impl Default for HostedDownload {
    fn default() -> Self {
        Self {
            platform: "windows".into(),
            label: "Windows".into(),
            filename: "".into(),
            file_url: "".into(),
            size: 0,
            uploaded_at: "".into(),
            version: "".into(),
            display_name: "".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct FaqItem {
    pub id: String,
    pub question: String,
    pub answer: String,
}

impl Default for FaqItem {
    fn default() -> Self {
        Self { id: "".into(), question: "".into(), answer: "".into() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct LandingBlock {
    pub id: String,
    #[serde(rename = "type")]
    pub block_type: String, // "hero", "download", "servers", "leaderboard", "stats", "instances", "news", "faq", "socials"
    pub enabled: bool,
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub options: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct LandingTheme {
    pub background: String,
    pub surface: String,
    pub accent: String,
    pub text: String,
    pub muted: String,
    pub max_width: u16,
    pub radius: u8,
    pub font: String,
    pub footer_text: String,
}

impl Default for LandingTheme {
    fn default() -> Self {
        Self {
            background: "#090a0f".into(),
            surface: "#151620".into(),
            accent: "#6d6af5".into(),
            text: "#f1f2f6".into(),
            muted: "#8b8f9a".into(),
            max_width: 1140,
            radius: 14,
            font: "Inter".into(),
            footer_text: "Your worlds, one click away.".into(),
        }
    }
}

impl Default for LandingBlock {
    fn default() -> Self {
        Self { id: "".into(), block_type: "hero".into(), enabled: true, title: None, subtitle: None, options: serde_json::json!({}) }
    }
}

/// `https://github.com/owner/repo[/releases...]` -> ("owner", "repo").
pub fn github_repo(url: &str) -> Option<(String, String)> {
    let rest = url.trim().strip_prefix("https://github.com/")?;
    let mut parts = rest.split('/').filter(|p| !p.is_empty());
    let (owner, repo) = (parts.next()?, parts.next()?.trim_end_matches(".git"));
    let ok = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
    (ok(owner) && ok(repo)).then(|| (owner.to_string(), repo.to_string()))
}

/// Which platform a release file is for, and what to call it.
pub fn classify_installer(filename: &str) -> Option<(&'static str, &'static str)> {
    let lower = filename.to_ascii_lowercase();
    if lower.ends_with("-setup.exe") || lower.ends_with(".msi") {
        Some(("windows", "Windows"))
    } else if lower.ends_with(".dmg") {
        Some(("mac", "macOS"))
    } else if lower.ends_with(".appimage") {
        Some(("linux", "Linux (AppImage)"))
    } else if lower.ends_with(".deb") {
        Some(("linux", "Linux (.deb)"))
    } else {
        None
    }
}

pub fn find_version(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit()
            && (i == 0
                || !bytes[i - 1].is_ascii_alphanumeric()
                || (matches!(bytes[i - 1], b'v' | b'V') && (i < 2 || !bytes[i - 2].is_ascii_alphanumeric())))
        {
            let mut j = i;
            let mut dots = 0;
            while j < bytes.len()
                && (bytes[j].is_ascii_digit() || (bytes[j] == b'.' && j + 1 < bytes.len() && bytes[j + 1].is_ascii_digit()))
            {
                if bytes[j] == b'.' {
                    dots += 1;
                }
                j += 1;
            }
            if dots >= 2 {
                return Some(text[i..j].to_string());
            }
            i = j.max(i + 1);
        } else {
            i += 1;
        }
    }
    None
}

/// The short, tidy name for a download: `Velora-Setup-1.0.1.exe`, `Velora-1.0.1-macOS.dmg`, `Velora-1.0.1-Linux.AppImage`...
pub fn short_name(brand: &str, platform: &str, filename: &str, label: &str, version: &str) -> String {
    let mut slug: String = brand.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    if slug.is_empty() {
        slug = "Velora".into();
    }
    slug.truncate(24);
    let lower = filename.to_ascii_lowercase();
    let ext = ["tar.gz", "appimage", "exe", "dmg", "pkg", "deb", "rpm", "apk", "ipa", "zip", "msi"]
        .iter()
        .find(|e| lower.ends_with(&format!(".{e}")))
        .map(|e| if *e == "appimage" { "AppImage" } else { e })
        .unwrap_or("");
    let version =
        if version.is_empty() { find_version(filename).or_else(|| find_version(label)).unwrap_or_default() } else { version.to_string() };
    let v = if version.is_empty() { String::new() } else { format!("-{version}") };
    let base = match platform {
        "windows" => format!("{slug}-Setup{v}"),
        "mac" => format!("{slug}{v}-macOS"),
        "linux" => format!("{slug}{v}-Linux"),
        _ => format!("{slug}{v}"),
    };
    if ext.is_empty() {
        base
    } else {
        format!("{base}.{ext}")
    }
}

#[cfg(test)]
mod release_tests {
    use super::*;

    #[test]
    fn downloads_get_short_tidy_names() {
        let hex = "11a905016bae9dca8818b4c67bbdabc936ac58f4b83f933c1b9e7979ee9deb5f-setup.exe";
        assert_eq!(short_name("Velora", "windows", hex, "Windows · 1.0.1", ""), "Velora-Setup-1.0.1.exe");
        assert_eq!(short_name("Velora", "windows", "Velora Launcher_1.0.1_x64-setup.exe", "", ""), "Velora-Setup-1.0.1.exe");
        assert_eq!(short_name("My Server!", "mac", "velora-launcher-1.2.3-macos-universal.dmg", "", ""), "MyServer-1.2.3-macOS.dmg");
        assert_eq!(short_name("Velora", "linux", "x.AppImage", "", "2.0.0"), "Velora-2.0.0-Linux.AppImage");
        assert_eq!(short_name("Velora", "linux", "pack.tar.gz", "", ""), "Velora-Linux.tar.gz");
        assert_eq!(short_name("Velora", "android", "scopenet-player-1.0.1.apk", "", ""), "Velora-1.0.1.apk");
        assert_eq!(short_name("", "ios", "a.ipa", "", "1.0.0-beta.1"), "Velora-1.0.0-beta.1.ipa");
        assert_eq!(find_version("v1.2.3-rc.1_x64").as_deref(), Some("1.2.3"));
        assert_eq!(find_version("build 12"), None);
    }

    #[test]
    fn finds_the_repo_and_sorts_installers() {
        assert_eq!(github_repo("https://github.com/VeloraMCDev/velora-launcher/releases"), Some(("VeloraMCDev".into(), "velora-launcher".into())));
        assert_eq!(github_repo("https://github.com/a/b.git"), Some(("a".into(), "b".into())));
        assert_eq!(github_repo("https://evil.test/a/b"), None);
        assert_eq!(github_repo("https://github.com/a"), None);
        assert_eq!(classify_installer("Velora Launcher_1.0.0_x64-setup.exe"), Some(("windows", "Windows")));
        assert_eq!(classify_installer("velora-launcher-1.0.0-macos-universal.dmg").map(|c| c.0), Some("mac"));
        assert_eq!(classify_installer("velora-launcher-1.0.0-linux-x64.AppImage").map(|c| c.0), Some("linux"));
        assert_eq!(classify_installer("velora-launcher-1.0.0-linux-x64.deb").map(|c| c.1), Some("Linux (.deb)"));
        assert_eq!(classify_installer("scopenet-paper-1.0.0.jar"), None);
    }
}

#[cfg(test)]
mod boundaries {
    use super::*;
    #[test]
    fn velora_fallback_changes_only_generated_display_names() {
        let original = HostedDownload {
            filename: "saved-artifact.msi".into(),
            file_url: "/downloads/saved-artifact.msi".into(),
            ..Default::default()
        };
        assert_eq!(short_name("", "windows", &original.filename, "", "1.2.3"), "Velora-Setup-1.2.3.msi");
        assert_eq!(short_name("你好", "mac", "old.dmg", "", ""), "Velora-macOS.dmg");
        assert_eq!(original.filename, "saved-artifact.msi");
        assert_eq!(original.file_url, "/downloads/saved-artifact.msi");
        assert_eq!(short_name("Velora", "linux", "old.AppImage", "", ""), "Velora-Linux.AppImage");
    }
    #[test]
    fn opaque_block_options_and_default_wire_fields_stay_compatible() {
        let block: LandingBlock =
            serde_json::from_value(serde_json::json!({"id":"supplied","type":"extension","options":{"opaque":{"custom":true}}})).unwrap();
        assert_eq!(block.block_type, "extension");
        assert!(block.enabled);
        let value = serde_json::to_value(block).unwrap();
        assert_eq!(value["options"]["opaque"]["custom"], true);
        assert_eq!(value["type"], "extension");
        assert_eq!(serde_json::to_value(HostedDownload::default()).unwrap().as_object().unwrap().len(), 8);
    }
    #[test]
    fn unsafe_repo_names_and_unknown_installers_remain_rejected() {
        assert!(github_repo("https://github.com/a/b?token=invalid").is_none());
        assert!(github_repo("http://github.com/a/b").is_none());
        assert_eq!(classify_installer("x.apk"), None);
        assert_eq!(find_version("prefix123"), None);
        assert_eq!(find_version("v1.2.3"), Some("1.2.3".into()));
    }
}
