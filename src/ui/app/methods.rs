// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use super::WizardApp;

impl WizardApp {
    pub(super) fn shutdown_and_exit(&mut self) -> ! {
        super::lifecycle::shutdown_and_exit(self)
    }

    pub(super) fn can_advance_from_current_step(&self) -> bool {
        super::nav::can_advance_from_current_step(self)
    }

    pub(super) fn ensure_step5_terminal(&mut self, ctx: &egui::Context) {
        super::step5_flow::ensure_step5_terminal(self, ctx);
    }

    pub(super) fn auto_save_step4_weidu_logs(&mut self) -> Result<(), String> {
        super::step5_flow::auto_save_step4_weidu_logs(self)
    }

    pub(super) fn sync_step3_from_step2(&mut self) {
        super::step3_sync_flow::sync_step3_from_step2(self);
    }

    pub(super) fn handle_step2_action(&mut self, action: crate::ui::pages::step2::Step2Action) {
        super::step2_router::handle_step2_action(self, action);
    }

    pub(super) fn handle_step4_action(
        &mut self,
        ctx: &egui::Context,
        action: crate::ui::pages::step4::Step4Action,
    ) {
        super::step5_flow::handle_step4_action(self, ctx, action);
    }

    pub(super) fn poll_step2_scan_events(&mut self) {
        super::step2_scan::poll_step2_scan_events(self);
    }

    pub(super) fn save_settings_best_effort(&mut self) {
        super::lifecycle::save_settings_best_effort(self);
    }

    pub(super) fn revalidate_compat(&mut self) {
        super::tp2_metadata::refresh_validator_tp2_metadata(self);
        self.state.compat = super::compat_flow::run_validation_for_both_games(&self.compat_validator, &self.state);
    }

    pub(super) fn revalidate_compat_step2_checked_order(&mut self) {
        super::tp2_metadata::refresh_validator_tp2_metadata(self);
        self.state.compat = super::compat_flow::run_validation_for_step2_checked_order(
            &self.compat_validator,
            &self.state,
        );
        super::step2_compat_overlay::apply_step2_checked_order_issues(self);
    }

    pub(super) fn check_compat_before_install(&mut self) -> bool {
        super::tp2_metadata::refresh_validator_tp2_metadata(self);
        self.state.compat = super::compat_flow::run_validation_for_both_games(&self.compat_validator, &self.state);
        self.state.compat.show_pre_install_modal = false;
        self.state.compat.error_count == 0
    }
}
