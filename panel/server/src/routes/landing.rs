//! Public landing page & admin landing builder API.

use crate::auth::AdminUser;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::state::RequestState as State;
use crate::store;
use axum::body::Body;
use axum::extract::{Multipart, Path};
use axum::http::header;
use axum::response::{IntoResponse, Redirect, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub use velora_panel_distribution::{HostedDownload, FaqItem, LandingBlock, LandingTheme, github_repo, classify_installer, find_version, short_name};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct LandingConfig {
    pub enabled: bool,
    pub brand_name: String,
    pub logo_url: Option<String>,
    pub tagline: String,
    pub hero_title: String,
    pub hero_subtitle: String,
    pub hero_cta_text: String,
    pub hero_bg_type: String, // "gradient", "image", "video"
    pub hero_bg_url: Option<String>,
    pub server_ip: String,
    pub server_port: u16,
    pub hosted_downloads: Vec<HostedDownload>,
    pub external_download_url: Option<String>,
    pub blocks: Vec<LandingBlock>,
    pub faqs: Vec<FaqItem>,
    pub custom_css: String,
    pub theme: LandingTheme,
}

impl Default for LandingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            brand_name: "Velora".into(),
            logo_url: None,
            tagline: "Your worlds, one click away.".into(),
            hero_title: "The Ultimate Minecraft Experience".into(),
            hero_subtitle: "Custom instances, automated modpack sync, zero hassle. Download the launcher and dive straight into our servers.".into(),
            hero_cta_text: "Download Launcher".into(),
            hero_bg_type: "gradient".into(),
            hero_bg_url: None,
            server_ip: "".into(),
            server_port: 25565,
            hosted_downloads: vec![],
            external_download_url: None,
            blocks: vec![
                LandingBlock { id: "b_hero".into(), block_type: "hero".into(), enabled: true, title: None, subtitle: None, options: serde_json::json!({}) },
                LandingBlock { id: "b_download".into(), block_type: "download".into(), enabled: true, title: Some("Get the Launcher".into()), subtitle: Some("Available on Windows, macOS, and Linux".into()), options: serde_json::json!({}) },
                LandingBlock { id: "b_servers".into(), block_type: "servers".into(), enabled: true, title: Some("Live Servers".into()), subtitle: Some("Check our current status and join the game".into()), options: serde_json::json!({}) },
                LandingBlock { id: "b_map".into(), block_type: "map".into(), enabled: true, title: Some("Explore the World".into()), subtitle: Some("Guild land, spawn, warps and shops on the live map. Player positions stay private.".into()), options: serde_json::json!({"server_id": 0, "height": 620, "show_claims": true, "show_pins": true, "show_players": false}) },
                LandingBlock { id: "b_stats".into(), block_type: "stats".into(), enabled: true, title: Some("Community By Numbers".into()), subtitle: Some("Every block placed, every second played".into()), options: serde_json::json!({}) },
                LandingBlock { id: "b_leaderboard".into(), block_type: "leaderboard".into(), enabled: true, title: Some("Top Players".into()), subtitle: Some("The champions of our network".into()), options: serde_json::json!({}) },
                LandingBlock { id: "b_instances".into(), block_type: "instances".into(), enabled: true, title: Some("Featured Instances".into()), subtitle: Some("Hand-crafted modpacks ready to launch".into()), options: serde_json::json!({}) },
                LandingBlock { id: "b_news".into(), block_type: "news".into(), enabled: true, title: Some("Latest News".into()), subtitle: Some("Updates, events, and patch notes".into()), options: serde_json::json!({}) },
                LandingBlock { id: "b_faq".into(), block_type: "faq".into(), enabled: true, title: Some("Frequently Asked Questions".into()), subtitle: Some("Everything you need to know".into()), options: serde_json::json!({}) },
                LandingBlock { id: "b_socials".into(), block_type: "socials".into(), enabled: true, title: Some("Join Our Community".into()), subtitle: Some("Connect with thousands of players".into()), options: serde_json::json!({}) },
            ],
            faqs: vec![
                FaqItem {
                    id: "faq_1".into(),
                    question: "How do I start playing?".into(),
                    answer: "Download our custom launcher, sign in or create an account, select an instance and hit Play! All mods and Java runtimes download automatically.".into(),
                },
                FaqItem {
                    id: "faq_2".into(),
                    question: "Do I need to install Java manually?".into(),
                    answer: "No. The Velora launcher automatically detects, downloads, and isolates the exact Java runtime required for each Minecraft version.".into(),
                },
                FaqItem {
                    id: "faq_3".into(),
                    question: "Can I customize my skin and cape?".into(),
                    answer: "Yes! In the launcher settings, you can upload HD skins, choose arm styles, pick your earned capes, and save multiple wardrobe presets to swap anytime.".into(),
                },
            ],
            custom_css: "".into(),
            theme: LandingTheme::default(),
        }
    }
}

