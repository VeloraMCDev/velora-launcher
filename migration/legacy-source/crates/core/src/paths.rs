use std::path::{Path, PathBuf};

/// On-disk layout. Versions, libraries, assets and Java runtimes are shared
/// between instances so a second instance of the same version costs almost
/// no disk space.
#[derive(Debug, Clone)]
pub struct Layout {
    pub root: PathBuf,
}

impl Layout {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
    pub fn libraries(&self) -> PathBuf {
        self.root.join("libraries")
    }
    pub fn versions(&self) -> PathBuf {
        self.root.join("versions")
    }
    pub fn version_dir(&self, id: &str) -> PathBuf {
        self.versions().join(id)
    }
    pub fn version_json(&self, id: &str) -> PathBuf {
        self.version_dir(id).join(format!("{id}.json"))
    }
    pub fn version_jar(&self, id: &str) -> PathBuf {
        self.version_dir(id).join(format!("{id}.jar"))
    }
    pub fn assets(&self) -> PathBuf {
        self.root.join("assets")
    }
    pub fn runtimes(&self) -> PathBuf {
        self.root.join("runtimes")
    }
    pub fn natives(&self, version_id: &str) -> PathBuf {
        self.root.join("natives").join(version_id)
    }
    pub fn instances(&self) -> PathBuf {
        self.root.join("instances")
    }
    pub fn instance_dir(&self, id: &str) -> PathBuf {
        self.instances().join(sanitize_id(id))
    }
    pub fn cache(&self) -> PathBuf {
        self.root.join("cache")
    }
}

/// Keep instance ids filesystem-safe no matter what the panel sends.
pub fn sanitize_id(id: &str) -> String {
    let s: String = id.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' { c } else { '_' }).collect();
    let s = s.trim_matches('.').to_string();
    if s.is_empty() {
        "_".into()
    } else {
        s
    }
}

/// Join a server-provided relative path onto `base`, refusing anything that
/// could escape it (`..`, absolute paths, drive letters).
pub fn safe_join(base: &Path, rel: &str) -> Option<PathBuf> {
    let rel = rel.replace('\\', "/");
    if rel.starts_with('/') || rel.contains(':') {
        return None;
    }
    let mut out = base.to_path_buf();
    for part in rel.split('/') {
        match part {
            "" | "." => continue,
            ".." => return None,
            p => out.push(p),
        }
    }
    if out == base {
        None
    } else {
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_join_blocks_traversal() {
        let base = Path::new("/game");
        assert_eq!(safe_join(base, "mods/a.jar"), Some(PathBuf::from("/game/mods/a.jar")));
        assert_eq!(safe_join(base, "config\\x.toml"), Some(PathBuf::from("/game/config/x.toml")));
        assert!(safe_join(base, "../evil").is_none());
        assert!(safe_join(base, "mods/../../evil").is_none());
        assert!(safe_join(base, "/etc/passwd").is_none());
        assert!(safe_join(base, "C:/Windows").is_none());
        assert!(safe_join(base, "").is_none());
    }

    #[test]
    fn ids_are_sanitized() {
        assert_eq!(sanitize_id("my pack/../x"), "my_pack_.._x");
        assert_eq!(sanitize_id(".."), "_");
    }
}
