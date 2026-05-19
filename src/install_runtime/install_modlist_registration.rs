// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `install_runtime::install_modlist_registration` — the **final P7 Fix-Run**
// (user decision 2026-05-18: "Full correct fix", resolution A).
//
// ## Why this exists (premise-checked against source — see the run report)
//
// SPEC §13.13 / plan P7.T11 / Verification-#5 require `modlist-import-code
// .txt` (+ install-start `latest_share_code` + `install_started_at`) for
// **every** install entry point. `start_hooks::write_install_start_artifacts`
// (commit `7e0e414`) is the reusable §13.13 bundle, but it is invoked **only**
// from `on_install_start`, which is reached **only** from the in-Workspace
// Step-5 button (`page_workspace_step5.rs`). Create→New / Create-import /
// Load-Draft all route Workspace→Step5→`on_install_start` ⇒ already covered.
// **Install-Modlist-paste & Reinstall** route through the Run-4a
// `auto_build_driver` pipeline (`stage_downloading::render_live`), which
// **bypasses `on_install_start`** — so the §13.13 bundle never ran for them.
//
// Additionally (premise-check, grep-confirmed): **nothing in
// `src/ui/install/` ever created a registry `ModlistEntry`** — a brand-new
// Install-Modlist-paste did not persist a modlist at all (it never showed on
// Home, `flip_to_installed` never fired for it because
// `maybe_flip_to_installed_on_clean_exit` keys off
// `workspace_view.loaded_workspace_id`, which is `None` on the Install
// screen). A Reinstall, by contrast, already has its registry entry (its
// `pending_reinstall_id` target) and its `Installed → InProgress` flip is
// already wired at `page_install.rs`'s Preview→Downloading Install-click
// (`start_hooks::reinstall_flip_at_install_click`, Run 4b).
//
// ## What this module does — `register_and_write_install_start_artifacts`
//
// The single orchestration seam the Install-Modlist Downloading screen calls
// **once** (gated by `InstallScreenState::pipeline_armed`) **after**
// `auto_build_driver::prepare_install_dirs_and_maybe_import` returned `Ok`
// (so `import_modlist_share_code` has populated `WizardState` — `pack_meta`
// inside the §13.13 bundle exports from it). In order:
//
//   1. **Resolve / register the target entry.**
//      - **Reinstall** (`pending_reinstall_id == Some(id)` AND that id is in
//        the registry): the entry already exists — do **NOT** register a
//        second; just take its id. (The `Installed → InProgress` flip is
//        the Install-click site's job, already wired — not here.)
//      - **Fresh Install-Modlist paste** (no usable `pending_reinstall_id`
//        entry): mint + persist a net-new in-progress `ModlistEntry` via
//        [`register_install_modlist_paste`] (the exact `create_modlist`
//        convention — same id-gen, same entry shape, same caller-anchored
//        empty-`workspace.json` + atomic `modlists.json` save the
//        `start_scratch` precedent uses). Idempotent: if an entry for this
//        destination was already registered this run (the latch should
//        prevent a re-call, but be defensive) reuse it.
//   2. **Write the SPEC §13.13 install-start bundle** for that entry via the
//      committed `start_hooks::write_install_start_artifacts` (variant from
//      `InstallButtonVariant::from_step5_and_reinstall` — Reinstall ⇒
//      overwrite, a fresh paste ⇒ `Install` ⇒ write; the Run-2 matrix
//      governs). An `Err` is logged + non-fatal (mirrors `on_install_start`
//      / SPEC §13.14 — the install proceeds; the registry already holds the
//      entry).
//   3. **Set `OrchestratorApp::active_install_modlist_id = Some(id)`** so
//      the C3 clean-exit edge (`maybe_flip_to_installed_on_clean_exit`)
//      flips THIS entry `InProgress → Installed` even though the Install
//      screen has no `loaded_workspace_id` (closing the broader
//      lifecycle gap — the entry now shows on Home In-progress → Installed).
//
// **Never flips `start_install_requested`** (the pipeline's
// `start_auto_build_install` owns that — a premature flip installs an empty
// per-install Mods folder, the documented P7.T17 hazard). **Never derives
// dirs / applies flag policies / does the Reinstall state-flip** (those are
// already done on the pipeline path — `render_live` /
// `reinstall_flip_at_install_click`). **Zero BIO source** — composes
// `create_modlist`'s convention + the committed `write_install_start_
// artifacts` (which composes `pack_meta`, which composes BIO read-only).
//
// SPEC: §13.13 (import code auto-write — every entry point), §13.1 (registry
//        lifecycle — an Install-Modlist paste registers an in-progress
//        entry), §13.3 (provenance carried from the pasted code), §4.x
//        (Install Modlist flow), §3.1 (Reinstall — entry already exists).

