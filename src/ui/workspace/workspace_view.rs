// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `workspace_view` — the top-level workspace renderer. Owns the layout of
// the workspace shell (SPEC §2.2 / wireframe `WorkspaceView`):
//
//   1. Header row — `Editing <modlist name>` (small caps).
//   2. `WorkspaceProgressBar` (4 steps, completed-state).
//   3. Per-step hint line.
//   4. Active step's content area (`workspace_step_router::render`).
//   5. `WorkspaceNavBar` (← Previous / step indicator / Next →).
//
// **Run-1 scope.** The header here is a **minimal title** (`Editing
// <name>`). The full `workspace_header` (✎ inline rename + Fork badge +
// `⑂ view fork details` + `save draft` / `Share import code`) is **Run 2
// (P6.T5 / P6.T6)** — explicitly NOT this run. The `SharePasteCodeDialog` /
// `ForkInfoPopup` overlays are Run 2/3, so `ctx` is unused this run (kept
// in the signature for API stability — later runs render popups over the
// shell with it).
//
// The nav bar's outcome drives step advancement + the progress-bar
// checkmarks: crossing forward into a new step marks the step being left as
// completed (wireframe `goNext`: `if (!completed.includes(tab))
// setCompleted([...completed, tab])`).
//
// Per the plan's file inventory the signature is
// `render(ui, orchestrator, modlist_id, ctx)`. The caller
// (`page_router`) has already ensured the modlist is loaded into the
// orchestrator's `WizardState` (populate / sync) before calling this.
//
// SPEC: §2.2, §6.1 (Step 2 layout = this exact stack).

use eframe::egui;

use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::shared::redesign_tokens::{ThemePalette, redesign_text_muted};
use crate::ui::workspace::state_workspace::WorkspaceStep;
use crate::ui::workspace::{
    workspace_hint_line, workspace_nav_bar, workspace_progress_bar, workspace_step_router,
};

/// Render the workspace shell for the modlist currently loaded into the
/// orchestrator's `WizardState`. `modlist_id` is the routed id (already
/// resolved + loaded by `page_router`); `_ctx` is reserved for the Run-2/3
/// popup overlays.
pub fn render(
    ui: &mut egui::Ui,
    orchestrator: &mut OrchestratorApp,
    _modlist_id: &str,
    _ctx: &egui::Context,
) {
    let palette = orchestrator.theme_palette;

    // ── 1. Header row — minimal `Editing <name>` (Run-1 scope; the full
    //    rename/fork/save-draft header is Run 2's `workspace_header`). ──
    render_minimal_header(ui, palette, &orchestrator.workspace_view.modlist_name);
    ui.add_space(10.0);

    // ── 2. Progress bar. ──
    workspace_progress_bar::render(ui, palette, &orchestrator.workspace_view);

    // ── 3. Per-step hint line. ──
    let current = orchestrator.workspace_view.current_step;
    workspace_hint_line::render(ui, palette, current);

    // ── 4. Active step content. Wrapped so the step body takes the
    //    remaining vertical space and the nav bar stays pinned at the
    //    bottom (wireframe `flex:1 minHeight:0` content + `flexShrink:0`
    //    nav bar). ──
    let nav_reserve = 84.0; // ~ WorkspaceNavBar footprint (20 margin + 14
    // pad + ~30 control row + breathing room) — keeps the nav bar visible.
    let avail_h = ui.available_height();
    let body_h = (avail_h - nav_reserve).max(0.0);
    ui.allocate_ui(egui::vec2(ui.available_width(), body_h), |ui| {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                workspace_step_router::render(ui, orchestrator);
            });
    });

    // ── 5. Nav bar. `disable_prev` is `false` in Run 1 (the Phase-7
    //    install-running / post-install gate sets it later). ──
    let outcome = workspace_nav_bar::render(ui, palette, current, false);

    // Apply nav outcome after the render borrows end.
    if outcome.next_clicked {
        if let Some(next) = current.next() {
            // #5 fix — mirror BIO's Step2→Step3 sync on the forward nav
            // edge. BIO's `WizardApp` rebuilds Step 3 from Step 2 in
            // `app_nav_actions::advance_after_next` when leaving Step 2
            // (`current_step == 1`); the orchestrator's own nav never did,
            // so Step 3 stayed empty/stale. We mirror BIO's EXACT trigger
            // + semantics here (no reimplementation, no BIO edit).
            if current == WorkspaceStep::Step2 {
                sync_step3_from_step2_on_nav_edge(orchestrator);
            }
            // Crossing forward marks the step being left as completed
            // (wireframe `goNext`).
            orchestrator.workspace_view.completed_steps.insert(current);
            orchestrator.workspace_view.current_step = next;
        }
    } else if outcome.prev_clicked {
        if let Some(prev) = current.prev() {
            orchestrator.workspace_view.current_step = prev;
        }
    }
}

