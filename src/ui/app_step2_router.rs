// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use super::WizardApp;

pub(super) fn handle_step2_action(
    app: &mut WizardApp,
    action: crate::ui::step2::action_step2::Step2Action,
) {
    match action {
        crate::ui::step2::action_step2::Step2Action::SelectBgeeViaLog => {
            super::step2_log::apply_weidu_log_selection(app, true);
        }
        crate::ui::step2::action_step2::Step2Action::SelectBg2eeViaLog => {
            super::step2_log::apply_weidu_log_selection(app, false);
        }
        action => crate::app::app_step2_router::handle_step2_action(
            &mut app.state,
            &mut app.step2_scan_rx,
            &mut app.step2_cancel,
            &mut app.step2_progress_queue,
            &mut app.step2_update_check_rx,
            &mut app.step2_update_download_rx,
            action,
        ),
    }
}
