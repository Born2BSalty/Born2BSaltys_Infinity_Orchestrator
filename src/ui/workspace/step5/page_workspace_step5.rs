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
// **Run-1 chrome scaffold (still in force).** The banner / post-install
// rows render nothing until the C3 clean-exit triple holds (Run 3 fills
// their bodies); the embedded panel is BIO's pre-install view until the
// terminal exists.
//
// **Run 2 scope (install-start + concurrency gate).** On the returned
// `Step5Action::StartInstall` the dispatcher now, in order:
//   (a) **P7.T9 concurrency gate** — `install_concurrency
//       ::install_in_progress`; if a *different* modlist's install is
//       running, refuse (SPEC §13.15 verbatim tooltip) and bail (no start
//       hook, no `start_install_requested`).
//   (b) **P7.T3 install-start hook** — `start_hooks::on_install_start`:
//       apply the #1/#5 flag policies (P7.T16) into the orchestrator-owned
//       `WizardState.step1`, compute the share code via `registry
//       ::share_export::pack_meta` (`allow_auto_install = false` — SPEC
//       §13.3 / §13.13), update `entry.latest_share_code`, write
//       `modlist-import-code.txt` variant-gated per SPEC §13.13, record
//       `install_started_at`, atomic registry write.
//   (c) flip `state.step5.start_install_requested = true` (only on the
//       hook's `Ok`) so BIO's `app_update_cycle::start_after_render`
//       (driven every frame by `OrchestratorApp::start_step5_after_render`,
//       P7.T1) kicks off the install — a fresh Create → New reaches here
//       and BIO's existing pipeline starts.
//   The install-clicked marker (P7.T8 `← Previous` lock) is still set on
//   the click. The Reinstall registry-flip (P7.T10) + the share-code-
//   consuming download pipeline (P7.T17) are out of Run-2 scope — left as
//   commented placeholders in `start_hooks`.
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
use tracing::warn;

use crate::install_runtime::flag_policies::InstallWorkflow;
use crate::install_runtime::install_concurrency;
use crate::install_runtime::start_hooks::{self, InstallButtonVariant};
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
pub fn render(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp, modlist_id: &str) {
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
        // **Per SPEC §9.2: mark the click for the P7.T8 `← Previous`
        // lock.** Disabled "once Install has been clicked — even before
        // the install completes". P7.T8's lock OR-combines this with
        // `WorkspaceViewState::install_complete` and
        // `state.step5.install_running`. Set unconditionally on the click
        // (the workspace stays the running install's workspace; the C5
        // rail lock + this marker keep `← Previous` disabled).
        orchestrator.workspace_step5.install_clicked = true;

        // ── (a) P7.T9 — install-concurrency gate (SPEC §13.15: only one
        //    install at a time). If an install is already running for a
        //    *different* modlist, refuse: do NOT run the start hooks, do
        //    NOT flip `start_install_requested`. (The C5 rail lock + the
        //    workspace-swap refusal normally make this unreachable from a
        //    second workspace; this is the documented defensive case —
        //    P7.T9 acceptance — for the same-frame race where this Install
        //    button is somehow clickable while another install runs.) ──
        if let Some(running) = install_concurrency::install_in_progress(orchestrator)
            && running.modlist_id != modlist_id
        {
            // SPEC §13.15 verbatim per-button tooltip; surfaced as a warn
            // here (the disabled-button-with-tooltip surface is the
            // primary UX — this is the belt-and-braces refusal so a race
            // window cannot start a second concurrent install).
            let running_name = orchestrator
                .registry
                .find(&running.modlist_id)
                .map_or_else(|| running.modlist_id.clone(), |e| e.name.clone());
            warn!(
                target = "orchestrator",
                "Install refused for {modlist_id}: {}",
                install_concurrency::per_button_gate_tooltip(&running_name)
            );
            return;
        }

        // ── (b) P7.T3 — install-start hook. Variant from BIO's live
        //    `state.step5` (the same logic BIO's install-row uses for the
        //    button label) + the orchestrator reinstall flag. P7.T10's
        //    `pending_reinstall_id` is Run 4b (commented placeholder in
        //    `start_hooks`), so `reinstall = false` this run. ──
        let variant = InstallButtonVariant::from_step5(&orchestrator.wizard_state, false);

        // Workflow for the workspace Step-5 path (SPEC §13.12 #5): a
        // modlist with a non-empty `forked_from` lineage was created via
        // Create → Import-and-modify (a share-code-consuming workflow ⇒
        // `--download` ON); an empty lineage is a fresh Create → New ⇒
        // `--download` follows Settings → Advanced. (Continue Partial
        // Install is the Install-Modlist destination-not-empty path, not
        // the workspace Step-5 path, so it never applies here.) The
        // share-code-consuming *download pipeline* itself is Run 4
        // (P7.T17); Run 2 only sets the correct flag — harmless for a
        // fresh Create → New (nothing to download yet).
        let workflow = orchestrator
            .registry
            .find(modlist_id)
            .filter(|e| !e.forked_from.is_empty())
            .map_or(InstallWorkflow::FreshCreate, |_| {
                InstallWorkflow::ShareCodeConsuming
            });

        // `Step1Settings` snapshot for the #5 fresh-create `--download`
        // fallback (the orchestrator-owned `wizard_state.step1` already
        // carries the Settings → Advanced values via the open-time
        // `sync_paths_from_settings`; the `From<Step1State>` projection is
        // the settings-model shape `compute_flags` expects).
        let settings: crate::settings::model::Step1Settings =
            orchestrator.wizard_state.step1.clone().into();

        // Split the `&mut orchestrator` borrow into the disjoint fields
        // `on_install_start` needs (`wizard_state` / `registry` are
        // distinct struct fields; `registry_store` is a third — a sound
        // split borrow, the same shape `page_step5::render`'s five-field
        // call above relies on).
        let OrchestratorApp {
            wizard_state,
            registry,
            registry_store,
            ..
        } = &mut *orchestrator;

        match start_hooks::on_install_start(
            modlist_id,
            variant,
            workflow,
            wizard_state,
            registry,
            registry_store,
            &settings,
        ) {
            Ok(()) => {
                // ── (c) Flip `state.step5.start_install_requested = true`.
                //    BIO's `app_update_cycle::start_after_render` (driven
                //    every frame by `OrchestratorApp
                //    ::start_step5_after_render`, P7.T1) picks this up on
                //    the next poll and kicks off the install — identical
                //    to the legacy wizard. A fresh Create → New install
                //    reaches here, so BIO's existing pipeline starts. ──
                orchestrator.wizard_state.step5.start_install_requested = true;
            }
            Err(err) => {
                // SPEC §13.14: the install-start hook failed (share-code
                // generation or the atomic registry write). Do NOT flip
                // `start_install_requested` — surfacing the failure to the
                // user is the success-path run's job (Run 3); Run 2 logs
                // it and aborts the start (the install simply does not
                // begin, leaving the workspace usable).
                warn!(
                    target = "orchestrator",
                    "install-start hook failed for {modlist_id}: {err} \
                     (install not started)"
                );
            }
        }
    }
}
