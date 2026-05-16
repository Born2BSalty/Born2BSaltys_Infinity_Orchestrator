// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `dev_seed` — Phase 3 dev-mode-only registry seeding helper.
//
// Per P3.T8: invoked from `home_stub.rs` when `dev_mode == true`. Creates a
// real `ModlistEntry` in `in-progress` state, writes a default
// `ModlistWorkspaceState` for it, and saves both to disk immediately. The
// home stub displays a toast on success.
//
// This is a real exercise of the data layer (not a mock) — restarting the
// app keeps the seeded entry.
//
// SPEC: §13.1 (registry CRUD; this is just exercising it without UI yet).

// rationale: doc-paragraph-length is a subjective style lint; the existing
// doc wording is intentional (Cat 3).
#![allow(clippy::too_long_first_doc_paragraph)]

use std::path::PathBuf;

use chrono::Utc;

use crate::registry::errors::RegistryError;
use crate::registry::ids::new_modlist_id;
use crate::registry::model::{Game, ModlistEntry, ModlistRegistry, ModlistState};
use crate::registry::store::RegistryStore;
use crate::registry::store_workspace::WorkspaceStore;
use crate::registry::workspace_model::ModlistWorkspaceState;

/// Seed a demo entry into the live registry and write its initial workspace
/// file. The caller (Home stub) passes an `&mut ModlistRegistry` (the
/// orchestrator's in-memory copy) plus the registry store and a workspace
/// store factory.
///
/// Returns the newly-created entry on success so the caller can render a
/// toast.
///
/// Side effects (all atomic on success — partial writes leave state
/// recoverable):
///   1. Push a new entry onto `registry.entries`.
///   2. `registry_store.save(registry)`.
///   3. Build a `WorkspaceStore` for the new id and write an empty workspace
///      file.
pub fn seed_demo_entry(
    registry: &mut ModlistRegistry,
    registry_store: &RegistryStore,
    workspace_store_factory: impl Fn(&str) -> WorkspaceStore,
) -> Result<ModlistEntry, RegistryError> {
    let id = new_modlist_id();
    let n = registry.entries.len() + 1;
    let now = Utc::now();
    let entry = ModlistEntry {
        id: id.clone(),
        name: format!("demo-modlist-{n}"),
        game: Game::BGEE,
        destination_folder: String::new(),
        state: ModlistState::InProgress,
        creation_date: now,
        last_touched_date: now,
        install_date: None,
        last_played_date: None,
        // Demo values mirror the wireframe's in-progress card mock so the
        // seeded entry renders a realistic Home meta line.
        mod_count: 9,
        component_count: 136,
        paused_at_step: Some(3),
        total_size_bytes: None,
        latest_share_code: None,
        // Phase-6 additive provenance fields (SPEC §13.3). A dev-seeded
        // demo entry is a from-scratch modlist: no author, empty lineage.
        author: None,
        forked_from: Vec::new(),
        workspace_file_relpath: PathBuf::from(format!("modlists/{id}/workspace.json")),
    };
    registry.entries.push(entry.clone());

    // Persist the registry first — if the workspace write fails the entry is
    // still on disk and we can fail loudly without losing the registry update.
    registry_store.save(registry)?;

    // Write the empty workspace file so the per-modlist directory is
    // pre-created.
    let ws_store = workspace_store_factory(&id);
    let ws = ModlistWorkspaceState::default();
    ws_store.save(&ws)?;

    Ok(entry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static C: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn seed_demo_entry_writes_registry_and_workspace() {
        let n = C.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("bio_devseed_{}_{}", std::process::id(), n));
        let registry_path = root.join("modlists.json");
        let registry_store = RegistryStore::new_with_path(&registry_path);
        let factory = {
            let root = root.clone();
            move |id: &str| {
                WorkspaceStore::new_with_path(root.join("modlists").join(id).join("workspace.json"))
            }
        };
        let mut registry = ModlistRegistry::default();

        let entry = seed_demo_entry(&mut registry, &registry_store, factory).expect("seed");

        assert_eq!(registry.entries.len(), 1);
        assert_eq!(entry.state, ModlistState::InProgress);
        assert!(registry_path.exists());
        assert!(
            root.join("modlists")
                .join(&entry.id)
                .join("workspace.json")
                .exists()
        );

        let _ = std::fs::remove_dir_all(&root);
    }
}
