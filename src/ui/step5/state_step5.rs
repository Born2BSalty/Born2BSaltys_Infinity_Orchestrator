// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::WizardState;

pub fn install_in_progress(state: &WizardState) -> bool {
    state.step5.install_running || state.step5.cancel_pending
}
