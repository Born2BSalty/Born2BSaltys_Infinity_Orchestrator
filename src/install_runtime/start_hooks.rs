// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::Path;

use crate::app::state::WizardState;

use super::{flag_policies_eet, flag_policies_log, flag_policies_single_game};

pub fn on_install_start(state: &mut WizardState, destination_folder: &Path) -> Result<(), String> {
    flag_policies_log::apply(state, destination_folder)?;
    if state.step1.game_install == "EET" {
        flag_policies_eet::apply(state, destination_folder)
    } else {
        flag_policies_single_game::apply(state, destination_folder)
    }
}
