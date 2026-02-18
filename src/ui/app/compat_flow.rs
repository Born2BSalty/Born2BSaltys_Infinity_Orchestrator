// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

mod map;
mod select;
mod validate;

use crate::compat::CompatValidator;
use crate::ui::state::{CompatState, WizardState};

pub fn run_validation_for_both_games(
    validator: &CompatValidator,
    state: &WizardState,
) -> CompatState {
    validate::run_validation_for_both_games(validator, state)
}

pub fn run_validation_for_step2_checked_order(
    validator: &CompatValidator,
    state: &WizardState,
) -> CompatState {
    validate::run_validation_for_step2_checked_order(validator, state)
}