// rationale: the orchestration fn legitimately threads several disjoint
// `OrchestratorApp` fields (registry + store + wizard_state + the install
// state + the new marker) — that is the irreducible surface of "register +
// §13.13 + mark", not a smell; `#[must_use]` on the `Result`-returning
// registry helper is churn (the caller consumes it for the entry); the
// doc-paragraph-length lint is subjective style on a load-bearing premise
// note (all Cat 3).
#![allow(
    clippy::must_use_candidate,
    clippy::too_long_first_doc_paragraph,
    clippy::too_many_lines
)]

use std::path::PathBuf;

use chrono::Utc;
use tracing::{info, warn};

use crate::app::modlist_share::ModlistSharePreview;
use crate::install_runtime::start_hooks::{self, InstallButtonVariant};
use crate::registry::errors::RegistryError;
use crate::registry::ids::new_modlist_id;
use crate::registry::model::{Game, ModlistEntry, ModlistRegistry, ModlistState};
use crate::registry::store_workspace::WorkspaceStore;
use crate::registry::workspace_model::ModlistWorkspaceState;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;

/// SPEC §4.2 honest fallback when the pasted code carries no packed `name`
/// (the exact string `stage_preview`'s `FALLBACK_TITLE` / `stage_installing`'s
/// `FALLBACK_NAME` use — never fabricate a modlist name).
const FALLBACK_NAME: &str = "Shared modlist";

/// Register a net-new **in-progress** `ModlistEntry` for an Install-Modlist
/// *paste* (SPEC §13.1 — "the act of pasting an Install Modlist share code …
/// creates a modlist in `in-progress` state").
///
/// Built from the **parsed share-code preview** + the chosen destination,
/// using the **exact `operations_create::create_modlist` convention**
/// (premise-checked — reused verbatim, not reinvented):
///   - id ← `new_modlist_id()` (the same 12-char ULID-style generator).
///   - `name` ← the code's packed `preview.name`, honest fallback
///     `Shared modlist` when absent (SPEC §4.2 — never fabricate).
///   - `game` ← `Game::from_legacy_string(&preview.game_install)` (SPEC §4 /
///     §13.12a — the Install screen never collects the game; it is the
///     payload's game).
///   - `destination_folder` ← the paste-stage `FolderInput` value (trimmed).
///   - `state` ← `InProgress`.
///   - `author` / `forked_from` ← the **pasted code's own provenance**
///     (`preview.author` / `preview.forked_from`) carried **verbatim** (a
///     paste-install is NOT a fork — it reproduces the shared modlist as-is,
///     so it inherits that code's identity + lineage so the original
///     creators stay credited; SPEC §13.3). An empty `author` ⇒ `None`.
///   - `creation_date` / `last_touched_date` ← now;
///     `workspace_file_relpath` ← `modlists/<id>/workspace.json` (the exact
///     `create_modlist` path).
///
/// **Does no IO.** Like `create_modlist`, the empty `workspace.json` write +
/// the atomic `modlists.json` persist are the caller's responsibility (the
/// `start_scratch` precedent — caller-anchored because only it can name the
/// canonical id-bound `WorkspaceStore::new_for_id` path). Pushes the entry
/// into the in-memory `registry` and returns a clone (the caller needs
/// `entry.id`). Errors only on the defensive empty-name backstop (which the
/// fallback makes unreachable in practice — kept for parity with
/// `create_modlist`).
///
/// `pub(crate)` (not `pub`): the `&ModlistSharePreview` parameter is BIO's
/// `pub(crate)` carve-out-#5 type (its visibility is not a redesign
/// decision), so this fn cannot be more public than the type it takes — the
/// `private_interfaces` lint, the **exact same** resolution as
/// `state_install::InstallScreenState::parsed_preview` and the registry
/// `ModlistEntry::forked_from`. Every caller is in-crate
/// (`register_and_write_install_start_artifacts`, the tests).
pub(crate) fn register_install_modlist_paste(
    preview: &ModlistSharePreview,
    destination: &str,
    registry: &mut ModlistRegistry,
) -> Result<ModlistEntry, RegistryError> {
    // SPEC §4.2 honest fallback — never fabricate a name. (The fallback is
    // non-empty, so the defensive empty-name guard below is unreachable in
    // practice; it mirrors `create_modlist` for shape parity.)
    let name = preview
        .name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(FALLBACK_NAME)
        .to_string();
    if name.trim().is_empty() {
        return Err(RegistryError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "modlist name cannot be empty",
        )));
    }

    // SPEC §4 / §13.12a: the Install screen never collects the game — it is
    // the pasted payload's game.
    let game = Game::from_legacy_string(&preview.game_install);

    // SPEC §13.3: a paste-install carries the pasted code's OWN provenance
    // verbatim (it reproduces that shared modlist; it is not a fork, so the
    // lineage is copied through unchanged so every ancestor stays credited).
    // An empty author string ⇒ `None` (never store an empty author).
    let author = preview
        .author
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    let id = new_modlist_id();
    let now = Utc::now();

    let entry = ModlistEntry {
        id: id.clone(),
        name,
        game,
        destination_folder: destination.trim().to_string(),
        state: ModlistState::InProgress,
        creation_date: now,
        last_touched_date: now,
        author,
        forked_from: preview.forked_from.clone(),
        workspace_file_relpath: PathBuf::from("modlists").join(&id).join("workspace.json"),
        ..Default::default()
    };

    registry.entries.push(entry.clone());
    Ok(entry)
}