pub async fn get_config(state: &AppState) -> AppResult<LandingConfig> {
    let mut cfg: LandingConfig = store::kv_get(state, "landing_page").await?;
    // Pages saved before the public map existed get a map section once; after that it is the admin's to move or delete.
    if !cfg.blocks.iter().any(|b| b.block_type == "map") {
        let added: bool = store::kv_get(state, "landing_map_added").await.unwrap_or(false);
        if !added {
            if let Some(map) = LandingConfig::default().blocks.into_iter().find(|b| b.block_type == "map") {
                let at = cfg
                    .blocks
                    .iter()
                    .position(|b| b.block_type == "servers")
                    .or_else(|| cfg.blocks.iter().position(|b| b.block_type == "download"))
                    .map(|i| i + 1)
                    .unwrap_or(cfg.blocks.len());
                cfg.blocks.insert(at.min(cfg.blocks.len()), map);
                store::kv_set(state, "landing_page", &cfg).await?;
            }
            store::kv_set(state, "landing_map_added", &true).await?;
        }
    }
    Ok(cfg)
}

pub async fn public_landing(State(state): State<AppState>) -> AppResult<Json<LandingConfig>> {
    let mut cfg = get_config(&state).await?;
    // If branding has been configured, adopt brand name & tagline if default
    if let Ok(branding) = store::branding(&state).await {
        if matches!(cfg.brand_name.as_str(), "SCOPENET" | "Velora") && !branding.name.is_empty() {
            cfg.brand_name = branding.name;
        }
        if cfg.tagline == "Your worlds, one click away." && !branding.tagline.is_empty() {
            cfg.tagline = branding.tagline;
        }
    }
    // If server ip is empty, try to populate from first server
    if cfg.server_ip.is_empty() {
        if let Ok(Some(first_server)) =
            sqlx::query_scalar::<_, String>("SELECT address FROM game_servers ORDER BY id ASC LIMIT 1").fetch_optional(&state.db).await
        {
            cfg.server_ip = first_server;
        }
    }
    // Installers published on the project's GitHub release fill any platform the admin has not uploaded by hand.
    if let Some(url) = cfg.external_download_url.clone() {
        for found in release_downloads(&state, &url).await {
            if !cfg.hosted_downloads.iter().any(|d| d.platform == found.platform) {
                cfg.hosted_downloads.push(found);
            }
        }
    }
    cfg.hosted_downloads.sort_by_key(|d| ["windows", "mac", "linux", "android", "ios"].iter().position(|p| *p == d.platform).unwrap_or(9));
    polish(&mut cfg);
    Ok(Json(cfg))
}

/// The latest release's installers, cached for ten minutes so the landing page never waits on GitHub twice.
async fn release_downloads(state: &AppState, url: &str) -> Vec<HostedDownload> {
    use std::sync::Mutex;
    use std::time::{Duration, Instant};
    static CACHE: Mutex<Option<(Instant, String, Vec<HostedDownload>)>> = Mutex::new(None);
    let Some((owner, repo)) = github_repo(url) else { return vec![] };
    let key = format!("{owner}/{repo}");
    if let Some((at, k, found)) = CACHE.lock().unwrap().as_ref() {
        if *k == key && at.elapsed() < Duration::from_secs(600) {
            return found.clone();
        }
    }
    let fetched = async {
        let response = state
            .http
            .get(format!("https://api.github.com/repos/{key}/releases/latest"))
            .header("User-Agent", "scopenet-panel")
            .header("Accept", "application/vnd.github+json")
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .ok()?
            .error_for_status()
            .ok()?;
        response.json::<serde_json::Value>().await.ok()
    }
    .await;
    let mut found = vec![];
    if let Some(release) = fetched {
        let version = release["tag_name"].as_str().unwrap_or("").trim_start_matches('v').to_string();
        for asset in release["assets"].as_array().into_iter().flatten() {
            let (Some(name), Some(link)) = (asset["name"].as_str(), asset["browser_download_url"].as_str()) else { continue };
            let Some((platform, label)) = classify_installer(name) else { continue };
            if !link.starts_with(&format!("https://github.com/{key}/")) {
                continue;
            }
            found.push(HostedDownload {
                platform: platform.into(),
                label: if version.is_empty() { label.into() } else { format!("{label} · {version}") },
                filename: name.into(),
                file_url: link.into(),
                size: asset["size"].as_i64().unwrap_or(0),
                uploaded_at: asset["updated_at"].as_str().unwrap_or("").into(),
                version: version.clone(),
                display_name: String::new(),
            });
        }
    }
    // A failed fetch is cached too, briefly, so an outage does not slow every visit.
    *CACHE.lock().unwrap() = Some((
        Instant::now().checked_sub(if found.is_empty() { Duration::from_secs(540) } else { Duration::ZERO }).unwrap_or_else(Instant::now),
        key,
        found.clone(),
    ));
    found
}

