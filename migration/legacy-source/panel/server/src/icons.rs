//! Icon library: ~140,000 Iconify glyphs from 37 packs, served one SVG at a time and searchable by name.
//!
//! The packs are built by `panel/icons/build.mjs` into gzip'd JSON files that are read on demand (a few stay in memory); the
//! names-only search index is built on first search. Without the data the picker is simply empty and everything else works.

use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, VecDeque},
    io::Read,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackMeta {
    pub id: String,
    pub label: String,
    pub palette: bool,
    pub license: String,
    #[serde(default)]
    pub homepage: String,
    pub total: usize,
}

#[derive(Deserialize)]
struct Glyph {
    body: String,
    width: Option<f64>,
    height: Option<f64>,
    left: Option<f64>,
    top: Option<f64>,
}
#[derive(Deserialize)]
struct Alias {
    parent: String,
}
#[derive(Deserialize)]
struct Pack {
    width: Option<f64>,
    height: Option<f64>,
    icons: HashMap<String, Glyph>,
    #[serde(default)]
    aliases: HashMap<String, Alias>,
}

const MAX_LOADED: usize = 4;

pub struct Library {
    dir: PathBuf,
    pub packs: Vec<PackMeta>,
    loaded: Mutex<VecDeque<(String, Arc<Pack>)>>,
    index: OnceLock<Vec<(u16, String)>>,
}

static LIBRARY: OnceLock<Arc<Library>> = OnceLock::new();

/// The library for `dir` (loaded once; later calls reuse it).
pub fn library(dir: &Path) -> Arc<Library> {
    LIBRARY.get_or_init(|| Arc::new(Library::open(dir))).clone()
}

