// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `page_router` — match on `OrchestratorApp::nav` and dispatch to the active
// destination's renderer (or stub).
//
// Per Phase 2 P2.T4: arms initially rendered Phase-2 stubs. Phase 4 wired the
// real `Settings` screen; Phase 5 P5.T15 wires the real `Home` screen and
// P5.T14 wires the real `Install` screen (Run 3: paste stage + stage-4 stub;
// the Preview / Downloading stages render Run-4 / Run-5 placeholders).
// `Create` / `Workspace` still render stubs until Phase 6.
// **The `Workspace` arm renders the placeholder stub — NOT the legacy
// `WizardApp::update_loop::run`.** Per H3 / C1 / C4: that path was reverted;
// Phase 6 wires the real workspace view (which calls BIO's per-step page
// renderers directly + an orchestrator-side Step 4 wrapper per C4).
//
// Per Phase 3 P3.T5: when `OrchestratorApp::registry_error` is `Some`, the
// router short-circuits to `registry_error_panel::render_registry_error` —
// the left rail + statusbar still render normally (they live in
// `OrchestratorApp::update`'s shell layout outside this router); only the
// main pane shows the error.
//
// **P6.T15 — nav-away flush (this run, SPEC §13.14 "On nav-away from the
// workspace").** When the user navigates *out of* a workspace (left-rail
// destination, a workspace-internal nav, Create's resume), the in-flight
// workspace-state edits must be flushed to disk **before the screen
// transitions** so the build is recoverable from Home / Resume even if the
// app is closed immediately after. Detection: at the top of `render`, if a
// workspace was loaded (`workspace_view.loaded_workspace_id == Some(id)`)
// but `nav` is no longer that workspace, we have left it — synchronously
// `extract_workspace_state_from_wizard` + `WorkspaceStore::save`, then clear
// `loaded_workspace_id` (so re-entering reloads cleanly via P6.T12). The
// rail renders *before* this router in `OrchestratorApp::update`, so a rail
// click's new `nav` is already visible here; a workspace-internal nav (e.g.
// Step-2 `← Previous` → Home) likewise set `nav` last frame / this frame.
// Per H4 this synchronous write + the on-exit `flush_all` are the two
// workspace persistence write paths (the debounced cadence is the third,
// for in-workspace edits). C5: a nav-away while an install runs is
// prevented by the rail-nav lock (Phase 7), so this never races an install.
//
// SPEC: §2.1, §13.14 (nav-away flush).

use eframe::egui;
use tracing::warn;

use crate::registry::model::ModlistEntry;
use crate::registry::store_workspace::WorkspaceStore;
use crate::registry::workspace_model::ModlistWorkspaceState;
use crate::ui::home::page_home;
use crate::ui::install::page_install;
use crate::ui::orchestrator::nav_destination::NavDestination;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::orchestrator::registry_error_panel;
use crate::ui::orchestrator::stubs;
use crate::ui::settings::page_settings;
use crate::ui::shared::redesign_tokens::{ThemePalette, redesign_text_faint};
use crate::ui::workspace::state_workspace::{ForkMeta, WorkspaceStep, WorkspaceStep2State};
use crate::ui::workspace::step2::step2_resume_scan;
use crate::ui::workspace::{workspace_state_loader, workspace_view};

