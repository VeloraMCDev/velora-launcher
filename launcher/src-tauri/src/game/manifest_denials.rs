use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use uuid::Uuid;

#[derive(Default)]
pub(crate) struct ManifestDenials {
    denied: Mutex<HashMap<PathBuf, Uuid>>,
}

impl ManifestDenials {
    pub(crate) fn observe(&self, path: &Path) -> Option<Uuid> {
        self.denied.lock().unwrap().get(path).copied()
    }

    pub(crate) fn deny(&self, path: &Path, invalidate: impl FnOnce() -> Result<()>) -> Result<()> {
        let mut denied = self.denied.lock().unwrap();
        denied.insert(path.to_owned(), Uuid::new_v4());
        invalidate()
    }

    pub(crate) fn refresh(&self, path: &Path, observed: Option<Uuid>, publish: impl FnOnce() -> Result<()>) -> Result<()> {
        let mut denied = self.denied.lock().unwrap();
        anyhow::ensure!(denied.get(path).copied() == observed, "manifest authorization changed while fetching");
        publish()?;
        denied.remove(path);
        Ok(())
    }

    pub(crate) fn read<Value>(&self, path: &Path, read: impl FnOnce() -> Result<Value>) -> Result<Value> {
        let denied = self.denied.lock().unwrap();
        anyhow::ensure!(!denied.contains_key(path), "offline manifest denied; online refresh required");
        read()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn failed_invalidation_fences_only_its_instance_until_successful_refresh() {
        let denials = ManifestDenials::default();
        let path = Path::new("synthetic-instance/manifest.json");
        assert!(denials.deny(path, || anyhow::bail!("synthetic invalidation failure")).is_err());
        let reads = Cell::new(0);
        assert!(denials
            .read(path, || {
                reads.set(reads.get() + 1);
                Ok(())
            })
            .is_err());
        assert_eq!(reads.get(), 0);
        denials.read(Path::new("other-instance/manifest.json"), || Ok(())).unwrap();
        let observed = denials.observe(path);
        assert!(denials.refresh(path, observed, || anyhow::bail!("synthetic publication failure")).is_err());
        assert!(denials.read(path, || Ok(())).is_err());
        denials.refresh(path, observed, || Ok(())).unwrap();
        denials.read(path, || Ok(())).unwrap();
    }

    #[test]
    fn success_started_before_new_denial_cannot_publish_or_clear_it() {
        let denials = ManifestDenials::default();
        let path = Path::new("synthetic-instance/manifest.json");
        for prior_denial in [false, true] {
            if prior_denial {
                denials.deny(path, || Ok(())).unwrap();
            }
            let observed = denials.observe(path);
            denials.deny(path, || Ok(())).unwrap();
            let publications = Cell::new(0);
            assert!(denials
                .refresh(path, observed, || {
                    publications.set(publications.get() + 1);
                    Ok(())
                })
                .is_err());
            assert_eq!(publications.get(), 0);
            assert!(denials.read(path, || Ok(())).is_err());
            denials.refresh(path, denials.observe(path), || Ok(())).unwrap();
        }
    }
}
