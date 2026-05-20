// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::platform_defaults::app_config_file;
use crate::registry::errors::RegistryError;
use crate::registry::model::ModlistRegistry;

const REGISTRY_FILE_NAME: &str = "modlists.json";

#[derive(Debug, Clone)]
pub struct RegistryStore {
    path: PathBuf,
}

impl RegistryStore {
    #[must_use]
    pub fn new_default() -> Self {
        let path = app_config_file(REGISTRY_FILE_NAME, ".");
        Self { path }
    }

    pub fn new_with_path(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> Result<ModlistRegistry, RegistryError> {
        let raw = match std::fs::read_to_string(&self.path) {
            Ok(s) => s,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Ok(ModlistRegistry::default());
            }
            Err(err) => return Err(RegistryError::Io(err)),
        };
        match serde_json::from_str::<ModlistRegistry>(&raw) {
            Ok(registry) => Ok(registry),
            Err(parse_err) => Err(RegistryError::corrupt(
                self.path.clone(),
                parse_err.to_string(),
            )),
        }
    }

    pub fn save(&self, registry: &ModlistRegistry) -> Result<(), RegistryError> {
        let raw = serde_json::to_string_pretty(registry)?;

        if let Some(parent) = self
            .path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent).map_err(RegistryError::Io)?;
        }

        let tmp_path = self.tmp_path();
        std::fs::write(&tmp_path, raw.as_bytes()).map_err(RegistryError::Io)?;
        std::fs::rename(&tmp_path, &self.path).map_err(RegistryError::Io)?;
        Ok(())
    }

    pub fn backup_corrupt_file(&self) -> std::io::Result<PathBuf> {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let new_path = self.path.with_extension(format!("json.corrupt-{ts}"));
        std::fs::rename(&self.path, &new_path)?;
        Ok(new_path)
    }

    fn tmp_path(&self) -> PathBuf {
        self.path.with_extension("json.tmp")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_path(label: &str) -> PathBuf {
        let n = TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir()
            .join(format!(
                "bio_registry_test_{}_{}_{}",
                std::process::id(),
                n,
                label
            ))
            .with_extension("json")
    }

    #[test]
    fn load_missing_file_returns_empty_registry() {
        let path = temp_path("missing");
        let _ = std::fs::remove_file(&path);
        let store = RegistryStore::new_with_path(&path);
        let r = store.load().expect("ok on missing");
        assert!(r.is_empty());
    }

    #[test]
    fn round_trip_save_then_load() {
        let path = temp_path("round_trip");
        let store = RegistryStore::new_with_path(&path);
        let mut r = ModlistRegistry::default();
        r.entries.push(crate::registry::model::ModlistEntry {
            id: "0123456789AB".to_string(),
            name: "demo".to_string(),
            ..Default::default()
        });
        store.save(&r).expect("save");
        let r2 = store.load().expect("load");
        assert_eq!(r2.entries.len(), 1);
        assert_eq!(r2.entries[0].id, "0123456789AB");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn corrupt_file_returns_corrupt_error() {
        let path = temp_path("corrupt");
        std::fs::write(&path, b"{ not json").expect("write garbage");
        let store = RegistryStore::new_with_path(&path);
        match store.load() {
            Err(RegistryError::Corrupt { path: p, .. }) => {
                assert_eq!(p, path);
            }
            other => panic!("expected Corrupt, got {other:?}"),
        }

        let still_there = std::fs::read_to_string(&path).expect("file still there");
        assert_eq!(still_there, "{ not json");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn save_creates_parent_dirs() {
        let dir = std::env::temp_dir().join(format!(
            "bio_registry_parent_{}_{}",
            std::process::id(),
            TMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let path = dir.join("nested").join("modlists.json");
        let _ = std::fs::remove_dir_all(&dir);
        let store = RegistryStore::new_with_path(&path);
        store.save(&ModlistRegistry::default()).expect("save");
        assert!(path.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn backup_corrupt_file_renames_to_unix_ts_suffix() {
        let path = temp_path("backup");
        std::fs::write(&path, b"{ bad").expect("write");
        let store = RegistryStore::new_with_path(&path);
        let new_path = store.backup_corrupt_file().expect("rename");
        assert!(!path.exists(), "original removed");
        assert!(new_path.exists(), "backup created");
        let name = new_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        assert!(
            name.contains("corrupt-"),
            "name `{name}` has timestamp suffix"
        );
        let _ = std::fs::remove_file(&new_path);
    }

    #[test]
    fn save_uses_tmp_file_then_rename() {
        let path = temp_path("atomic");
        let store = RegistryStore::new_with_path(&path);
        store.save(&ModlistRegistry::default()).expect("save");
        assert!(!path.with_extension("json.tmp").exists());
        let _ = std::fs::remove_file(&path);
    }
}
