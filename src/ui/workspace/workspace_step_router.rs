// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `workspace_step_router` — dispatch the active `WorkspaceStep` to its
// renderer. **All dispatch happens at the router layer for consistency**:
// step renderers return their action; the router dispatches via
// `step_action_dispatch::dispatch_stepN`.
//
//   - Step 2: the **C4 chrome wrapper** `bio::ui::workspace::step2::
//     workspace_step2::render(ui, orchestrator) -> Option<Step2Action>`
//     (P6.T2c). Net-new redesign chrome (title, full-width `flex` search,
//     the SPEC §6 toolbar set, **no** "Restart App With Diagnostics",
//     Details pane hidden-by-default per SPEC §6) that reuses **only**
//     BIO's tree / details / popup sub-renderers. BIO's `page_step2` /
//     `frame_step2` are **not** called. Any returned action →
//     `step_action_dispatch::dispatch_step2`.
//   - Step 3: `bio::ui::step3::page_step3::render(...)` returns `()` per H2
//     (no `Step3Action` enum — the page handles its own intents via direct
//     `WizardState` mutation: drag-reorder, undo/redo, block-select). The
//     router calls it and ignores the return; no dispatch arm.
//   - Step 4: the **C4 orchestrator-side renderer** `bio::ui::workspace::
//     step4::workspace_step4::render(ui, orchestrator) -> Option<Step4Action>`
//     (P6.T2b). Net-new redesign chrome (Save row + EET game-tab strip +
//     line-numbered three-colour review list / exact-log viewer). BIO's
//     `page_step4::render` is **never** called by the workspace router (per
//     C4 — it would double the Save button). Any returned action →
//     `step_action_dispatch::dispatch_step4`.
//   - Step 5: `workspace_step5_stub::render` (Phase 7 replaces the stub).
//
// To satisfy the borrow checker, Step 2's returned action is captured first
// (the `&mut orchestrator.wizard_state` + `&orchestrator.exe_fingerprint`
// borrows must end before `dispatch_step2(&mut orchestrator)` runs). The
// `exe_fingerprint` is cloned for the same reason.
//
// SPEC: §2.2, §6, §7, §8.

use eframe::egui;

use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::workspace::state_workspace::WorkspaceStep;
use crate::ui::workspace::step_action_dispatch;
use crate::ui::workspace::workspace_step5_stub;

/// Render the workspace's current step into `ui`.
pub fn render(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp) {
    match orchestrator.workspace_view.current_step {
        WorkspaceStep::Step2 => {
            // C4 chrome wrapper (P6.T2c): net-new redesign chrome around
            // BIO's reused tree / details / popup sub-renderers. BIO's
            // `page_step2` / `frame_step2` are NOT called.
            let action = crate::ui::workspace::step2::workspace_step2::render(ui, orchestrator);
            if let Some(a) = action {
                step_action_dispatch::dispatch_step2(a, orchestrator);
            }
        }
        WorkspaceStep::Step3 => {
            let dev_mode = orchestrator.dev_mode;
            let exe_fp = orchestrator.exe_fingerprint.clone();
            // Per H2: Step 3 returns `()`; no action dispatch arm. The page
            // handles its own intents via direct `WizardState` mutation.
            crate::ui::step3::page_step3::render(
                ui,
                &mut orchestrator.wizard_state,
                dev_mode,
                &exe_fp,
            );
        }
        WorkspaceStep::Step4 => {
            // C4 orchestrator-side renderer (P6.T2b): net-new redesign
            // chrome (Save row + EET game-tab strip + line-numbered
            // three-colour review list / exact-log viewer). BIO's
            // `page_step4::render` is intentionally NOT called (per C4 — it
            // would render a second Save button). Any returned action →
            // `dispatch_step4` (M11 — all dispatch at the router layer).
            let action = crate::ui::workspace::step4::workspace_step4::render(ui, orchestrator);
            if let Some(a) = action {
                step_action_dispatch::dispatch_step4(a, orchestrator);
            }
        }
        WorkspaceStep::Step5 => workspace_step5_stub::render(ui, orchestrator),
    }
}
