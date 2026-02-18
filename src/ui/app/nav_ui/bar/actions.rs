// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::app::WizardApp;

use super::super::logic;

pub(super) fn handle_reset(app: &mut WizardApp) {
    super::super::super::step2_scan::cancel_step2_scan(app);
    app.step2_scan_rx = None;
    app.step2_cancel = None;
    app.step2_progress_queue.clear();
    if let Some(term) = app.step5_terminal.as_mut() {
        term.shutdown();
    }
    app.state.reset_workflow_keep_step1();
    app.last_step2_sync_signature = None;
    app.save_settings_best_effort();
}

pub(super) fn handle_back(app: &mut WizardApp) {
    if app.state.step1.have_weidu_logs && (app.state.current_step == 3 || app.state.current_step == 4) {
        app.state.current_step = 0;
    } else {
        app.state.go_back();
    }
    app.save_settings_best_effort();
}

pub(super) fn handle_next(app: &mut WizardApp) {
    if logic::should_show_step1_clean_confirm(app) {
        app.state.step1_clean_confirm_open = true;
    } else {
        logic::advance_after_next(app);
    }
}
