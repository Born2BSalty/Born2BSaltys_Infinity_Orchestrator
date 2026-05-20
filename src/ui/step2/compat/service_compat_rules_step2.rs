// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::state::{Step1State, Step2ModState};

pub(crate) fn apply_compat_rules(
    step1: &Step1State,
    first_game_mods: &mut [Step2ModState],
    second_game_mods: &mut [Step2ModState],
) -> Option<String> {
    crate::app::compat_logic::apply_step2_compat_rules(step1, first_game_mods, second_game_mods)
}
