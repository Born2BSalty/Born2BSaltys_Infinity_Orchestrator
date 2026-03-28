// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::{Step1State, Step2ModState};

pub(crate) fn apply_compat_rules(
    step1: &Step1State,
    bgee_mods: &mut [Step2ModState],
    bg2ee_mods: &mut [Step2ModState],
) {
    crate::ui::compat_logic::apply_step2_compat_rules(step1, bgee_mods, bg2ee_mods);
}
