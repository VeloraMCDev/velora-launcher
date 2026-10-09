//! Forge / NeoForge. Their installers ship a version JSON plus a list of
//! "processors" (small Java programs that patch the game jar). We replay
//! exactly what the official installer does, headlessly.

use crate::http::{self, Download};
use crate::libraries;
use crate::maven::Coord;
use crate::meta::forge_installer_url;
use crate::paths::Layout;
use crate::progress::{self, Event, Reporter, Stage};
use crate::rules::Env;
use crate::version::{Library, VersionJson};
use anyhow::{anyhow, bail, Context, Result};
use velora_shared::Loader;
use serde::Deserialize;
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use tokio::io::{AsyncBufReadExt, BufReader};

pub struct ForgeInstall<'a> {
    pub client: &'a reqwest::Client,
    pub layout: &'a Layout,
    pub loader: Loader,
    pub mc: &'a str,
    /// Maven version, e.g. `1.20.1-47.2.0` (Forge) or `21.1.77` (NeoForge).
    pub version: &'a str,
    /// Console Java executable used to run processors.
    pub java: &'a Path,
    pub client_jar: &'a Path,
    pub concurrency: usize,
    pub reporter: &'a Reporter,
}

#[derive(Deserialize)]
struct InstallProfile {
    #[serde(default = "default_json")]
    json: String,
    #[serde(default)]
    data: HashMap<String, DataEntry>,
    #[serde(default)]
    processors: Vec<Processor>,
    #[serde(default)]
    libraries: Vec<Library>,
}

fn default_json() -> String {
    "/version.json".into()
}

#[derive(Deserialize)]
struct DataEntry {
    client: String,
}

#[derive(Deserialize)]
struct Processor {
    #[serde(default)]
    sides: Option<Vec<String>>,
    jar: String,
    #[serde(default)]
    classpath: Vec<String>,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    outputs: HashMap<String, String>,
}

type Zip = zip::ZipArchive<std::fs::File>;

fn read_entry(zip: &mut Zip, name: &str) -> Result<Vec<u8>> {
    let mut entry = zip.by_name(name.trim_start_matches('/')).with_context(|| format!("installer is missing {name}"))?;
    let mut buf = Vec::with_capacity(entry.size() as usize);
    entry.read_to_end(&mut buf)?;
    Ok(buf)
}

fn extract_entry(zip: &mut Zip, name: &str, dest: &Path) -> Result<bool> {
    let Ok(mut entry) = zip.by_name(name.trim_start_matches('/')) else { return Ok(false) };
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut buf = Vec::with_capacity(entry.size() as usize);
    entry.read_to_end(&mut buf)?;
    std::fs::write(dest, buf)?;
    Ok(true)
}

fn marker_path(layout: &Layout, id: &str) -> PathBuf {
    layout.version_dir(id).join(".scopenet-installed")
}