pub fn render(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp, ctx: &egui::Context) {
    let palette = orchestrator.theme_palette;

    // Phase 3 — registry-error short-circuit. Left rail + statusbar continue
    // to render around this; only the main pane is replaced.
    if let Some(err) = orchestrator.registry_error.as_ref() {
        registry_error_panel::render_registry_error(
            ui,
            palette,
            err,
            orchestrator.registry_backup_path.as_ref(),
        );
        return;
    }

    // P6.T15 — nav-away-from-workspace flush. Detected before the nav match
    // so the synchronous write completes *before* the new destination
    // renders (the build is recoverable even if the app closes immediately).
    flush_workspace_on_nav_away(orchestrator);

    match orchestrator.nav.clone() {
        // Phase 5 P5.T15 — Home stub replaced with the real Home screen.
        NavDestination::Home => page_home::render(ui, orchestrator, ctx),
        // Phase 5 P5.T14 — Install stub replaced with the real Install
        // Modlist screen (Run 3: paste stage + stage-4 stub; Preview /
        // Downloading render Run-4 / Run-5 placeholders).
        NavDestination::Install => page_install::render(ui, orchestrator, ctx),
        // Phase 6 P6.T13 — Create stub replaced with the real Create screen
        // (Run 3: the `choose` mode + Load Draft dialog; the fork sub-flow
        // renders the Run-4 deferred placeholder).
        NavDestination::Create => crate::ui::create::page_create::render(ui, orchestrator, ctx),
        // Phase 4 P4.T8 — Settings stub replaced with the real 5-tab screen.
        NavDestination::Settings => page_settings::render(ui, orchestrator, ctx),
        NavDestination::Workspace { modlist_id } => match modlist_id {
            Some(id) => render_workspace(ui, orchestrator, &id, ctx),
            // Phase 6 keeps the dev placeholder for `Workspace { None }` so
            // testing can navigate without a real id (the Phase-2 dev-mode
            // path).
            None => stubs::render_workspace_stub(ui, palette, None),
        },
    }
}

