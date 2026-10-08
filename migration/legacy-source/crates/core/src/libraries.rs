//! Turning version-JSON libraries into files on disk + a classpath.

use crate::http::Download;
use crate::maven::Coord;
use crate::rules::{self, Env};
use crate::version::Library;
use anyhow::Result;
use std::io::Read;
use std::path::{Path, PathBuf};

pub const MOJANG_LIBRARIES: &str = "https://libraries.minecraft.net/";

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedLib {
    pub name: String,
    /// Path relative to the libraries dir (`/`-separated).
    pub path: String,
    /// `None` when the file must come from somewhere else (e.g. a Forge installer).
    pub url: Option<String>,
    pub sha1: Option<String>,
    pub size: Option<u64>,
    /// Goes on the classpath.
    pub classpath: bool,
    /// Contains native libraries that must be extracted.
    pub native: bool,
    pub exclude: Vec<String>,
}

impl ResolvedLib {
    pub fn file(&self, libraries_dir: &Path) -> PathBuf {
        libraries_dir.join(&self.path)
    }

    pub fn download(&self, libraries_dir: &Path) -> Option<Download> {
        self.url.as_ref().map(|url| Download::new(url.clone(), self.file(libraries_dir), self.sha1.clone(), self.size))
    }
}

fn join_url(base: &str, path: &str) -> String {
    if base.ends_with('/') {
        format!("{base}{path}")
    } else {
        format!("{base}/{path}")
    }
}

pub fn resolve(libs: &[Library], env: &Env) -> Vec<ResolvedLib> {
    let mut out = Vec::new();
    for lib in libs {
        if let Some(rules) = &lib.rules {
            if !rules::allowed(rules, env) {
                continue;
            }
        }
        let Some(coord) = Coord::parse(&lib.name) else { continue };
        let exclude = lib.extract.as_ref().map(|e| e.exclude.clone()).unwrap_or_default();

        // Legacy natives (LWJGL 2 / pre-1.19): a separate classifier jar
        // that is extracted but never put on the classpath.
        if let Some(natives) = &lib.natives {
            if let Some(classifier) = natives.get(env.os) {
                let classifier = classifier.replace("${arch}", env.bits());
                let from_downloads = lib.downloads.as_ref().and_then(|d| d.classifiers.as_ref()).and_then(|c| c.get(&classifier));
                let native_coord = coord.with_classifier(&classifier);
                let (path, url, sha1, size) = match from_downloads {
                    Some(a) => (
                        a.path.clone().unwrap_or_else(|| native_coord.path()),
                        Some(a.url.clone()).filter(|u| !u.is_empty()),
                        a.sha1.clone(),
                        a.size,
                    ),
                    None => {
                        let p = native_coord.path();
                        let base = lib.url.as_deref().unwrap_or(MOJANG_LIBRARIES);
                        (p.clone(), Some(join_url(base, &p)), None, None)
                    }
                };
                out.push(ResolvedLib {
                    name: format!("{}:{}", lib.name, classifier),
                    path,
                    url,
                    sha1,
                    size,
                    classpath: false,
                    native: true,
                    exclude: exclude.clone(),
                });
            }
            // A natives-map library only has a main artifact if downloads.artifact exists.
            let Some(artifact) = lib.downloads.as_ref().and_then(|d| d.artifact.as_ref()) else { continue };
            out.push(ResolvedLib {
                name: lib.name.clone(),
                path: artifact.path.clone().unwrap_or_else(|| coord.path()),
                url: Some(artifact.url.clone()).filter(|u| !u.is_empty()),
                sha1: artifact.sha1.clone(),
                size: artifact.size,
                classpath: true,
                native: false,
                exclude,
            });
            continue;
        }

        let is_native_jar = coord.classifier.as_deref().is_some_and(|c| c.starts_with("natives-"));
        let resolved = match lib.downloads.as_ref().and_then(|d| d.artifact.as_ref()) {
            Some(a) => ResolvedLib {
                name: lib.name.clone(),
                path: a.path.clone().filter(|p| !p.is_empty()).unwrap_or_else(|| coord.path()),
                url: Some(a.url.clone()).filter(|u| !u.is_empty()),
                sha1: a.sha1.clone(),
                size: a.size,
                classpath: true,
                native: is_native_jar,
                exclude,
            },
            None => {
                if lib.downloads.is_some() {
                    // `downloads` present but no artifact: classifier-only entry we don't need.
                    continue;
                }
                let p = coord.path();
                let base = lib.url.as_deref().unwrap_or(MOJANG_LIBRARIES);
                ResolvedLib {
                    name: lib.name.clone(),
                    path: p.clone(),
                    url: Some(join_url(base, &p)),
                    sha1: lib.sha1.clone(),
                    size: lib.size,
                    classpath: true,
                    native: is_native_jar,
                    exclude,
                }
            }
        };
        out.push(resolved);
    }
    out
}