pub async fn install(ctx: ForgeInstall<'_>) -> Result<VersionJson> {
    let label = if ctx.loader == Loader::NeoForge { "NeoForge" } else { "Forge" };
    progress::stage(ctx.reporter, Stage::Loader, format!("Installing {label} {}", ctx.version));

    let installer = ctx.layout.cache().join("installers").join(format!("{}-{}-installer.jar", ctx.loader.as_str(), ctx.version));
    if !installer.exists() {
        let url = forge_installer_url(ctx.loader, ctx.version);
        http::download_one(ctx.client, &Download::new(url, installer.clone(), None, None), &|_| {}).await?;
    }
    let mut zip = zip::ZipArchive::new(std::fs::File::open(&installer)?).context("opening installer")?;
    let profile_raw: serde_json::Value = serde_json::from_slice(&read_entry(&mut zip, "install_profile.json")?)?;

    if profile_raw.get("versionInfo").is_some() {
        return install_legacy(&ctx, &mut zip, profile_raw);
    }

    let profile: InstallProfile = serde_json::from_value(profile_raw)?;
    let mut version: VersionJson = serde_json::from_slice(&read_entry(&mut zip, &profile.json)?)?;
    let libs_dir = ctx.layout.libraries();

    // Files bundled in the installer's maven/ folder.
    let names: Vec<String> = zip.file_names().filter(|n| n.starts_with("maven/") && !n.ends_with('/')).map(String::from).collect();
    for name in names {
        let dest = libs_dir.join(name.trim_start_matches("maven/"));
        if !dest.exists() {
            extract_entry(&mut zip, &name, &dest)?;
        }
    }

    // Libraries needed by the processors and by the game.
    let env = Env::current();
    let mut all_libs = libraries::resolve(&profile.libraries, &env);
    all_libs.extend(libraries::resolve(&version.libraries, &env));
    let mut downloads = Vec::new();
    for lib in &all_libs {
        let file = lib.file(&libs_dir);
        if http::file_ok(&file, lib.sha1.as_deref(), lib.size, false) {
            continue;
        }
        match lib.download(&libs_dir) {
            Some(dl) => downloads.push(dl),
            None => {
                extract_entry(&mut zip, &format!("maven/{}", lib.path), &file)?;
            }
        }
    }
    downloads.dedup_by(|a, b| a.dest == b.dest);
    http::download_all(ctx.client, downloads, ctx.concurrency, Stage::Loader, ctx.reporter).await?;

    let marker = marker_path(ctx.layout, &version.id);
    if std::fs::read_to_string(&marker).ok().as_deref() != Some(ctx.version) {
        run_processors(&ctx, &mut zip, &profile, &installer).await?;
        std::fs::create_dir_all(marker.parent().unwrap())?;
        std::fs::write(&marker, ctx.version)?;
    }

    version.inherits_from = Some(ctx.mc.to_string());
    save_version(ctx.layout, &version)?;
    Ok(version)
}

fn save_version(layout: &Layout, version: &VersionJson) -> Result<()> {
    let path = layout.version_json(&version.id);
    std::fs::create_dir_all(path.parent().unwrap())?;
    std::fs::write(path, serde_json::to_vec_pretty(version)?)?;
    Ok(())
}

/// Forge ≤ 1.12: `install_profile.json` holds `install` + `versionInfo`.
fn install_legacy(ctx: &ForgeInstall<'_>, zip: &mut Zip, raw: serde_json::Value) -> Result<VersionJson> {
    let info = raw.get("versionInfo").cloned().unwrap_or_default();
    // Old profiles point at the long-dead files.minecraftforge.net/maven.
    let fixed = serde_json::to_string(&info)?
        .replace("http://files.minecraftforge.net/maven/", "https://maven.minecraftforge.net/")
        .replace("https://files.minecraftforge.net/maven/", "https://maven.minecraftforge.net/");
    let mut version: VersionJson = serde_json::from_str(&fixed)?;
    let install = raw.get("install").ok_or_else(|| anyhow!("legacy installer missing 'install'"))?;
    let coord = install.get("path").and_then(|v| v.as_str()).and_then(Coord::parse).ok_or_else(|| anyhow!("bad install.path"))?;
    let file_path = install.get("filePath").and_then(|v| v.as_str()).ok_or_else(|| anyhow!("bad install.filePath"))?;
    let dest = ctx.layout.libraries().join(coord.path());
    if !dest.exists() {
        extract_entry(zip, file_path, &dest)?;
    }
    version.inherits_from = Some(ctx.mc.to_string());
    save_version(ctx.layout, &version)?;
    Ok(version)
}

fn resolve_data_value(value: &str, libs_dir: &Path, tmp: &Path, zip: &mut Zip) -> Result<String> {
    if value.starts_with('[') && value.ends_with(']') {
        let coord = Coord::parse(value).ok_or_else(|| anyhow!("bad coordinate {value}"))?;
        return Ok(libs_dir.join(coord.path()).to_string_lossy().into_owned());
    }
    if value.len() >= 2 && value.starts_with('\'') && value.ends_with('\'') {
        return Ok(value[1..value.len() - 1].to_string());
    }
    if value.starts_with('/') {
        let dest = tmp.join(value.trim_start_matches('/'));
        extract_entry(zip, value, &dest)?;
        return Ok(dest.to_string_lossy().into_owned());
    }
    Ok(value.to_string())
}

