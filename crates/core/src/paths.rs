use std::path::PathBuf;

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

pub use velora_platform_utils::paths::{safe_join, sanitize_id};
