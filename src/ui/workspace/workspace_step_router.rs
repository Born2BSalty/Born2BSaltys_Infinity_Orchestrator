// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `workspace_step_router` — dispatch the active `WorkspaceStep` to its
// renderer. **All dispatch happens at the router layer for consistency**:
// step renderers return their action; the router dispatches via
// `step_action_dispatch::dispatch_stepN`.
//
//   - Step 2: `bio::ui::step2::page_step2::render(ui, &mut
//     orchestrator.wizard_state, dev_mode, &exe_fingerprint) ->
//     Option<Step2Action>` — called **directly** (the wireframe content of
//     Step 2 is unchanged from today's BIO; no orchestrator wrapper). Any
//     returned action → `step_action_dispatch::dispatch_step2`.
//   - Step 3: `bio::ui::step3::page_step3::render(...)` returns `()` per H2
//     (no `Step3Action` enum — the page handles its own intents via direct
//     `WizardState` mutation: drag-reorder, undo/redo, block-select). The
//     router calls it and ignores the return; no dispatch arm.
//   - Step 4: a **minimal honest placeholder** this run. The real C4
//     orchestrator-side Step 4 renderer is **Run 2 (P6.T2b)** — explicitly
//     NOT Run 1. BIO's `page_step4::render` is never called by the
//     workspace router (per C4).
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
use crate::ui::shared::redesign_tokens::{ThemePalette, redesign_text_faint, redesign_text_muted};
use crate::ui::workspace::state_workspace::WorkspaceStep;
use crate::ui::workspace::step_action_dispatch;
use crate::ui::workspace::workspace_step5_stub;

/// Render the workspace's current step into `ui`.
pub fn render(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp) {
    match orchestrator.workspace_view.current_step {
        WorkspaceStep::Step2 => {
            let dev_mode = orchestrator.dev_mode;
            // Clone the fingerprint so the `&orchestrator` borrow ends
            // before the `&mut orchestrator` dispatch below.
            let exe_fp = orchestrator.exe_fingerprint.clone();
            let action = crate::ui::step2::page_step2::render(
                ui,
                &mut orchestrator.wizard_state,
                dev_mode,
                &exe_fp,
            );
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
            // Run 1: minimal honest placeholder. The C4 orchestrator-side
            // Step 4 renderer (Save row + game tab strip + line-numbered
            // review list / exact-log viewer) is Run 2 (P6.T2b). BIO's
            // `page_step4::render` is intentionally NOT called (per C4).
            render_step4_placeholder(ui, orchestrator.theme_palette);
        }
        WorkspaceStep::Step5 => workspace_step5_stub::render(ui, orchestrator),
    }
}

/// The Run-1 Step 4 placeholder. Replaced by `workspace_step4::render`
/// (P6.T2b — Run 2).
fn render_step4_placeholder(ui: &mut egui::Ui, palette: ThemePalette) {
    ui.label(
        egui::RichText::new("Step 4 \u{2014} Review")
            .size(15.0)
            .family(egui::FontFamily::Name("poppins_medium".into()))
            .color(redesign_text_muted(palette)),
    );
    ui.add_space(6.0);
    ui.label(
        egui::RichText::new(
            "The full Step 4 review renderer (Save weidu.log's + game tab strip + line-numbered \
             order list) lands in Run 2.",
        )
        .size(13.0)
        .family(egui::FontFamily::Proportional)
        .color(redesign_text_faint(palette)),
    );
}