fn resolve_arg(arg: &str, data: &HashMap<String, String>, libs_dir: &Path) -> Result<String> {
    if arg.starts_with('[') && arg.ends_with(']') {
        let coord = Coord::parse(arg).ok_or_else(|| anyhow!("bad coordinate {arg}"))?;
        return Ok(libs_dir.join(coord.path()).to_string_lossy().into_owned());
    }
    if arg.starts_with('{') && arg.ends_with('}') {
        let key = &arg[1..arg.len() - 1];
        return data.get(key).cloned().ok_or_else(|| anyhow!("installer references unknown data key {key}"));
    }
    if arg.len() >= 2 && arg.starts_with('\'') && arg.ends_with('\'') {
        return Ok(arg[1..arg.len() - 1].to_string());
    }
    let mut out = arg.to_string();
    for (k, v) in data {
        let needle = format!("{{{k}}}");
        if out.contains(&needle) {
            out = out.replace(&needle, v);
        }
    }
    Ok(out)
}

pub fn jar_main_class(jar: &Path) -> Result<String> {
    let mut zip = zip::ZipArchive::new(std::fs::File::open(jar).with_context(|| format!("opening {}", jar.display()))?)?;
    let manifest = String::from_utf8_lossy(&read_entry(&mut zip, "META-INF/MANIFEST.MF")?).into_owned();
    parse_main_class(&manifest).ok_or_else(|| anyhow!("{} has no Main-Class", jar.display()))
}

fn parse_main_class(manifest: &str) -> Option<String> {
    // Manifest lines wrap at 72 bytes; continuation lines start with a space.
    let mut unwrapped = String::new();
    for line in manifest.lines() {
        if let Some(rest) = line.strip_prefix(' ') {
            unwrapped.push_str(rest);
        } else {
            unwrapped.push('\n');
            unwrapped.push_str(line.trim_end_matches('\r'));
        }
    }
    unwrapped.lines().find_map(|l| l.strip_prefix("Main-Class:").map(|v| v.trim().to_string()))
}

