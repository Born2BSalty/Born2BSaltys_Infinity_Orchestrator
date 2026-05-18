// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `page_workspace_step5` — the top-level Step-5 install-runtime **chrome**
// for the workspace. Replaces `workspace_step5_stub`.
//
// Per SPEC §9 + the CRITICAL DIRECTIVE this is net-new redesign chrome
// that composes **around** BIO's existing embedded Step-5 panel. It does
// NOT reach into BIO's Step-5 module tree and does NOT edit any BIO Step-5
// file — it calls the verified public top-level renderer
// `bio::ui::step5::page_step5::render` (signature verified:
// `pub fn render(ui, &mut WizardState, &mut Step5ConsoleViewState,
// Option<&mut EmbeddedTerminal>, Option<&str>, dev_mode, exe_fingerprint)
// -> Option<Step5Action>`) and wraps it with the new chrome rows.
//
// **Render order (per H9 — chrome rows ABOVE the embedded panel).**
//   1. Success-banner row    (`success_banner::render`) — empty
//      pre-install (the C3 clean-exit triple is false).
//   2. Post-install action row (`post_install_actions::render`),
//      immediately below the banner row — empty/hidden pre-install. Per H9
//      this places the post-install actions visually adjacent to BIO's
//      Install button, which sits at the **top** of `page_step5::render`'s
//      panel.
//   3. BIO's entire Step-5 panel (`page_step5::render`) — Command card,
//      Summary card, Actions/Diagnostics menus, Prompt Answers, console
//      box, prompt input row. `terminal = step5_terminal.as_mut()` is
//      `None` pre-install ⇒ BIO renders its pre-install panel with no live
//      child (the Run-1 breakpoint state).
//   4. Dispatch the returned `Option<Step5Action>`.
//
// **Run 1 scope (Step-5 runtime spine + workspace chrome).** This run
// implements ONLY the chrome scaffold:
//   - The banner / post-install rows render nothing (C3 false — no install
//     has run).
//   - The embedded panel is BIO's pre-install view (no terminal).
//   - For the returned `Step5Action::StartInstall`, Run 1 ONLY sets the
//     install-clicked marker (`orchestrator.workspace_step5
//     .install_clicked = true`) that drives P7.T8's `← Previous` lock —
//     it does **not** start the install (no `state.step5
//     .start_install_requested = true`, no install-start hook). The real
//     install-start hook + the concurrency gate are Run 2 (P7.T3 / P7.T9),
//     flagged with the commented placeholder below.
//
// **Borrow discipline.** `page_step5::render` borrows five disjoint
// `orchestrator` fields simultaneously (`wizard_state`,
// `step5_console_view`, `step5_terminal`, `step5_terminal_error`,
// `exe_fingerprint`); split borrows of distinct struct fields are sound,
// so they are passed as direct field accesses. `exe_fingerprint` is cloned
// up front (the same `&self.exe_fingerprint` + `&mut self.…` shape the
// Step-2 router clones to satisfy the borrow checker) so the post-render
// marker write (`&mut orchestrator.workspace_step5`) has no live
// conflicting borrow.
//
// SPEC: §9.1, §9.2, §9.3 (H9 positioning), §13.13.

use eframe::egui;

use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::step5::action_step5::Step5Action;
use crate::ui::workspace::step5::{post_install_actions, success_banner};

