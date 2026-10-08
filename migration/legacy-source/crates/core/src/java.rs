//! Automatic Java: Mojang publishes the exact runtimes its launcher uses.
//! We download the one each Minecraft version asks for, so players never
//! have to install Java themselves.

use crate::http::{self, Download};
use crate::paths::Layout;
use crate::progress::{self, Reporter, Stage};
use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub const RUNTIME_INDEX: &str =
    "https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json";

#[derive(Deserialize)]
struct RuntimeEntry {
    manifest: ManifestRef,
    version: RuntimeVersion,
}
#[derive(Deserialize)]
struct ManifestRef {
    sha1: String,
    url: String,
}
#[derive(Deserialize)]
struct RuntimeVersion {
    name: String,
}

#[derive(Deserialize)]
struct RuntimeManifest {
    files: HashMap<String, RuntimeFile>,
}

#[derive(Deserialize)]
struct RuntimeFile {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    executable: bool,
    #[serde(default)]
    downloads: Option<RuntimeDownloads>,
    #[serde(default)]
    target: Option<String>,
}
#[derive(Deserialize)]
struct RuntimeDownloads {
    raw: RawDownload,
}
#[derive(Deserialize)]
struct RawDownload {
    sha1: String,
    size: u64,
    url: String,
}

pub fn platform_key() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86") => "windows-x86",
        ("windows", "aarch64") => "windows-arm64",
        ("windows", _) => "windows-x64",
        ("macos", "aarch64") => "mac-os-arm64",
        ("macos", _) => "mac-os",
        ("linux", "x86") => "linux-i386",
        _ => "linux",
    }
}

/// Path to the Java executable inside a runtime directory.
/// `windowed` picks `javaw.exe` on Windows so no console window appears.
pub fn java_exe(dir: &Path, windowed: bool) -> PathBuf {
    if cfg!(windows) {
        dir.join("bin").join(if windowed { "javaw.exe" } else { "java.exe" })
    } else if cfg!(target_os = "macos") {
        dir.join("jre.bundle/Contents/Home/bin/java")
    } else {
        dir.join("bin/java")
    }
}

/// For a user-supplied path, derive the console/windowed sibling.
pub fn sibling_exe(java: &Path, windowed: bool) -> PathBuf {
    if !cfg!(windows) {
        return java.to_path_buf();
    }
    let name = if windowed { "javaw.exe" } else { "java.exe" };
    let candidate = java.with_file_name(name);
    if candidate.exists() {
        candidate
    } else {
        java.to_path_buf()
    }
}

/// Make sure the Mojang runtime `component` (e.g. `java-runtime-delta`) is
/// installed. Returns its directory.
pub async fn ensure_runtime(
    client: &reqwest::Client,
    layout: &Layout,
    component: &str,
    concurrency: usize,
    reporter: &Reporter,
) -> Result<PathBuf> {
    let dir = layout.runtimes().join(component);
    let marker = dir.join(".scopenet-runtime");
    let installed = std::fs::read_to_string(&marker).ok();

    let index: HashMap<String, HashMap<String, Vec<RuntimeEntry>>> = match http::get_json(client, RUNTIME_INDEX).await {
        Ok(i) => i,
        Err(e) if installed.is_some() && java_exe(&dir, false).exists() => {
            tracing::warn!("java index unreachable ({e}); using installed {component}");
            return Ok(dir);
        }
        Err(e) => return Err(e).context("fetching Java runtime index"),
    };
    let entry = index
        .get(platform_key())
        .and_then(|p| p.get(component))
        .and_then(|v| v.first())
        .ok_or_else(|| anyhow!("Mojang has no '{component}' Java runtime for {}", platform_key()))?;

    if installed.as_deref() == Some(entry.manifest.sha1.as_str()) && java_exe(&dir, false).exists() {
        return Ok(dir);
    }

    progress::stage(reporter, Stage::Java, format!("Installing Java {}", entry.version.name));
    let manifest: RuntimeManifest = http::get_json(client, &entry.manifest.url).await?;
    let mut downloads = Vec::new();
    let mut executables = Vec::new();
    let mut links = Vec::new();
    for (rel, file) in &manifest.files {
        let Some(path) = crate::paths::safe_join(&dir, rel) else { continue };
        match file.kind.as_str() {
            "directory" => std::fs::create_dir_all(&path)?,
            "file" => {
                let Some(dl) = &file.downloads else { continue };
                if file.executable {
                    executables.push(path.clone());
                }
                if !http::file_ok(&path, Some(&dl.raw.sha1), Some(dl.raw.size), true) {
                    downloads.push(Download::new(dl.raw.url.clone(), path, Some(dl.raw.sha1.clone()), Some(dl.raw.size)));
                }
            }
            "link" => {
                if let Some(target) = &file.target {
                    links.push((path, target.clone()));
                }
            }
            _ => {}
        }
    }
    http::download_all(client, downloads, concurrency, Stage::Java, reporter).await?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for exe in executables {
            if let Ok(meta) = std::fs::metadata(&exe) {
                let mut perms = meta.permissions();
                perms.set_mode(0o755);
                std::fs::set_permissions(&exe, perms).ok();
            }
        }
        for (path, target) in links {
            std::fs::remove_file(&path).ok();
            std::os::unix::fs::symlink(target, &path).ok();
        }
    }
    #[cfg(not(unix))]
    let _ = (executables, links);

    std::fs::write(&marker, &entry.manifest.sha1)?;
    Ok(dir)
}

/// Best-effort `java -version` parse for a custom Java path.
pub async fn detect_major(java: &Path) -> Option<u32> {
    let mut cmd = tokio::process::Command::new(sibling_exe(java, false));
    cmd.arg("-version");
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000);
    let out = cmd.output().await.ok()?;
    let text = String::from_utf8_lossy(&out.stderr).to_string() + &String::from_utf8_lossy(&out.stdout);
    parse_java_version(&text)
}

pub fn parse_java_version(text: &str) -> Option<u32> {
    let start = text.find('"')? + 1;
    let rest = &text[start..];
    let ver = &rest[..rest.find('"')?];
    let mut parts = ver.split(['.', '_', '-', '+']);
    let first: u32 = parts.next()?.parse().ok()?;
    if first == 1 {
        parts.next()?.parse().ok()
    } else {
        Some(first)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_versions() {
        assert_eq!(parse_java_version("java version \"1.8.0_381\""), Some(8));
        assert_eq!(parse_java_version("openjdk version \"21.0.2\" 2024-01-16"), Some(21));
        assert_eq!(parse_java_version("openjdk version \"17\" 2021-09-14"), Some(17));
    }
}
