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
// ## Fork variant — `create_forked_modlist` (Run 4 / P6.T8)
//
// `create_forked_modlist(name, game, destination, author, parent_name,
// parent_author, parent_forked_from, registry)` is the Create →
// Import-and-modify registry op (SPEC §5.3 / §13.3 Provenance). It is the
// **exact same shape** as `create_modlist` — registry-in-memory only,
// **ZERO IO**, returns the entry clone; the empty `workspace.json` write +
// the atomic `modlists.json` persist (SPEC §13.14) stay caller-anchored in
// `page_create` (the `start_scratch` precedent) — so this fn's tests touch
// no config dir (the DATA-LOSS-class invariant). Beyond what `create_modlist`
// sets, it additionally records:
//   - `author` ← the local user's handle (`RedesignSettings.user_name`,
//     trimmed; empty ⇒ `None`) — the author of *this* fork (SPEC §13.3).
//   - `forked_from` ← `parent.forked_from ++ [ForkAncestor { parent.name,
//     parent.author }]` — **append-only** (the credit guarantee, SPEC §13.3 /
//     §5.3): the parent's existing chain is carried through verbatim and the
//     immediate parent is appended last; no ancestor is ever rewritten or
//     dropped, the child's own identity never appears in its own chain. The
//     parent fields are read off the parsed `ModlistSharePreview`
//     (carve-out #5) at the call site and passed in by value.
// **No share code is generated at fork time** — `pack_meta` generation
// happens later at install-start / `flip_to_installed` (Phase 7) and reads
// these entry fields.
//
// SPEC: §5.1 (Create — new from downloaded mods), §5.3 (fork — Import and
//       modify), §13.1 (registry entry shape), §13.3 (Provenance / append
//       rule), §13.14 (atomic registry add — caller-anchored).

// rationale: `#[must_use]` on these `Result`-returning helpers is churn (the
// caller consumes the `Result` for the returned entry); the
// doc-paragraph-length lint is subjective style; `create_forked_modlist`'s
// 8 args are the irreducible inputs of the SPEC §13.3 lineage append
// (name/game/destination + user_name + the 3 parent-provenance fields read
// off the parsed `ModlistSharePreview`) — a "params struct" here would be a
// single-call-site indirection with no behaviour value (default-gate clean;
// pedantic `too_many_arguments` only) — all Cat 3.
#![allow(
    clippy::must_use_candidate,
    clippy::too_long_first_doc_paragraph,
    clippy::too_many_arguments
)]

use std::io;
use std::path::PathBuf;

use chrono::Utc;

use crate::app::modlist_share::ForkAncestor;
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