/// Render the workspace Step-5 chrome for `_modlist_id` (the routed +
/// loaded modlist — already resolved by `page_router::render_workspace`;
/// the live identity is `orchestrator.workspace_view.modlist_id`). The
/// `modlist_id` argument keeps the plan's `render(ui, orchestrator,
/// modlist_id)` signature stable for the Run-2/Run-3 hooks (install-start,
/// registry transition, post-install actions) even though the Run-1
/// scaffold does not yet need it.
pub fn render(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp, _modlist_id: &str) {
    // Per-modlist guard: if the chrome's install-clicked marker is set but
    // the *workspace* identity has moved on (a modlist swap — the loader
    // resets `WorkspaceStep2State`/`WorkspaceViewState` the same way; see
    // `page_router::render_workspace`), clear the Step-5 chrome state so a
    // stale marker from a previous modlist cannot keep this modlist's
    // `← Previous` locked. (Run 1 has no install, so `install_clicked` is
    // the only persistent Step-5 chrome signal; this keeps it scoped to
    // the modlist that actually clicked Install.)
    if orchestrator.workspace_step5.install_clicked
        && orchestrator.workspace_view.loaded_workspace_id.as_deref()
            != Some(orchestrator.workspace_view.modlist_id.as_str())
    {
        orchestrator.workspace_step5.reset_for_modlist();
    }

    // ── 1. Success-banner row (ABOVE the embedded panel). Empty
    //    pre-install — the C3 clean-exit triple is false until an install
    //    has completed cleanly (Run 3 / P7.T4 fills the body). ──
    success_banner::render(ui, &orchestrator.wizard_state);

    // ── 2. Post-install action row, immediately below the banner row and
    //    above the embedded panel (per H9 — visually adjacent to BIO's
    //    Install button at the top of `page_step5::render`'s panel). Empty
    //    pre-install (Run 3 / P7.T5 fills the buttons). ──
    post_install_actions::render(ui, &orchestrator.wizard_state);

    // ── 3. BIO's entire embedded Step-5 panel — called DIRECTLY (the
    //    verified public top-level renderer; BIO's Step-5 tree is reused
    //    read-only and never edited). `step5_terminal` is `None`
    //    pre-install ⇒ BIO renders the pre-install Command card / Summary
    //    card / console box / prompt input with no live child process —
    //    exactly the Run-1 breakpoint state. The five field borrows are
    //    disjoint (split borrow); `exe_fingerprint` is cloned so the
    //    post-render `&mut orchestrator.workspace_step5` write has no live
    //    conflicting borrow. ──
    let exe_fingerprint = orchestrator.exe_fingerprint.clone();
    let action: Option<Step5Action> = crate::ui::step5::page_step5::render(
        ui,
        &mut orchestrator.wizard_state,
        &mut orchestrator.step5_console_view,
        orchestrator.step5_terminal.as_mut(),
        orchestrator.step5_terminal_error.as_deref(),
        orchestrator.dev_mode,
        &exe_fingerprint,
    );

    // ── 4. Dispatch the returned action. Today BIO's Step-5 panel only
    //    ever returns `Step5Action::StartInstall` (the single-variant enum
    //    — verified `action_step5.rs`). ──
    if let Some(Step5Action::StartInstall) = action {
        // **Run 1 ONLY: set the install-clicked marker.** Per SPEC §9.2
        // the workspace `← Previous` is disabled "once Install has been
        // clicked — even before the install completes". In Run 1 there is
        // no real install, so this marker is the sole "Install was
        // clicked" signal; P7.T8's lock OR-combines it with
        // `WorkspaceViewState::install_complete` and
        // `state.step5.install_running` (the real signals wired Run 2/3).
        orchestrator.workspace_step5.install_clicked = true;

        // P7.T3 (Run 2): install-start hooks + concurrency gate land here.
        // The Run-2 dispatcher will, in order: (a) check the
        // `install_concurrency::install_in_progress` gate (refuse + SPEC
        // §13.15 tooltip if a *different* modlist's install is running);
        // (b) run `install_runtime::start_hooks::on_install_start`
        // (compute the share code via `registry::share_export::pack_meta`
        // with `allow_auto_install = false`, write `modlist-import-code
        // .txt` variant-gated per SPEC §13.13, record `install_started_at`,
        // flip Reinstall registry state); (c) `state.step5
        // .start_install_requested = true` so BIO's
        // `app_update_cycle::start_after_render` (already driven every
        // frame by `OrchestratorApp::start_step5_after_render`, P7.T1)
        // kicks off the install. **Run 1 deliberately does NOT do (a)/(b)/
        // (c)** — it only marks the click for the Previous-lock. Do not
        // implement the install-start hook here.
    }
}