/// Find an already-registered Install-Modlist-paste entry for this
/// destination (idempotency backstop — the `pipeline_armed` latch should
/// make this a no-op, but a re-call must never push a duplicate). Matches a
/// **non-empty** trimmed `destination_folder`. `None` ⇒ register fresh.
fn existing_entry_id_for_destination(
    registry: &ModlistRegistry,
    destination: &str,
) -> Option<String> {
    let dest = destination.trim();
    if dest.is_empty() {
        return None;
    }
    registry
        .entries
        .iter()
        .find(|e| e.destination_folder.trim() == dest)
        .map(|e| e.id.clone())
}

/// **The final-P7-Fix-Run orchestration seam.** Called **once** (gated by
/// `InstallScreenState::pipeline_armed`) from `stage_downloading::render_live`
/// **after** `auto_build_driver::prepare_install_dirs_and_maybe_import`
/// returned `Ok` (so the import populated `WizardState`). See the module
/// header for the ordered contract.
///
/// `parsed_preview_available` is the only precondition for the *fresh-paste*
/// registration: the entry's name/game/provenance come from the parsed
/// preview, which is set on Paste→Preview (`run_preview_parse`) and by
/// `reinstall_route::start_reinstall`. If it is somehow `None` (shouldn't
/// happen — the user reached Downloading only via Preview), the fresh-paste
/// registration is skipped (logged) — the §13.13 write still runs for a
/// Reinstall (its entry already exists) but not for a paste with no entry
/// (the `write_install_start_artifacts` first `?` would `Err` on a missing
/// entry — logged-non-fatal, same as a Reinstall whose entry vanished).
///
/// Never returns `Result` — every failure mode is logged + non-fatal (the
/// install must proceed; SPEC §13.14). Returns `true` iff
/// `active_install_modlist_id` was set to a resolved entry (the caller may
/// log).
pub fn register_and_write_install_start_artifacts(orchestrator: &mut OrchestratorApp) -> bool {
    let destination = orchestrator
        .install_screen_state
        .destination
        .trim()
        .to_string();

    // ── 1. Resolve / register the target entry. ────────────────────────────
    //
    // A Reinstall already has its registry entry (the `pending_reinstall_id`
    // target — `reinstall_route::start_reinstall` populated the screen from
    // it). Do NOT register a second; the `Installed → InProgress` flip is
    // the Install-click site's job (already wired —
    // `start_hooks::reinstall_flip_at_install_click`), not here.
    let reinstall_id = orchestrator
        .pending_reinstall_id
        .as_ref()
        .filter(|id| orchestrator.registry.find(id).is_some())
        .cloned();

    let modlist_id: String = if let Some(id) = reinstall_id {
        info!(
            target = "orchestrator",
            "Install start (Reinstall): reusing existing registry entry {id} \
             (no second registration — SPEC §3.1); writing §13.13 artifacts"
        );
        id
    } else if let Some(id) = existing_entry_id_for_destination(&orchestrator.registry, &destination)
    {
        // Idempotency backstop — an entry for this destination is already
        // registered (e.g. a defensive re-call). Reuse it; never duplicate.
        info!(
            target = "orchestrator",
            "Install start (Install-Modlist paste): reusing already-registered \
             entry {id} for destination {destination} (idempotent)"
        );
        id
    } else {
        // Fresh Install-Modlist paste — register a net-new in-progress entry
        // from the parsed preview (SPEC §13.1). The preview is the source of
        // the name/game/provenance; it is set on Paste→Preview.
        let Some(preview) = orchestrator.install_screen_state.parsed_preview.clone() else {
            warn!(
                target = "orchestrator",
                "Install start: no parsed preview to register an Install-Modlist \
                 entry from (Downloading reached without a cached preview?) — \
                 skipping registration + §13.13 artifacts (the install still \
                 proceeds; nothing to persist)"
            );
            return false;
        };

        let entry = match register_install_modlist_paste(
            &preview,
            &destination,
            &mut orchestrator.registry,
        ) {
            Ok(e) => e,
            Err(err) => {
                warn!(
                    target = "orchestrator",
                    "Install start: register_install_modlist_paste failed: {err} \
                     — skipping §13.13 artifacts (non-fatal; the install \
                     proceeds, just unpersisted — SPEC §13.14)"
                );
                return false;
            }
        };

        // Caller-anchored IO — the `start_scratch` (Create→New) precedent,
        // reused verbatim so a fresh Install-Modlist-paste entry is fully
        // consistent with the rest of the registry (it shows on Home
        // In-progress AND its `resume` Home card opens a loadable workspace
        // rather than the missing-`workspace.json` terminal error —
        // `page_router::render_workspace` calls `WorkspaceStore::load`,
        // which `Err`s on a missing file):
        //   - write the empty `workspace.json` at the canonical id-bound
        //     path + register the store + state in the orchestrator maps;
        //   - SPEC §13.14: registry adds are atomic + non-queued — persist
        //     `modlists.json` immediately, then anchor the persistence-cycle
        //     debounce so its diff is a no-op afterward.
        let canonical_store = WorkspaceStore::new_for_id(&entry.id);
        let empty = ModlistWorkspaceState::default();
        if let Err(err) = canonical_store.save(&empty) {
            warn!(
                target = "orchestrator",
                "Install start: writing canonical workspace.json for {} failed: \
                 {err} (the entry is still registered; a later `resume` would \
                 surface the missing-workspace error — non-fatal to the \
                 install)",
                entry.id
            );
        }
        orchestrator.workspace_state.insert(entry.id.clone(), empty);
        orchestrator
            .workspace_stores
            .insert(entry.id.clone(), canonical_store);

        if let Err(err) = orchestrator.registry_store.save(&orchestrator.registry) {
            warn!(
                target = "orchestrator",
                "Install start: atomic registry persist for the new \
                 Install-Modlist entry {} failed: {err} (entry is in memory + \
                 workspace.json is on disk; recoverable — SPEC §13.14)",
                entry.id
            );
        }
        orchestrator
            .persistence_cycle
            .mark_registry_dirty(std::time::Instant::now());

        info!(
            target = "orchestrator",
            "Install start (Install-Modlist paste): registered net-new \
             in-progress entry {} \"{}\" ({:?}) at {} — SPEC §13.1; writing \
             §13.13 artifacts",
            entry.id,
            entry.name,
            entry.game,
            destination
        );
        entry.id
    };

    // ── 2. Write the SPEC §13.13 install-start bundle for the resolved
    //    entry (pack_meta `allow_auto_install=false` → entry
    //    .latest_share_code → install_started_at → atomic save →
    //    variant-gated modlist-import-code.txt). The variant comes from the
    //    shared `from_step5_and_reinstall` (Reinstall ⇒ overwrite; a fresh
    //    paste's `state.step5` is `!resume_available && !has_run_once` ⇒
    //    `Install` ⇒ write — the Run-2 matrix governs). An `Err` is logged +
    //    non-fatal (the install proceeds; SPEC §13.14 — mirrors
    //    `on_install_start`). This runs AFTER `import_modlist_share_code`
    //    (the caller's precondition) so `pack_meta`'s
    //    `export_modlist_share_code(&wizard_state)` sees the imported logs.
    //    Split the disjoint &mut fields (the established split-borrow shape).
    let variant = InstallButtonVariant::from_step5_and_reinstall(
        &orchestrator.wizard_state,
        &modlist_id,
        orchestrator.pending_reinstall_id.as_deref(),
    );
    {
        let OrchestratorApp {
            wizard_state,
            registry,
            registry_store,
            ..
        } = &mut *orchestrator;
        if let Err(err) = start_hooks::write_install_start_artifacts(
            &modlist_id,
            variant,
            wizard_state,
            registry,
            registry_store,
        ) {
            warn!(
                target = "orchestrator",
                "Install start: write_install_start_artifacts for {modlist_id} \
                 failed: {err} (non-fatal — the install proceeds; SPEC §13.14 / \
                 mirrors on_install_start's handling)"
            );
        }
    }

    // ── 3. Mark the active install modlist so the C3 clean-exit edge flips
    //    THIS entry InProgress → Installed even though the Install screen has
    //    no `workspace_view.loaded_workspace_id`
    //    (`maybe_flip_to_installed_on_clean_exit` falls back to this).
    //    Cleared on nav-away-from-Install if the install never reached a
    //    clean exit, and right after the flip — mirroring
    //    `pending_reinstall_id`'s lifecycle. ──
    orchestrator.active_install_modlist_id = Some(modlist_id.clone());
    info!(
        target = "orchestrator",
        "Install start: active_install_modlist_id = {modlist_id} (the C3 \
         clean-exit flip will move it InProgress → Installed; it shows on \
         Home In-progress until then)"
    );
    true
}

