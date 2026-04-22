// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::state::WizardState;

pub fn clear_path_check_if_step1_changed(state: &mut WizardState, changed: bool) {
    if changed {
        state.step1_path_check = None;
        state.step1_mods_folder_has_tp2 = None;
    }
}
