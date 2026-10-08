//! Modpack import: Modrinth (.mrpack), CurseForge (.zip + API) and plain
//! instance zips. Everything is flattened into a list of files the launcher
//! downloads — mods straight from the CDN, overrides from this panel.

mod error;
pub use error::{PackError as AppError, PackResult as AppResult};
use std::future::Future;
use std::path::PathBuf;

pub trait PackHost: Sync {
    fn http(&self) -> &reqwest::Client;
    fn files_dir(&self) -> PathBuf;
    fn write_pool(&self) -> &sqlx::SqlitePool;
    fn platform_pool(&self) -> &sqlx::SqlitePool;
    fn curseforge_key(&self) -> impl Future<Output = AppResult<String>> + Send;
    fn now(&self) -> String;
    fn modrinth_api(&self) -> &str {
        "https://api.modrinth.com/v2"
    }
    fn curseforge_api(&self) -> &str {
        "https://api.curseforge.com/v1"
    }
}

use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::io::{Cursor, Read};
use std::path::Path;
pub use velora_platform_contracts::Loader;
use velora_platform_utils::paths::safe_join;

const PATH_SEGMENT: &AsciiSet = &NON_ALPHANUMERIC.remove(b'-').remove(b'.').remove(b'_').remove(b'~');

#[derive(Debug, Clone)]
pub struct NewFile {
    pub path: String,
    pub url: String,
    pub sha1: String,
    pub size: u64,
    pub origin: &'static str,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct PackInfo {
    pub mc_version: String,
    pub loader: Loader,
    pub loader_version: Option<String>,
    pub name: String,
    pub version: String,
    pub files: Vec<NewFile>,
}

/// Panel-relative URL for a file stored under `data/files/<instance>/`.
pub fn files_url(instance_id: &str, rel: &str) -> String {
    let encoded: Vec<String> = rel.split('/').map(|s| utf8_percent_encode(s, PATH_SEGMENT).to_string()).collect();
    format!("/files/{}/{}", utf8_percent_encode(instance_id, PATH_SEGMENT), encoded.join("/"))
}

type Zip = zip::ZipArchive<Cursor<Vec<u8>>>;

fn read_json<T: serde::de::DeserializeOwned>(zip: &mut Zip, name: &str) -> Option<T> {
    let mut entry = zip.by_name(name).ok()?;
    let mut buf = Vec::new();
    entry.read_to_end(&mut buf).ok()?;
    serde_json::from_slice(&buf).ok()
}

/// Paths we never ship from a plain instance export.
fn is_junk(rel: &str) -> bool {
    const DIRS: &[&str] = &[
        "logs/",
        "crash-reports/",
        "screenshots/",
        "versions/",
        "libraries/",
        "assets/",
        "natives/",
        "__MACOSX/",
        ".fabric/",
        ".scopenet/",
    ];
    const FILES: &[&str] = &[
        "usercache.json",
        "usernamecache.json",
        "launcher_profiles.json",
        "launcher_accounts.json",
        ".DS_Store",
        "instance.cfg",
        "mmc-pack.json",
    ];
    DIRS.iter().any(|d| rel.starts_with(d)) || FILES.iter().any(|f| rel == *f || rel.ends_with(&format!("/{f}")))
}

