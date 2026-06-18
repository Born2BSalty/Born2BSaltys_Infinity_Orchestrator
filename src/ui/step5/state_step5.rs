// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::state::WizardState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step5ConsoleViewState {
    pub filter: ConsoleOutputFilter,
    pub auto_scroll: bool,
    pub request_input_focus: bool,
    pub last_selected_console_text_len: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsoleOutputFilter {
    General,
    Important,
    Installed,
}

impl Default for Step5ConsoleViewState {
    fn default() -> Self {
        Self {
            filter: ConsoleOutputFilter::General,
            auto_scroll: true,
            request_input_focus: false,
            last_selected_console_text_len: 0,
        }
    }
}

#[must_use]
pub const fn install_in_progress(state: &WizardState) -> bool {
    state.step5.prep_running || state.step5.install_running || state.step5.cancel_pending
}
