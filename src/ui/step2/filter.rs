// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::{Step2ModState, Step2State};

pub fn active_mods_mut(step2: &mut Step2State) -> &mut Vec<Step2ModState> {
    if step2.active_game_tab == "BGEE" {
        &mut step2.bgee_mods
    } else {
        &mut step2.bg2ee_mods
    }
}

pub fn mod_matches_filter(mod_state: &Step2ModState, filter: &str) -> bool {
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
