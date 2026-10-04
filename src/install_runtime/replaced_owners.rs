// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use tracing::{info, warn};

use crate::registry::errors::RegistryError;
use crate::registry::model::ModlistEntry;
use crate::registry::operations;
use crate::registry::store_workspace;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;

pub(crate) struct HeldOwners {
    pub(crate) entries: Vec<(usize, ModlistEntry)>,
    pub(crate) replacement_id: Option<String>,
}

pub(crate) fn hold_owners(
    orchestrator: &mut OrchestratorApp,
    old_ids: &[String],
) -> Result<HeldOwners, RegistryError> {
    let mut snapshots: Vec<(usize, ModlistEntry)> = Vec::with_capacity(old_ids.len());
    for old_id in old_ids {
        let Some(index) = orchestrator
            .registry
            .entries
            .iter()
            .position(|e| e.id == *old_id)
        else {
            return Err(RegistryError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("modlist {old_id} is no longer in the registry"),
            )));
        };
        snapshots.push((index, orchestrator.registry.entries[index].clone()));
    }

    let mut removed: Vec<(usize, ModlistEntry)> = Vec::new();
    for (old_index, old_entry) in snapshots {
        let old_id = old_entry.id.clone();
        let remove_result = operations::remove_entry_keep_folder(
            &old_id,
            &orchestrator.registry_store,
            &mut orchestrator.registry,
        );
        if let Err(err) = remove_result {
            removed.push((old_index, old_entry));
            restore_owners(
                orchestrator,
                HeldOwners {
                    entries: removed,
                    replacement_id: None,
                },
            );
            return Err(err);
        }
        orchestrator.workspace_state.remove(&old_id);
        orchestrator.workspace_stores.remove(&old_id);
        removed.push((old_index, old_entry));
    }

    orchestrator.persistence_cycle.last_saved_registry = orchestrator.registry.clone();

    Ok(HeldOwners {
        entries: removed,
        replacement_id: None,
    })
}

pub(crate) fn restore_owners(orchestrator: &mut OrchestratorApp, held: HeldOwners) {
    if let Some(replacement_id) = held.replacement_id {
        orchestrator
            .registry
            .entries
            .retain(|e| e.id != replacement_id);
        orchestrator.workspace_state.remove(&replacement_id);
        orchestrator.workspace_stores.remove(&replacement_id);
    }
    let restored_ids: Vec<String> = held.entries.iter().map(|(_, e)| e.id.clone()).collect();
    reinsert_removed(orchestrator, &held.entries);
    if let Err(err) = orchestrator.registry_store.save(&orchestrator.registry) {
        warn!(
            target = "orchestrator",
            "restore_owners: registry persist failed: {err}"
        );
    }
    orchestrator
        .persistence_cycle
        .mark_registry_dirty(std::time::Instant::now());
    info!(
        target = "orchestrator",
        "restore_owners: restored {restored_ids:?} after a failed replace"
    );
}

pub(crate) fn finalize_owners(held: HeldOwners, started_id: &str) {
    if held.entries.iter().any(|(_, e)| e.id == started_id) {
        warn!(
            target = "orchestrator",
            "finalize_owners: the started install is one of the held owners \
             {started_id}; nothing to remove"
        );
        return;
    }
    for (_, entry) in held.entries {
        match store_workspace::remove_modlist_data_dir(&entry.id) {
            Ok(()) => {
                info!(
                    target = "orchestrator",
                    "finalize_owners: the held owner {}'s data dir was removed \
                     after the new list started successfully",
                    entry.id
                );
            }
            Err(err) => {
                warn!(
                    target = "orchestrator",
                    "finalize_owners: removing the data dir for the held owner \
                     {} failed: {err}",
                    entry.id
                );
            }
        }
    }
}

