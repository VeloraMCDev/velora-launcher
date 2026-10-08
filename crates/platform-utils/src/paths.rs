use std::path::{Path, PathBuf};

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
