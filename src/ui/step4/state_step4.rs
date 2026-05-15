// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::state::WizardState;

pub const fn active_tab_mut(state: &mut WizardState) -> &mut String {
    &mut state.step3.active_game_tab
}
