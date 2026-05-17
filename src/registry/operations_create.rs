// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `operations_create` — the high-level "create a new modlist" registry
// operation (P6.T7, SPEC §5.1).
//
// `create_modlist(name, game, destination, registry)
//   -> Result<ModlistEntry, RegistryError>`.
//
// ## PLAN GAP (resolved minimally; surfaced in the run report)
//
// The phase-06 file inventory pins this signature as
// `create_modlist(name, game, destination, registry, workspace_store)` —
// "Allocates an ID, inserts the entry, writes the empty workspace state."
// That 5-arg shape is **internally inconsistent**: a `WorkspaceStore` is
// **id-bound** (`WorkspaceStore::new_for_id(id)` resolves
// `<config>/modlists/<id>/workspace.json`), but the id is *minted inside*
// `create_modlist` — so a caller cannot construct the correct
// `WorkspaceStore` to pass *before* the id exists (chicken/egg). It is also a
// DATA-LOSS hazard for the unit tests (a real-config-dir
// `WorkspaceStore::new_for_id` write from `cargo test --lib` clobbers the
// user's `%APPDATA%\bio\` — directive-grade).
//
// **Minimal resolution (no public-contract redesign beyond dropping the
// unconstructible arg):** `create_modlist` owns *only* the registry-side
// work (mint id → build the `in_progress` entry with
// `workspace_file_relpath = modlists/<id>/workspace.json` → push into the
// in-memory registry → return a clone). The "write the empty workspace
// state" step moves to the **caller** (`create::page_create::start_scratch`),
// which is its natural owner: it has `OrchestratorApp` access, learns
// `entry.id` *after* the call, can build the canonical
// `WorkspaceStore::new_for_id(&entry.id)`, write the empty state there, and
// register the store + state in the orchestrator maps so the first
// `page_router::render_workspace` (which calls `WorkspaceStore::load`,
// erroring on a missing file) finds a loadable file. This keeps
// `create_modlist` pure of `WorkspaceStore` (so its tests touch no config
// dir — zero DATA-LOSS surface) while still satisfying the plan's intent
// end-to-end (the empty workspace IS written before the workspace opens —
// just by the caller, the only party that can name the canonical path). The
// `delete_modlist` precedent already splits "registry mutation here / IO +
// persistence anchored by the caller" the same way.
//
// ## Registry persistence is the caller's job (SPEC §13.14)
//
// `create_modlist` performs **no** `modlists.json` write. SPEC §13.14
// requires registry adds to be *atomic and non-queued*; the caller calls
// `orchestrator.registry_store.save(&orchestrator.registry)` immediately
// after a successful `create_modlist` (the established
// `operations::delete_modlist`-caller precedent). The persistence-cycle's
// debounced diff is then a no-op (idempotent).
//
// ## Fork variant — Run 4 (NOT this run)
//
// `create_forked_modlist(...)` (the fork-lineage `author` + `forked_from`
// append per SPEC §13.3 / §5.3) is **Phase 6 Run 4 (P6.T8)** — deliberately
// NOT defined here. Run 3 ships only the from-scratch `create_modlist`.
//
// SPEC: §5.1 (Create — new from downloaded mods), §13.1 (registry entry
//       shape), §13.14 (atomic registry add — caller-anchored).

// rationale: `#[must_use]` on this `Result`-returning helper is churn (the
// caller consumes the `Result` for the returned entry); the
// doc-paragraph-length lint is subjective style — both Cat 3.
#![allow(clippy::must_use_candidate, clippy::too_long_first_doc_paragraph)]

use std::io;
use std::path::PathBuf;

use chrono::Utc;

use crate::registry::errors::RegistryError;
use crate::registry::ids::new_modlist_id;
use crate::registry::model::{Game, ModlistEntry, ModlistRegistry, ModlistState};

