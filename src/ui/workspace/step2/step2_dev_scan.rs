// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `step2_dev_scan` — the **dev-only** Step-2 scan-folder affordance
// (P6.T2c). Behind `orchestrator.dev_mode` only; **never shipped in normal
// mode** (the control is simply not rendered when `dev_mode == false`).
//
// **Why this exists.** There is no global Settings "mods" folder — Settings
// → Paths supplies game-source / Mods-*archive* (content-addressed
// downloads) / backup / tool paths only. The **scannable mods folder is
// per-install, extracted at prep time (post-download, pre-Step-2) by the
// Phase-7 P7.T17 pipeline** (SPEC §13.12a). So pre-Phase-7 the production
// scan legitimately finds nothing — that is correct, not a bug. This
// affordance lets a developer point BIO's scan at an arbitrary existing
// folder so the Step-2 chrome + reused BIO tree + dispatch is visually
// verifiable before Phase 7 binds the live pipeline.
//
// Mechanism (mirrors `step2_log_glue`'s rfd pattern — replicated, not
// invented): open an `rfd` folder dialog, write the chosen path into
// `wizard_state.step1.mods_folder` (the field BIO's scan worker reads —
// `bio::app::step2::scan::worker` clones `state.step1` and uses
// `step1.mods_folder`), **record the folder into the persisted
// `ModlistWorkspaceState`** so a cold resume can rebuild it (the #1 fix —
// `step2_resume_scan` reads `dev_scanned_mods_folder` on workspace open),
// **snapshot the current selection** for the SPEC §6.3 rescan-reconcile
// (the #2 fix — the dev scan is the functional rescan path pre-Phase-7),
// then trigger the scan via the **existing** `Step2Action::StartScan`
// dispatch path (`step_action_dispatch::dispatch_step2`). The re-apply runs
// on scan-completion in `step2_rescan_reconcile::reconcile_on_scan_complete`
// (driven from `OrchestratorApp::update` after the channel drain). No BIO
// source is touched; no scan code is reimplemented.
//
// **Why persist the folder (the #1 fix).** BIO's scan worker persists its
// own scan cache (`save_scan_cache`), so on resume re-running BIO's scan
// reads from cache and skips WeiDU. But `cache_context` needs `mods_root`,
// and the dev-scan's chosen folder is only ever set on the in-memory
// `wizard_state.step1.mods_folder` — lost on relaunch (`sync_paths_from_
// settings` overwrites it from settings, which has no per-install mods
// folder pre-Phase-7, SPEC §13.12a). Recording it into the per-modlist
// `workspace.json` (the orchestrator's own serde model — net-new field) is
// what makes resume able to re-point the scan. Written **synchronously,
// before** `StartScan` (conservative-by-default, the §13.13 artifact
// rationale — the recording must survive even if the scan/app crashes), via
// the same `extract` + `WorkspaceStore::save` path `workspace_header::
// save_draft` uses.
//
// SPEC: §6, §13.1 (per-modlist workspace state), §13.12a (per-install
//       extracted-mods folder, Phase 7), §1 (decision order: sibling for
//       simple workflows).

use rfd::FileDialog;
use tracing::warn;

use crate::registry::store_workspace::WorkspaceStore;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::step2::action_step2::Step2Action;
use crate::ui::workspace::step_action_dispatch;
use crate::ui::workspace::step2::step2_rescan_reconcile;
use crate::ui::workspace::workspace_state_loader;