fn is_native_file(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.ends_with(".dll") || n.ends_with(".so") || n.ends_with(".dylib") || n.ends_with(".jnilib")
}

/// Extract native libraries (flattened) into `dest`.
pub fn extract_natives(libs: &[ResolvedLib], libraries_dir: &Path, dest: &Path) -> Result<()> {
    std::fs::create_dir_all(dest)?;
    for lib in libs.iter().filter(|l| l.native) {
        let path = lib.file(libraries_dir);
        let Ok(file) = std::fs::File::open(&path) else {
            tracing::warn!("missing native jar {}", path.display());
            continue;
        };
        let mut zip = zip::ZipArchive::new(file)?;
        for i in 0..zip.len() {
            let mut entry = zip.by_index(i)?;
            if entry.is_dir() {
                continue;
            }
            let name = entry.name().to_string();
            if name.starts_with("META-INF") || lib.exclude.iter().any(|e| name.starts_with(e.as_str())) {
                continue;
            }
            if !is_native_file(&name) {
                continue;
            }
            let file_name = name.rsplit('/').next().unwrap_or(&name);
            // Skip natives for other architectures (LWJGL 3.3 ships several).
            let lower = name.to_ascii_lowercase();
            if cfg!(target_arch = "x86_64") && (lower.contains("arm64") || lower.contains("aarch64") || lower.contains("/x86/")) {
                continue;
            }
            let target = dest.join(file_name);
            if target.exists() && std::fs::metadata(&target).map(|m| m.len() == entry.size()).unwrap_or(false) {
                continue;
            }
            let mut buf = Vec::with_capacity(entry.size() as usize);
            entry.read_to_end(&mut buf)?;
            std::fs::write(&target, buf)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::version::VersionJson;
    use std::collections::HashMap;

    #[test]
    fn resolves_legacy_natives_and_maven_libs() {
        let v: VersionJson = serde_json::from_str(
            r#"{"id":"x","libraries":[
              {"name":"org.lwjgl.lwjgl:lwjgl-platform:2.9.4-nightly-20150209",
               "natives":{"linux":"natives-linux","windows":"natives-windows-${arch}"},
               "extract":{"exclude":["META-INF/"]},
               "downloads":{"classifiers":{"natives-windows-64":{"path":"p/w64.jar","sha1":"s","size":5,"url":"https://l/w64.jar"}}}},
              {"name":"net.fabricmc:intermediary:1.20.1","url":"https://maven.fabricmc.net/"},
              {"name":"com.mojang:only-mac:1","rules":[{"action":"allow","os":{"name":"osx"}}]},
              {"name":"net.minecraftforge:forge:1.20.1-47.2.0:universal","downloads":{"artifact":{"path":"net/minecraftforge/forge/1.20.1-47.2.0/forge-1.20.1-47.2.0-universal.jar","url":"","sha1":"u","size":9}}}
            ]}"#,
        )
        .unwrap();
        let env = Env { os: "windows", arch: "x86_64", features: HashMap::new() };
        let libs = resolve(&v.libraries, &env);
        assert_eq!(libs.len(), 3);
        assert!(libs[0].native && !libs[0].classpath);
        assert_eq!(libs[0].path, "p/w64.jar");
        assert_eq!(libs[1].url.as_deref(), Some("https://maven.fabricmc.net/net/fabricmc/intermediary/1.20.1/intermediary-1.20.1.jar"));
        assert!(libs[2].url.is_none(), "empty url = bundled in installer");
    }
}