async fn run_processors(ctx: &ForgeInstall<'_>, zip: &mut Zip, profile: &InstallProfile, installer: &Path) -> Result<()> {
    let libs_dir = ctx.layout.libraries();
    let tmp = ctx.layout.cache().join("forge-tmp").join(ctx.version);
    std::fs::create_dir_all(&tmp)?;

    let mut data = HashMap::new();
    for (key, entry) in &profile.data {
        data.insert(key.clone(), resolve_data_value(&entry.client, &libs_dir, &tmp, zip)?);
    }
    data.insert("SIDE".into(), "client".into());
    data.insert("MINECRAFT_JAR".into(), ctx.client_jar.to_string_lossy().into_owned());
    data.insert("MINECRAFT_VERSION".into(), ctx.mc.to_string());
    data.insert("ROOT".into(), ctx.layout.root.to_string_lossy().into_owned());
    data.insert("INSTALLER".into(), installer.to_string_lossy().into_owned());
    data.insert("LIBRARY_DIR".into(), libs_dir.to_string_lossy().into_owned());

    let sep = if cfg!(windows) { ";" } else { ":" };
    let processors: Vec<&Processor> =
        profile.processors.iter().filter(|p| p.sides.as_ref().map(|s| s.iter().any(|x| x == "client")).unwrap_or(true)).collect();
    let total = processors.len() as u32;

    for (i, proc) in processors.into_iter().enumerate() {
        ctx.reporter.as_ref()(Event::Progress {
            stage: Stage::Loader,
            done: i as u64,
            total: total as u64,
            files_done: i as u32,
            files_total: total,
        });

        // Skip processors whose outputs already exist with the right hash.
        if !proc.outputs.is_empty() {
            let mut all_ok = true;
            for (k, v) in &proc.outputs {
                let path = resolve_arg(k, &data, &libs_dir)?;
                let sha = resolve_arg(v, &data, &libs_dir)?;
                if !http::file_ok(Path::new(&path), Some(&sha), None, true) {
                    all_ok = false;
                    break;
                }
            }
            if all_ok {
                continue;
            }
        }

        let jar = libs_dir.join(Coord::parse(&proc.jar).ok_or_else(|| anyhow!("bad processor jar {}", proc.jar))?.path());
        let main_class = jar_main_class(&jar)?;
        let mut cp = vec![jar.to_string_lossy().into_owned()];
        for c in &proc.classpath {
            let coord = Coord::parse(c).ok_or_else(|| anyhow!("bad classpath entry {c}"))?;
            cp.push(libs_dir.join(coord.path()).to_string_lossy().into_owned());
        }
        let args = proc.args.iter().map(|a| resolve_arg(a, &data, &libs_dir)).collect::<Result<Vec<_>>>()?;

        let mut cmd = tokio::process::Command::new(ctx.java);
        cmd.arg("-cp").arg(cp.join(sep)).arg(&main_class).args(&args);
        cmd.current_dir(&tmp);
        cmd.stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).kill_on_drop(true);
        #[cfg(windows)]
        cmd.creation_flags(0x0800_0000);
        let mut child = cmd.spawn().with_context(|| format!("starting processor {main_class}"))?;

        let tail = std::sync::Arc::new(std::sync::Mutex::new(std::collections::VecDeque::<String>::new()));
        let pump = |stream: Option<Box<dyn tokio::io::AsyncRead + Unpin + Send>>| {
            let reporter = ctx.reporter.clone();
            let tail = tail.clone();
            async move {
                let Some(stream) = stream else { return };
                let mut lines = BufReader::new(stream).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    {
                        let mut t = tail.lock().unwrap();
                        t.push_back(line.clone());
                        if t.len() > 30 {
                            t.pop_front();
                        }
                    }
                    reporter(Event::Log { line });
                }
            }
        };
        let out = child.stdout.take().map(|s| Box::new(s) as Box<dyn tokio::io::AsyncRead + Unpin + Send>);
        let err = child.stderr.take().map(|s| Box::new(s) as Box<dyn tokio::io::AsyncRead + Unpin + Send>);
        let (status, _, _) = tokio::join!(child.wait(), pump(out), pump(err));
        let status = status?;
        if !status.success() {
            let tail = tail.lock().unwrap().iter().cloned().collect::<Vec<_>>().join("\n");
            bail!("{main_class} failed ({status}):\n{tail}");
        }
    }
    std::fs::remove_dir_all(&tmp).ok();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_main_class_with_wrapping() {
        let mf = "Manifest-Version: 1.0\r\nMain-Class: net.minecraftforge.binarypatcher.Consol\r\n eTool\r\nOther: x\r\n";
        assert_eq!(parse_main_class(mf).as_deref(), Some("net.minecraftforge.binarypatcher.ConsoleTool"));
    }

    #[test]
    fn resolves_processor_args() {
        let libs = Path::new("/libs");
        let mut data = HashMap::new();
        data.insert("MC_SLIM".to_string(), "/libs/slim.jar".to_string());
        assert_eq!(resolve_arg("{MC_SLIM}", &data, libs).unwrap(), "/libs/slim.jar");
        assert_eq!(resolve_arg("--flag", &data, libs).unwrap(), "--flag");
        assert_eq!(
            resolve_arg("[net.minecraft:client:1.20.1:srg]", &data, libs).unwrap(),
            Path::new("/libs").join("net/minecraft/client/1.20.1/client-1.20.1-srg.jar").to_string_lossy()
        );
        assert!(resolve_arg("{MISSING}", &data, libs).is_err());
    }
}
