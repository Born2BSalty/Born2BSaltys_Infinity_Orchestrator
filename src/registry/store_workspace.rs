// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::{Path, PathBuf};

use crate::platform_defaults::app_config_dir;
use crate::registry::errors::RegistryError;
use crate::registry::workspace_model::ModlistWorkspaceState;

const MODLISTS_DIR: &str = "modlists";

const WORKSPACE_FILE_NAME: &str = "workspace.json";

#[must_use]
pub fn modlist_data_dir(modlist_id: &str) -> PathBuf {
    app_config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(MODLISTS_DIR)
        .join(modlist_id)
}

pub fn remove_modlist_data_dir(modlist_id: &str) -> Result<(), std::io::Error> {
    let trimmed = modlist_id.trim();
    if trimmed.is_empty() || !trimmed.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "modlist id must be alphanumeric",
        ));
    }
    match std::fs::remove_dir_all(modlist_data_dir(trimmed)) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err),
    }
}

#[derive(Debug, Clone)]
pub struct WorkspaceStore {
    path: PathBuf,
}

impl WorkspaceStore {
    #[must_use]
    pub fn new_for_id(modlist_id: &str) -> Self {
        let path = modlist_data_dir(modlist_id).join(WORKSPACE_FILE_NAME);
        Self { path }
    }

    pub fn new_with_path(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> Result<ModlistWorkspaceState, RegistryError> {
        let raw = match std::fs::read_to_string(&self.path) {
            Ok(s) => s,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Err(RegistryError::corrupt(
                    self.path.clone(),
                    "workspace file is missing".to_string(),
                ));
            }
            Err(err) => return Err(RegistryError::Io(err)),
        };
        match serde_json::from_str::<ModlistWorkspaceState>(&raw) {
            Ok(state) => Ok(state),
            Err(parse_err) => Err(RegistryError::corrupt(
                self.path.clone(),
                parse_err.to_string(),
            )),
        }
    }

    pub fn save(&self, state: &ModlistWorkspaceState) -> Result<(), RegistryError> {
        let raw = serde_json::to_string_pretty(state)?;

        if let Some(parent) = self
            .path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent).map_err(RegistryError::Io)?;
        }

        let tmp_path = self.path.with_extension("json.tmp");
        std::fs::write(&tmp_path, raw.as_bytes()).map_err(RegistryError::Io)?;
        std::fs::rename(&tmp_path, &self.path).map_err(RegistryError::Io)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static C: AtomicU64 = AtomicU64::new(0);

    fn temp_root(label: &str) -> PathBuf {
        let n = C.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "bio_workspace_test_{}_{}_{}",
            std::process::id(),
            n,
            label
        ))
    }

    #[test]
    fn save_creates_modlists_subdir() {
        let root = temp_root("subdir");
        let path = root.join("modlists/ABCDEFGHIJKL/workspace.json");
        let store = WorkspaceStore::new_with_path(&path);
        let state = ModlistWorkspaceState {
            last_share_code: Some("X".to_string()),
            ..Default::default()
        };
        store.save(&state).expect("save");
        assert!(path.exists(), "workspace file written");
        assert!(path.parent().unwrap().exists(), "modlists subdir created");
        let loaded = store.load().expect("load");
        assert_eq!(loaded.last_share_code.as_deref(), Some("X"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn missing_workspace_returns_corrupt_error() {
        let root = temp_root("missing");
        let path = root.join("modlists/MISSING/workspace.json");
        let store = WorkspaceStore::new_with_path(&path);
        match store.load() {
            Err(RegistryError::Corrupt { .. }) => {}
            other => panic!("expected Corrupt for missing file, got {other:?}"),
        }
    }

    #[test]
    fn corrupt_workspace_returns_corrupt_error() {
        let root = temp_root("corrupt");
        let path = root.join("modlists/CORRUPT/workspace.json");
        std::fs::create_dir_all(path.parent().unwrap()).expect("mkdir");
        std::fs::write(&path, b"{ not json").expect("write garbage");
        let store = WorkspaceStore::new_with_path(&path);
        match store.load() {
            Err(RegistryError::Corrupt { .. }) => {}
            other => panic!("expected Corrupt, got {other:?}"),
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    fn with_config_dir_override<F: FnOnce(&PathBuf)>(label: &str, f: F) {
        let root = temp_root(label);
        std::fs::create_dir_all(&root).expect("mkdir root");
        crate::platform_defaults::set_config_dir_override(Some(root.clone()));
        f(&root);
        crate::platform_defaults::clear_config_dir_override_if(&root);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn remove_data_dir_refuses_a_blank_id() {
        with_config_dir_override("refuse-blank", |root| {
            let keep = root.join("modlists").join("KEEP00000001");
            std::fs::create_dir_all(&keep).expect("mkdir keep");

            for id in ["", "  "] {
                match remove_modlist_data_dir(id) {
                    Err(err) => assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput),
                    other => panic!("expected InvalidInput, got {other:?}"),
                }
            }
            assert!(keep.exists(), "a sibling folder must survive a refusal");
        });
    }

    #[test]
    fn remove_data_dir_refuses_a_path_like_id() {
        with_config_dir_override("refuse-path-like", |_root| {
            for id in ["../x", "A/B"] {
                match remove_modlist_data_dir(id) {
                    Err(err) => assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput),
                    other => panic!("expected InvalidInput, got {other:?}"),
                }
            }
        });
    }

    #[test]
    fn remove_data_dir_missing_folder_is_ok() {
        with_config_dir_override("missing-ok", |_root| {
            remove_modlist_data_dir("NOFOLDER0001").expect("missing folder is Ok");
        });
    }

    #[test]
    fn remove_data_dir_removes_the_folder() {
        with_config_dir_override("removes", |root| {
            let gone = root.join("modlists").join("GONE00000001");
            std::fs::create_dir_all(&gone).expect("mkdir gone");
            std::fs::write(gone.join("workspace.json"), b"{}").expect("seed workspace.json");
            let keep = root.join("modlists").join("KEEP00000001");
            std::fs::create_dir_all(&keep).expect("mkdir keep");

            remove_modlist_data_dir("GONE00000001").expect("remove ok");

            assert!(!gone.exists(), "the target folder is gone");
            assert!(keep.exists(), "the sibling folder survives");
        });
    }
}
