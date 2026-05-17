// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `operations_rename` — the high-level modlist-rename operation (P6.T5,
// SPEC §2.2).
//
// `rename_modlist(id, new_name, registry) -> Result<(), RegistryError>`.
//
// ## SPEC §2.2 — the hard invariant: **registry-entry rename ONLY**
//
//   "**Rename only touches the registry entry** — the install folder on
//    disk is **not** renamed (paths embedded in `weidu.log` or shared codes
//    stay valid)."
//
// This function therefore mutates **exactly one field**: the in-memory
// `ModlistEntry.name` for `id`. It performs **no filesystem operation at
// all** — no `std::fs::rename`, no directory move, nothing touches
// `destination_folder` or `workspace_file_relpath`. (Contrast
// `operations::delete_modlist`, which *does* touch the install folder behind
// a guard; rename never does, by SPEC.)
//
// ## SPEC §2.2 / §13.14 — the write is **debounced**, not immediate
//
//   "Registry write debounced like all other state changes
//    ([§13.14](#1314-persistence-timing))."
//
// Unlike `delete_modlist` (which §13.14 classes as an atomic, non-queued
// write that goes straight through `RegistryStore::save`), a rename is an
// **ordinary debounced state change**. So `rename_modlist` takes only
// `&mut ModlistRegistry` (the plan's pinned signature — note: no
// `RegistryStore`): it mutates the in-memory entry and returns. The actual
// disk write happens through the **existing registry persistence path** —
// `OrchestratorApp::tick_persistence` →
// `RegistryPersistenceCycle::persist_registry_if_needed`, which already
// diffs the in-memory registry against the last-saved snapshot every frame
// and writes it once the debounce window elapses. The caller
// (`workspace_header`) anchors the debounce timer with
// `persistence_cycle.mark_registry_dirty(now)` so the write is *debounced*
// (a missing dirty mark would mean "force-write immediately", which is the
// delete path's contract, not rename's). This is **not**
// `workspace_state_dirty` (that flag is the per-modlist `workspace.json`
// cycle — Run 4); rename rides the registry cycle.
//
// SPEC: §2.2 (rename = registry-only, no folder rename, debounced), §13.14
//       (persistence timing — debounced registry write), §13.1.

// rationale: `#[must_use]` on this `Result`-returning helper is churn (the
// caller already handles the `Result`) — Cat 3.
#![allow(clippy::must_use_candidate)]

use std::io;

use crate::registry::errors::RegistryError;
use crate::registry::model::ModlistRegistry;

