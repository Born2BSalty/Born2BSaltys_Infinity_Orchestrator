// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::sync::mpsc::Receiver;

use eframe::egui;

use crate::app::app_step1_github_oauth::{self, GitHubOAuthFlowResult};
use crate::app::state::WizardState;
use crate::ui::step1::action_step1::Step1Action;

pub(crate) fn start_github_flow(
    state: &mut WizardState,
    github_auth_rx: &mut Option<Receiver<GitHubOAuthFlowResult>>,
    force_start: bool,
) {
    if github_auth_rx.is_some() || state.github_auth_running {
        state.github_auth_popup_open = true;
        return;
    }
    if !force_start && !state.github_auth_login.trim().is_empty() {
        state.github_auth_popup_open = true;
        return;
    }

    match app_step1_github_oauth::start_github_oauth_device_flow() {
        Ok((prompt, rx)) => {
            state.github_auth_popup_open = true;
            state.github_auth_running = true;
            state.github_auth_user_code = prompt.user_code;
            state.github_auth_verification_uri = prompt.verification_uri;
            state.github_auth_status_text = prompt.status_text;
            state.step2.scan_status = "Waiting for GitHub authorization...".to_string();
            *github_auth_rx = Some(rx);
        }
        Err(err) => {
            state.github_auth_popup_open = true;
            state.github_auth_running = false;
            state.github_auth_status_text = format!("GitHub authorization could not start: {err}");
            state.step2.scan_status = state.github_auth_status_text.clone();
        }
    }
}

pub(crate) fn disconnect_github(
    state: &mut WizardState,
    github_auth_rx: &mut Option<Receiver<GitHubOAuthFlowResult>>,
) {
    *github_auth_rx = None;
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

pub(crate) fn poll_github_flow(
    state: &mut WizardState,
    github_auth_rx: &mut Option<Receiver<GitHubOAuthFlowResult>>,
) {
    app_step1_github_oauth::poll_github_oauth_flow(state, github_auth_rx);
}

pub fn render_github_popup_if_open(
    ctx: &egui::Context,
    state: &mut WizardState,
) -> Option<Step1Action> {
    let mut action = None;
    crate::ui::step1::github_auth_popup_step1::render(ctx, state, &mut action);
    action
}