#[cfg(test)]
mod tests {
    // NOTE: like `operations_create`'s tests, the registry-side helper
    // (`register_install_modlist_paste`) touches **no** `WorkspaceStore` /
    // config dir (it does no IO — the workspace write is caller-anchored, the
    // `start_scratch` precedent), so `cargo test --lib` cannot clobber the
    // user's `%APPDATA%\bio\modlists.json` (DATA-LOSS-class invariant — the
    // orchestrator skill). Only the in-memory `ModlistRegistry` is asserted.
    // `register_and_write_install_start_artifacts` needs a real
    // `OrchestratorApp` (its `new()` binds the real config dir + its
    // `pack_meta` needs BIO file-backed state a unit test cannot stand up —
    // the same constraint `start_hooks` / `share_export` / `reinstall_route`
    // document), so it is exercised by the manual breakpoint; the pure,
    // orchestrator-free registry projection is what is unit-tested here.
    use super::*;
    use crate::app::modlist_share::ForkAncestor;

    fn preview(
        name: Option<&str>,
        game: &str,
        author: Option<&str>,
        forked_from: Vec<ForkAncestor>,
    ) -> ModlistSharePreview {
        ModlistSharePreview {
            bio_version: "x".to_string(),
            game_install: game.to_string(),
            install_mode: "build-from-scanned-mods".to_string(),
            bgee_entries: 0,
            bg2ee_entries: 0,
            has_source_overrides: false,
            has_installed_refs: false,
            bgee_log_text: String::new(),
            bg2ee_log_text: String::new(),
            source_overrides_text: String::new(),
            installed_refs_text: String::new(),
            mod_config_count: 0,
            mod_configs_text: String::new(),
            allow_auto_install: true,
            name: name.map(str::to_string),
            author: author.map(str::to_string),
            forked_from,
        }
    }