/// Fills in the tidy names (and versions) of every hosted download, for entries saved before they existed too.
fn polish(cfg: &mut LandingConfig) {
    let brand = cfg.brand_name.clone();
    for d in &mut cfg.hosted_downloads {
        if d.version.is_empty() {
            d.version = find_version(&d.filename).or_else(|| find_version(&d.label)).unwrap_or_default();
        }
        d.display_name = short_name(&brand, &d.platform, &d.filename, &d.label, &d.version);
    }
}

pub async fn admin_get_landing(_: AdminUser, State(state): State<AppState>) -> AppResult<Json<LandingConfig>> {
    let mut cfg = get_config(&state).await?;
    polish(&mut cfg);
    Ok(Json(cfg))
}

pub async fn admin_put_landing(
    _: AdminUser,
    State(state): State<AppState>,
    Json(cfg): Json<LandingConfig>,
) -> AppResult<Json<LandingConfig>> {
    if cfg.blocks.len() > 100 || cfg.custom_css.len() > 100_000 {
        return Err(AppError::bad_request("landing page exceeds the editor limits"));
    }
    let mut ids = std::collections::HashSet::new();
    for block in &cfg.blocks {
        if block.id.is_empty() || !ids.insert(&block.id) {
            return Err(AppError::bad_request("landing section IDs must be unique"));
        }
    }
    store::kv_set(&state, "landing_page", &cfg).await?;
    // Whatever the admin saved is final: the map section is never added behind their back afterwards.
    store::kv_set(&state, "landing_map_added", &true).await?;
    Ok(Json(cfg))
}

/// Desktop installers for macOS and Linux (Windows goes through the launcher-update publish, which checks the installer; the phone
/// apps through the mobile-apps publish). A platform can hold several files of different kinds, e.g. an AppImage and a .deb.
pub async fn upload_launcher(_: AdminUser, State(state): State<AppState>, mut form: Multipart) -> AppResult<Json<LandingConfig>> {
    let mut platform = String::new();
    let mut version = String::new();
    let mut filename = String::new();
    let mut file_bytes = Vec::new();

    while let Some(field) = form.next_field().await? {
        match field.name().unwrap_or_default() {
            "platform" => platform = field.text().await?,
            "version" => version = field.text().await?.trim().chars().take(64).collect(),
            "file" => {
                if let Some(f_name) = field.file_name() {
                    filename = f_name.to_string();
                }
                file_bytes = field.bytes().await?.to_vec();
            }
            _ => {}
        }
    }
    let allowed: &[&str] = match platform.as_str() {
        "mac" => &["dmg", "pkg", "zip"],
        "linux" => &["appimage", "deb", "rpm", "tar.gz", "zip"],
        _ => {
            return Err(AppError::bad_request(
                "Upload macOS or Linux installers here; Windows and the phone apps have their own publish step",
            ))
        }
    };
    if file_bytes.is_empty() {
        return Err(AppError::bad_request("no launcher file was uploaded"));
    }
    if file_bytes.len() > 512 * 1024 * 1024 {
        return Err(AppError::bad_request("Installers can be at most 512 MB"));
    }
    // Sanitize filename to prevent directory traversal
    let safe_name = PathBuf::from(&filename)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .filter(|n| !n.is_empty() && n.len() <= 180)
        .ok_or_else(|| AppError::bad_request("The installer needs a file name"))?;
    let lower = safe_name.to_ascii_lowercase();
    let Some(kind) = allowed.iter().find(|e| lower.ends_with(&format!(".{e}"))) else {
        return Err(AppError::bad_request(format!(
            "A {platform} installer ends in {}",
            allowed.iter().map(|e| format!(".{e}")).collect::<Vec<_>>().join(", ")
        )));
    };

    let dir = state.cfg.downloads_dir();
    tokio::fs::create_dir_all(&dir).await?;
    tokio::fs::write(dir.join(&safe_name), &file_bytes).await?;

    let size = file_bytes.len() as i64;
    let file_url = format!("/download/launcher/{platform}?f={}", urlencoding_name(&safe_name));
    let mut cfg = get_config(&state).await?;
    let ts = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let label = if platform == "mac" { "macOS" } else { "Linux" }.to_string();
    let entry = HostedDownload {
        platform: platform.clone(),
        label,
        filename: safe_name.clone(),
        file_url,
        size,
        uploaded_at: ts,
        version,
        display_name: String::new(),
    };
    // Replace the same kind of file (a new AppImage replaces the old AppImage), keep the others.
    let replaced: Vec<HostedDownload> = cfg
        .hosted_downloads
        .iter()
        .filter(|d| d.platform == platform && d.filename.to_ascii_lowercase().ends_with(&format!(".{kind}")))
        .cloned()
        .collect();
    cfg.hosted_downloads.retain(|d| !(d.platform == platform && d.filename.to_ascii_lowercase().ends_with(&format!(".{kind}"))));
    cfg.hosted_downloads.push(entry);
    store::kv_set(&state, "landing_page", &cfg).await?;
    for old in replaced {
        if old.filename != safe_name && !cfg.hosted_downloads.iter().any(|d| d.filename == old.filename) {
            tokio::fs::remove_file(dir.join(&old.filename)).await.ok();
        }
    }
    polish(&mut cfg);
    Ok(Json(cfg))
}

