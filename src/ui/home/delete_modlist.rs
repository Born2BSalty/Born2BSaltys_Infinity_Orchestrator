// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use tracing::warn;

use crate::registry::model::ModlistEntry;
use crate::registry::operations;
use crate::registry::store_workspace;
use crate::ui::orchestrator::orchestrator_app::{OrchestratorApp, PendingFolderDelete};

pub(crate) fn delete_confirmed_modlist(orchestrator: &mut OrchestratorApp, entry: &ModlistEntry) {
    let name = entry.name.clone();
    match operations::remove_entry_and_save(
        &entry.id,
        &orchestrator.registry_store,
        &mut orchestrator.registry,
    ) {
        Ok(target) => {
            orchestrator.workspace_state.remove(&entry.id);
            orchestrator.workspace_stores.remove(&entry.id);
            if let Err(err) = store_workspace::remove_modlist_data_dir(&entry.id) {
                warn!(
                    target = "orchestrator",
                    "delete: removing the data dir for {} failed: {err}", entry.id
                );
            }
            orchestrator.persistence_cycle.last_saved_registry = orchestrator.registry.clone();
            match target {
                Some(target) => {
                    orchestrator
                        .notification_manager
                        .info(format!("Deleting \"{}\"\u{2026}", target.name));
                    let rx = operations::spawn_delete_folder_worker(target.dest);
                    orchestrator
                        .pending_folder_deletes
                        .push(PendingFolderDelete {
                            modlist_name: target.name,
                            rx,
                        });
                }
                None => {
                    orchestrator
                        .notification_manager
                        .success(format!("Deleted \"{name}\""));
                }
            }
        }
        Err(err) => {
            orchestrator
                .notification_manager
                .error(format!("Couldn't delete \"{name}\": {err}"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::model::{Game, ModlistState};

    #[test]
    fn deleting_a_modlist_removes_its_entry_workspace_and_data_dir() {
        let mut app = OrchestratorApp::new_isolated_for_test("delete-modlist");
        let entry = ModlistEntry {
            id: "DEL000000001".to_string(),
            name: "Doomed".to_string(),
            game: Game::EET,
            destination_folder: String::new(),
            state: ModlistState::InProgress,
            ..Default::default()
        };
        app.registry.entries.push(entry.clone());
        let data_dir = store_workspace::modlist_data_dir("DEL000000001");
        std::fs::create_dir_all(&data_dir).expect("seed the data dir");
        app.workspace_state.insert(
            "DEL000000001".to_string(),
            crate::registry::workspace_model::ModlistWorkspaceState::default(),
        );
        app.workspace_stores.insert(
            "DEL000000001".to_string(),
            crate::registry::store_workspace::WorkspaceStore::new_for_id("DEL000000001"),
        );

        delete_confirmed_modlist(&mut app, &entry);

        assert!(app.registry.find("DEL000000001").is_none());
        assert!(!app.workspace_state.contains_key("DEL000000001"));
        assert!(!app.workspace_stores.contains_key("DEL000000001"));
        assert!(!data_dir.exists());
        assert_eq!(app.pending_folder_deletes.len(), 0);
    }
}
