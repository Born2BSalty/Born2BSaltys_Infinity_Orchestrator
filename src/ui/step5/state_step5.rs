// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::state::WizardState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step5ConsoleViewState {
    pub important_only: bool,
    pub installed_only: bool,
    pub auto_scroll: bool,
    pub request_input_focus: bool,
}

impl Default for Step5ConsoleViewState {
    fn default() -> Self {
        Self {
            important_only: false,
            installed_only: false,
            auto_scroll: true,
            request_input_focus: false,
        }
    }
}

pub fn install_in_progress(state: &WizardState) -> bool {
    state.step5.prep_running || state.step5.install_running || state.step5.cancel_pending
}
