//! Installs everything needed to launch a version: vanilla game, loader,
//! libraries, assets and Java.

use crate::assets;
use crate::http::{self, Download};
use crate::java;
use crate::libraries::{self, ResolvedLib};
use crate::loaders;
use crate::meta;
use crate::paths::Layout;
use crate::progress::{self, Reporter, Stage};
use crate::rules::Env;
use crate::version::{self, VersionJson};
use anyhow::{anyhow, bail, Context, Result};
use scopenet_shared::Loader;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct InstallSpec {
    pub mc_version: String,
    pub loader: Loader,
    pub loader_version: Option<String>,
    /// User-selected Java executable (skips the automatic runtime).
    pub java_override: Option<PathBuf>,
    pub game_dir: PathBuf,
    pub concurrency: usize,
    /// Re-hash every file instead of trusting sizes (Repair).
    pub deep_verify: bool,
}

#[derive(Debug, Clone)]
pub struct Installed {
    pub version: VersionJson,
    /// Windowed Java (javaw on Windows).
    pub java: PathBuf,
    pub java_major: u32,
    pub client_jar: PathBuf,
    pub natives_dir: PathBuf,
    pub libraries: Vec<ResolvedLib>,
    pub assets_root: PathBuf,
    pub game_assets: PathBuf,
    pub assets_index: String,
    pub logging_arg: Option<String>,
}

impl Installed {
    pub fn classpath(&self, layout: &Layout) -> Vec<PathBuf> {
        let libs_dir = layout.libraries();
        let mut seen = std::collections::HashSet::new();
        let mut cp: Vec<PathBuf> =
            self.libraries.iter().filter(|l| l.classpath).map(|l| l.file(&libs_dir)).filter(|p| seen.insert(p.clone())).collect();
        cp.push(self.client_jar.clone());
        cp
    }
}

/// Vanilla version JSON: from Mojang (cached), falling back to disk.
async fn vanilla_json(client: &reqwest::Client, layout: &Layout, mc: &str) -> Result<VersionJson> {
    let path = layout.version_json(mc);
    let manifest = match meta::mojang_manifest(client).await {
        Ok(m) => m,
        Err(e) => {
            let bytes = std::fs::read(&path).map_err(|_| anyhow!("can't reach Mojang ({e}) and {mc} isn't installed yet"))?;
            return Ok(serde_json::from_slice(&bytes)?);
        }
    };
    let entry =
        manifest.versions.iter().find(|v| v.id == mc).ok_or_else(|| anyhow!("Minecraft {mc} doesn't exist in Mojang's version list"))?;
    if !http::file_ok(&path, entry.sha1.as_deref(), None, true) {
        http::download_one(client, &Download::new(entry.url.clone(), path.clone(), entry.sha1.clone(), None), &|_| {}).await?;
    }
    Ok(serde_json::from_slice(&std::fs::read(&path)?)?)
}

fn library_downloads(libs: &[ResolvedLib], layout: &Layout, deep: bool) -> Vec<Download> {
    let dir = layout.libraries();
    let mut out: Vec<Download> =
        libs.iter().filter(|l| !http::file_ok(&l.file(&dir), l.sha1.as_deref(), l.size, deep)).filter_map(|l| l.download(&dir)).collect();
    out.sort_by(|a, b| a.dest.cmp(&b.dest));
    out.dedup_by(|a, b| a.dest == b.dest);
    out
}

