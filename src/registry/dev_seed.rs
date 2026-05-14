// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::PathBuf;

use chrono::Utc;

use crate::registry::errors::RegistryError;
use crate::registry::model::{Game, ModlistEntry, ModlistRegistry, ModlistState};
use crate::registry::store::RegistryStore;
use crate::registry::store_workspace::WorkspaceStore;
use crate::registry::workspace_model::ModlistWorkspaceState;

pub fn seed_demo_entry(
    registry: &mut ModlistRegistry,
    store: &RegistryStore,
) -> Result<ModlistEntry, RegistryError> {
    let id = next_demo_id(registry);
    let now = Utc::now();
    let entry = ModlistEntry {
        id: id.clone(),
        name: id.clone(),
        game: Game::BG2EE,
        state: ModlistState::InProgress,
        creation_date: now,
        last_touched_date: now,
        mod_count: 3,
        component_count: 12,
        workspace_file_relpath: PathBuf::from(format!("modlists/{id}/workspace.json")),
        ..Default::default()
    };

    WorkspaceStore::new_for_id(&id).save(&ModlistWorkspaceState::default())?;
    let mut next_registry = registry.clone();
    next_registry.entries.push(entry.clone());
    store.save(&next_registry)?;
    *registry = next_registry;

    Ok(entry)
}

fn next_demo_id(registry: &ModlistRegistry) -> String {
    let mut index = registry.entries.len() + 1;
    loop {
        let candidate = format!("demo-modlist-{index}");
        if !registry.entries.iter().any(|entry| entry.id == candidate) {
            return candidate;
        }
        index += 1;
    }
}
