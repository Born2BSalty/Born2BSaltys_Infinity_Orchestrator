// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `WorkspaceStore` — load/save for per-modlist
// `<config_dir>/modlists/<id>/workspace.json` files.
//
// Per Phase 3 P3.T3:
//   - `new_for_id(id)`           → resolves the per-modlist path inside the
//                                  platform config dir.
//   - `load()`                   → returns `Err(Corrupt)` for unparseable
//                                  workspace files. Returns `Err(Io)` for
//                                  permission / disk failures. **Missing file**
//                                  returns `Ok(default)` for first-write
//                                  callers; the registry-entry-says-file-exists
//                                  callers must check existence themselves.
//                                  Phase 3's `dev_seed` is fine with `Ok(default)`
//                                  because it always saves immediately afterwards.
//   - `save(&state)`             → atomic via temp file + rename; creates the
//                                  `modlists/<id>/` parent dir on first write.
//
// **Decision tree per P3.T3 spec note:** "if the registry has no entry for
// this id, the workspace store should not be queried (caller bug)." We keep
// `load()` permissive — returning `Ok(default)` for missing files — so the
// caller can decide what to do (treat missing as fresh, or report an error).
// The registry holds the source of truth for "this modlist exists."
//
// SPEC: §13.1, §13.14.

use std::path::{Path, PathBuf};

use crate::platform_defaults::app_config_dir;
use crate::registry::errors::RegistryError;
use crate::registry::workspace_model::ModlistWorkspaceState;

/// Per-modlist directory name inside the platform config dir.
const MODLISTS_DIR: &str = "modlists";
/// Per-modlist file name.
const WORKSPACE_FILE_NAME: &str = "workspace.json";

#[derive(Debug, Clone)]
pub struct WorkspaceStore {
    path: PathBuf,
}

impl WorkspaceStore {
    /// Resolve the workspace file path for a given modlist id.
    ///
    /// Layout: `<app_config_dir>/modlists/<id>/workspace.json`. Falls back to
    /// the current working directory if the platform config dir is unavailable.
    pub fn new_for_id(modlist_id: &str) -> Self {
        let base = app_config_dir().unwrap_or_else(|| PathBuf::from("."));
        let path = base
            .join(MODLISTS_DIR)
            .join(modlist_id)
            .join(WORKSPACE_FILE_NAME);
        Self { path }
    }

    /// Override path (used by `dev_seed` tests + custom-rooted callers).
    pub fn new_with_path(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// On-disk file path for diagnostics + the terminal error UI.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Load the workspace state.
    ///
    /// - Missing file → `Ok(ModlistWorkspaceState::default())`.
    /// - Present-but-unreadable → `Err(Corrupt)`.
    /// - IO failure → `Err(Io)`.
    pub fn load(&self) -> Result<ModlistWorkspaceState, RegistryError> {
        let raw = match std::fs::read_to_string(&self.path) {
            Ok(s) => s,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Ok(ModlistWorkspaceState::default());
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

    /// Persist the workspace state atomically via temp file + rename.
    pub fn save(&self, state: &ModlistWorkspaceState) -> Result<(), RegistryError> {
        let raw = serde_json::to_string_pretty(state)?;

        if let Some(parent) = self.path.parent()
            && !parent.as_os_str().is_empty()
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
        let mut state = ModlistWorkspaceState::default();
        state.last_share_code = Some("X".to_string());
        store.save(&state).expect("save");
        assert!(path.exists(), "workspace file written");
        assert!(path.parent().unwrap().exists(), "modlists subdir created");
        let loaded = store.load().expect("load");
        assert_eq!(loaded.last_share_code.as_deref(), Some("X"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn missing_workspace_returns_default() {
        let root = temp_root("missing");
        let path = root.join("modlists/MISSING/workspace.json");
        let store = WorkspaceStore::new_with_path(&path);
        let state = store.load().expect("ok on missing");
        assert!(state.is_empty());
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
}
