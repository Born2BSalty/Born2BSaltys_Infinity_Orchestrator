// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::registry::errors::RegistryError;
use crate::registry::model::ModlistRegistry;
use crate::registry::store::RegistryStore;
use crate::registry::store_workspace::WorkspaceStore;
use crate::registry::workspace_model::ModlistWorkspaceState;

pub const DEFAULT_DEBOUNCE_MS: u64 = 1000;

const REGISTRY_KEY: &str = "::registry::";

#[derive(Debug)]
pub struct RegistryPersistenceCycle {
    pub last_saved_registry: ModlistRegistry,

    pub last_saved_workspaces: HashMap<String, ModlistWorkspaceState>,

    pub debounce_ms: u64,

    pub last_dirty_at: HashMap<String, Instant>,

    pub workspace_extract_debug_count: u64,
}

impl Default for RegistryPersistenceCycle {
    fn default() -> Self {
        Self {
            last_saved_registry: ModlistRegistry::default(),
            last_saved_workspaces: HashMap::new(),
            debounce_ms: DEFAULT_DEBOUNCE_MS,
            last_dirty_at: HashMap::new(),
            workspace_extract_debug_count: 0,
        }
    }
}

impl RegistryPersistenceCycle {
    #[must_use]
    pub fn new_with_baseline(loaded: ModlistRegistry) -> Self {
        Self {
            last_saved_registry: loaded,
            ..Self::default()
        }
    }

    pub fn mark_registry_dirty(&mut self, now: Instant) {
        self.last_dirty_at.insert(REGISTRY_KEY.to_string(), now);
    }

    pub fn mark_workspace_dirty(&mut self, modlist_id: &str, now: Instant) {
        self.last_dirty_at.insert(modlist_id.to_string(), now);
    }

    pub const fn note_workspace_extract(&mut self) {
        self.workspace_extract_debug_count = self.workspace_extract_debug_count.saturating_add(1);
    }

    pub fn persist_registry_if_needed(
        &mut self,
        in_memory: &ModlistRegistry,
        store: &RegistryStore,
        now: Instant,
    ) -> Result<bool, RegistryError> {
        if in_memory == &self.last_saved_registry {
            return Ok(false);
        }
        if !self.is_debounce_elapsed(REGISTRY_KEY, now) {
            return Ok(false);
        }
        store.save(in_memory)?;
        self.last_saved_registry = in_memory.clone();
        self.last_dirty_at.remove(REGISTRY_KEY);
        Ok(true)
    }

    pub fn persist_workspace_if_needed(
        &mut self,
        modlist_id: &str,
        in_memory: &ModlistWorkspaceState,
        store: &WorkspaceStore,
        now: Instant,
    ) -> Result<bool, RegistryError> {
        let needs_write = self.last_saved_workspaces.get(modlist_id) != Some(in_memory);
        if !needs_write {
            return Ok(false);
        }
        if !self.is_debounce_elapsed(modlist_id, now) {
            return Ok(false);
        }
        store.save(in_memory)?;
        self.last_saved_workspaces
            .insert(modlist_id.to_string(), in_memory.clone());
        self.last_dirty_at.remove(modlist_id);
        Ok(true)
    }

    pub fn flush_all(
        &mut self,
        in_memory_registry: &ModlistRegistry,
        registry_store: &RegistryStore,
        in_memory_workspaces: &HashMap<String, ModlistWorkspaceState>,
        workspace_stores: &HashMap<String, WorkspaceStore>,
    ) -> Vec<RegistryError> {
        let mut errs = Vec::new();

        if in_memory_registry == &self.last_saved_registry {
            self.last_dirty_at.remove(REGISTRY_KEY);
        } else {
            match registry_store.save(in_memory_registry) {
                Ok(()) => {
                    self.last_saved_registry = in_memory_registry.clone();
                    self.last_dirty_at.remove(REGISTRY_KEY);
                }
                Err(err) => errs.push(err),
            }
        }

        for (id, ws) in in_memory_workspaces {
            let Some(store) = workspace_stores.get(id) else {
                continue;
            };
            let differs = self.last_saved_workspaces.get(id) != Some(ws);
            if !differs {
                continue;
            }
            match store.save(ws) {
                Ok(()) => {
                    self.last_saved_workspaces.insert(id.clone(), ws.clone());
                    self.last_dirty_at.remove(id);
                }
                Err(err) => errs.push(err),
            }
        }

        errs
    }

