// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::{Step2ModState, WizardState};

pub(super) fn active_mods_ref(state: &WizardState) -> &Vec<Step2ModState> {
    if state.step2.active_game_tab == "BGEE" {
        &state.step2.bgee_mods
    } else {
        &state.step2.bg2ee_mods
    }
}
