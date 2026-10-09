//! The Velora Map.
//!
//! Game servers draw their own world: the plugin/mod renders top-down map tiles from the region files and uploads only
//! the small PNGs that changed. The panel just stores them and serves them, together with live player positions and an
//! overlay (guild claims, pins) that the game server keeps up to date. There is nothing to install or run beside the panel.
//!
//! Layout on disk: `<data>/map/<server id>/<dimension>/<zoom>/<x>_<y>.png`. Zoom 0 is one pixel per block with 256-pixel
//! tiles; each higher zoom halves the resolution.

use rand::RngCore;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub const TILE_SIZE: u32 = 256;
pub const MAX_ZOOM: u8 = 6;
const MAX_TILE_BYTES: usize = 1024 * 1024;
const MAX_COORD: i32 = 1 << 20;
pub const TOKEN_TTL: Duration = Duration::from_secs(3600);
/// Player positions older than this mean "the server stopped sending".
const PLAYERS_FRESH: Duration = Duration::from_secs(30);
const MAX_PLAYERS: usize = 500;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LivePlayer {
    pub uuid: String,
    pub name: String,
    pub dimension: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    #[serde(default)]
    pub yaw: f32,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Stats {
    pub tiles_received: u64,
    pub bytes_received: u64,
    pub last_upload_at: Option<String>,
    pub last_players_at: Option<String>,
    pub last_overlay_at: Option<String>,
}

struct PlayersSnap {
    at: Instant,
    players: Vec<LivePlayer>,
}

pub struct WorldMap {
    root: PathBuf,
    players: Mutex<HashMap<i64, PlayersSnap>>,
    tokens: Mutex<HashMap<String, (i64, Instant)>>,
    overlays: Mutex<HashMap<i64, Value>>,
    stats: Mutex<HashMap<i64, Stats>>,
    diagnostics: Mutex<HashMap<i64, (Instant, Value)>>,
}

/// A dimension id ("minecraft:the_nether") as a safe folder name ("the_nether"). Other namespaces keep theirs ("mod__space").
pub fn dim_slug(id: &str) -> Option<String> {
    let id = id.trim();
    let trimmed = id.strip_prefix("minecraft:").unwrap_or(id);
    let slug = match trimmed {
        "nether" => "the_nether".to_string(),
        "end" => "the_end".to_string(),
        other => other.replace(':', "__"),
    };
    let ok = !slug.is_empty()
        && slug.len() <= 48
        && slug.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '_' | '-' | '.'))
        && !slug.contains("..");
    ok.then_some(slug)
}

pub fn dim_label(slug: &str) -> String {
    match slug {
        "overworld" => "Overworld".into(),
        "the_nether" => "The Nether".into(),
        "the_end" => "The End".into(),
        other => other.rsplit("__").next().unwrap_or(other).replace(['_', '-'], " "),
    }
}

/// One decoded upload record.
#[derive(Debug, PartialEq)]
pub struct TileRecord {
    pub zoom: u8,
    pub x: i32,
    pub y: i32,
    pub png: Vec<u8>,
}

/// `[u8 zoom][i32 x][i32 y][u32 length][PNG bytes]` repeated, big endian.
pub fn parse_tiles(body: &[u8]) -> Result<Vec<TileRecord>, String> {
    let mut out = Vec::new();
    let mut at = 0usize;
    while at < body.len() {
        if body.len() - at < 13 {
            return Err("truncated tile header".into());
        }
        let zoom = body[at];
        let x = i32::from_be_bytes(body[at + 1..at + 5].try_into().unwrap());
        let y = i32::from_be_bytes(body[at + 5..at + 9].try_into().unwrap());
        let len = u32::from_be_bytes(body[at + 9..at + 13].try_into().unwrap()) as usize;
        at += 13;
        if zoom > MAX_ZOOM || x.abs() > MAX_COORD || y.abs() > MAX_COORD {
            return Err("tile out of range".into());
        }
        if len < 8 || len > MAX_TILE_BYTES || body.len() - at < len {
            return Err("bad tile length".into());
        }
        let png = &body[at..at + len];
        if png[..8] != [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a] {
            return Err("tiles must be PNG images".into());
        }
        out.push(TileRecord { zoom, x, y, png: png.to_vec() });
        at += len;
    }
    Ok(out)
}

fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

impl WorldMap {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            root: data_dir.join("map"),
            players: Default::default(),
            tokens: Default::default(),
            overlays: Default::default(),
            stats: Default::default(),
            diagnostics: Default::default(),
        }
    }

    fn server_dir(&self, id: i64) -> PathBuf {
        self.root.join(id.to_string())
    }

    pub fn tile_path(&self, id: i64, slug: &str, zoom: u8, x: i32, y: i32) -> PathBuf {
        self.server_dir(id).join(slug).join(zoom.to_string()).join(format!("{x}_{y}.png"))
    }

    // ---- tiles ---------------------------------------------------------------------------

    /// Store a batch. Each tile is written to a temporary file and renamed, so a viewer never sees half a PNG.
    pub fn store_tiles(&self, id: i64, slug: &str, tiles: &[TileRecord]) -> std::io::Result<usize> {
        for t in tiles {
            let path = self.tile_path(id, slug, t.zoom, t.x, t.y);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let tmp = path.with_extension("png.tmp");
            std::fs::write(&tmp, &t.png)?;
            std::fs::rename(&tmp, &path)?;
        }
        let bytes: u64 = tiles.iter().map(|t| t.png.len() as u64).sum();
        let mut stats = self.stats.lock().unwrap();
        let s = stats.entry(id).or_default();
        s.tiles_received += tiles.len() as u64;
        s.bytes_received += bytes;
        s.last_upload_at = Some(now_rfc3339());
        Ok(tiles.len())
    }

    /// The PNG and its modification time (seconds), if that tile exists.
    pub fn read_tile(&self, id: i64, slug: &str, zoom: u8, x: i32, y: i32) -> Option<(Vec<u8>, u64)> {
        let path = self.tile_path(id, slug, zoom, x, y);
        let bytes = std::fs::read(&path).ok()?;
        let modified = std::fs::metadata(&path).ok()?.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs();
        Some((bytes, modified))
    }

    /// Which dimensions have tiles, and how far they reach (in zoom-0 tiles).
    pub fn summary(&self, id: i64) -> Value {
        let mut dims = Vec::new();
        let Ok(rd) = std::fs::read_dir(self.server_dir(id)) else { return json!({ "dimensions": dims }) };
        let mut names: Vec<String> =
            rd.flatten().filter(|e| e.path().is_dir()).filter_map(|e| e.file_name().to_str().map(String::from)).collect();
        names.sort_by_key(|n| (n != "overworld", n != "the_nether", n != "the_end", n.clone()));
        for slug in names {
            let (mut count, mut bytes) = (0u64, 0u64);
            let mut coords: Vec<(i32, i32)> = Vec::new();
            let (mut min_x, mut max_x, mut min_y, mut max_y) = (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
            if let Ok(files) = std::fs::read_dir(self.server_dir(id).join(&slug).join("0")) {
                for f in files.flatten() {
                    let name = f.file_name().to_string_lossy().to_string();
                    let Some((xs, ys)) = name.strip_suffix(".png").and_then(|n| n.split_once('_')) else { continue };
                    let (Ok(x), Ok(y)) = (xs.parse::<i32>(), ys.parse::<i32>()) else { continue };
                    count += 1;
                    coords.push((x, y));
                    bytes += f.metadata().map(|m| m.len()).unwrap_or(0);
                    (min_x, max_x, min_y, max_y) = (min_x.min(x), max_x.max(x), min_y.min(y), max_y.max(y));
                }
            }
            if count == 0 {
                continue;
            }
            // How many tiles each zoom level holds, so a viewer never waits on a level that was never uploaded.
            let levels: Vec<u64> = (0..=MAX_ZOOM)
                .map(|z| {
                    if z == 0 {
                        count
                    } else {
                        std::fs::read_dir(self.server_dir(id).join(&slug).join(z.to_string()))
                            .map(|d| d.flatten().count() as u64)
                            .unwrap_or(0)
                    }
                })
                .collect();
            // The populated core (10th to 90th percentile), so one far-away stray region doesn't pull the first view into empty space.
            let core = |mut v: Vec<i32>| {
                v.sort_unstable();
                let at = |q: usize| v[(v.len() - 1) * q / 100];
                (at(10), at(90))
            };
            let (cx0, cx1) = core(coords.iter().map(|c| c.0).collect());
            let (cy0, cy1) = core(coords.iter().map(|c| c.1).collect());
            dims.push(json!({
                "id": if slug.contains("__") { slug.replacen("__", ":", 1) } else { format!("minecraft:{slug}") },
                "slug": slug, "label": dim_label(&slug), "available": true, "tiles": count, "bytes": bytes,
                "bounds": { "min_x": min_x, "max_x": max_x, "min_y": min_y, "max_y": max_y },
                "core": { "min_x": cx0, "max_x": cx1, "min_y": cy0, "max_y": cy1 },
                "levels": levels,
            }));
        }
        json!({ "dimensions": dims })
    }

    // ---- zoom pyramid --------------------------------------------------------------------

    /// Makes sure every coarser zoom level exists and is at least as new as the tiles it is made from, climbing from the given
    /// zoom-0 tiles. Game servers upload their own coarse levels too, but a viewer must never depend on that: worlds drawn by an
    /// older plugin build, or a batch that failed half way, would otherwise leave the zoomed-out map empty. Returns how many
    /// tiles were (re)built.
    pub fn build_pyramid(&self, id: i64, slug: &str, zero: &[(i32, i32)]) -> usize {
        let mut built = 0;
        let mut level: std::collections::BTreeSet<(i32, i32)> = zero.iter().copied().collect();
        for z in 0..MAX_ZOOM {
            let parents: std::collections::BTreeSet<(i32, i32)> = level.iter().map(|&(x, y)| (x >> 1, y >> 1)).collect();
            for &(px, py) in &parents {
                let path = self.tile_path(id, slug, z + 1, px, py);
                let parent_time = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
                let mut newest_child = None;
                for (qx, qy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let child = self.tile_path(id, slug, z, px * 2 + qx, py * 2 + qy);
                    if let Ok(t) = std::fs::metadata(&child).and_then(|m| m.modified()) {
                        newest_child = Some(newest_child.map_or(t, |n: std::time::SystemTime| n.max(t)));
                    }
                }
                let Some(newest) = newest_child else { continue };
                if parent_time.is_some_and(|p| p >= newest) {
                    continue;
                }
                if self.compose_parent(id, slug, z, px, py).unwrap_or(false) {
                    built += 1;
                }
            }
            level = parents;
        }
        built
    }

    /// Builds the zoom pyramid for everything already stored. Runs once at start-up so maps drawn before this existed heal themselves.
    pub fn backfill_pyramids(&self) -> usize {
        let mut built = 0;
        let Ok(servers) = std::fs::read_dir(&self.root) else { return 0 };
        for server in servers.flatten() {
            let Some(id) = server.file_name().to_str().and_then(|n| n.parse::<i64>().ok()) else { continue };
            let Ok(dims) = std::fs::read_dir(server.path()) else { continue };
            for dim in dims.flatten().filter(|d| d.path().is_dir()) {
                let Some(slug) = dim.file_name().to_str().map(String::from) else { continue };
                let coords = self.zero_tiles(id, &slug);
                if !coords.is_empty() {
                    built += self.build_pyramid(id, &slug, &coords);
                }
            }
        }
        built
    }

    fn zero_tiles(&self, id: i64, slug: &str) -> Vec<(i32, i32)> {
        let Ok(files) = std::fs::read_dir(self.server_dir(id).join(slug).join("0")) else { return Vec::new() };
        files
            .flatten()
            .filter_map(|f| {
                let name = f.file_name().to_string_lossy().to_string();
                let (xs, ys) = name.strip_suffix(".png")?.split_once('_')?;
                Some((xs.parse().ok()?, ys.parse().ok()?))
            })
            .collect()
    }

    /// One coarse tile from its four finer ones: each 2 x 2 block of pixels becomes the mean of its opaque pixels, so zoomed-out levels are smooth
    /// like the plugin's. False if there was nothing to draw.
    fn compose_parent(&self, id: i64, slug: &str, zoom: u8, px: i32, py: i32) -> Result<bool, Box<dyn std::error::Error>> {
        let size = TILE_SIZE;
        let half = size / 2;
        let mut out = image::RgbaImage::new(size, size);
        let mut any = false;
        for (qx, qy) in [(0u32, 0u32), (1, 0), (0, 1), (1, 1)] {
            let path = self.tile_path(id, slug, zoom, px * 2 + qx as i32, py * 2 + qy as i32);
            let Ok(bytes) = std::fs::read(&path) else { continue };
            let Ok(img) = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png) else { continue };
            let img = img.to_rgba8();
            if img.width() != size || img.height() != size {
                continue;
            }
            for y in 0..half {
                for x in 0..half {
                    let samples = [
                        *img.get_pixel(x * 2, y * 2),
                        *img.get_pixel(x * 2 + 1, y * 2),
                        *img.get_pixel(x * 2, y * 2 + 1),
                        *img.get_pixel(x * 2 + 1, y * 2 + 1),
                    ];
                    let opaque: Vec<_> = samples.iter().filter(|s| s[3] != 0).collect();
                    let best = (!opaque.is_empty()).then(|| {
                        let n = opaque.len() as u32;
                        let mean = |c: usize| (opaque.iter().map(|s| s[c] as u32).sum::<u32>() / n) as u8;
                        image::Rgba([mean(0), mean(1), mean(2), 255])
                    });
                    if let Some(c) = best {
                        out.put_pixel(qx * half + x, qy * half + y, c);
                        any = true;
                    }
                }
            }
        }
        if !any {
            return Ok(false);
        }
        let mut png = Vec::new();
        out.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)?;
        let path = self.tile_path(id, slug, zoom + 1, px, py);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tmp = path.with_extension("png.tmp");
        std::fs::write(&tmp, &png)?;
        std::fs::rename(&tmp, &path)?;
        Ok(true)
    }

    /// Delete every tile and the overlay; game servers notice the new epoch and redraw everything.
    pub fn reset(&self, id: i64) -> std::io::Result<()> {
        let dir = self.server_dir(id);
        let epoch = self.epoch(id) + 1;
        if dir.exists() {
            std::fs::remove_dir_all(&dir)?;
        }
        std::fs::create_dir_all(&dir)?;
        std::fs::write(dir.join("epoch"), epoch.to_string())?;
        self.overlays.lock().unwrap().remove(&id);
        *self.stats.lock().unwrap().entry(id).or_default() = Stats::default();
        self.diagnostics.lock().unwrap().remove(&id);
        Ok(())
    }

    /// Changes whenever the map was reset, telling the game server to start over.
    pub fn epoch(&self, id: i64) -> u64 {
        std::fs::read_to_string(self.server_dir(id).join("epoch")).ok().and_then(|s| s.trim().parse().ok()).unwrap_or(0)
    }

    /// Remove everything for a deleted server.
    pub fn forget_server(&self, id: i64) {
        let _ = std::fs::remove_dir_all(self.server_dir(id));
        self.players.lock().unwrap().remove(&id);
        self.overlays.lock().unwrap().remove(&id);
        self.stats.lock().unwrap().remove(&id);
        self.diagnostics.lock().unwrap().remove(&id);
    }

    pub fn stats(&self, id: i64) -> Stats {
        self.stats.lock().unwrap().get(&id).cloned().unwrap_or_default()
    }

    pub fn set_diagnostics(&self, id: i64, value: Value) {
        self.diagnostics.lock().unwrap().insert(id, (Instant::now(), value));
    }

    pub fn diagnostics(&self, id: i64) -> Value {
        self.diagnostics
            .lock()
            .unwrap()
            .get(&id)
            .map(|(at, value)| {
                let mut value = value.clone();
                value["age_seconds"] = json!(at.elapsed().as_secs());
                value
            })
            .unwrap_or(Value::Null)
    }

    // ---- players -------------------------------------------------------------------------

    pub fn set_players(&self, id: i64, mut players: Vec<LivePlayer>) {
        players.truncate(MAX_PLAYERS);
        players.retain(|p| p.x.is_finite() && p.y.is_finite() && p.z.is_finite() && p.x.abs() < 3e7 && p.z.abs() < 3e7);
        for p in &mut players {
            p.name = p.name.chars().filter(|c| !c.is_control()).take(32).collect();
            p.uuid = p.uuid.chars().filter(|c| c.is_ascii_hexdigit() || *c == '-').take(36).collect();
            p.dimension = p.dimension.chars().filter(|c| !c.is_control()).take(48).collect();
        }
        self.players.lock().unwrap().insert(id, PlayersSnap { at: Instant::now(), players });
        self.stats.lock().unwrap().entry(id).or_default().last_players_at = Some(now_rfc3339());
    }

    pub fn live_players(&self, id: i64) -> Vec<LivePlayer> {
        self.players.lock().unwrap().get(&id).filter(|s| s.at.elapsed() < PLAYERS_FRESH).map(|s| s.players.clone()).unwrap_or_default()
    }

    /// Drop a player from every server's roster (account deleted).
    pub fn forget_player(&self, uuid: &str) {
        for snap in self.players.lock().unwrap().values_mut() {
            snap.players.retain(|p| !p.uuid.eq_ignore_ascii_case(uuid));
        }
    }

    // ---- overlay -------------------------------------------------------------------------

    /// The last overlay (claims, pins) a game server sent. Kept on disk so a panel restart doesn't blank the map.
    pub fn set_overlay(&self, id: i64, overlay: Value) {
        let dir = self.server_dir(id);
        if std::fs::create_dir_all(&dir).is_ok() {
            let _ = std::fs::write(dir.join("overlay.json"), overlay.to_string());
        }
        self.overlays.lock().unwrap().insert(id, overlay);
        self.stats.lock().unwrap().entry(id).or_default().last_overlay_at = Some(now_rfc3339());
    }

    pub fn overlay(&self, id: i64) -> Value {
        if let Some(v) = self.overlays.lock().unwrap().get(&id) {
            return v.clone();
        }
        let loaded = std::fs::read_to_string(self.server_dir(id).join("overlay.json"))
            .ok()
            .and_then(|s| serde_json::from_str::<Value>(&s).ok())
            .unwrap_or_else(|| json!({ "claims": [], "pins": [] }));
        self.overlays.lock().unwrap().insert(id, loaded.clone());
        loaded
    }

    // ---- tile tokens ---------------------------------------------------------------------

    /// A short-lived key that lets `<img>` tags load tiles without an Authorization header. Random, in memory only.
    pub fn issue_token(&self, id: i64) -> String {
        let mut bytes = [0u8; 24];
        rand::thread_rng().fill_bytes(&mut bytes);
        let token = hex::encode(bytes);
        let mut tokens = self.tokens.lock().unwrap();
        tokens.retain(|_, (_, at)| at.elapsed() < TOKEN_TTL);
        tokens.insert(token.clone(), (id, Instant::now()));
        token
    }

    pub fn token_ok(&self, id: i64, token: &str) -> bool {
        self.tokens.lock().unwrap().get(token).is_some_and(|(sid, at)| *sid == id && at.elapsed() < TOKEN_TTL)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PNG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];

    fn record(zoom: u8, x: i32, y: i32, payload: &[u8]) -> Vec<u8> {
        let mut out = vec![zoom];
        out.extend(x.to_be_bytes());
        out.extend(y.to_be_bytes());
        let mut png = PNG.to_vec();
        png.extend_from_slice(payload);
        out.extend((png.len() as u32).to_be_bytes());
        out.extend(png);
        out
    }

    #[test]
    fn dimension_ids_become_safe_folder_names() {
        assert_eq!(dim_slug("minecraft:overworld").as_deref(), Some("overworld"));
        assert_eq!(dim_slug("minecraft:the_nether").as_deref(), Some("the_nether"));
        assert_eq!(dim_slug("nether").as_deref(), Some("the_nether"));
        assert_eq!(dim_slug("mymod:caves").as_deref(), Some("mymod__caves"));
        assert_eq!(dim_slug("../etc"), None);
        assert_eq!(dim_slug("a/b"), None);
        assert_eq!(dim_slug(""), None);
        assert_eq!(dim_label("the_nether"), "The Nether");
        assert_eq!(dim_label("mymod__deep_dark"), "deep dark");
    }

    #[test]
    fn tile_batches_are_parsed_and_checked() {
        let mut body = record(0, -3, 7, b"abc");
        body.extend(record(2, 1, -1, b"zz"));
        let tiles = parse_tiles(&body).unwrap();
        assert_eq!(tiles.len(), 2);
        assert_eq!((tiles[0].zoom, tiles[0].x, tiles[0].y), (0, -3, 7));
        assert_eq!((tiles[1].zoom, tiles[1].x, tiles[1].y), (2, 1, -1));
        assert!(parse_tiles(&[]).unwrap().is_empty());
        assert!(parse_tiles(&body[..10]).is_err(), "truncated header");
        assert!(parse_tiles(&body[..20]).is_err(), "truncated body");
        assert!(parse_tiles(&record(9, 0, 0, b"x")).is_err(), "zoom too deep");
        assert!(parse_tiles(&record(0, i32::MAX, 0, b"x")).is_err(), "far outside any world");
        let mut not_png = record(0, 0, 0, b"x");
        not_png[13] = b'X';
        assert!(parse_tiles(&not_png).is_err());
    }

    #[test]
    fn tiles_are_stored_summarised_and_reset() {
        let dir = std::env::temp_dir().join(format!("velora-map-{}", rand::random::<u64>()));
        let map = WorldMap::new(&dir);
        let tiles = parse_tiles(&[record(0, -1, 2, b"a"), record(0, 3, 4, b"bb"), record(1, 0, 0, b"c")].concat()).unwrap();
        assert_eq!(map.store_tiles(5, "overworld", &tiles).unwrap(), 3);
        map.store_tiles(5, "the_nether", &tiles[..1]).unwrap();
        assert!(map.read_tile(5, "overworld", 0, -1, 2).unwrap().0.ends_with(b"a"));
        assert!(map.read_tile(5, "overworld", 0, 9, 9).is_none());
        let s = map.summary(5);
        let dims = s["dimensions"].as_array().unwrap();
        assert_eq!(dims[0]["slug"], "overworld");
        assert_eq!(dims[0]["tiles"], 2, "only zoom 0 counts");
        assert_eq!(dims[0]["bounds"]["min_x"], -1);
        assert_eq!(dims[0]["bounds"]["max_y"], 4);
        assert_eq!(dims[0]["id"], "minecraft:overworld");
        assert_eq!(dims[1]["slug"], "the_nether");
        assert_eq!(map.stats(5).tiles_received, 4);

        assert_eq!(map.epoch(5), 0);
        map.reset(5).unwrap();
        assert_eq!(map.epoch(5), 1);
        assert!(map.summary(5)["dimensions"].as_array().unwrap().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn solid_png(rgba: [u8; 4]) -> Vec<u8> {
        let img = image::RgbaImage::from_pixel(TILE_SIZE, TILE_SIZE, image::Rgba(rgba));
        let mut png = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).unwrap();
        png
    }

    #[test]
    fn missing_zoom_levels_are_built_from_full_resolution_tiles() {
        let dir = std::env::temp_dir().join(format!("velora-map-{}", rand::random::<u64>()));
        let map = WorldMap::new(&dir);
        // Only zoom 0 ever arrived: a viewer zoomed out would otherwise find nothing at all.
        let tiles = vec![
            TileRecord { zoom: 0, x: 0, y: 0, png: solid_png([10, 200, 30, 255]) },
            TileRecord { zoom: 0, x: 1, y: 0, png: solid_png([200, 180, 90, 255]) },
        ];
        map.store_tiles(7, "overworld", &tiles).unwrap();
        assert!(map.read_tile(7, "overworld", 1, 0, 0).is_none());

        assert_eq!(map.backfill_pyramids(), 6, "one merged tile on each of the six levels above zoom 0");
        let levels = map.summary(7)["dimensions"][0]["levels"].clone();
        assert_eq!(levels, json!([2, 1, 1, 1, 1, 1, 1]));

        // The zoom-1 tile (0, 0) holds both neighbours: the left one in its left half, the right one in its right half.
        let (png, _) = map.read_tile(7, "overworld", 1, 0, 0).unwrap();
        let img = image::load_from_memory_with_format(&png, image::ImageFormat::Png).unwrap().to_rgba8();
        assert_eq!(img.get_pixel(10, 10).0, [10, 200, 30, 255]);
        assert_eq!(img.get_pixel(200, 10).0, [200, 180, 90, 255]);
        assert_eq!(img.get_pixel(10, 200).0[3], 0, "nothing was drawn below");

        // Nothing changed, nothing is rebuilt; a newer tile refreshes the levels above it.
        assert_eq!(map.build_pyramid(7, "overworld", &[(1, 0)]), 0);
        std::thread::sleep(Duration::from_millis(20));
        map.store_tiles(7, "overworld", &[TileRecord { zoom: 0, x: 1, y: 0, png: solid_png([1, 2, 3, 255]) }]).unwrap();
        assert_eq!(map.build_pyramid(7, "overworld", &[(1, 0)]), 6);
        let (png, _) = map.read_tile(7, "overworld", 1, 0, 0).unwrap();
        let img = image::load_from_memory_with_format(&png, image::ImageFormat::Png).unwrap().to_rgba8();
        assert_eq!(img.get_pixel(200, 10).0, [1, 2, 3, 255]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn tokens_are_per_server_and_players_are_cleaned() {
        let map = WorldMap::new(&std::env::temp_dir());
        let t = map.issue_token(3);
        assert!(map.token_ok(3, &t));
        assert!(!map.token_ok(4, &t), "a token only opens its own server");
        assert!(!map.token_ok(3, "nope"));

        let p = |name: &str, x: f64| LivePlayer {
            uuid: "ab-12\u{7}zz".into(),
            name: format!("{name}\n"),
            dimension: "minecraft:overworld".into(),
            x,
            y: 64.0,
            z: 0.0,
            yaw: 0.0,
        };
        map.set_players(3, vec![p("Alex", 1.0), p("Bad", f64::NAN), p("Far", 1e9)]);
        let live = map.live_players(3);
        assert_eq!(live.len(), 1);
        assert_eq!(live[0].name, "Alex");
        assert_eq!(live[0].uuid, "ab-12", "only hex digits and dashes survive");
        map.forget_player("AB-12");
        assert!(map.live_players(3).is_empty());
    }
}
