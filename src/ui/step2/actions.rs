// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::Step2ModState;

use super::filter::mod_matches_filter;

pub fn clear_all(mods: &mut [Step2ModState], next_selection_order: &mut usize) {
    for mod_state in mods {
        mod_state.checked = false;
        for component in &mut mod_state.components {
            component.checked = false;
            component.selected_order = None;
        }
    }
    *next_selection_order = 1;
}

pub fn select_visible(mods: &mut [Step2ModState], filter: &str, next_selection_order: &mut usize) {
    for mod_state in mods {
        if !mod_matches_filter(mod_state, filter) {
            continue;
        }
        let mod_name_match = filter.is_empty() || mod_state.name.to_lowercase().contains(filter);
        for component in &mut mod_state.components {
            let is_visible =
                mod_name_match || component.label.to_lowercase().contains(filter);
            if !is_visible || component.disabled {
                continue;
            }
            component.checked = true;
            if component.selected_order.is_none() {
                component.selected_order = Some(*next_selection_order);
                *next_selection_order += 1;
            }
        }
        mod_state.checked = mod_state
            .components
            .iter()
            .filter(|c| !c.disabled)
            .all(|c| c.checked);
    }
}
