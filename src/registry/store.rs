// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::platform_defaults::app_config_file;
use crate::registry::errors::RegistryError;
use crate::registry::model::ModlistRegistry;

#[derive(Debug, Clone)]
pub struct RegistryStore {
    path: PathBuf,
}

impl RegistryStore {
    pub fn new_default() -> Self {
        Self::new(app_config_file("modlists.json", "."))
    }

    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> Result<ModlistRegistry, RegistryError> {
        let raw = match std::fs::read_to_string(&self.path) {
            Ok(raw) => raw,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                return Ok(ModlistRegistry::default());
            }
            Err(source) => {
                return Err(RegistryError::Io {
                    path: self.path.clone(),
                    source,
                });
            }
        };

        serde_json::from_str::<ModlistRegistry>(&raw).map_err(|err| RegistryError::Corrupt {
            path: self.path.clone(),
            message: err.to_string(),
        })
    }

    pub fn save(&self, registry: &ModlistRegistry) -> Result<(), RegistryError> {
        let raw =
            serde_json::to_string_pretty(registry).map_err(|err| RegistryError::Serialize {
                path: self.path.clone(),
                message: err.to_string(),
            })?;

        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| RegistryError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }

        let temp_path = temp_registry_path(&self.path);
        let mut temp_file =
            std::fs::File::create(&temp_path).map_err(|source| RegistryError::Io {
                path: temp_path.clone(),
                source,
            })?;
        temp_file
            .write_all(raw.as_bytes())
            .map_err(|source| RegistryError::Io {
                path: temp_path.clone(),
                source,
            })?;
        temp_file.sync_all().map_err(|source| RegistryError::Io {
            path: temp_path.clone(),
            source,
        })?;
        drop(temp_file);

        replace_registry_file(&temp_path, &self.path).map_err(|source| RegistryError::Io {
            path: self.path.clone(),
            source,
        })
    }

    pub fn backup_corrupt_file(&self) -> std::io::Result<PathBuf> {
        let backup_path = corrupt_backup_path(&self.path);
        std::fs::rename(&self.path, &backup_path)?;
        Ok(backup_path)
    }
}

fn corrupt_backup_path(path: &Path) -> PathBuf {
    let mut backup_path = path.to_path_buf();
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_else(|| "modlists.json".into());
    let unix_timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    backup_path.set_file_name(format!("{file_name}.corrupt-{unix_timestamp}"));
    backup_path
}

fn temp_registry_path(path: &Path) -> PathBuf {
    let mut temp_path = path.to_path_buf();
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_else(|| "modlists.json".into());
    temp_path.set_file_name(format!("{file_name}.tmp"));
    temp_path
}

#[cfg(not(target_os = "windows"))]
fn replace_registry_file(temp_path: &Path, final_path: &Path) -> std::io::Result<()> {
    std::fs::rename(temp_path, final_path)
}

#[cfg(target_os = "windows")]
fn replace_registry_file(temp_path: &Path, final_path: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };

    let temp_wide: Vec<u16> = temp_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let final_wide: Vec<u16> = final_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let result = unsafe {
        MoveFileExW(
            temp_wide.as_ptr(),
            final_wide.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if result == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::model::{Game, ModlistEntry};

    fn temp_registry_path(name: &str) -> PathBuf {
        let unique = format!("bio-registry-store-test-{name}-{}", std::process::id());
        std::env::temp_dir().join(unique).join("modlists.json")
    }

    #[test]
    fn missing_file_loads_empty_registry() {
        let path = temp_registry_path("missing");
        let _ = std::fs::remove_file(&path);
        let store = RegistryStore::new(path);

        let registry = store.load().expect("load missing registry");

        assert_eq!(registry, ModlistRegistry::default());
    }

    #[test]
    fn save_then_load_preserves_registry() {
        let path = temp_registry_path("round-trip");
        let _ = std::fs::remove_file(&path);
        let store = RegistryStore::new(path.clone());
        let registry = ModlistRegistry {
            entries: vec![ModlistEntry {
                id: "demo".to_string(),
                name: "Demo".to_string(),
                game: Game::BG2EE,
                ..Default::default()
            }],
            ..Default::default()
        };

        store.save(&registry).expect("save registry");
        let loaded = store.load().expect("load registry");

        assert_eq!(loaded, registry);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn save_replaces_existing_registry() {
        let path = temp_registry_path("replace-existing");
        let _ = std::fs::remove_file(&path);
        let store = RegistryStore::new(path.clone());
        let old_registry = ModlistRegistry {
            entries: vec![ModlistEntry {
                id: "old".to_string(),
                name: "Old".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        };
        let new_registry = ModlistRegistry {
            entries: vec![ModlistEntry {
                id: "new".to_string(),
                name: "New".to_string(),
                game: Game::EET,
                ..Default::default()
            }],
            ..Default::default()
        };

        store.save(&old_registry).expect("save old registry");
        store.save(&new_registry).expect("replace registry");
        let loaded = store.load().expect("load replaced registry");

        assert_eq!(loaded, new_registry);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn successful_save_leaves_no_temp_file() {
        let path = temp_registry_path("no-temp");
        let temp_path = super::temp_registry_path(&path);
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(&temp_path);
        let store = RegistryStore::new(path.clone());

        store
            .save(&ModlistRegistry::default())
            .expect("save registry");

        assert!(path.is_file());
        assert!(!temp_path.exists());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn invalid_json_returns_corrupt() {
        let path = temp_registry_path("corrupt");
        let parent = path.parent().expect("parent");
        std::fs::create_dir_all(parent).expect("create temp parent");
        std::fs::write(&path, "{ not json").expect("write invalid json");
        let store = RegistryStore::new(path.clone());

        let err = store.load().expect_err("corrupt registry should fail");

        match err {
            RegistryError::Corrupt { path: err_path, .. } => assert_eq!(err_path, path),
            other => panic!("expected corrupt error, got {other:?}"),
        }
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn save_io_error_preserves_path_and_source() {
        let path = std::env::temp_dir().join(format!(
            "bio-registry-store-test-file-parent-{}",
            std::process::id()
        ));
        std::fs::write(&path, "not a directory").expect("create file parent");
        let registry_path = path.join("modlists.json");
        let store = RegistryStore::new(registry_path);

        let err = store
            .save(&ModlistRegistry::default())
            .expect_err("save should fail when parent is a file");

        match err {
            RegistryError::Io {
                path: err_path,
                source: _,
            } => assert_eq!(err_path, path),
            other => panic!("expected io error, got {other:?}"),
        }
        let _ = std::fs::remove_file(path);
    }
}
