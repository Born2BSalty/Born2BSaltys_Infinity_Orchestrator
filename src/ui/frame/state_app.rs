// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::app::WizardApp;

use super::action_app::AppLaunchAction;

pub fn build_app_state(action: AppLaunchAction) -> WizardApp {
    WizardApp::new(action.dev_mode)
}
