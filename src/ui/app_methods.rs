// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use super::WizardApp;

impl WizardApp {
    pub(super) fn handle_step1_action(
        &mut self,
        action: crate::ui::step1::action_step1::Step1Action,
    ) {
        match action {
            crate::ui::step1::action_step1::Step1Action::ConnectGitHub
            | crate::ui::step1::action_step1::Step1Action::ReconnectGitHub => {
                let force_start = matches!(
                    action,
                    crate::ui::step1::action_step1::Step1Action::ReconnectGitHub
                );
                if self.step1_github_auth_rx.is_some() || self.state.github_auth_running {
                    self.state.github_auth_popup_open = true;
                    return;
                }
                if !force_start && !self.state.github_auth_login.trim().is_empty() {
                    self.state.github_auth_popup_open = true;
                    return;
                }
                match crate::app::app_step1_github_oauth::start_github_oauth_device_flow() {
                    Ok((prompt, rx)) => {
                        self.state.github_auth_popup_open = true;
                        self.state.github_auth_running = true;
                        self.state.github_auth_user_code = prompt.user_code;
                        self.state.github_auth_verification_uri = prompt.verification_uri;
                        self.state.github_auth_status_text = prompt.status_text;
                        self.state.step2.scan_status =
                            "Waiting for GitHub authorization...".to_string();
                        self.step1_github_auth_rx = Some(rx);
                    }
                    Err(err) => {
                        self.state.github_auth_popup_open = true;
                        self.state.github_auth_running = false;
                        self.state.github_auth_status_text =
                            format!("GitHub authorization could not start: {err}");
                        self.state.step2.scan_status =
                            self.state.github_auth_status_text.clone();
                    }
                }
            }
            crate::ui::step1::action_step1::Step1Action::DisconnectGitHub => {
                self.step1_github_auth_rx = None;
                self.state.github_auth_running = false;
                self.state.github_auth_login.clear();
                self.state.github_auth_user_code.clear();
                self.state.github_auth_verification_uri.clear();
                self.state.github_auth_popup_open = true;
                match crate::app::app_step2_update_github_auth::clear_github_oauth_token() {
                    Ok(()) => {
                        self.state.github_auth_status_text = "GitHub disconnected.".to_string();
                        self.state.step2.scan_status = self.state.github_auth_status_text.clone();
                    }
                    Err(err) => {
                        self.state.github_auth_status_text =
                            format!("GitHub disconnect failed: {err}");
                        self.state.step2.scan_status = self.state.github_auth_status_text.clone();
                    }
                }
            }
            crate::ui::step1::action_step1::Step1Action::PathsChanged => {}
        }
    }

    pub(super) fn shutdown_and_exit(&mut self) -> ! {
        super::lifecycle::shutdown_and_exit(self)
    }

    pub(super) fn handle_step2_action(
        &mut self,
        action: crate::ui::step2::action_step2::Step2Action,
    ) {
        super::step2_router::handle_step2_action(self, action);
    }

    pub(super) fn handle_step4_action(
        &mut self,
        _ctx: &egui::Context,
        action: crate::ui::step4::action_step4::Step4Action,
    ) {
        super::step4_flow::handle_step4_action(&mut self.state, action);
    }

    pub(super) fn save_settings_best_effort(&mut self) {
        super::lifecycle::save_settings_best_effort(self);
    }
}