/// Extract entries under `prefix` into the instance's file store.
fn extract_prefix(
    zip: &mut Zip,
    prefix: &str,
    dest_root: &Path,
    instance_id: &str,
    origin: &'static str,
    skip_junk: bool,
) -> AppResult<Vec<NewFile>> {
    let mut out = Vec::new();
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| AppError::bad_request(format!("corrupt zip: {e}")))?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().replace('\\', "/");
        let Some(rel) = name.strip_prefix(prefix) else { continue };
        if rel.is_empty() || (skip_junk && is_junk(rel)) {
            continue;
        }
        let Some(dest) = safe_join(dest_root, rel) else { continue };
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut buf = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut buf)?;
        let sha1 = velora_platform_utils::http::sha1_bytes(&buf);
        std::fs::write(&dest, &buf)?;
        out.push(NewFile { path: rel.to_string(), url: files_url(instance_id, rel), sha1, size: buf.len() as u64, origin, note: None });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Modrinth
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MrIndex {
    #[serde(default)]
    name: String,
    #[serde(default)]
    version_id: String,
    files: Vec<MrFile>,
    dependencies: HashMap<String, String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MrFile {
    path: String,
    hashes: HashMap<String, String>,
    #[serde(default)]
    env: Option<HashMap<String, String>>,
    downloads: Vec<String>,
    #[serde(default)]
    file_size: u64,
}

fn import_mrpack(zip: &mut Zip, index: MrIndex, dest_root: &Path, instance_id: &str) -> AppResult<PackInfo> {
    let deps = &index.dependencies;
    let mc = deps.get("minecraft").cloned().ok_or_else(|| AppError::bad_request("mrpack has no minecraft dependency"))?;
    let (loader, loader_version) = if let Some(v) = deps.get("fabric-loader") {
        (Loader::Fabric, Some(v.clone()))
    } else if let Some(v) = deps.get("quilt-loader") {
        (Loader::Quilt, Some(v.clone()))
    } else if let Some(v) = deps.get("neoforge") {
        (Loader::NeoForge, Some(v.clone()))
    } else if let Some(v) = deps.get("forge") {
        (Loader::Forge, Some(velora_platform_utils::meta::normalize_forge_version(Loader::Forge, &mc, v)))
    } else {
        (Loader::Vanilla, None)
    };

    let mut files: Vec<NewFile> = Vec::new();
    for f in index.files {
        if f.env.as_ref().and_then(|e| e.get("client")).map(|c| c == "unsupported").unwrap_or(false) {
            continue;
        }
        let (Some(url), Some(sha1)) = (f.downloads.first(), f.hashes.get("sha1")) else { continue };
        if safe_join(dest_root, &f.path).is_none() {
            continue;
        }
        files.push(NewFile {
            path: f.path.replace('\\', "/"),
            url: url.clone(),
            sha1: sha1.clone(),
            size: f.file_size,
            origin: "pack",
            note: None,
        });
    }
    // client-overrides win over overrides.
    let mut overrides = extract_prefix(zip, "overrides/", dest_root, instance_id, "override", false)?;
    overrides.extend(extract_prefix(zip, "client-overrides/", dest_root, instance_id, "override", false)?);
    merge_files(&mut files, overrides);

    Ok(PackInfo { mc_version: mc, loader, loader_version, name: index.name, version: index.version_id, files })
}

fn merge_files(files: &mut Vec<NewFile>, extra: Vec<NewFile>) {
    for f in extra {
        files.retain(|x| x.path != f.path);
        files.push(f);
    }
}

#[derive(Deserialize, serde::Serialize)]
pub struct MrVersion {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub version_number: String,
    #[serde(default)]
    pub game_versions: Vec<String>,
    #[serde(default)]
    pub loaders: Vec<String>,
    pub files: Vec<MrVersionFile>,
    #[serde(default)]
    pub date_published: String,
}

#[derive(Deserialize, serde::Serialize)]
pub struct MrVersionFile {
    pub url: String,
    pub filename: String,
    #[serde(default)]
    pub primary: bool,
    #[serde(default)]
    pub hashes: HashMap<String, String>,
}

#[derive(Deserialize)]
struct MrProject {
    title: String,
    #[serde(default)]
    icon_url: Option<String>,
}

pub async fn fetch_modrinth(state: &impl PackHost, version_id: &str) -> AppResult<(Vec<u8>, String, Option<String>)> {
    let version: MrVersion =
        velora_platform_utils::http::get_json(state.http(), &format!("{}/version/{version_id}", state.modrinth_api())).await?;
    let project: MrProject =
        velora_platform_utils::http::get_json(state.http(), &format!("{}/project/{}", state.modrinth_api(), version.project_id)).await?;
    let file = version
        .files
        .iter()
        .find(|f| f.primary && f.filename.ends_with(".mrpack"))
        .or_else(|| version.files.iter().find(|f| f.filename.ends_with(".mrpack")))
        .ok_or_else(|| AppError::bad_request("that version has no .mrpack file"))?;
    let bytes = download_bytes(state, &file.url).await?;
    if let Some(expected) = file.hashes.get("sha1") {
        if !velora_platform_utils::http::sha1_bytes(&bytes).eq_ignore_ascii_case(expected) {
            return Err(AppError::bad_request("downloaded pack failed its checksum"));
        }
    }
    Ok((bytes, format!("Modrinth · {} {}", project.title, version.version_number), project.icon_url))
}

async fn download_bytes(state: &impl PackHost, url: &str) -> AppResult<Vec<u8>> {
    let resp = state.http().get(url).send().await.map_err(|e| AppError::from(anyhow::Error::from(e)))?;
    if !resp.status().is_success() {
        return Err(AppError::bad_request(format!("download failed: {} returned {}", url, resp.status())));
    }
    Ok(resp.bytes().await.map_err(|e| AppError::from(anyhow::Error::from(e)))?.to_vec())
}

// ---------------------------------------------------------------------------
// CurseForge
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfManifest {
    minecraft: CfMinecraft,
    #[serde(default)]
    files: Vec<CfManifestFile>,
    #[serde(default = "default_overrides")]
    overrides: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    version: String,
}
fn default_overrides() -> String {
    "overrides".into()
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfMinecraft {
    version: String,
    #[serde(default)]
    mod_loaders: Vec<CfLoader>,
}
#[derive(Deserialize)]
struct CfLoader {
    id: String,
    #[serde(default)]
    primary: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfManifestFile {
    #[serde(rename = "projectID")]
    project_id: i64,
    #[serde(rename = "fileID")]
    file_id: i64,
    #[serde(default = "yes")]
    required: bool,
}
fn yes() -> bool {
    true
}

#[derive(Deserialize)]
struct CfData<T> {
    data: T,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfFile {
    id: i64,
    mod_id: i64,
    file_name: String,
    #[serde(default)]
    download_url: Option<String>,
    #[serde(default)]
    file_length: u64,
    #[serde(default)]
    hashes: Vec<CfHash>,
}
#[derive(Deserialize)]
struct CfHash {
    value: String,
    algo: i32,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfMod {
    id: i64,
    name: String,
    #[serde(default)]
    class_id: Option<i64>,
    #[serde(default)]
    links: Option<CfLinks>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfLinks {
    #[serde(default)]
    website_url: Option<String>,
}

pub fn parse_cf_loader(mc: &str, id: &str) -> (Loader, Option<String>) {
    let (name, version) = id.split_once('-').unwrap_or((id, ""));
    match Loader::parse(name) {
        Some(Loader::Forge) => (Loader::Forge, Some(velora_platform_utils::meta::normalize_forge_version(Loader::Forge, mc, version))),
        Some(l) if l != Loader::Vanilla => (l, Some(version.to_string()).filter(|v| !v.is_empty())),
        _ => (Loader::Vanilla, None),
    }
}

fn cf_folder(class_id: Option<i64>) -> &'static str {
    match class_id {
        Some(12) => "resourcepacks",
        Some(6552) => "shaderpacks",
        Some(4546) => "config",
        _ => "mods",
    }
}

async fn cf_post<T: serde::de::DeserializeOwned>(state: &impl PackHost, key: &str, path: &str, body: serde_json::Value) -> AppResult<T> {
    let resp = state
        .http()
        .post(format!("{}{path}", state.curseforge_api()))
        .header("x-api-key", key)
        .json(&body)
        .send()
        .await
        .map_err(|e| AppError::from(anyhow::Error::from(e)))?;
    if resp.status().as_u16() == 403 {
        return Err(AppError::bad_request("CurseForge rejected the API key"));
    }
    if !resp.status().is_success() {
        return Err(AppError::bad_request(format!("CurseForge returned {}", resp.status())));
    }
    Ok(resp.json::<CfData<T>>().await.map_err(|e| AppError::from(anyhow::Error::from(e)))?.data)
}

async fn cf_get<T: serde::de::DeserializeOwned>(state: &impl PackHost, key: &str, path: &str) -> AppResult<T> {
    let resp = state
        .http()
        .get(format!("{}{path}", state.curseforge_api()))
        .header("x-api-key", key)
        .send()
        .await
        .map_err(|e| AppError::from(anyhow::Error::from(e)))?;
    if !resp.status().is_success() {
        return Err(AppError::bad_request(format!("CurseForge returned {}", resp.status())));
    }
    Ok(resp.json::<CfData<T>>().await.map_err(|e| AppError::from(anyhow::Error::from(e)))?.data)
}

pub async fn cf_raw_get(state: &impl PackHost, path: &str) -> AppResult<serde_json::Value> {
    let key = state.curseforge_key().await?;
    cf_get(state, &key, path).await
}

async fn resolve_cf_files(state: &impl PackHost, manifest_files: &[CfManifestFile]) -> AppResult<Vec<NewFile>> {
    let wanted: Vec<&CfManifestFile> = manifest_files.iter().filter(|f| f.required).collect();
    if wanted.is_empty() {
        return Ok(vec![]);
    }
    let key = state.curseforge_key().await?;
    let mut files: Vec<CfFile> = Vec::new();
    let mut mods: HashMap<i64, CfMod> = HashMap::new();
    for chunk in wanted.chunks(500) {
        let ids: Vec<i64> = chunk.iter().map(|f| f.file_id).collect();
        files.extend(cf_post::<Vec<CfFile>>(state, &key, "/mods/files", json!({ "fileIds": ids })).await?);
        let mod_ids: Vec<i64> = chunk.iter().map(|f| f.project_id).collect();
        for m in cf_post::<Vec<CfMod>>(state, &key, "/mods", json!({ "modIds": mod_ids })).await? {
            mods.insert(m.id, m);
        }
    }

    let mut out = Vec::new();
    for f in files {
        let m = mods.get(&f.mod_id);
        let folder = cf_folder(m.and_then(|m| m.class_id));
        let sha1 = f.hashes.iter().find(|h| h.algo == 1).map(|h| h.value.clone()).unwrap_or_default();
        let url = f.download_url.clone().unwrap_or_default();
        let note = if url.is_empty() {
            let name = m.map(|m| m.name.clone()).unwrap_or_else(|| format!("project {}", f.mod_id));
            let site = m.and_then(|m| m.links.as_ref()).and_then(|l| l.website_url.clone()).unwrap_or_default();
            Some(format!("{name} blocks third-party downloads. Download it from {site}/files/{} and upload it here.", f.id))
        } else {
            None
        };
        out.push(NewFile { path: format!("{folder}/{}", f.file_name), url, sha1, size: f.file_length, origin: "pack", note });
    }
    Ok(out)
}

pub async fn fetch_curseforge(state: &impl PackHost, mod_id: i64, file_id: i64) -> AppResult<(Vec<u8>, String, Option<String>)> {
    let key = state.curseforge_key().await?;
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Info {
        download_url: Option<String>,
        display_name: String,
    }
    #[derive(Deserialize)]
    struct Logo {
        #[serde(rename = "thumbnailUrl")]
        thumbnail_url: Option<String>,
    }
    #[derive(Deserialize)]
    struct ModInfo {
        name: String,
        logo: Option<Logo>,
    }
    let info: Info = cf_get(state, &key, &format!("/mods/{mod_id}/files/{file_id}")).await?;
    let project: ModInfo = cf_get(state, &key, &format!("/mods/{mod_id}")).await?;
    let url = info
        .download_url
        .ok_or_else(|| AppError::bad_request("this modpack can't be downloaded through the API; download the zip and upload it instead"))?;
    let bytes = download_bytes(state, &url).await?;
    Ok((bytes, format!("CurseForge · {} ({})", project.name, info.display_name), project.logo.and_then(|l| l.thumbnail_url)))
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Fallback versions for plain zips (which don't declare their own).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Fallback {
    pub mc_version: Option<String>,
    pub loader: Option<Loader>,
    pub loader_version: Option<String>,
}

/// Result of the blocking parse step: a finished pack, or a CurseForge
/// manifest (+ extracted overrides) that still needs API resolution.
type Parsed = (Option<(PackInfo, &'static str)>, Option<(CfManifest, Vec<NewFile>)>);

/// Import any supported zip for `instance_id`.
pub async fn import_zip(
    state: &impl PackHost,
    instance_id: &str,
    bytes: Vec<u8>,
    fallback: Fallback,
) -> AppResult<(PackInfo, &'static str)> {
    let dest_root = state.files_dir().join(velora_platform_utils::paths::sanitize_id(instance_id));
    std::fs::create_dir_all(&dest_root)?;
    let id = instance_id.to_string();

    // Parsing + extraction is CPU/disk bound: keep it off the async workers.
    let root = dest_root.clone();
    let (parsed, cf_manifest) = tokio::task::spawn_blocking(move || -> AppResult<Parsed> {
        let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| AppError::bad_request(format!("not a valid zip: {e}")))?;
        if let Some(index) = read_json::<MrIndex>(&mut zip, "modrinth.index.json") {
            return Ok((Some((import_mrpack(&mut zip, index, &root, &id)?, "modrinth")), None));
        }
        if let Some(manifest) = read_json::<CfManifest>(&mut zip, "manifest.json") {
            let prefix = format!("{}/", manifest.overrides.trim_end_matches('/'));
            let overrides = extract_prefix(&mut zip, &prefix, &root, &id, "override", false)?;
            return Ok((None, Some((manifest, overrides))));
        }
        // Plain instance zip: find the game directory inside it.
        let names: Vec<String> = zip.file_names().map(|n| n.replace('\\', "/")).collect();
        let prefix = detect_root(&names);
        let files = extract_prefix(&mut zip, &prefix, &root, &id, "override", true)?;
        Ok((Some((PackInfo { files, ..Default::default() }, "zip")), None))
    })
    .await
    .map_err(|e| AppError::bad_request(format!("import crashed: {e}")))??;

    if let Some((manifest, overrides)) = cf_manifest {
        let mc = manifest.minecraft.version.clone();
        let primary = manifest.minecraft.mod_loaders.iter().find(|l| l.primary).or(manifest.minecraft.mod_loaders.first());
        let (loader, loader_version) = primary.map(|l| parse_cf_loader(&mc, &l.id)).unwrap_or((Loader::Vanilla, None));
        let mut files = resolve_cf_files(state, &manifest.files).await?;
        merge_files(&mut files, overrides);
        return Ok((
            PackInfo { mc_version: mc, loader, loader_version, name: manifest.name, version: manifest.version, files },
            "curseforge",
        ));
    }

    let (mut info, kind) = parsed.expect("parsed");
    if kind == "zip" {
        info.mc_version = fallback.mc_version.filter(|v| !v.is_empty()).ok_or_else(|| {
            AppError::bad_request("this zip isn't a Modrinth or CurseForge pack — pick the Minecraft version and loader for it")
        })?;
        info.loader = fallback.loader.unwrap_or_default();
        info.loader_version = fallback.loader_version.filter(|v| !v.is_empty());
    }
    Ok((info, kind))
}

/// Find the game directory inside a zip: a `.minecraft/` folder, or a
/// single top-level folder wrapping everything.
pub fn detect_root(names: &[String]) -> String {
    if let Some(pos) = names.iter().filter_map(|n| n.find(".minecraft/").map(|i| &n[..i + ".minecraft/".len()])).min_by_key(|p| p.len()) {
        return pos.to_string();
    }
    let markers = ["mods/", "config/", "resourcepacks/", "shaderpacks/", "options.txt"];
    if names.iter().any(|n| markers.iter().any(|m| n.starts_with(m))) {
        return String::new();
    }
    let tops: std::collections::HashSet<&str> = names.iter().filter_map(|n| n.split_once('/').map(|(t, _)| t)).collect();
    if tops.len() == 1 && names.iter().all(|n| n.contains('/')) {
        return format!("{}/", tops.into_iter().next().unwrap());
    }
    String::new()
}

/// Replace the instance's pack/override files with `info.files`, keep files
/// the admin uploaded by hand, update versions and bump the revision.
pub async fn apply(
    state: &impl PackHost,
    instance_id: &str,
    info: &PackInfo,
    kind: &str,
    label: &str,
    source_ref: serde_json::Value,
) -> AppResult<()> {
    let files: Vec<_> = info
        .files
        .iter()
        .map(|f| velora_panel_instances::mutations::DistributionFile {
            path: &f.path,
            url: &f.url,
            sha1: &f.sha1,
            size: f.size,
            origin: f.origin,
            note: f.note.as_deref(),
        })
        .collect();
    let replacement = velora_panel_instances::mutations::PackReplacement {
        mc_version: &info.mc_version,
        loader: info.loader.as_str(),
        loader_version: info.loader_version.as_deref(),
        files: &files,
    };
    velora_panel_instances::mutations::replace_pack(
        state.write_pool(),
        instance_id,
        &replacement,
        kind,
        label,
        &source_ref.to_string(),
        || state.now(),
    )
    .await?;
    gc_files(state, instance_id).await?;
    Ok(())
}

/// Delete stored files no longer referenced by the instance.
pub async fn gc_files(state: &impl PackHost, instance_id: &str) -> AppResult<()> {
    let root = state.files_dir().join(velora_platform_utils::paths::sanitize_id(instance_id));
    let referenced: std::collections::HashSet<String> = velora_panel_instances::instance_files(state.platform_pool(), instance_id)
        .await?
        .into_iter()
        .filter(|f| f.url.starts_with("/files/"))
        .map(|f| f.path)
        .collect();
    tokio::task::spawn_blocking(move || {
        fn walk(dir: &Path, root: &Path, keep: &std::collections::HashSet<String>) {
            let Ok(entries) = std::fs::read_dir(dir) else { return };
            for e in entries.flatten() {
                let p = e.path();
                if p.is_dir() {
                    walk(&p, root, keep);
                    std::fs::remove_dir(&p).ok(); // only succeeds when empty
                } else if let Ok(rel) = p.strip_prefix(root) {
                    let rel = rel.to_string_lossy().replace('\\', "/");
                    if !keep.contains(&rel) {
                        std::fs::remove_file(&p).ok();
                    }
                }
            }
        }
        walk(&root, &root, &referenced);
    })
    .await
    .ok();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cf_loader_ids() {
        assert_eq!(parse_cf_loader("1.20.1", "forge-47.2.0"), (Loader::Forge, Some("1.20.1-47.2.0".into())));
        assert_eq!(parse_cf_loader("1.21.1", "neoforge-21.1.77"), (Loader::NeoForge, Some("21.1.77".into())));
        assert_eq!(parse_cf_loader("1.21.1", "fabric-0.16.9"), (Loader::Fabric, Some("0.16.9".into())));
    }

    #[test]
    fn detects_zip_roots() {
        let v = |xs: &[&str]| xs.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(detect_root(&v(&["mods/a.jar", "config/b.toml"])), "");
        assert_eq!(detect_root(&v(&["Pack/mods/a.jar", "Pack/config/b.toml"])), "Pack/");
        assert_eq!(detect_root(&v(&["x/.minecraft/mods/a.jar", "x/instance.cfg"])), "x/.minecraft/");
    }

    #[test]
    fn encodes_file_urls() {
        assert_eq!(files_url("smp", "mods/My Mod+1.jar"), "/files/smp/mods/My%20Mod%2B1.jar");
    }

    #[test]
    fn junk_is_skipped() {
        assert!(is_junk("logs/latest.log"));
        assert!(is_junk("usercache.json"));
        assert!(!is_junk("mods/sodium.jar"));
    }
}