pub async fn install(client: &reqwest::Client, layout: &Layout, spec: &InstallSpec, reporter: &Reporter) -> Result<Installed> {
    let env = Env::current();
    let mc = spec.mc_version.as_str();
    progress::stage(reporter, Stage::Preparing, format!("Preparing Minecraft {mc}"));

    // 1. Vanilla metadata ----------------------------------------------------
    let vanilla = vanilla_json(client, layout, mc).await?;

    // 2. Java ----------------------------------------------------------------
    let (java_dir, java_major) = match &spec.java_override {
        Some(p) if p.exists() => (None, java::detect_major(p).await.unwrap_or(vanilla.java_major())),
        Some(p) => bail!("the Java path in your settings doesn't exist: {}", p.display()),
        None => {
            let component = vanilla.java_component();
            progress::stage(reporter, Stage::Java, "Checking Java");
            (Some(java::ensure_runtime(client, layout, &component, spec.concurrency, reporter).await?), vanilla.java_major())
        }
    };
    let (java_windowed, java_console) = match (&java_dir, &spec.java_override) {
        (Some(dir), _) => (java::java_exe(dir, true), java::java_exe(dir, false)),
        (None, Some(p)) => (java::sibling_exe(p, true), java::sibling_exe(p, false)),
        _ => unreachable!(),
    };

    // 3. Vanilla files (client jar, libraries, assets, log config) ---------
    progress::stage(reporter, Stage::Game, format!("Downloading Minecraft {mc}"));
    let vanilla_client = layout.version_jar(mc);
    let mut downloads = Vec::new();
    if let Some(client_dl) = vanilla.downloads.as_ref().and_then(|d| d.client.as_ref()) {
        if !http::file_ok(&vanilla_client, client_dl.sha1.as_deref(), client_dl.size, spec.deep_verify) {
            downloads.push(Download::new(client_dl.url.clone(), vanilla_client.clone(), client_dl.sha1.clone(), client_dl.size));
        }
    }
    downloads.extend(library_downloads(&libraries::resolve(&vanilla.libraries, &env), layout, spec.deep_verify));

    let asset_ref = vanilla.asset_index.clone().ok_or_else(|| anyhow!("version {mc} has no asset index"))?;
    let asset_index = assets::load_index(client, layout, &asset_ref).await?;
    downloads.extend(assets::missing_objects(layout, &asset_index, spec.deep_verify));

    let mut logging_arg = None;
    if let Some(log) = vanilla.logging.as_ref().and_then(|l| l.client.as_ref()) {
        let path = layout.assets().join("log_configs").join(&log.file.id);
        if !http::file_ok(&path, log.file.sha1.as_deref(), log.file.size, spec.deep_verify) {
            downloads.push(Download::new(log.file.url.clone(), path.clone(), log.file.sha1.clone(), log.file.size));
        }
        logging_arg = Some(log.argument.replace("${path}", &path.to_string_lossy()));
    }
    http::download_all(client, downloads, spec.concurrency, Stage::Game, reporter).await?;

    // 4. Loader --------------------------------------------------------------
    let child: Option<VersionJson> = match spec.loader {
        Loader::Vanilla => None,
        Loader::Fabric | Loader::Quilt => {
            let lv = meta::resolve_loader_version(client, spec.loader, mc, spec.loader_version.as_deref()).await?.unwrap();
            progress::stage(
                reporter,
                Stage::Loader,
                format!("Installing {} {lv}", if spec.loader == Loader::Fabric { "Fabric" } else { "Quilt" }),
            );
            Some(loaders::fabric::profile(client, layout, spec.loader, mc, &lv).await?)
        }
        Loader::Forge | Loader::NeoForge => {
            let lv = meta::resolve_loader_version(client, spec.loader, mc, spec.loader_version.as_deref()).await?.unwrap();
            Some(
                loaders::forge::install(loaders::forge::ForgeInstall {
                    client,
                    layout,
                    loader: spec.loader,
                    mc,
                    version: &lv,
                    java: &java_console,
                    client_jar: &vanilla_client,
                    concurrency: spec.concurrency,
                    reporter,
                })
                .await
                .with_context(|| format!("installing {} {lv}", spec.loader.as_str()))?,
            )
        }
    };
    let version = match child {
        Some(child) => version::merge(vanilla, child),
        None => vanilla,
    };

    // 5. Remaining (loader) libraries -----------------------------------------
    let libs = libraries::resolve(&version.libraries, &env);
    let lib_downloads = library_downloads(&libs, layout, false);
    if !lib_downloads.is_empty() {
        progress::stage(reporter, Stage::Loader, "Downloading loader libraries");
        http::download_all(client, lib_downloads, spec.concurrency, Stage::Loader, reporter).await?;
    }
    let libs_dir = layout.libraries();
    if let Some(missing) = libs.iter().find(|l| l.classpath && !l.file(&libs_dir).exists()) {
        bail!("library {} is missing and has no download URL", missing.name);
    }

    // 6. Natives, client jar, legacy assets ---------------------------------
    let natives_dir = layout.natives(&version.id);
    libraries::extract_natives(&libs, &libs_dir, &natives_dir)?;

    // Forge's module system ignores the game jar by *file name*
    // (`${version_name}.jar`), so the jar must be named after the final id.
    let client_jar = if version.id != mc {
        let jar = layout.version_jar(&version.id);
        let src_len = std::fs::metadata(&vanilla_client).map(|m| m.len()).unwrap_or(0);
        if std::fs::metadata(&jar).map(|m| m.len()).unwrap_or(u64::MAX) != src_len {
            std::fs::create_dir_all(jar.parent().unwrap())?;
            std::fs::copy(&vanilla_client, &jar)?;
        }
        jar
    } else {
        vanilla_client
    };

    let assets_root = layout.assets();
    let game_assets =
        assets::materialize_legacy(layout, &asset_ref.id, &asset_index, &spec.game_dir)?.unwrap_or_else(|| assets_root.clone());

    Ok(Installed {
        java: java_windowed,
        java_major,
        client_jar,
        natives_dir,
        libraries: libs,
        assets_root,
        game_assets,
        assets_index: asset_ref.id.clone(),
        logging_arg,
        version,
    })
}
