// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::registry::errors::RegistryError;
use crate::registry::model::ModlistRegistry;
use crate::registry::store::RegistryStore;
use crate::registry::store_workspace::WorkspaceStore;
use crate::registry::workspace_model::ModlistWorkspaceState;

const DEFAULT_DEBOUNCE: Duration = Duration::from_secs(1);

#[derive(Debug, Clone)]
pub struct RegistryPersistenceCycle {
    last_saved_registry: ModlistRegistry,
    last_saved_workspaces: HashMap<String, ModlistWorkspaceState>,
    debounce: Duration,
    registry_last_dirty_at: Option<Instant>,
    workspace_last_dirty_at: HashMap<String, Instant>,
}

impl RegistryPersistenceCycle {
    #[must_use]
    pub fn new(last_saved_registry: ModlistRegistry) -> Self {
        Self::with_debounce(last_saved_registry, DEFAULT_DEBOUNCE)
    }

    #[must_use]
    pub fn with_debounce(last_saved_registry: ModlistRegistry, debounce: Duration) -> Self {
        Self {
            last_saved_registry,
            last_saved_workspaces: HashMap::new(),
            debounce,
            registry_last_dirty_at: None,
            workspace_last_dirty_at: HashMap::new(),
        }
    }

    pub fn persist_registry_if_needed(
        &mut self,
        in_memory: &ModlistRegistry,
        store: &RegistryStore,
    ) -> Result<(), RegistryError> {
        if in_memory == &self.last_saved_registry {
            self.registry_last_dirty_at = None;
            return Ok(());
        }

        let now = Instant::now();
        let dirty_at = self.registry_last_dirty_at.get_or_insert(now);
        if now.duration_since(*dirty_at) >= self.debounce {
            self.flush_registry(in_memory, store)?;
        }
        Ok(())
    }

    pub fn persist_workspace_if_needed(
        &mut self,
        modlist_id: &str,
        in_memory: &ModlistWorkspaceState,
        store: &WorkspaceStore,
    ) -> Result<(), RegistryError> {
        if self.last_saved_workspaces.get(modlist_id) == Some(in_memory) {
            self.workspace_last_dirty_at.remove(modlist_id);
            return Ok(());
        }

        let now = Instant::now();
        let dirty_at = self
            .workspace_last_dirty_at
            .entry(modlist_id.to_string())
            .or_insert(now);
        if now.duration_since(*dirty_at) >= self.debounce {
            self.flush_workspace(modlist_id, in_memory, store)?;
        }
        Ok(())
    }

    pub fn flush_registry(
        &mut self,
        in_memory: &ModlistRegistry,
        store: &RegistryStore,
    ) -> Result<(), RegistryError> {
        if in_memory != &self.last_saved_registry {
            store.save(in_memory)?;
            self.last_saved_registry = in_memory.clone();
        }
        self.registry_last_dirty_at = None;
        Ok(())
    }

    pub fn flush_workspace(
        &mut self,
        modlist_id: &str,
        in_memory: &ModlistWorkspaceState,
        store: &WorkspaceStore,
    ) -> Result<(), RegistryError> {
        if self.last_saved_workspaces.get(modlist_id) != Some(in_memory) {
            store.save(in_memory)?;
            self.last_saved_workspaces
                .insert(modlist_id.to_string(), in_memory.clone());
        }
        self.workspace_last_dirty_at.remove(modlist_id);
        Ok(())
    }