fn valid_name(s: &str) -> bool {
    !s.is_empty() && s.len() <= 80 && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

impl Library {
    pub fn open(dir: &Path) -> Self {
        let packs = std::fs::read(dir.join("packs.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        Self { dir: dir.to_path_buf(), packs, loaded: Mutex::new(VecDeque::new()), index: OnceLock::new() }
    }

    fn pack(&self, id: &str) -> Option<Arc<Pack>> {
        if !valid_name(id) || !self.packs.iter().any(|p| p.id == id) {
            return None;
        }
        {
            let mut cache = self.loaded.lock().ok()?;
            if let Some(i) = cache.iter().position(|(k, _)| k == id) {
                let hit = cache.remove(i)?;
                cache.push_back(hit.clone());
                return Some(hit.1);
            }
        }
        let mut raw = Vec::new();
        flate2::read::GzDecoder::new(std::fs::File::open(self.dir.join(format!("{id}.json.gz"))).ok()?).read_to_end(&mut raw).ok()?;
        let pack: Arc<Pack> = Arc::new(serde_json::from_slice(&raw).ok()?);
        let mut cache = self.loaded.lock().ok()?;
        cache.push_back((id.to_string(), pack.clone()));
        while cache.len() > MAX_LOADED {
            cache.pop_front();
        }
        Some(pack)
    }

    /// The SVG for `pack:name`. `color` (hex without `#`) replaces `currentColor` in single-colour packs; palette packs keep theirs.
    pub fn svg(&self, pack_id: &str, name: &str, color: Option<&str>) -> Option<String> {
        if !valid_name(name) {
            return None;
        }
        let pack = self.pack(pack_id)?;
        let glyph = pack.icons.get(name).or_else(|| pack.aliases.get(name).and_then(|a| pack.icons.get(&a.parent)))?;
        let (w, h) = (glyph.width.or(pack.width).unwrap_or(16.0), glyph.height.or(pack.height).unwrap_or(16.0));
        let (l, t) = (glyph.left.unwrap_or(0.0), glyph.top.unwrap_or(0.0));
        let color = match color {
            Some(c) if matches!(c.len(), 3 | 6 | 8) && c.chars().all(|c| c.is_ascii_hexdigit()) => format!("#{c}"),
            _ => "#ffffff".into(),
        };
        let body = glyph.body.replace("currentColor", &color);
        Some(format!(r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="{l} {t} {w} {h}" width="{w}" height="{h}">{body}</svg>"#))
    }

    fn names(&self) -> &Vec<(u16, String)> {
        self.index.get_or_init(|| {
            let mut all = Vec::new();
            for (i, meta) in self.packs.iter().enumerate() {
                if let Some(pack) = self.pack(&meta.id) {
                    all.extend(pack.icons.keys().chain(pack.aliases.keys()).map(|n| (i as u16, n.clone())));
                }
            }
            all.sort_by(|a, b| a.1.cmp(&b.1).then(a.0.cmp(&b.0)));
            all
        })
    }

    /// Search names across all packs (or one). Every word must appear; exact and prefix hits rank first.
    /// Returns (total hits, one page of `(pack, name)`).
    pub fn search(&self, query: &str, pack: Option<&str>, offset: usize, limit: usize) -> (usize, Vec<(String, String)>) {
        let words: Vec<String> = query.to_lowercase().split(|c: char| !c.is_ascii_alphanumeric()).filter(|w| !w.is_empty()).map(String::from).collect();
        let only = pack.and_then(|p| self.packs.iter().position(|m| m.id == p)).map(|i| i as u16);
        if pack.is_some() && only.is_none() {
            return (0, vec![]);
        }
        let joined = words.join("-");
        let mut hits: Vec<(u8, usize)> = Vec::new();
        for (i, (p, name)) in self.names().iter().enumerate() {
            if only.is_some_and(|o| o != *p) {
                continue;
            }
            if !words.iter().all(|w| name.contains(w.as_str())) {
                continue;
            }
            let rank = if words.is_empty() || *name == joined {
                0
            } else if name.starts_with(&joined) {
                1
            } else if name.split('-').any(|part| words.first().is_some_and(|w| part == w)) {
                2
            } else {
                3
            };
            hits.push((rank, i));
        }
        hits.sort_by_key(|(r, i)| (*r, self.names()[*i].1.len(), *i));
        let page = hits
            .iter()
            .skip(offset)
            .take(limit)
            .map(|(_, i)| {
                let (p, n) = &self.names()[*i];
                (self.packs[*p as usize].id.clone(), n.clone())
            })
            .collect();
        (hits.len(), page)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_pack(dir: &Path, id: &str, json: &str) {
        let mut gz = flate2::write::GzEncoder::new(std::fs::File::create(dir.join(format!("{id}.json.gz"))).unwrap(), flate2::Compression::fast());
        gz.write_all(json.as_bytes()).unwrap();
        gz.finish().unwrap();
    }

    #[test]
    fn serves_colours_aliases_and_searches_across_packs() {
        let t = tempfile::tempdir().unwrap();
        write_pack(t.path(), "lucide", r##"{"width":24,"height":24,"icons":{"sword":{"body":"<path stroke=\"currentColor\" d=\"M0 0\"/>"},"swords":{"body":"<g/>","width":32}},"aliases":{"blade":{"parent":"sword"}}}"##);
        write_pack(t.path(), "logos", r##"{"width":256,"height":256,"icons":{"sword-logo":{"body":"<path fill=\"#f00\"/>"}}}"##);
        std::fs::write(
            t.path().join("packs.json"),
            r#"[{"id":"lucide","label":"Lucide","palette":false,"license":"ISC","total":3},{"id":"logos","label":"Logos","palette":true,"license":"CC0","total":1}]"#,
        )
        .unwrap();
        let lib = Library::open(t.path());

        let svg = lib.svg("lucide", "sword", Some("ffd700")).unwrap();
        assert!(svg.contains(r##"stroke="#ffd700""##) && svg.contains(r#"viewBox="0 0 24 24""#));
        assert!(lib.svg("lucide", "sword", Some("zz<script>")).unwrap().contains("#ffffff"), "bad colours fall back");
        assert!(lib.svg("lucide", "blade", None).is_some(), "aliases resolve");
        assert!(lib.svg("logos", "sword-logo", Some("00f")).unwrap().contains("#f00"), "palette packs keep their colours");
        assert!(lib.svg("lucide", "../x", None).is_none() && lib.svg("nope", "sword", None).is_none() && lib.svg("lucide", "missing", None).is_none());

        let (total, hits) = lib.search("sword", None, 0, 10);
        assert_eq!(total, 3);
        assert_eq!(hits[0], ("lucide".to_string(), "sword".to_string()), "exact match first");
        assert_eq!(lib.search("sword", Some("logos"), 0, 10).1, vec![("logos".to_string(), "sword-logo".to_string())]);
        assert_eq!(lib.search("sword", None, 2, 10).1.len(), 1, "paging");
        assert_eq!(lib.search("logo sword", None, 0, 10).0, 1, "every word must match");
        assert_eq!(lib.search("", Some("lucide"), 0, 50).0, 3, "empty query browses a pack");
        assert_eq!(lib.search("sword", Some("bogus"), 0, 10).0, 0);
    }
}
