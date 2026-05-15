// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::state::{Step1State, Step2ModState};

pub(crate) fn apply_compat_rules(
    step1: &Step1State,
    primary_game_mods: &mut [Step2ModState],
    secondary_game_mods: &mut [Step2ModState],
) -> Option<String> {
    crate::app::compat_logic::apply_step2_compat_rules(
        step1,
        primary_game_mods,
        secondary_game_mods,
    )
}
