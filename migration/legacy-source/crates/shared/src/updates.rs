use serde::{Deserialize, Serialize};

/// A published installer. Panel URLs are relative; repository URLs are absolute.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LauncherUpdate {
    pub version: String,
    pub notes: String,
    pub url: String,
    pub size: u64,
    #[serde(default)]
    pub sha256: Option<String>,
}

pub fn release_version(value: &str) -> Option<semver::Version> {
    if value.len() > 128 {
        return None;
    }
    semver::Version::parse(value.trim().trim_start_matches('v')).ok()
}

pub fn newer_release(candidate: &str, current: &str) -> bool {
    match (release_version(candidate), release_version(current)) {
        (Some(next), Some(installed)) => next.cmp_precedence(&installed).is_gt(),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compares_release_precedence_without_build_metadata() {
        assert!(newer_release("v0.10.0", "0.9.9"));
        assert!(newer_release("1.0.0", "1.0.0-beta.2"));
        assert!(newer_release("1.0.0-beta.10", "1.0.0-beta.2"));
        assert!(!newer_release("1.0.0-beta.2", "1.0.0"));
        assert!(!newer_release("1.0.0+build.2", "1.0.0+build.1"));
        assert!(!newer_release("not-a-version", "0.9.0"));
    }
}
