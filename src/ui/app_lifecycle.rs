// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use super::WizardApp;

pub(super) fn shutdown_and_exit(app: &mut WizardApp) -> ! {
    crate::app::app_lifecycle::shutdown_and_exit(
        &app.state,
        &mut app.step5_terminal,
        &app.settings_store,
        &mut app.last_saved_step1,
        app.dev_mode,
        app.exe_fingerprint.as_str(),
    )
}

pub(super) fn save_settings_best_effort(app: &mut WizardApp) {
    crate::app::app_lifecycle::save_settings_best_effort(
        &app.state,
        &app.settings_store,
        &mut app.last_saved_step1,
        app.dev_mode,
        app.exe_fingerprint.as_str(),
    );
}
