// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use super::WizardApp;

mod actions {
    use crate::ui::app::WizardApp;

    pub(super) fn handle_reset(app: &mut WizardApp) {
        crate::app::app_nav_actions::handle_reset(
            &mut app.state,
            &mut app.step2_scan_rx,
            &mut app.step2_cancel,
            &mut app.step2_progress_queue,
            &mut app.step5_terminal,
            &app.settings_store,
            &mut app.last_saved_step1,
            app.dev_mode,
            app.exe_fingerprint.as_str(),
        );
        app.step5_console_view = crate::ui::step5::state_step5::Step5ConsoleViewState::default();
    }

    pub(super) fn handle_back(app: &mut WizardApp) {
        crate::app::app_nav_actions::handle_back(
            &mut app.state,
            &app.settings_store,
            &mut app.last_saved_step1,
            app.dev_mode,
            app.exe_fingerprint.as_str(),
        );
    }

    pub(super) fn handle_next(app: &mut WizardApp) {
        crate::app::app_nav_actions::handle_next(
            &mut app.state,
            &app.settings_store,
            &mut app.last_saved_step1,
            app.dev_mode,
            app.exe_fingerprint.as_str(),
        );
    }

    pub(super) fn handle_exit(app: &mut WizardApp) -> ! {
        app.shutdown_and_exit();
    }

    pub(super) fn handle_test_paths(app: &mut WizardApp) {
        app.state.run_step1_path_check();
    }

    pub(super) fn handle_clean_confirm_yes(app: &mut WizardApp) {
        crate::app::app_nav_actions::handle_clean_confirm_yes(
            &mut app.state,
            &app.settings_store,
            &mut app.last_saved_step1,
            app.dev_mode,
            app.exe_fingerprint.as_str(),
        );
    }

    pub(super) fn handle_clean_confirm_no(app: &mut WizardApp) {
        crate::app::app_nav_actions::handle_clean_confirm_no(&mut app.state);
    }

    pub(super) fn dismiss_step4_save_error(app: &mut WizardApp) {
        crate::app::app_nav_actions::dismiss_step4_save_error(&mut app.state);
    }
}

pub(crate) fn current_step(app: &WizardApp) -> usize {
    super::nav::current_step(&app.state)
}

pub(crate) fn can_advance(app: &WizardApp) -> bool {
    super::nav::can_advance_from_current_step(&app.state)
}

pub(crate) fn can_go_back(app: &WizardApp) -> bool {
    super::nav::can_go_back(&app.state)
}

pub(crate) fn on_last_step(app: &WizardApp) -> bool {
    super::nav::on_last_step(&app.state)
}

pub(crate) fn step5_install_running(app: &WizardApp) -> bool {
    super::nav::step5_install_running(&app.state)
}

pub(crate) fn step1_clean_confirm_open(app: &WizardApp) -> bool {
    super::nav::step1_clean_confirm_open(&app.state)
}

pub(crate) fn step4_save_error_open(app: &WizardApp) -> bool {
    super::nav::step4_save_error_open(&app.state)
}

pub(crate) fn step4_save_error_text(app: &WizardApp) -> &str {
    super::nav::step4_save_error_text(&app.state)
}

pub(crate) fn handle_reset(app: &mut WizardApp) {
    actions::handle_reset(app);
}

pub(crate) fn handle_back(app: &mut WizardApp) {
    actions::handle_back(app);
}

pub(crate) fn handle_next(app: &mut WizardApp) {
    actions::handle_next(app);
}

pub(crate) fn handle_exit(app: &mut WizardApp) -> ! {
    actions::handle_exit(app)
}

pub(crate) fn handle_test_paths(app: &mut WizardApp) {
    actions::handle_test_paths(app);
}

pub(crate) fn handle_clean_confirm_yes(app: &mut WizardApp) {
    actions::handle_clean_confirm_yes(app);
}

pub(crate) fn handle_clean_confirm_no(app: &mut WizardApp) {
    actions::handle_clean_confirm_no(app);
}

pub(crate) fn dismiss_step4_save_error(app: &mut WizardApp) {
    actions::dismiss_step4_save_error(app);
}