/// Create a new from-scratch (non-forked) modlist (P6.T7 / SPEC §5.1).
///
/// Allocates a fresh 12-char ULID-style id, builds an `in_progress`
/// `ModlistEntry` (`workspace_file_relpath = modlists/<id>/workspace.json`,
/// creation/last-touched = now), pushes it into the in-memory `registry`,
/// and returns a clone (the caller needs `entry.id` to write the canonical
/// empty `workspace.json` and to set
/// `NavDestination::Workspace { Some(id) }`).
///
/// **Does no IO.** Writing the empty per-modlist `workspace.json` and the
/// atomic `modlists.json` persist are the caller's responsibility (SPEC
/// §13.14 — see the module header for why the workspace write moved
/// caller-side).
///
/// Errors:
///   - trimmed `name` is empty → `Io(InvalidInput)` (defensive backstop; the
///     dispatcher already guards an empty name before calling this).
pub fn create_modlist(
    name: &str,
    game: Game,
    destination: &str,
    registry: &mut ModlistRegistry,
) -> Result<ModlistEntry, RegistryError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(RegistryError::Io(io::Error::new(
            io::ErrorKind::InvalidInput,
            "modlist name cannot be empty",
        )));
    }

    let id = new_modlist_id();
    let now = Utc::now();

    let entry = ModlistEntry {
        id: id.clone(),
        name: trimmed.to_string(),
        game,
        destination_folder: destination.trim().to_string(),
        state: ModlistState::InProgress,
        creation_date: now,
        last_touched_date: now,
        workspace_file_relpath: PathBuf::from("modlists").join(&id).join("workspace.json"),
        ..Default::default()
    };

    registry.entries.push(entry.clone());
    Ok(entry)
}

#[cfg(test)]
mod tests {
    // NOTE: these tests deliberately touch **no** `WorkspaceStore` and
    // **no** config dir — `create_modlist` does no IO (the workspace write
    // is caller-side). So `cargo test --lib` cannot clobber the user's
    // `%APPDATA%\bio\modlists.json` here (DATA-LOSS-class invariant — the
    // orchestrator skill). Only the in-memory `ModlistRegistry` is asserted.
    use super::*;

    #[test]
    fn create_inserts_in_progress_entry_and_returns_it() {
        let mut reg = ModlistRegistry::default();
        let entry =
            create_modlist("Tactical EET 2026", Game::EET, "D:\\eet", &mut reg).expect("create ok");

        assert_eq!(entry.name, "Tactical EET 2026");
        assert_eq!(entry.game, Game::EET);
        assert_eq!(entry.destination_folder, "D:\\eet");
        assert_eq!(entry.state, ModlistState::InProgress);
        assert_eq!(entry.id.len(), 12, "ULID-style 12-char id");
        assert_eq!(reg.entries.len(), 1);
        assert_eq!(reg.find(&entry.id).unwrap().name, "Tactical EET 2026");
        // No forked lineage on a from-scratch create (fork is Run 4).
        assert_eq!(entry.author, None);
        assert!(entry.forked_from.is_empty());
    }

    #[test]
    fn workspace_relpath_is_modlists_id_workspace_json() {
        let mut reg = ModlistRegistry::default();
        let entry = create_modlist("X", Game::BGEE, "", &mut reg).expect("ok");
        assert_eq!(
            entry.workspace_file_relpath,
            PathBuf::from("modlists")
                .join(&entry.id)
                .join("workspace.json")
        );
    }

    #[test]
    fn name_and_destination_are_trimmed() {
        let mut reg = ModlistRegistry::default();
        let entry =
            create_modlist("   Spaced Name   ", Game::BG2EE, "  /x  ", &mut reg).expect("ok");
        assert_eq!(entry.name, "Spaced Name");
        assert_eq!(entry.destination_folder, "/x");
    }

    #[test]
    fn empty_name_is_rejected_and_registry_untouched() {
        let mut reg = ModlistRegistry::default();
        let err = create_modlist("   ", Game::EET, "/x", &mut reg).unwrap_err();
        match err {
            RegistryError::Io(e) => assert_eq!(e.kind(), io::ErrorKind::InvalidInput),
            other => panic!("expected Io(InvalidInput), got {other:?}"),
        }
        assert!(reg.entries.is_empty(), "no entry added on rejection");
    }

    #[test]
    fn each_create_gets_a_distinct_id() {
        let mut reg = ModlistRegistry::default();
        let a = create_modlist("A", Game::EET, "", &mut reg).expect("a");
        let b = create_modlist("B", Game::EET, "", &mut reg).expect("b");
        assert_ne!(a.id, b.id, "ids must be unique");
        assert_eq!(reg.entries.len(), 2);
    }

    #[test]
    fn iwdee_game_is_preserved() {
        let mut reg = ModlistRegistry::default();
        let entry = create_modlist("Icewind", Game::IWDEE, "/iwd", &mut reg).expect("ok");
        assert_eq!(entry.game, Game::IWDEE);
        assert_eq!(reg.find(&entry.id).unwrap().game, Game::IWDEE);
    }
}