    #[test]
    fn registers_in_progress_entry_from_preview_packed_name_and_game() {
        let mut reg = ModlistRegistry::default();
        let p = preview(Some("Tactical EET 2026"), "EET", Some("@b2bs"), vec![]);
        let e = register_install_modlist_paste(&p, "  D:\\eet  ", &mut reg).expect("register ok");

        assert_eq!(e.name, "Tactical EET 2026");
        assert_eq!(e.game, Game::EET, "game = the payload's game (SPEC §4)");
        assert_eq!(e.destination_folder, "D:\\eet", "destination trimmed");
        assert_eq!(
            e.state,
            ModlistState::InProgress,
            "a pasted-code install is in-progress until it succeeds (SPEC §13.1)"
        );
        assert_eq!(
            e.id.len(),
            12,
            "the create_modlist ULID-style id convention"
        );
        assert_eq!(e.author.as_deref(), Some("@b2bs"), "the code's own author");
        assert!(
            e.forked_from.is_empty(),
            "a non-forked code carries no lineage"
        );
        assert_eq!(
            e.workspace_file_relpath,
            PathBuf::from("modlists").join(&e.id).join("workspace.json"),
            "the exact create_modlist workspace_file_relpath convention"
        );
        assert_eq!(reg.entries.len(), 1);
        assert_eq!(reg.find(&e.id).unwrap().name, "Tactical EET 2026");
    }