/// File names in a query string: letters, digits and `.-_` pass, everything else is percent-encoded.
fn urlencoding_name(name: &str) -> String {
    name.bytes()
        .map(|b| if b.is_ascii_alphanumeric() || b"-_.".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") })
        .collect()
}

#[derive(Deserialize)]
pub struct FileQuery {
    f: Option<String>,
}

/// `DELETE /api/admin/landing/launcher/{platform}?f=<file>`: take a hosted installer off the website (all of the platform's when `f` is left out).
pub async fn remove_launcher(
    _: AdminUser,
    State(state): State<AppState>,
    Path(platform): Path<String>,
    axum::extract::Query(q): axum::extract::Query<FileQuery>,
) -> AppResult<Json<LandingConfig>> {
    let mut cfg = get_config(&state).await?;
    let (gone, kept): (Vec<_>, Vec<_>) =
        cfg.hosted_downloads.into_iter().partition(|d| d.platform == platform && q.f.as_deref().is_none_or(|f| f == d.filename));
    cfg.hosted_downloads = kept;
    store::kv_set(&state, "landing_page", &cfg).await?;
    for d in gone {
        // Only files this page hosts itself live under their own name; the Windows and phone installers are removed by their own endpoints.
        if d.file_url.starts_with("/download/launcher/") && !cfg.hosted_downloads.iter().any(|k| k.filename == d.filename) {
            tokio::fs::remove_file(state.cfg.downloads_dir().join(&d.filename)).await.ok();
        }
    }
    polish(&mut cfg);
    Ok(Json(cfg))
}

pub async fn download_launcher(
    State(state): State<AppState>,
    Path(platform): Path<String>,
    axum::extract::Query(q): axum::extract::Query<FileQuery>,
) -> AppResult<Response> {
    let mut cfg = get_config(&state).await?;
    polish(&mut cfg);
    let entry = cfg
        .hosted_downloads
        .iter()
        .find(|d| (d.platform == platform || d.platform == "all") && q.f.as_deref().is_none_or(|f| f == d.filename));

    if let Some(d) = entry {
        let file_path = state.cfg.downloads_dir().join(&d.filename);
        if file_path.exists() {
            let bytes = tokio::fs::read(&file_path).await?;
            let cd = format!("attachment; filename=\"{}\"", if d.display_name.is_empty() { &d.filename } else { &d.display_name });
            let mime = if d.filename.ends_with(".exe") || d.filename.ends_with(".msi") {
                "application/x-msdownload"
            } else if d.filename.ends_with(".dmg") {
                "application/x-apple-diskimage"
            } else if d.filename.ends_with(".deb") {
                "application/vnd.debian.binary-package"
            } else if d.filename.ends_with(".AppImage") {
                "application/x-executable"
            } else {
                "application/octet-stream"
            };
            return Ok(([(header::CONTENT_TYPE, mime), (header::CONTENT_DISPOSITION, cd.as_str())], Body::from(bytes)).into_response());
        }
    }

    if let Some(ext) = cfg.external_download_url.filter(|u| !u.is_empty()) {
        return Ok(Redirect::temporary(&ext).into_response());
    }

    Err(AppError::not_found("no launcher download available for this platform yet"))
}