    fn is_debounce_elapsed(&self, key: &str, now: Instant) -> bool {
        self.last_dirty_at.get(key).is_none_or(|t| {
            now.saturating_duration_since(*t) >= Duration::from_millis(self.debounce_ms)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::model::ModlistEntry;
    use crate::registry::store_workspace::WorkspaceStore;

    fn temp_registry_path(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "bio_cycle_test_{}_{}_modlists.json",
            std::process::id(),
            label
        ))
    }

    #[test]
    fn unchanged_registry_does_not_save() {
        let path = temp_registry_path("unchanged");
        let _ = std::fs::remove_file(&path);
        let store = RegistryStore::new_with_path(&path);
        let registry = ModlistRegistry::default();
        let mut cycle = RegistryPersistenceCycle::new_with_baseline(registry.clone());

        let wrote = cycle
            .persist_registry_if_needed(&registry, &store, Instant::now())
            .expect("ok");
        assert!(!wrote);
        assert!(!path.exists(), "file not created when nothing changed");
    }

    #[test]
    fn changed_registry_saves_after_debounce() {
        let path = temp_registry_path("changed");
        let _ = std::fs::remove_file(&path);
        let store = RegistryStore::new_with_path(&path);
        let baseline = ModlistRegistry::default();
        let mut cycle = RegistryPersistenceCycle::new_with_baseline(baseline);
        cycle.debounce_ms = 0;

        let mut in_memory = ModlistRegistry::default();
        in_memory.entries.push(ModlistEntry {
            id: "TESTID000000".to_string(),
            ..Default::default()
        });
        cycle.mark_registry_dirty(Instant::now());

        let wrote = cycle
            .persist_registry_if_needed(&in_memory, &store, Instant::now())
            .expect("ok");
        assert!(wrote);
        assert!(path.exists());

        let wrote2 = cycle
            .persist_registry_if_needed(&in_memory, &store, Instant::now())
            .expect("ok");
        assert!(!wrote2);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn workspace_persistence_runs_per_id() {
        let root = std::env::temp_dir().join(format!("bio_cycle_ws_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let path = root.join("modlists/IDX/workspace.json");
        let store = WorkspaceStore::new_with_path(&path);
        let mut cycle = RegistryPersistenceCycle {
            debounce_ms: 0,
            ..Default::default()
        };
        let ws = ModlistWorkspaceState {
            last_share_code: Some("ABC".to_string()),
            ..Default::default()
        };
        cycle.mark_workspace_dirty("IDX", Instant::now());
        let wrote = cycle
            .persist_workspace_if_needed("IDX", &ws, &store, Instant::now())
            .expect("ok");
        assert!(wrote);
        assert!(path.exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn workspace_extract_debug_count_starts_zero_and_only_bumps_when_noted() {
        let mut cycle = RegistryPersistenceCycle::default();
        assert_eq!(cycle.workspace_extract_debug_count, 0);

        let root = std::env::temp_dir().join(format!("bio_h1_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let store = WorkspaceStore::new_with_path(root.join("modlists/H1/workspace.json"));
        let ws = ModlistWorkspaceState::default();
        let _ = cycle.persist_workspace_if_needed("H1", &ws, &store, Instant::now());
        assert_eq!(
            cycle.workspace_extract_debug_count, 0,
            "the cadence itself must never bump the H1 counter"
        );

        cycle.note_workspace_extract();
        cycle.note_workspace_extract();
        assert_eq!(cycle.workspace_extract_debug_count, 2);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn flush_all_is_idempotent_when_nothing_changes() {
        let path = temp_registry_path("idempotent");
        let _ = std::fs::remove_file(&path);
        let store = RegistryStore::new_with_path(&path);
        let registry = ModlistRegistry::default();
        let mut cycle = RegistryPersistenceCycle::new_with_baseline(registry.clone());
        let errs = cycle.flush_all(&registry, &store, &HashMap::new(), &HashMap::new());
        assert!(errs.is_empty());
        assert!(!path.exists(), "no save when nothing changed");
    }
}
