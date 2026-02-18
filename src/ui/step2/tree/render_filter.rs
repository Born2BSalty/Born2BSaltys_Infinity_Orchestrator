// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::Step2ModState;

pub(super) fn mod_matches_filter(mod_state: &Step2ModState, filter: &str) -> bool {
    if filter.is_empty() {
        return true;
    }
    if mod_state.name.to_lowercase().contains(filter) {
        return true;
    }
    mod_state
        .components
        .iter()
        .any(|component| component.label.to_lowercase().contains(filter))
}

pub(super) fn finalize_mod_checked_state(mod_state: &mut Step2ModState) {
    let has_components = !mod_state.components.is_empty();
    mod_state.checked = has_components
        && mod_state.components.iter().filter(|c| !c.disabled).count() > 0
        && mod_state
            .components
            .iter()
            .filter(|component| !component.disabled)
            .all(|component| component.checked);
}
