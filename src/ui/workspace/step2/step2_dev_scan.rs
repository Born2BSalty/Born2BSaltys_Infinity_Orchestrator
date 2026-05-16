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
// `step1.mods_folder`), then trigger the scan via the **existing**
// `Step2Action::StartScan` dispatch path (`step_action_dispatch::
// dispatch_step2`). No BIO source is touched; no scan code is
// reimplemented.
//
// SPEC: §6, §13.12a (per-install extracted-mods folder, Phase 7), §1
//       (decision order: sibling for simple workflows).

use rfd::FileDialog;

use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::step2::action_step2::Step2Action;
use crate::ui::workspace::step_action_dispatch;

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
    orchestrator.wizard_state.step1.mods_folder = picked.to_string_lossy().to_string();

    // Kick the scan through the **existing** dispatch path — identical to
    // what the toolbar "Scan Mods Folder" button does (BIO's
    // `Step2Action::StartScan` → `bio::app::app_step2_router::
    // handle_step2_action` → `app_step2_scan::start_step2_scan`).
    step_action_dispatch::dispatch_step2(Step2Action::StartScan, orchestrator);
}
