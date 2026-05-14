// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `oauth_glue` — glue functions that drive the GitHub OAuth device flow
// from the orchestrator side.
//
// Per Phase 4 P4.T12 + the C2 audit:
//   - `start_github_flow(orchestrator)` — kicks off the device flow by
//     calling `bio::app::app_step1_github_oauth::start_github_oauth_device_flow`
//     directly (`pub(crate)`, reachable same-crate per carve-out #3).
//   - `render_github_popup_if_open(orchestrator, ctx)` — invokes BIO's
//     existing popup renderer at `bio::ui::step1::github_auth_popup_step1::render`
//     (`pub fn` per line 10 of that file). Any `Step1Action` returned is
//     dispatched in-place via the same `bio::app::*` public helpers that
//     BIO's `WizardApp::handle_step1_action` uses — **without** the
//     WizardApp wrapper (per the C2 audit: `handle_step1_action` mutates
//     `WizardApp.step1_github_auth_rx`, so the orchestrator owns its own
//     receiver field on `OrchestratorApp`).
//   - `poll_github_oauth_flow(orchestrator)` — per-frame poll of the
//     in-flight receiver, calling `bio::app::app_step1_github_oauth::poll_github_oauth_flow`
//     directly.
//
// **No BIO source modifications.** Everything is reached via `bio::app::*`
// and `bio::ui::step1::*` public surface (visibility verified at top of file).
//
// SPEC: §11.4, §13.2.

use eframe::egui;
use tracing::warn;

use crate::app::app_step1_github_oauth as oauth;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::step1::action_step1::Step1Action;
use crate::ui::step1::github_auth_popup_step1;

/// Kick off the GitHub OAuth device flow. Called from the Accounts tab's
/// `connect` button. Mirrors the body of `WizardApp::handle_step1_action`'s
/// `ConnectGitHub` / `ReconnectGitHub` arms.
pub fn start_github_flow(orchestrator: &mut OrchestratorApp, force_start: bool) {
    let state = &mut orchestrator.wizard_state;
    if orchestrator.github_auth_rx.is_some() || state.github_auth_running {
        state.github_auth_popup_open = true;
        return;
    }
    if !force_start && !state.github_auth_login.trim().is_empty() {
        state.github_auth_popup_open = true;
        return;
    }
    match oauth::start_github_oauth_device_flow() {
        Ok((prompt, rx)) => {
            state.github_auth_popup_open = true;
            state.github_auth_running = true;
            state.github_auth_user_code = prompt.user_code;
            state.github_auth_verification_uri = prompt.verification_uri;
            state.github_auth_status_text = prompt.status_text;
            state.step2.scan_status = "Waiting for GitHub authorization...".to_string();
            orchestrator.github_auth_rx = Some(rx);
        }
        Err(err) => {
            state.github_auth_popup_open = true;
            state.github_auth_running = false;
            state.github_auth_status_text =
                format!("GitHub authorization could not start: {err}");
            state.step2.scan_status = state.github_auth_status_text.clone();
        }
    }
}

/// Disconnect helper — mirrors the body of `WizardApp::handle_step1_action`'s
/// `DisconnectGitHub` arm.
pub fn disconnect_github(orchestrator: &mut OrchestratorApp) {
    let state = &mut orchestrator.wizard_state;
    orchestrator.github_auth_rx = None;
    state.github_auth_running = false;
    state.github_auth_login.clear();
    state.github_auth_user_code.clear();
    state.github_auth_verification_uri.clear();
    state.github_auth_popup_open = true;
    match crate::app::app_step2_update_github_auth::clear_github_oauth_token() {
        Ok(()) => {
            state.github_auth_status_text = "GitHub disconnected.".to_string();
            state.step2.scan_status = state.github_auth_status_text.clone();
        }
        Err(err) => {
            state.github_auth_status_text = format!("GitHub disconnect failed: {err}");
            state.step2.scan_status = state.github_auth_status_text.clone();
        }
    }
}

/// Per-frame poll of the in-flight receiver. Called from
/// `OrchestratorApp::update`. Delegates to BIO's `poll_github_oauth_flow`
/// which mutates `WizardState` directly (it's a `pub(crate) fn`).
pub fn poll_github_oauth_flow(orchestrator: &mut OrchestratorApp) {
    oauth::poll_github_oauth_flow(&mut orchestrator.wizard_state, &mut orchestrator.github_auth_rx);
}

/// Render the popup if `wizard_state.github_auth_popup_open` is set. Called
/// from `OrchestratorApp::update` once per frame so the popup overlays
/// whichever destination is active. Any returned `Step1Action` is dispatched
/// inline.
pub fn render_github_popup_if_open(orchestrator: &mut OrchestratorApp, ctx: &egui::Context) {
    if !orchestrator.wizard_state.github_auth_popup_open {
        return;
    }
    let mut action: Option<Step1Action> = None;
    github_auth_popup_step1::render(ctx, &mut orchestrator.wizard_state, &mut action);

    if let Some(act) = action {
        match act {
            Step1Action::ConnectGitHub | Step1Action::ReconnectGitHub => {
                start_github_flow(
                    orchestrator,
                    matches!(act, Step1Action::ReconnectGitHub),
                );
            }
            Step1Action::DisconnectGitHub => {
                disconnect_github(orchestrator);
            }
            Step1Action::PathsChanged => {
                // No-op for the orchestrator — paths-changed is handled by
                // the per-edit debounce in `validate_debounce::tick`.
            }
        }
    }
}

/// Load the stored GitHub token (if any) and populate `state.github_auth_login`
/// with the resolved user name. Called once at orchestrator startup from
/// `OrchestratorApp::new` (Phase 4 wires this).
pub fn load_persisted_login(orchestrator: &mut OrchestratorApp) {
    match oauth::load_github_login_from_stored_token() {
        Ok(Some(login)) => {
            orchestrator.wizard_state.github_auth_login = login;
        }
        Ok(None) => {}
        Err(err) => {
            warn!(target = "orchestrator", "github login load failed: {err}");
        }
    }
}
