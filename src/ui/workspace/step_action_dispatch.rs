// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `step_action_dispatch` — the thin layer that maps each `Step2Action` /
// `Step4Action` a step-page renderer returns to the corresponding BIO
// public action-handler entry point in `bio::app::*`.
//
// **The orchestrator does not call `WizardApp::handle_*`** (those are
// `WizardApp`-internal). It calls the same `bio::app::*` public functions
// those handlers call — exactly as `bio::ui::app::update_loop::run` (the
// H3-corrected real path) does. Per the Phase-6 "Step action dispatch
// surface" sub-section:
//
//   - Step 2: 22 of 24 `Step2Action` variants are one direct call to
//     `bio::app::app_step2_router::handle_step2_action(&mut state, &mut
//     scan_rx, &mut cancel, &mut progress_queue, &mut update_check_rx, &mut
//     update_download_rx, action)` (the orchestrator owns those 5 receiver
//     fields). The 2 `SelectBgeeViaLog` / `SelectBg2eeViaLog` variants route
//     to the `step2_log_glue` sibling (rfd picker + settings-persistence
//     trigger), because BIO's UI-layer log wrapper couples those to a
//     `&mut WizardApp` (outside carve-out #4).
//   - Step 3: no action enum (`page_step3::render` returns `()` per H2) —
//     no `dispatch_step3` exists; the router calls the page and ignores the
//     return.
//   - Step 4: both `Step4Action` variants are a direct call to
//     `bio::app::app_step4_flow::handle_step4_action(&mut state, action)`.
//
// **Workspace-dirty marking.** Mutating variants call
// `orchestrator.mark_workspace_dirty()` so Run 4's debounced workspace
// write picks them up. **Run 1 only sets the flag** — nothing drains it
// yet (the debounced write is P6.T11 / Run 4). The `OpenSelected*` shell-
// open variants do not mutate `WizardState` (BIO's router routes them to
// `open_in_shell`), so they do not mark dirty. Conservatism is safe: the
// Run-4 consumer does an extract+compare before writing, so a spurious
// dirty is at worst one no-op compare.
//
// SPEC: §6 (Step 2), §8 (Step 4), §13.14 (persistence — dirty marking),
//       §1 (decision order — direct reuse / sibling for simple workflows).

use crate::app::app_step2_router;
use crate::app::app_step4_flow;
use crate::app::step4_action::Step4Action;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::step2::action_step2::Step2Action;
use crate::ui::workspace::step2_log_glue;

/// Dispatch a `Step2Action` returned by `bio::ui::step2::page_step2::render`.
///
/// 22 variants → one direct call to `bio::app::app_step2_router::
/// handle_step2_action` with the orchestrator's owned receivers. The 2
/// `SelectVia*Log` variants → the `step2_log_glue` sibling.
pub fn dispatch_step2(action: Step2Action, orchestrator: &mut OrchestratorApp) {
    match action {
        // The two log-picker variants are the sibling path (rfd picker +
        // settings-persistence trigger + underlying state mutation). They
        // also apply a log selection (mutate `step2`), so mark dirty.
        Step2Action::SelectBgeeViaLog => {
            step2_log_glue::apply_weidu_log_selection_for_orchestrator(orchestrator, true);
            orchestrator.mark_workspace_dirty();
        }
        Step2Action::SelectBg2eeViaLog => {
            step2_log_glue::apply_weidu_log_selection_for_orchestrator(orchestrator, false);
            orchestrator.mark_workspace_dirty();
        }

        // The `OpenSelected*` variants only shell-open a file / URL (BIO's
        // router → `open_in_shell`); they do not mutate `WizardState`, so
        // they dispatch but do NOT mark the workspace dirty.
        Step2Action::OpenSelectedReadme(_)
        | Step2Action::OpenSelectedWeb(_)
        | Step2Action::OpenSelectedTp2Folder(_)
        | Step2Action::OpenSelectedTp2(_)
        | Step2Action::OpenSelectedIni(_) => {
            handle_step2_via_bio(action, orchestrator);
        }

        // Everything else routes through BIO's public router and can mutate
        // `state.step2` (scan, update-check/download, source edits, compat
        // popup, lock toggles, …) — mark the workspace dirty.
        _ => {
            handle_step2_via_bio(action, orchestrator);
            orchestrator.mark_workspace_dirty();
        }
    }
}

/// Single direct call to BIO's public Step 2 router with the orchestrator's
/// owned channel receivers — the exact 7-arg signature
/// `bio::app::app_step2_router::handle_step2_action` exposes
/// (`src/core/app/app_step2_router.rs:15`). Note this takes **5** receivers
/// (`scan_rx`, `cancel`, `progress_queue`, `update_check_rx`,
/// `update_download_rx`) — `step2_update_extract_rx` is only required by
/// `poll_before_render`, not by the action router.
fn handle_step2_via_bio(action: Step2Action, orchestrator: &mut OrchestratorApp) {
    app_step2_router::handle_step2_action(
        &mut orchestrator.wizard_state,
        &mut orchestrator.step2_scan_rx,
        &mut orchestrator.step2_cancel,
        &mut orchestrator.step2_progress_queue,
        &mut orchestrator.step2_update_check_rx,
        &mut orchestrator.step2_update_download_rx,
        action,
    );
}

/// Dispatch a `Step4Action`. Both variants are a direct call to
/// `bio::app::app_step4_flow::handle_step4_action(&mut state, action)` (the
/// `pub(crate) fn` per `src/core/app/app_step4_flow.rs:8`, same-crate
/// reachable; already takes `&mut WizardState`). `SaveWeiduLog` /
/// `CheckMissingMods` both mutate `WizardState` → mark the workspace dirty.
///
/// The orchestrator-side Step 4 renderer (C4) is Run 2 (P6.T2b); this
/// dispatch arm is provided in Run 1 so the router has a uniform
/// `dispatch_step4` to call once that renderer lands.
pub fn dispatch_step4(action: Step4Action, orchestrator: &mut OrchestratorApp) {
    app_step4_flow::handle_step4_action(&mut orchestrator.wizard_state, action);
    orchestrator.mark_workspace_dirty();
}
