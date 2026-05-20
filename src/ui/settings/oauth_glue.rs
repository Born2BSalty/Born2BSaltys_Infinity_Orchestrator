// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::app_step1_github_oauth as oauth;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::step1::action_step1::Step1Action;
use crate::ui::step1::github_auth_popup_step1;

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
            state.github_auth_status_text = format!("GitHub authorization could not start: {err}");
            state.step2.scan_status = state.github_auth_status_text.clone();
        }
    }
}

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

pub fn poll_github_oauth_flow(orchestrator: &mut OrchestratorApp) {
    oauth::poll_github_oauth_flow(
        &mut orchestrator.wizard_state,
        &mut orchestrator.github_auth_rx,
    );
}

pub fn render_github_popup_if_open(orchestrator: &mut OrchestratorApp, ctx: &egui::Context) {
    if !orchestrator.wizard_state.github_auth_popup_open {
        return;
    }
    let mut action: Option<Step1Action> = None;
    github_auth_popup_step1::render(ctx, &mut orchestrator.wizard_state, &mut action);

    if let Some(act) = action {
        match act {
            Step1Action::ConnectGitHub | Step1Action::ReconnectGitHub => {
                start_github_flow(orchestrator, matches!(act, Step1Action::ReconnectGitHub));
            }
            Step1Action::DisconnectGitHub => {
                disconnect_github(orchestrator);
            }
            Step1Action::PathsChanged => {}
        }
    }
}