fn reinsert_removed(orchestrator: &mut OrchestratorApp, removed: &[(usize, ModlistEntry)]) {
    let mut ordered: Vec<&(usize, ModlistEntry)> = removed.iter().collect();
    ordered.sort_by_key(|(index, _)| *index);
    for (index, entry) in ordered {
        if !orchestrator
            .registry
            .entries
            .iter()
            .any(|e| e.id == entry.id)
        {
            orchestrator.registry.entries.insert(
                (*index).min(orchestrator.registry.entries.len()),
                entry.clone(),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::model::{Game, ModlistState};

    fn entry(id: &str, name: &str) -> ModlistEntry {
        ModlistEntry {
            id: id.to_string(),
            name: name.to_string(),
            game: Game::EET,
            state: ModlistState::InProgress,
            ..Default::default()
        }
    }

    fn app_with_three() -> OrchestratorApp {
        let mut app = OrchestratorApp::new_isolated_for_test("replaced-owners");
        app.registry.entries.push(entry("A00000000001", "A"));
        app.registry.entries.push(entry("B00000000002", "B"));
        app.registry.entries.push(entry("C00000000003", "C"));
        app.workspace_state.insert(
            "A00000000001".to_string(),
            crate::registry::workspace_model::ModlistWorkspaceState::default(),
        );
        app.workspace_state.insert(
            "C00000000003".to_string(),
            crate::registry::workspace_model::ModlistWorkspaceState::default(),
        );
        app.workspace_stores.insert(
            "A00000000001".to_string(),
            crate::registry::store_workspace::WorkspaceStore::new_for_id("A00000000001"),
        );
        app.workspace_stores.insert(
            "C00000000003".to_string(),
            crate::registry::store_workspace::WorkspaceStore::new_for_id("C00000000003"),
        );
        app
    }

    #[test]
    fn hold_removes_the_owners_and_their_in_memory_workspaces() {
        let mut app = app_with_three();

        let held = hold_owners(
            &mut app,
            &["A00000000001".to_string(), "C00000000003".to_string()],
        )
        .expect("hold ok");

        assert_eq!(app.registry.entries.len(), 1);
        assert_eq!(app.registry.entries[0].id, "B00000000002");
        assert!(!app.workspace_state.contains_key("A00000000001"));
        assert!(!app.workspace_state.contains_key("C00000000003"));
        assert!(!app.workspace_stores.contains_key("A00000000001"));
        assert!(!app.workspace_stores.contains_key("C00000000003"));
        assert_eq!(held.entries.len(), 2);
        assert_eq!(held.entries[0].0, 0);
        assert_eq!(held.entries[1].0, 2);
    }

    #[test]
    fn hold_fails_before_removing_anything_when_an_id_is_missing() {
        let mut app = app_with_three();

        let result = hold_owners(
            &mut app,
            &["A00000000001".to_string(), "MISSING00001".to_string()],
        );

        assert!(result.is_err());
        assert_eq!(app.registry.entries.len(), 3);
        assert!(app.registry.find("A00000000001").is_some());
    }

    #[test]
    fn restore_reinserts_at_the_original_indices_and_drops_the_replacement() {
        let mut app = app_with_three();
        let mut held = hold_owners(
            &mut app,
            &["A00000000001".to_string(), "C00000000003".to_string()],
        )
        .expect("hold ok");
        app.registry.entries.push(entry("REPLACE000001", "New"));
        held.replacement_id = Some("REPLACE000001".to_string());

        restore_owners(&mut app, held);

        let ids: Vec<&str> = app.registry.entries.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["A00000000001", "B00000000002", "C00000000003"]);
        assert!(app.registry.find("REPLACE000001").is_none());
    }

    #[test]
    fn finalize_removes_each_held_data_dir() {
        let mut app = app_with_three();
        let held = hold_owners(
            &mut app,
            &["A00000000001".to_string(), "C00000000003".to_string()],
        )
        .expect("hold ok");
        let dir_a = crate::registry::store_workspace::modlist_data_dir("A00000000001");
        let dir_c = crate::registry::store_workspace::modlist_data_dir("C00000000003");
        std::fs::create_dir_all(&dir_a).expect("mkdir a");
        std::fs::create_dir_all(&dir_c).expect("mkdir c");

        finalize_owners(held, "UNRELATED0001");

        assert!(!dir_a.exists());
        assert!(!dir_c.exists());
    }

    #[test]
    fn finalize_leaves_everything_when_the_started_list_is_a_held_owner() {
        let mut app = app_with_three();
        let held = hold_owners(
            &mut app,
            &["A00000000001".to_string(), "C00000000003".to_string()],
        )
        .expect("hold ok");
        let dir_a = crate::registry::store_workspace::modlist_data_dir("A00000000001");
        let dir_c = crate::registry::store_workspace::modlist_data_dir("C00000000003");
        std::fs::create_dir_all(&dir_a).expect("mkdir a");
        std::fs::create_dir_all(&dir_c).expect("mkdir c");

        finalize_owners(held, "A00000000001");

        assert!(dir_a.exists());
        assert!(dir_c.exists());
    }
}
