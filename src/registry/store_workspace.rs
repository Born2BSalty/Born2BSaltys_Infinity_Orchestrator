// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::{Path, PathBuf};

use crate::platform_defaults::app_config_file;
use crate::registry::errors::RegistryError;
use crate::registry::workspace_model::ModlistWorkspaceState;

#[derive(Debug, Clone)]
pub struct WorkspaceStore {
    path: PathBuf,
}

impl WorkspaceStore {
    pub fn new_for_id(modlist_id: &str) -> Self {
        Self::new(app_config_file(
            &format!("modlists/{modlist_id}/workspace.json"),
            ".",
        ))
    }

    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> Result<ModlistWorkspaceState, RegistryError> {
        let raw = match std::fs::read_to_string(&self.path) {
            Ok(raw) => raw,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                return Err(RegistryError::Corrupt {
                    path: self.path.clone(),
                    message: "expected workspace file is missing".to_string(),
                });
            }
            Err(source) => {
                return Err(RegistryError::Io {
                    path: self.path.clone(),
                    source,
                });
            }
        };

        serde_json::from_str::<ModlistWorkspaceState>(&raw).map_err(|err| RegistryError::Corrupt {
            path: self.path.clone(),
            message: err.to_string(),
        })
    }

    pub fn save(&self, workspace: &ModlistWorkspaceState) -> Result<(), RegistryError> {
        let raw =
            serde_json::to_string_pretty(workspace).map_err(|err| RegistryError::Serialize {
                path: self.path.clone(),
                message: err.to_string(),
            })?;

        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| RegistryError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }

        std::fs::write(&self.path, raw).map_err(|source| RegistryError::Io {
            path: self.path.clone(),
            source,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::workspace_model::{ComponentRef, PromptOverride};
    use std::collections::HashMap;

    fn temp_workspace_path(name: &str) -> PathBuf {
        let unique = format!("bio-workspace-store-test-{name}-{}", std::process::id());
        std::env::temp_dir()
            .join(unique)
            .join("modlists")
            .join("demo")
            .join("workspace.json")
    }

    #[test]
    fn populated_workspace_round_trips() {
        let path = temp_workspace_path("round-trip");
        let _ = std::fs::remove_file(&path);
        let store = WorkspaceStore::new(path.clone());
        let mut prompt_overrides = HashMap::new();
        prompt_overrides.insert(
            "demo:0".to_string(),
            PromptOverride {
                answer: "y".to_string(),
            },
        );
        let workspace = ModlistWorkspaceState {
            order_bgee: vec![ComponentRef {
                tp2: "DEMO.TP2".to_string(),
                id: 0,
                language: 0,
            }],
            prompt_overrides,
            last_share_code: Some("BIO-MODLIST-V1 demo".to_string()),
            ..Default::default()
        };

        store.save(&workspace).expect("save workspace");
        let loaded = store.load().expect("load workspace");

        assert_eq!(loaded, workspace);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn missing_expected_workspace_returns_corrupt() {
        let path = temp_workspace_path("missing");
        let _ = std::fs::remove_file(&path);
        let store = WorkspaceStore::new(path.clone());

        let err = store.load().expect_err("missing workspace should fail");

        match err {
            RegistryError::Corrupt { path: err_path, .. } => assert_eq!(err_path, path),
            other => panic!("expected corrupt error, got {other:?}"),
        }
    }

    #[test]
    fn save_creates_modlist_parent_directory() {
        let path = temp_workspace_path("creates-parent");
        let parent = path.parent().expect("parent").to_path_buf();
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir_all(path.ancestors().nth(3).expect("test root"));
        let store = WorkspaceStore::new(path.clone());

        store
            .save(&ModlistWorkspaceState::default())
            .expect("save workspace");

        assert!(parent.is_dir());
        assert!(path.is_file());
        let _ = std::fs::remove_file(path);
    }
}
