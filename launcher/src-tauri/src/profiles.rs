use crate::persistence;
use anyhow::Result;
use std::path::Path;
use velora_shared::SkinProfile;

pub fn load(data_dir: &Path) -> Result<Vec<SkinProfile>> {
    Ok(persistence::read_json(&data_dir.join("skin_profiles.json"))?.unwrap_or_default())
}

pub fn update(data_dir: &Path, change: impl FnOnce(&mut Vec<SkinProfile>)) -> Result<Vec<SkinProfile>> {
    let mut profiles = load(data_dir)?;
    change(&mut profiles);
    persistence::atomic_write_json(&data_dir.join("skin_profiles.json"), &profiles)?;
    Ok(profiles)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Directory(std::path::PathBuf);

    impl Directory {
        fn new() -> Self {
            let directory = Self(std::env::temp_dir().join(format!("velora-profile-fixture-{}", uuid::Uuid::new_v4())));
            std::fs::create_dir(&directory.0).unwrap();
            directory
        }
    }

    impl Drop for Directory {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }

    #[test]
    fn legacy_profiles_retain_ids_defaults_and_content_on_update() {
        let directory = Directory::new();
        let path = directory.0.join("skin_profiles.json");
        std::fs::write(&path, br#"[{"id":"synthetic-old","name":"Original","skin_data":null}]"#).unwrap();
        let original = load(&directory.0).unwrap().remove(0);
        assert_eq!(original.skin_model, "classic");
        let mut additional = original.clone();
        additional.id = "synthetic-new".into();
        let updated = update(&directory.0, |profiles| profiles.push(additional.clone())).unwrap();
        assert_eq!(updated, vec![original, additional]);
        assert_eq!(load(&directory.0).unwrap(), updated);
        assert_eq!(std::fs::read_dir(&directory.0).unwrap().count(), 1);
    }

    #[test]
    fn corrupt_profiles_are_not_treated_as_an_empty_wardrobe() {
        let directory = Directory::new();
        let path = directory.0.join("skin_profiles.json");
        std::fs::write(&path, b"synthetic truncated profiles").unwrap();
        assert!(load(&directory.0).is_err());
        assert!(update(&directory.0, |_| panic!("must reject corrupt input before modifying")).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"synthetic truncated profiles");
    }

    #[test]
    fn missing_profiles_allow_first_save_but_nonregular_paths_fail_closed() {
        let directory = Directory::new();
        assert!(load(&directory.0).unwrap().is_empty());
        assert!(update(&directory.0, |_| {}).unwrap().is_empty());
        let path = directory.0.join("skin_profiles.json");
        std::fs::remove_file(&path).unwrap();
        std::fs::create_dir(&path).unwrap();
        std::fs::write(path.join("sentinel"), b"synthetic preserved").unwrap();
        assert!(update(&directory.0, |_| {}).is_err());
        assert_eq!(std::fs::read(path.join("sentinel")).unwrap(), b"synthetic preserved");
    }
}