    pub fn flush_all(
        &mut self,
        in_memory_registry: &ModlistRegistry,
        registry_store: &RegistryStore,
        in_memory_workspaces: &HashMap<String, ModlistWorkspaceState>,
        workspace_stores: &HashMap<String, WorkspaceStore>,
    ) -> Result<(), RegistryError> {
        self.flush_registry(in_memory_registry, registry_store)?;
        for (modlist_id, workspace) in in_memory_workspaces {
            if let Some(store) = workspace_stores.get(modlist_id) {
                self.flush_workspace(modlist_id, workspace, store)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::model::{Game, ModlistEntry};
    use crate::registry::workspace_model::ComponentRef;

    fn temp_registry_path(name: &str) -> std::path::PathBuf {
        let unique = format!("bio-registry-cycle-test-{name}-{}", std::process::id());
        std::env::temp_dir().join(unique).join("modlists.json")
    }

    fn temp_workspace_path(name: &str, id: &str) -> std::path::PathBuf {
        let unique = format!("bio-registry-cycle-test-{name}-{}", std::process::id());
        std::env::temp_dir()
            .join(unique)
            .join("modlists")
            .join(id)
            .join("workspace.json")
    }

    fn registry_with_id(id: &str) -> ModlistRegistry {
        ModlistRegistry {
            entries: vec![ModlistEntry {
                id: id.to_string(),
                name: id.to_string(),
                game: Game::BG2EE,
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    fn workspace_with_tp2(tp2: &str) -> ModlistWorkspaceState {
        ModlistWorkspaceState {
            order_bgee: vec![ComponentRef {
                tp2: tp2.to_string(),
                id: 0,
                language: 0,
            }],
            ..Default::default()
        }
    }

    #[test]
    fn unchanged_registry_does_not_write() {
        let path = temp_registry_path("unchanged-registry");
        let _ = std::fs::remove_file(&path);
        let store = RegistryStore::new(path.clone());
        let registry = ModlistRegistry::default();
        let mut cycle = RegistryPersistenceCycle::with_debounce(registry.clone(), Duration::ZERO);

        cycle
            .persist_registry_if_needed(&registry, &store)
            .expect("persist registry");

        assert!(!path.exists());
    }

    #[test]
    fn changed_registry_with_zero_debounce_writes_once() {
        let path = temp_registry_path("changed-registry");
        let _ = std::fs::remove_file(&path);
        let store = RegistryStore::new(path.clone());
        let registry = registry_with_id("demo");
        let mut cycle =
            RegistryPersistenceCycle::with_debounce(ModlistRegistry::default(), Duration::ZERO);

        cycle
            .persist_registry_if_needed(&registry, &store)
            .expect("persist registry");
        cycle
            .persist_registry_if_needed(&registry, &store)
            .expect("persist unchanged registry");

        assert_eq!(store.load().expect("load registry"), registry);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn changed_workspace_with_zero_debounce_writes_once() {
        let path = temp_workspace_path("changed-workspace", "demo");
        let _ = std::fs::remove_file(&path);
        let store = WorkspaceStore::new(path.clone());
        let workspace = workspace_with_tp2("DEMO.TP2");
        let mut cycle =
            RegistryPersistenceCycle::with_debounce(ModlistRegistry::default(), Duration::ZERO);

        cycle
            .persist_workspace_if_needed("demo", &workspace, &store)
            .expect("persist workspace");
        cycle
            .persist_workspace_if_needed("demo", &workspace, &store)
            .expect("persist unchanged workspace");

        assert_eq!(store.load().expect("load workspace"), workspace);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn flush_all_saves_changed_registry_and_workspaces() {
        let registry_path = temp_registry_path("flush-all");
        let workspace_path = temp_workspace_path("flush-all", "demo");
        let _ = std::fs::remove_file(&registry_path);
        let _ = std::fs::remove_file(&workspace_path);
        let registry_store = RegistryStore::new(registry_path.clone());
        let workspace_store = WorkspaceStore::new(workspace_path.clone());
        let registry = registry_with_id("demo");
        let workspace = workspace_with_tp2("DEMO.TP2");
        let mut workspaces = HashMap::new();
        workspaces.insert("demo".to_string(), workspace.clone());
        let mut stores = HashMap::new();
        stores.insert("demo".to_string(), workspace_store.clone());
        let mut cycle =
            RegistryPersistenceCycle::with_debounce(ModlistRegistry::default(), Duration::ZERO);

        cycle
            .flush_all(&registry, &registry_store, &workspaces, &stores)
            .expect("flush all");

        assert_eq!(registry_store.load().expect("load registry"), registry);
        assert_eq!(workspace_store.load().expect("load workspace"), workspace);
        let _ = std::fs::remove_file(registry_path);
        let _ = std::fs::remove_file(workspace_path);
    }

    #[test]
    fn debounce_prevents_immediate_registry_write() {
        let path = temp_registry_path("debounce");
        let _ = std::fs::remove_file(&path);
        let store = RegistryStore::new(path.clone());
        let registry = registry_with_id("demo");
        let mut cycle = RegistryPersistenceCycle::with_debounce(
            ModlistRegistry::default(),
            Duration::from_mins(1),
        );

        cycle
            .persist_registry_if_needed(&registry, &store)
            .expect("persist registry");

        assert!(!path.exists());
    }
}