/// Mirror BIO's Step2→Step3 sync at the orchestrator's Step2→Step3 forward
/// nav edge (the #5 fix).
///
/// **What BIO does (the mirrored call site + semantics).** BIO's
/// `WizardApp` Next handler (`bio::app::app_nav_actions::advance_after_next`,
/// `app_nav_actions.rs:131-156`) asks `bio::app::app_nav::decide_next_action`
/// (`app_nav.rs:85-114`) for the action. When leaving Step 2
/// (`current_step == 1`) and the Step-2 selection changed since the last
/// sync (or Step 3 has no real items), that returns
/// `NextAction::SyncStep3AndAdvance { signature }`, on which
/// `advance_after_next` runs **exactly**:
///
/// ```ignore
/// super::app_step3_sync_flow::sync_step3_from_step2(state);
/// state.set_last_step2_sync_signature(signature.clone());
/// ```
///
/// We replicate **that** arm verbatim — calling BIO's own `pub(crate)`
/// `decide_next_action` (so the change-detection signature is BIO's own,
/// carried in the enum payload — zero logic copied) and BIO's own
/// `pub(crate)` `sync_step3_from_step2`. The orchestrator owns its own
/// step machine, so we do NOT run BIO's `apply_next_action`/`go_next` or
/// its settings-save; `wizard_state.current_step` is temporarily set to
/// BIO's Step-2 index `1` only so `decide_next_action` evaluates the right
/// branch, then restored (it is a pure `&WizardState` read with no
/// mutation, so save/restore is sound and leaves no residue).
///
/// **Clobber protection (the Step-3 reorder concern).** This is BIO's own
/// design, inherited by mirroring it exactly:
///   - If the user only reordered in Step 3 and the Step-2 selection is
///     unchanged, `decide_next_action` finds the signature unchanged AND
///     Step 3 has real items, so it returns a NON-sync variant →
///     `sync_step3_from_step2` is **not called** → the Step-3 order is
///     left untouched.
///   - If the Step-2 selection did change, `sync_step3_from_step2` →
///     `reconcile_step3_items` (`app_step3_sync_flow.rs:32-77`) preserves
///     the relative order of still-selected Step-3 items and appends only
///     the newly-selected ones — exactly BIO's behavior.
fn sync_step3_from_step2_on_nav_edge(orchestrator: &mut OrchestratorApp) {
    use crate::app::app_nav::{NextAction, decide_next_action};
    use crate::app::app_step3_sync_flow::sync_step3_from_step2;

    let state = &mut orchestrator.wizard_state;

    // Temporarily present "we are on Step 2" to BIO's decision fn (BIO's
    // Step-2 index is 1). `decide_next_action` is a pure read; restore after.
    let saved_step = state.current_step;
    state.current_step = 1;
    let action = decide_next_action(state);
    state.current_step = saved_step;

    // Replicate ONLY `advance_after_next`'s `SyncStep3AndAdvance` arm
    // (`app_nav_actions.rs:137-140`) — BIO's own sync + signature write.
    if let NextAction::SyncStep3AndAdvance { signature } = action {
        sync_step3_from_step2(state);
        state.set_last_step2_sync_signature(signature);
    }
}

/// The Run-1 minimal header: `Editing <name>` in the wireframe's small-caps
/// title style (Poppins 13 / 500, muted). The ✎ rename affordance + Fork
/// badge + right-side buttons are Run 2's `workspace_header`.
fn render_minimal_header(ui: &mut egui::Ui, palette: ThemePalette, name: &str) {
    let title = if name.trim().is_empty() {
        "Editing modlist".to_string()
    } else {
        format!("Editing {name}")
    };
    ui.label(
        egui::RichText::new(title)
            .size(13.0)
            .family(egui::FontFamily::Name("poppins_medium".into()))
            .color(redesign_text_muted(palette)),
    );
}

/// The vertical footprint the nav bar reserves at the bottom (exported so a
/// future header run can keep the layout math in one place).
pub const NAV_BAR_RESERVE_PX: f32 = 84.0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nav_reserve_constant_is_reasonable() {
        // Sanity: the reserve must cover the nav bar's ~64px footprint
        // (sub_flow_footer's FOOTER_HEIGHT_PX) plus the 20px top margin.
        assert!(NAV_BAR_RESERVE_PX >= 64.0);
        assert!(NAV_BAR_RESERVE_PX <= 120.0);
    }

    #[test]
    fn step_advance_logic_matches_wireframe_gonext() {
        // Pure logic mirror of the in-render nav-outcome handling: forward
        // from Step 2 marks Step 2 completed and lands on Step 3.
        let mut completed = std::collections::HashSet::new();
        let current = WorkspaceStep::Step2;
        // simulate next_clicked:
        if let Some(next) = current.next() {
            completed.insert(current);
            assert_eq!(next, WorkspaceStep::Step3);
        }
        assert!(completed.contains(&WorkspaceStep::Step2));
    }
}