/// Open a folder picker; if the user picks a folder, point BIO's scan at it
/// and kick the scan via the existing `StartScan` dispatch path.
///
/// Start directory: the currently-configured `mods_folder` (if any) so a
/// developer iterating on the same test corpus doesn't re-navigate.
pub fn pick_folder_and_scan(orchestrator: &mut OrchestratorApp) {
    let current = orchestrator.wizard_state.step1.mods_folder.trim();
    let mut dialog = FileDialog::new().set_title("Dev: pick a mods folder to scan");
    if !current.is_empty() {
        dialog = dialog.set_directory(current);
    }
    let Some(picked) = dialog.pick_folder() else {
        return; // user cancelled — leave the configured folder untouched.
    };

    // Set the scan source (the exact field BIO's scan worker reads).
    let folder = picked.to_string_lossy().to_string();
    orchestrator.wizard_state.step1.mods_folder = folder.clone();

    // #1 fix — record the chosen folder into the persisted per-modlist
    // `workspace.json` so a cold resume can re-point BIO's scan at it (and
    // BIO's persisted scan cache then makes the resume scan WeiDU-free).
    // Written synchronously **before** the scan (conservative-by-default —
    // the §13.13 "survive a crash" rationale): the recording must outlive
    // the scan even if it crashes/cancels.
    record_dev_scan_folder(orchestrator, &folder);

    // SPEC §6.3 — rescan is **non-destructive**: snapshot the current
    // selection BEFORE the scan so it can be re-applied onto the
    // freshly-scanned mod set on scan-completion (the dev scan is the
    // functional rescan path pre-Phase-7; the production Rescan button is
    // inert per §13.12a). Must run *before* `StartScan` is dispatched.
    step2_rescan_reconcile::snapshot_current_selection(orchestrator);

    // Kick the scan through the **existing** dispatch path — identical to
    // what the toolbar "Scan Mods Folder" button does (BIO's
    // `Step2Action::StartScan` → `bio::app::app_step2_router::
    // handle_step2_action` → `app_step2_scan::start_step2_scan`).
    step_action_dispatch::dispatch_step2(Step2Action::StartScan, orchestrator);
}

/// Persist the chosen dev-scan `folder` into the modlist's
/// `workspace.json::dev_scanned_mods_folder` so a cold resume
/// (`step2_resume_scan`) can re-point BIO's scan at it. Mirrors
/// `workspace_header::save_draft` exactly: `extract` (carrying the prior
/// egui-side fields), set the new field, `WorkspaceStore::save`, then sync
/// the in-memory map + the persistence-cycle baseline so the Run-4 debounced
/// cycle doesn't redundantly rewrite the identical state. No-op (warn only)
/// if there is no loaded modlist or the write fails — the dev-scan still runs
/// this session; only cross-restart resume is affected.
fn record_dev_scan_folder(orchestrator: &mut OrchestratorApp, folder: &str) {
    let id = orchestrator.workspace_view.modlist_id.clone();
    if id.is_empty() {
        return;
    }

    // `prior` = the workspace state currently in the orchestrator's map (or
    // default if not yet loaded). `extract` carries `prior`'s egui-side
    // fields (Step-2 expand map, prompt overrides, last_share_code, and the
    // dev-scan folder itself) through unchanged.
    let prior = orchestrator
        .workspace_state
        .get(&id)
        .cloned()
        .unwrap_or_default();
    let mut extracted = workspace_state_loader::extract_workspace_state_from_wizard(
        &orchestrator.wizard_state,
        &prior,
    );
    extracted.dev_scanned_mods_folder = Some(folder.to_string());

    let store = orchestrator
        .workspace_stores
        .entry(id.clone())
        .or_insert_with(|| WorkspaceStore::new_for_id(&id));

    match store.save(&extracted) {
        Ok(()) => {
            // Keep the in-memory map + the persistence-cycle baseline in
            // sync (same as `save_draft`) so the Run-4 debounced cycle
            // doesn't immediately rewrite the identical state.
            orchestrator
                .workspace_state
                .insert(id.clone(), extracted.clone());
            orchestrator
                .persistence_cycle
                .last_saved_workspaces
                .insert(id, extracted);
        }
        Err(err) => {
            warn!(
                target = "orchestrator",
                "recording dev-scan folder for the workspace failed: {err} \
                 (this session's scan still runs; cross-restart resume won't)"
            );
        }
    }
}
