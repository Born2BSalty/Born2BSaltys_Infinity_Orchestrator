// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::WizardState;

pub fn recompute_selection_counts(state: &mut WizardState) {
    let mods = if state.step2.active_game_tab == "BGEE" {
        &state.step2.bgee_mods
    } else {
        &state.step2.bg2ee_mods
    };
    let mut total = 0usize;
    let mut selected = 0usize;
    for mod_state in mods {
        for component in &mod_state.components {
            total += 1;
            if component.checked {
                selected += 1;
            }
        }
    }
    state.step2.total_count = total;
    state.step2.selected_count = selected;
}