/// Resolve + load a modlist into the orchestrator's owned `WizardState`,
/// then render the workspace shell (P6.T12).
///
/// Steps (per the plan):
///   1. Look the id up in `orchestrator.registry`.
///   2. **C5 gate (safety net):** if a *different* install is running, the
///      rail-nav lock (Phase 7 P7.T9b) should have prevented this
///      transition; as a backstop, re-pin nav to the running modlist's
///      workspace. `install_in_progress` is Phase 7 — stubbed `None` here.
///   3. If `loaded_workspace_id != Some(id)`, lazy-load the modlist's
///      `workspace.json` into the orchestrator maps and
///      `populate_wizard_state_from_workspace`; set `loaded_workspace_id`.
///   4. `workspace_view::render`. (Path sync is open-only — done once in
///      `populate_wizard_state_from_workspace`; Settings → Paths edits the
///      same in-memory `wizard_state.step1`, so no per-frame sync is needed.)
fn render_workspace(
    ui: &mut egui::Ui,
    orchestrator: &mut OrchestratorApp,
    id: &str,
    ctx: &egui::Context,
) {
    let palette = orchestrator.theme_palette;

    // 1. Registry lookup. Clone the entry out so the immutable
    //    `orchestrator.registry` borrow ends before the `&mut wizard_state`
    //    populate.
    let Some(entry) = orchestrator.registry.find(id).cloned() else {
        render_missing_modlist(ui, palette, id);
        return;
    };

    // 2. C5 safety net. `install_runtime::install_concurrency::
    //    install_in_progress(orchestrator)` is Phase 7 P7.T9b; until it
    //    exists there is no running install, so this is `None`.
    let install_in_progress: Option<String> = None; // Phase 7 P7.T9b
    if let Some(running_id) = install_in_progress.as_ref()
        && running_id != id
    {
        // Refuse the swap; re-pin nav to the running install's workspace.
        orchestrator.nav = NavDestination::Workspace {
            modlist_id: Some(running_id.clone()),
        };
        return;
    }

    // 3. Lazy-load + populate on first open / on a modlist swap.
    if orchestrator.workspace_view.loaded_workspace_id.as_deref() != Some(id) {
        // Ensure the workspace state + store are in the orchestrator maps.
        if !orchestrator.workspace_state.contains_key(id) {
            let store = WorkspaceStore::new_for_id(id);
            let loaded = match store.load() {
                Ok(ws) => ws,
                Err(err) => {
                    // `WorkspaceStore::load` maps *both* a missing file and
                    // a corrupt parse to `RegistryError::Corrupt` (a
                    // freshly-created modlist's workspace.json write is
                    // pending until Create's Run-3 path / first save). Run 1
                    // degrades safely to an empty workspace + a warn — the
                    // full SPEC §13.14 per-workspace terminal-error UI is
                    // part of the persistence integration (Run 4 / out of
                    // Run-1 scope per the brief), not a half path shipped
                    // here. (Reported as a deferred item.)
                    warn!(
                        target = "orchestrator",
                        "workspace.json for {id} not loadable ({err}); using empty state \
                         (per-workspace terminal-error UI deferred to the persistence run)"
                    );
                    ModlistWorkspaceState::default()
                }
            };
            orchestrator.workspace_state.insert(id.to_string(), loaded);
            orchestrator.workspace_stores.insert(id.to_string(), store);
        }

        // Clone the workspace state out so the `&orchestrator.workspace_state`
        // borrow ends before the `&mut orchestrator.wizard_state` populate.
        let workspace = orchestrator
            .workspace_state
            .get(id)
            .cloned()
            .unwrap_or_default();

        workspace_state_loader::populate_wizard_state_from_workspace(
            &workspace,
            &entry,
            &orchestrator.settings_store,
            &mut orchestrator.wizard_state,
        );

        // Refresh the workspace view's identity for the freshly-loaded
        // modlist. (The full header/fork population is Run 2; Run 1 sets the
        // name + game + loaded-id so the shell renders correctly and the
        // loader's swap detection works.)
        orchestrator.workspace_view.modlist_id = id.to_string();
        orchestrator.workspace_view.modlist_name = entry.name.clone();
        orchestrator.workspace_view.game = entry.game;
        orchestrator.workspace_view.current_step = WorkspaceStep::Step2;
        orchestrator.workspace_view.completed_steps.clear();
        // Fix-Run 1 (Bug B) — reset the orchestrator-owned Step-2 chrome
        // transients on a modlist swap so a stale `rescan_snapshot` /
        // `resume_pending` / `was_scanning` / drop-warning / Details-pane
        // state from the *previous* modlist can't mis-fire the
        // rescan-reconcile completion seam against the swapped-in modlist
        // (the loader resets the `WizardState` Step-2/3 *set*;
        // `WorkspaceStep2State::default()` is the documented "reset with
        // the rest of the view state on a modlist swap" contract — see
        // `state_workspace.rs`). `maybe_trigger_resume_scan` below sets a
        // fresh snapshot for *this* modlist if it is a cold-resume draft.
        orchestrator.workspace_view.step2 =
            crate::ui::workspace::state_workspace::WorkspaceStep2State::default();
        orchestrator.workspace_view.loaded_workspace_id = Some(id.to_string());
        // P6.T5 — populate `fork_meta` from the registry entry. `Some` iff
        // this modlist's `forked_from` chain is non-empty (SPEC §2.2 — the
        // fork badge + `⑂ view fork details` show only then; `step2_tab_row`
        // also keys "is this a fork" off `fork_meta.is_some()`). The
        // immediate parent is the last `forked_from` ancestor (the append
        // rule, SPEC §13.3). `mods`/`components` use the entry's cached
        // counts (the honest available denormalised figures).
        orchestrator.workspace_view.fork_meta = fork_meta_from_entry(&entry);

        // Run 2b — the #1 fix: cold-resume Step-2 restore. `populate`
        // applied the persisted `order_<tab>` onto the (empty, on a cold
        // resume) scanned mod set, so nothing matched. If this workspace
        // recorded a dev-scan folder + has a persisted order, re-point
        // `step1.mods_folder` and re-run BIO's scan (which reads its own
        // persisted scan cache — read-only — skipping WeiDU on a cache
        // hit); the rescan-reconcile completion seam re-applies the
        // persisted order + rebuilds Step 3 once the async scan lands.
        // No-op in production (no dev-scan folder is ever recorded
        // pre-Phase-7 — SPEC §13.12a) and outside dev mode.
        step2_resume_scan::maybe_trigger_resume_scan(orchestrator, &workspace);
    }

    // 4. Render the workspace shell. Path sync is open-only (done once in
    //    `populate_wizard_state_from_workspace`): Settings → Paths edits the
    //    same in-memory `orchestrator.wizard_state.step1` this renders from,
    //    so paths are live by construction — no per-frame disk read.
    workspace_view::render(ui, orchestrator, id, ctx);
}