    #[test]
    fn honest_fallback_name_when_code_has_no_packed_name() {
        // SPEC §4.2: never fabricate a name — the exact `Shared modlist`
        // string `stage_preview` / `stage_installing` use.
        let mut reg = ModlistRegistry::default();
        let p = preview(None, "BGEE", None, vec![]);
        let e = register_install_modlist_paste(&p, "/x", &mut reg).expect("ok");
        assert_eq!(e.name, "Shared modlist");
        assert_eq!(e.game, Game::BGEE);
        assert_eq!(e.author, None, "no packed author ⇒ None");
    }

    #[test]
    fn empty_or_whitespace_packed_name_falls_back_not_errors() {
        // A packed-but-empty/whitespace name is normalized to the honest
        // fallback (not stored as "" and not an error).
        let mut reg = ModlistRegistry::default();
        let p = preview(Some("   "), "EET", Some("  "), vec![]);
        let e = register_install_modlist_paste(&p, "/x", &mut reg).expect("ok");
        assert_eq!(e.name, "Shared modlist");
        assert_eq!(e.author, None, "whitespace author ⇒ None");
    }

    #[test]
    fn carries_the_pasted_codes_lineage_verbatim_for_credit() {
        // SPEC §13.3: a paste-install is NOT a fork — it reproduces the
        // shared modlist, so it carries the code's own `forked_from`
        // verbatim (every ancestor stays credited; the chain is NOT
        // appended-to here — appending is the Create→Import fork path).
        let lineage = vec![
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
        let p = preview(Some("Shared deep build"), "BG2EE", Some("@sharer"), lineage);
        let e = register_install_modlist_paste(&p, "/d", &mut reg).expect("ok");

        assert_eq!(e.forked_from.len(), 2, "the code's chain carried verbatim");
        assert_eq!(e.forked_from[0].name, "Original");
        assert_eq!(e.forked_from[0].author, "@root");
        assert_eq!(e.forked_from[1].name, "Mid");
        assert_eq!(e.forked_from[1].author, "@mid");
        assert_eq!(
            e.author.as_deref(),
            Some("@sharer"),
            "the entry's own author = the code's author (the sharer)"
        );
        // The entry's own identity is NEVER spliced into its own chain.
        assert!(
            !e.forked_from.iter().any(|a| a.name == "Shared deep build"),
            "a modlist's own identity must never appear in its own forked_from"
        );
    }

    #[test]
    fn unknown_game_string_defaults_to_bgee_like_create() {
        // Mirrors `create_modlist` / `Game::from_legacy_string` behavior —
        // an unrecognized payload game string defaults to BGEE (BIO parity).
        let mut reg = ModlistRegistry::default();
        let p = preview(Some("X"), "???", None, vec![]);
        let e = register_install_modlist_paste(&p, "/x", &mut reg).expect("ok");
        assert_eq!(e.game, Game::BGEE);
    }

    #[test]
    fn each_registration_gets_a_distinct_id() {
        let mut reg = ModlistRegistry::default();
        let p = preview(Some("A"), "EET", None, vec![]);
        let a = register_install_modlist_paste(&p, "/a", &mut reg).expect("a");
        let b = register_install_modlist_paste(&p, "/b", &mut reg).expect("b");
        assert_ne!(
            a.id, b.id,
            "ids must be unique (the create_modlist ids convention)"
        );
        assert_eq!(reg.entries.len(), 2);
    }

    #[test]
    fn existing_entry_id_for_destination_matches_trimmed_nonempty_only() {
        let mut reg = ModlistRegistry::default();
        let p = preview(Some("A"), "EET", None, vec![]);
        let a = register_install_modlist_paste(&p, "D:\\dest one", &mut reg).expect("a");

        // Same destination (trimmed) ⇒ found (idempotency backstop — never
        // register a duplicate for the same destination).
        assert_eq!(
            existing_entry_id_for_destination(&reg, "  D:\\dest one  "),
            Some(a.id.clone())
        );
        // A different destination ⇒ not found (register fresh).
        assert_eq!(existing_entry_id_for_destination(&reg, "D:\\other"), None);
        // Empty / whitespace destination ⇒ never matches (no duplicate
        // suppression on a blank destination).
        assert_eq!(existing_entry_id_for_destination(&reg, ""), None);
        assert_eq!(existing_entry_id_for_destination(&reg, "   "), None);
    }
}