/// Rename a modlist's registry entry (SPEC §2.2). **Registry-entry rename
/// ONLY** — the on-disk install folder, `destination_folder`, and
/// `workspace_file_relpath` are **never** touched (paths embedded in
/// `weidu.log` / shared codes stay valid).
///
/// Mutates the in-memory `ModlistEntry.name` for `id` in place. The disk
/// write is **debounced** through the existing registry persistence cycle —
/// this function does no IO itself (SPEC §13.14).
///
/// Errors (no IO is performed, so these are validation failures wrapped as
/// `RegistryError::Io` for the single `RegistryError` surface):
///   - the trimmed `new_name` is empty → `InvalidInput` (the caller's
///     wireframe-parity guard `if (tempName.trim())` already prevents this;
///     this is a defensive backstop).
///   - no entry with `id` exists → `NotFound` (a stale rename after the
///     entry was deleted).
///
/// On success the entry's `name` is the trimmed `new_name`; if it already
/// equals that, this is a successful no-op (the persistence cycle's diff
/// then writes nothing — idempotent).
pub fn rename_modlist(
    id: &str,
    new_name: &str,
    registry: &mut ModlistRegistry,
) -> Result<(), RegistryError> {
    let trimmed = new_name.trim();
    if trimmed.is_empty() {
        return Err(RegistryError::Io(io::Error::new(
            io::ErrorKind::InvalidInput,
            "modlist name cannot be empty",
        )));
    }

    let Some(entry) = registry.find_mut(id) else {
        return Err(RegistryError::Io(io::Error::new(
            io::ErrorKind::NotFound,
            format!("no modlist with id {id}"),
        )));
    };

    // The ONLY mutation. `destination_folder` / `workspace_file_relpath` /
    // the on-disk install folder are deliberately untouched (SPEC §2.2).
    entry.name = trimmed.to_string();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::model::{Game, ModlistEntry, ModlistState};
    use std::path::PathBuf;

    fn reg_with(id: &str, name: &str, dest: &str) -> ModlistRegistry {
        let mut r = ModlistRegistry::default();
        r.entries.push(ModlistEntry {
            id: id.to_string(),
            name: name.to_string(),
            game: Game::EET,
            destination_folder: dest.to_string(),
            state: ModlistState::InProgress,
            workspace_file_relpath: PathBuf::from(format!("modlists/{id}/workspace.json")),
            ..Default::default()
        });
        r
    }

    /// The happy path: only `name` changes.
    #[test]
    fn rename_updates_name_only() {
        let mut r = reg_with("ABC000000000", "old name", "/install/here");
        rename_modlist("ABC000000000", "new name", &mut r).expect("rename ok");
        let e = r.find("ABC000000000").unwrap();
        assert_eq!(e.name, "new name");
    }

    /// SPEC §2.2 — the install folder + workspace relpath are **never**
    /// touched by a rename (paths embedded in weidu.log / share codes stay
    /// valid).
    #[test]
    fn rename_never_touches_destination_or_workspace_path() {
        let mut r = reg_with("ABC000000000", "old", "/games/eet-install");
        let dest_before = r.find("ABC000000000").unwrap().destination_folder.clone();
        let ws_before = r
            .find("ABC000000000")
            .unwrap()
            .workspace_file_relpath
            .clone();

        rename_modlist("ABC000000000", "Totally Different Name", &mut r).expect("ok");

        let e = r.find("ABC000000000").unwrap();
        assert_eq!(
            e.destination_folder, dest_before,
            "destination_folder must be unchanged (SPEC §2.2)"
        );
        assert_eq!(
            e.workspace_file_relpath, ws_before,
            "workspace_file_relpath must be unchanged (SPEC §2.2)"
        );
        assert_eq!(e.name, "Totally Different Name");
    }

    /// The name is trimmed (matches the wireframe `saveRename`'s
    /// `tempName.trim()`).
    #[test]
    fn rename_trims_whitespace() {
        let mut r = reg_with("ID0000000000", "x", "");
        rename_modlist("ID0000000000", "   spaced name   ", &mut r).expect("ok");
        assert_eq!(r.find("ID0000000000").unwrap().name, "spaced name");
    }

    /// An empty / whitespace-only new name is rejected (defensive backstop;
    /// the caller's wireframe-parity guard already prevents it reaching here).
    #[test]
    fn empty_name_is_rejected() {
        let mut r = reg_with("ID0000000000", "keep", "");
        let err = rename_modlist("ID0000000000", "   ", &mut r).unwrap_err();
        match err {
            RegistryError::Io(e) => assert_eq!(e.kind(), io::ErrorKind::InvalidInput),
            other => panic!("expected Io(InvalidInput), got {other:?}"),
        }
        // Name unchanged on rejection.
        assert_eq!(r.find("ID0000000000").unwrap().name, "keep");
    }

    /// A rename targeting a non-existent id is a `NotFound` error (stale
    /// rename after delete) and mutates nothing.
    #[test]
    fn unknown_id_is_not_found() {
        let mut r = reg_with("REAL00000000", "real", "");
        let err = rename_modlist("GONE00000000", "x", &mut r).unwrap_err();
        match err {
            RegistryError::Io(e) => assert_eq!(e.kind(), io::ErrorKind::NotFound),
            other => panic!("expected Io(NotFound), got {other:?}"),
        }
        assert_eq!(r.entries.len(), 1);
        assert_eq!(r.find("REAL00000000").unwrap().name, "real");
    }

    /// Renaming to the same name is a successful no-op (the persistence
    /// cycle's diff then writes nothing — idempotent).
    #[test]
    fn rename_to_same_name_is_ok_noop() {
        let mut r = reg_with("SAME00000000", "Same Name", "");
        rename_modlist("SAME00000000", "Same Name", &mut r).expect("ok");
        assert_eq!(r.find("SAME00000000").unwrap().name, "Same Name");
    }
}
