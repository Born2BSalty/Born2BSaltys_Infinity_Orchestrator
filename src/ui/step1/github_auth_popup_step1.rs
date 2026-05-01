// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::controller::util::open_in_shell;
use crate::app::state::WizardState;
use crate::ui::step1::action_step1::Step1Action;

pub fn render(ctx: &egui::Context, state: &mut WizardState, action: &mut Option<Step1Action>) {
    if !state.github_auth_popup_open {
        return;
    }

    let mut open = state.github_auth_popup_open;
    egui::Window::new("Connect GitHub")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .movable(true)
        .default_size(egui::vec2(420.0, 180.0))
        .show(ctx, |ui| {
            if !state.github_auth_login.trim().is_empty() && !state.github_auth_running {
                ui.label(format!("Connected as {}", state.github_auth_login.trim()));
                ui.add_space(8.0);
            }
            if !state.github_auth_status_text.trim().is_empty() {
                ui.label(&state.github_auth_status_text);
                ui.add_space(8.0);
            }
            if !state.github_auth_user_code.trim().is_empty() {
                ui.label("Enter this code on GitHub:");
                ui.monospace(&state.github_auth_user_code);
                ui.add_space(8.0);
            }
            if !state.github_auth_verification_uri.trim().is_empty() {
                ui.label("Open this URL:");
                ui.monospace(&state.github_auth_verification_uri);
                ui.add_space(8.0);
            }
            if state.github_auth_running {
                ui.label("Waiting for GitHub authorization...");
            }
            if !state.github_auth_login.trim().is_empty() && !state.github_auth_running {
                ui.add_space(8.0);
                ui.label(
                    "Revoke access in GitHub Settings > Applications > Authorized OAuth Apps.",
                );
            }
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if !state.github_auth_verification_uri.trim().is_empty()
                    && ui.button("Open GitHub").clicked()
                    && let Err(err) = open_in_shell(&state.github_auth_verification_uri)
                {
                    state.github_auth_status_text = format!("Open GitHub failed: {err}");
                }
                if !state.github_auth_login.trim().is_empty()
                    && !state.github_auth_running
                    && ui.button("Reconnect").clicked()
                {
                    *action = Some(Step1Action::ReconnectGitHub);
                }
                if !state.github_auth_login.trim().is_empty()
                    && !state.github_auth_running
                    && ui.button("Disconnect").clicked()
                {
                    *action = Some(Step1Action::DisconnectGitHub);
                }
                if ui.button("Close").clicked() {
                    state.github_auth_popup_open = false;
                }
            });
        });
    state.github_auth_popup_open = open && state.github_auth_popup_open;
}