/// Fix-Run 4 (Part 2) — is a cold-resume restore pending/unreconciled for
/// the active modlist? True when a `rescan_snapshot` is stashed (a
/// snapshot/resume is in flight) **or** `resume_pending` is still set (the
/// resume reconcile never consumed it). While either holds, the in-memory
/// `WizardState` Step-2/3 set is not yet the restored set, so **no save
/// path** may extract it over the real per-modlist `workspace.json` (the
/// on-disk file is already the correct, complete state). Pure predicate so
/// both orchestrator-owned save paths (`flush_workspace_on_nav_away` here,
/// `OrchestratorApp::sync_active_workspace_if_dirty`) share one definition
/// and it is unit-testable without an `OrchestratorApp` (the
/// `order_for_tab` pure-helper precedent).
pub(crate) fn restore_pending(step2: &WorkspaceStep2State) -> bool {
    step2.rescan_snapshot.is_some() || step2.resume_pending
}

/// P6.T15 — flush the active workspace's state to disk when the user has
/// navigated away from it (SPEC §13.14 "On nav-away from the workspace").
///
/// "Navigated away" = a workspace is loaded (`loaded_workspace_id ==
/// Some(id)`) but the current `nav` is **not** `Workspace { Some(id) }`
/// (still inside the same workspace ⇒ no flush; the debounced cadence owns
/// in-workspace edits). On nav-away: `extract_workspace_state_from_wizard`
/// (carrying the prior file's egui-only fields through unchanged — the same
/// non-drop guarantee save-draft / the debounce cycle give), write it
/// **synchronously** via `WorkspaceStore::save`, sync the persistence-cycle
/// baseline (so the debounced cadence doesn't immediately re-write the same
/// bytes), and clear `loaded_workspace_id` so re-entering reloads cleanly
/// (P6.T12). A save error is logged, not fatal — the in-memory state stays
/// (the on-exit `flush_all`, H4, is the backstop).
///
/// Per C5 this is never reached mid-install: the rail-nav lock (Phase 7
/// P7.T9b) prevents navigating out of a workspace while its install runs.
fn flush_workspace_on_nav_away(orchestrator: &mut OrchestratorApp) {
    let Some(id) = orchestrator.workspace_view.loaded_workspace_id.clone() else {
        return; // no workspace loaded — nothing to flush.
    };
    // Still inside the same workspace ⇒ not a nav-away (the debounced
    // cadence handles in-workspace edits).
    if let NavDestination::Workspace {
        modlist_id: Some(cur),
    } = &orchestrator.nav
        && cur == &id
    {
        return;
    }

    // Fix-Run 4 (Part 2) — restore-pending save guard. While a cold-resume
    // restore is pending/unreconciled for the active modlist
    // (`rescan_snapshot` set OR `resume_pending`), the in-memory
    // `WizardState` Step-2/3 set is the *empty post-`populate` shell* (the
    // resume-triggered scan + reconcile have not landed yet). The on-disk
    // `workspace.json` is already correct and there is nothing legitimate to
    // persist until the restore reconciles. Extracting + saving here would
    // write that empty shell over the real per-modlist file (and poison the
    // in-memory `workspace_state` map). SKIP the extract/save/map-insert
    // entirely — but still clear `loaded_workspace_id` so re-entering this
    // (or another) workspace reloads cleanly via P6.T12's swap detection
    // (preserving the existing post-flush behavior; only the write is
    // dropped). The Fix-Run-3 `order_for_tab` guard remains correct for the
    // production/never-refilled path; this covers the dev fast-scan window
    // where the scanned set *will* be refilled but isn't yet.
    if restore_pending(&orchestrator.workspace_view.step2) {
        orchestrator.workspace_view.loaded_workspace_id = None;
        return;
    }

    // Fix-Run 1 (Bug A) — make the live Step-2 selection visible to
    // `extract`. A Step-2 checkbox toggle mutates only `step2.<tab>_mods`
    // (BIO's reused tree emits no action and doesn't touch Step 3);
    // `extract` reads the persisted order from `step3.<tab>_items`. Without
    // this BIO-faithful, Step-3-reorder-safe sync, a toggle made on Step 2
    // and then a nav Home would extract the stale pre-toggle order and the
    // edit would be lost on resume.
    workspace_state_loader::sync_step3_from_step2_if_changed(&mut orchestrator.wizard_state);

    // Left the workspace — extract + synchronous write (the save-draft
    // P6.T6 precedent, but triggered by the nav transition).
    let prior = orchestrator
        .workspace_state
        .get(&id)
        .cloned()
        .unwrap_or_default();
    let extracted = workspace_state_loader::extract_workspace_state_from_wizard(
        &orchestrator.wizard_state,
        &prior,
    );
    orchestrator
        .workspace_state
        .insert(id.clone(), extracted.clone());

    if let Some(store) = orchestrator.workspace_stores.get(&id) {
        match store.save(&extracted) {
            Ok(()) => {
                // Sync the persistence-cycle baseline so the debounced
                // cadence sees "already saved" and doesn't re-write the
                // identical bytes a frame later (idempotent — the
                // save-draft P6.T6 baseline-sync precedent).
                orchestrator
                    .persistence_cycle
                    .last_saved_workspaces
                    .insert(id.clone(), extracted);
            }
            Err(err) => warn!(
                target = "orchestrator",
                "nav-away workspace flush for {id} failed: {err} \
                 (in-memory state retained; on-exit flush_all is the backstop)"
            ),
        }
    } else {
        warn!(
            target = "orchestrator",
            "nav-away flush: no WorkspaceStore registered for {id} \
             (state kept in memory; on-exit flush_all is the backstop)"
        );
    }

    // Clear so re-entering this (or another) workspace reloads cleanly via
    // P6.T12's loaded-id swap detection.
    orchestrator.workspace_view.loaded_workspace_id = None;
}