/// Create a forked modlist via Create → Import-and-modify (P6.T8 / SPEC §5.3,
/// §13.3 Provenance).
///
/// Same shape + IO contract as [`create_modlist`] (registry-in-memory only,
/// **no IO**; the caller writes the empty `workspace.json` + the atomic
/// `modlists.json` persist — the `start_scratch` precedent). Beyond
/// name/game/destination it records the fork lineage (the credit guarantee,
/// SPEC §13.3):
///
/// - `author` ← `user_name` trimmed; an empty/whitespace handle ⇒ `None`
///   (SPEC §13.3 — never store an empty author string; absent means absent).
/// - `forked_from` ← `parent_forked_from ++ [ForkAncestor { parent_name,
///   parent_author }]`. **Append-only:** the parent's chain is carried
///   through verbatim (oldest → newest) and the immediate parent is appended
///   as the new last element. The child's own `name`/`author` live in the
///   top-level entry fields, never in its own `forked_from` (the append
///   rule). Because earlier entries are never rewritten, every ancestor
///   author down the chain stays credited no matter how deep the fork tree.
///
/// `parent_name` / `parent_author` / `parent_forked_from` come from the
/// parsed `ModlistSharePreview` (carve-out #5 provenance fields) at the
/// fork-preview call site. `parent_name` is the parsed `name`; when the
/// parent code carried no packed name the caller passes the honest fallback
/// (`Shared modlist`) — this fn does not invent one. **No share code is
/// generated here** (`pack_meta` is Phase 7).
///
/// Errors: trimmed `name` empty → `Io(InvalidInput)` (same defensive
/// backstop as `create_modlist`; the dispatcher guards it first).
pub fn create_forked_modlist(
    name: &str,
    game: Game,
    destination: &str,
    user_name: &str,
    parent_name: &str,
    parent_author: &str,
    parent_forked_from: &[ForkAncestor],
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

    // SPEC §13.3: `author` ← `RedesignSettings.user_name`; empty ⇒ omitted.
    let author = {
        let t = user_name.trim();
        if t.is_empty() {
            None
        } else {
            Some(t.to_string())
        }
    };

    // SPEC §13.3 / §5.3 append rule: the parent's existing chain, then the
    // immediate parent appended last. Append-only — never rewrite an ancestor.
    let mut forked_from = parent_forked_from.to_vec();
    forked_from.push(ForkAncestor {
        name: parent_name.to_string(),
        author: parent_author.to_string(),
    });

    let entry = ModlistEntry {
        id: id.clone(),
        name: trimmed.to_string(),
        game,
        destination_folder: destination.trim().to_string(),
        state: ModlistState::InProgress,
        creation_date: now,
        last_touched_date: now,
        author,
        forked_from,
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

    // ── create_forked_modlist (P6.T8 / SPEC §5.3, §13.3) ──
    // Like the `create_modlist` tests: zero `WorkspaceStore`, zero config dir
    // (`create_forked_modlist` does no IO), `ModlistRegistry::default()` only
    // — so `cargo test --lib` cannot clobber the user's
    // `%APPDATA%\bio\modlists.json` (DATA-LOSS-class invariant).

    #[test]
    fn fork_of_a_root_appends_the_immediate_parent_only() {
        // Parent is itself an original (empty `forked_from`). The child's
        // chain becomes exactly `[parent]`.
        let mut reg = ModlistRegistry::default();
        let child = create_forked_modlist(
            "My EET fork",
            Game::EET,
            "D:\\fork",
            "  @me  ",
            "Born2BSalty's EET",
            "@b2bs",
            &[],
            &mut reg,
        )
        .expect("fork ok");

        assert_eq!(child.name, "My EET fork");
        assert_eq!(child.game, Game::EET);
        assert_eq!(child.destination_folder, "D:\\fork");
        assert_eq!(child.state, ModlistState::InProgress);
        assert_eq!(child.author.as_deref(), Some("@me"), "author trimmed");
        assert_eq!(child.forked_from.len(), 1);
        assert_eq!(child.forked_from[0].name, "Born2BSalty's EET");
        assert_eq!(child.forked_from[0].author, "@b2bs");
        assert_eq!(reg.entries.len(), 1);
        assert_eq!(reg.find(&child.id).unwrap().forked_from.len(), 1);
    }

    #[test]
    fn fork_of_a_fork_is_append_only_credit_preserved() {
        // Parent already has a 2-deep chain (root → mid). Forking it appends
        // the parent itself, yielding root → mid → parent (3 deep). Every
        // earlier ancestor (and its author) is preserved verbatim — the
        // SPEC §13.3 credit invariant.
        let parent_chain = vec![
            ForkAncestor {
                name: "Original".to_string(),
                author: "@root".to_string(),
            },
            ForkAncestor {
                name: "Mid".to_string(),
                author: "@mid".to_string(),
            },
        ];
        let mut reg = ModlistRegistry::default();
        let child = create_forked_modlist(
            "Deep fork",
            Game::BG2EE,
            "/d",
            "@forker",
            "Parent build",
            "@parent",
            &parent_chain,
            &mut reg,
        )
        .expect("ok");

        assert_eq!(child.forked_from.len(), 3, "parent chain + parent");
        // Earlier ancestors preserved verbatim, in order (append-only).
        assert_eq!(child.forked_from[0].name, "Original");
        assert_eq!(child.forked_from[0].author, "@root");
        assert_eq!(child.forked_from[1].name, "Mid");
        assert_eq!(child.forked_from[1].author, "@mid");
        // The immediate parent is appended LAST.
        assert_eq!(child.forked_from[2].name, "Parent build");
        assert_eq!(child.forked_from[2].author, "@parent");
        // The child's own identity is NEVER in its own chain.
        assert!(
            !child.forked_from.iter().any(|a| a.name == "Deep fork"),
            "a modlist's own identity must never appear in its own forked_from"
        );
    }

    #[test]
    fn empty_user_name_yields_none_author() {
        // SPEC §13.3: an empty / whitespace handle ⇒ `None` (never store an
        // empty author string).
        let mut reg = ModlistRegistry::default();
        let child =
            create_forked_modlist("F", Game::EET, "", "   ", "P", "@p", &[], &mut reg).expect("ok");
        assert_eq!(child.author, None);
        // The lineage is still recorded (the parent is still credited).
        assert_eq!(child.forked_from.len(), 1);
        assert_eq!(child.forked_from[0].author, "@p");
    }

    #[test]
    fn fork_empty_name_is_rejected_and_registry_untouched() {
        let mut reg = ModlistRegistry::default();
        let err = create_forked_modlist("  ", Game::EET, "/x", "@me", "P", "@p", &[], &mut reg)
            .unwrap_err();
        match err {
            RegistryError::Io(e) => assert_eq!(e.kind(), io::ErrorKind::InvalidInput),
            other => panic!("expected Io(InvalidInput), got {other:?}"),
        }
        assert!(reg.entries.is_empty(), "no entry added on rejection");
    }

    #[test]
    fn fork_gets_distinct_id_and_workspace_relpath() {
        let mut reg = ModlistRegistry::default();
        let a =
            create_forked_modlist("A", Game::EET, "", "@m", "P", "@p", &[], &mut reg).expect("a");
        let b =
            create_forked_modlist("B", Game::EET, "", "@m", "P", "@p", &[], &mut reg).expect("b");
        assert_ne!(a.id, b.id, "fork ids must be unique");
        assert_eq!(
            a.workspace_file_relpath,
            PathBuf::from("modlists").join(&a.id).join("workspace.json")
        );
        assert_eq!(reg.entries.len(), 2);
    }

    #[test]
    fn parent_chain_is_not_aliased_or_mutated_by_caller() {
        // The caller passes the parent chain by `&[ForkAncestor]`; the fn
        // must own its copy (`.to_vec()`), so the caller's slice is unchanged
        // and a later parent re-fork does not see the child's appended entry.
        let parent_chain = vec![ForkAncestor {
            name: "Root".to_string(),
            author: "@r".to_string(),
        }];
        let mut reg = ModlistRegistry::default();
        let _ = create_forked_modlist("C", Game::EET, "", "@m", "P", "@p", &parent_chain, &mut reg)
            .expect("ok");
        assert_eq!(parent_chain.len(), 1, "caller's parent chain untouched");
        assert_eq!(parent_chain[0].name, "Root");
    }
}