/// Build `WorkspaceViewState::fork_meta` from a registry entry (P6.T5).
/// Returns `None` for a from-scratch (non-forked) modlist — `forked_from`
/// empty — so the fork badge / sub-line / `⑂ view fork details` are all
/// hidden (SPEC §2.2). For a forked modlist the immediate parent is the
/// **last** `forked_from` ancestor (the append rule — SPEC §13.3); the full
/// chain is carried through for the reused `ForkInfoPopup`.
fn fork_meta_from_entry(entry: &ModlistEntry) -> Option<ForkMeta> {
    if entry.forked_from.is_empty() {
        return None;
    }
    let parent = entry.forked_from.last();
    Some(ForkMeta {
        parent_name: parent.map(|p| p.name.clone()).unwrap_or_default(),
        parent_author: parent.map(|p| p.author.clone()).unwrap_or_default(),
        mods: entry.mod_count,
        components: entry.component_count,
        forked_from: entry.forked_from.clone(),
    })
}

/// Shown when the routed workspace id is not in the registry (e.g. a stale
/// nav after the entry was deleted from Home). Not a terminal error — the
/// registry itself is fine; only this id is gone.
fn render_missing_modlist(ui: &mut egui::Ui, palette: ThemePalette, id: &str) {
    ui.add_space(8.0);
    ui.label(
        egui::RichText::new(format!(
            "Modlist \"{id}\" is no longer in the registry. It may have been deleted.",
        ))
        .size(13.0)
        .family(egui::FontFamily::Proportional)
        .color(redesign_text_faint(palette)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::workspace::state_workspace::{RescanSelection, RescanSnapshot};

    // ── Fix-Run 4 Part 2 — the restore-pending save guard. ──
    //
    // Both orchestrator-owned save paths (`flush_workspace_on_nav_away`
    // here, `OrchestratorApp::sync_active_workspace_if_dirty`) early-return
    // before the extract/`workspace_state.insert`/`WorkspaceStore::save`
    // when `restore_pending(&workspace_view.step2)` is true — so the
    // in-memory `workspace_state` entry is left at its prior good value and
    // nothing is written over the (already-correct) per-modlist
    // `workspace.json` while a cold-resume restore is in flight. The gate is
    // this pure predicate (the `order_for_tab` pure-helper precedent —
    // tested without an `OrchestratorApp`, per the test-hygiene rule: no
    // real-config-dir store). These pin the exact field combinations the
    // resume/rescan paths set, and the negative (a genuine deselect-all with
    // NO restore pending) proving the guard does not over-block.

    fn snap() -> RescanSnapshot {
        RescanSnapshot {
            bgee: vec![RescanSelection {
                tp2_upper: "BG1UB/BG1UB.TP2".to_string(),
                component_id: "0".to_string(),
                selected_order: Some(1),
            }],
            bg2ee: Vec::new(),
        }
    }

    /// Cold-resume in flight as `step2_resume_scan::maybe_trigger_resume_
    /// scan` leaves it: `rescan_snapshot = Some`, `resume_pending = true`.
    /// The empty post-`populate` wizard order must NOT be saved → the gate
    /// fires. (No `OrchestratorApp` is constructed: the early return in both
    /// save paths is driven solely by this predicate over `step2`.)
    #[test]
    fn fixrun4_resume_pending_blocks_the_save() {
        let step2 = WorkspaceStep2State {
            rescan_snapshot: Some(snap()),
            resume_pending: true,
            ..Default::default()
        };
        assert!(
            restore_pending(&step2),
            "resume in flight (snapshot + resume_pending) must block extract/save \
             so the empty post-populate order is NOT written over workspace.json"
        );
    }

    /// A dev *rescan* in flight (`rescan_snapshot` set, `resume_pending`
    /// false — the §6.3 reconcile path): still pending, still must not save
    /// the transient empty/mid-scan state.
    #[test]
    fn fixrun4_rescan_snapshot_alone_blocks_the_save() {
        let step2 = WorkspaceStep2State {
            rescan_snapshot: Some(snap()),
            resume_pending: false,
            ..Default::default()
        };
        assert!(
            restore_pending(&step2),
            "a snapshot in flight (rescan, no resume) must also block the save"
        );
    }

    /// `resume_pending` still set but the snapshot already `.take()`-n by
    /// the reconcile (the brief's OR arm): the resume reconcile has not
    /// finished consuming `resume_pending` yet → still blocked.
    #[test]
    fn fixrun4_resume_pending_without_snapshot_still_blocks() {
        let step2 = WorkspaceStep2State {
            rescan_snapshot: None,
            resume_pending: true,
            ..Default::default()
        };
        assert!(
            restore_pending(&step2),
            "resume_pending set (snapshot taken, reconcile mid-flight) must still block"
        );
    }

    /// **Negative — the guard must not over-block.** No restore pending
    /// (`rescan_snapshot = None`, `resume_pending = false`) — a genuine
    /// empty (scanned set non-empty, user deselected everything). The save
    /// must proceed: the predicate is `false`, so neither save path
    /// early-returns and the legitimate empty order is persisted (parity
    /// with the Fix-Run-3 `fixrun3_genuine_deselect_all_*` negative).
    #[test]
    fn fixrun4_no_restore_pending_lets_the_save_proceed() {
        let step2 = WorkspaceStep2State::default();
        assert!(
            !step2.rescan_snapshot.is_some() && !step2.resume_pending,
            "precondition: nothing pending"
        );
        assert!(
            !restore_pending(&step2),
            "no restore pending ⇒ guard must NOT fire (a genuine deselect-all \
             edit still persists; the guard must not over-block)"
        );
    }
}
