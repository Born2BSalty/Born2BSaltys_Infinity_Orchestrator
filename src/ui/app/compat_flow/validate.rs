// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::compat::CompatValidator;
use crate::ui::state::{CompatState, WizardState};

use super::map;
use super::select;

pub(super) fn run_validation(validator: &CompatValidator, state: &WizardState) -> CompatState {
    let game = state.step1.game_install.as_str();
    let items = match game {
        "BG2EE" => &state.step3.bg2ee_items,
        _ => &state.step3.bgee_items,
    };

    let selected = select::build_selected_components(items);
    let result = validator.validate(&selected, game);
    map::to_state(result)
}

pub(super) fn run_validation_for_both_games(
    validator: &CompatValidator,
    state: &WizardState,
) -> CompatState {
    let game = state.step1.game_install.as_str();

    match game {
        "EET" => {
            let bgee_selected = select::build_selected_components(&state.step3.bgee_items);
            let bg2ee_selected = select::build_selected_components(&state.step3.bg2ee_items);

            // In EET installs both streams execute in EET runtime context.
            let bgee_result = validator.validate(&bgee_selected, "EET");
            let bg2ee_result = validator.validate(&bg2ee_selected, "EET");

            let left = map::to_state(bgee_result);
            let right = map::to_state(bg2ee_result);
            map::combine_states(left, right)
        }
        _ => run_validation(validator, state),
    }
}

pub(super) fn run_validation_for_step2_checked_order(
    validator: &CompatValidator,
    state: &WizardState,
) -> CompatState {
    let game = state.step1.game_install.as_str();

    match game {
        "EET" => {
            let bgee_selected = select::build_selected_components_from_step2(&state.step2.bgee_mods);
            let bg2ee_selected = select::build_selected_components_from_step2(&state.step2.bg2ee_mods);

            // In EET installs both tabs should validate against EET, not standalone BGEE/BG2EE.
            let bgee_result = validator.validate(&bgee_selected, "EET");
            let bg2ee_result = validator.validate(&bg2ee_selected, "EET");

            let left = map::to_state(bgee_result);
            let right = map::to_state(bg2ee_result);
            map::combine_states(left, right)
        }
        "BGEE" => {
            let selected = select::build_selected_components_from_step2(&state.step2.bgee_mods);
            let result = validator.validate(&selected, "BGEE");
            map::to_state(result)
        }
        _ => {
            let selected = select::build_selected_components_from_step2(&state.step2.bg2ee_mods);
            let result = validator.validate(&selected, "BG2EE");
            map::to_state(result)
        }
    }
}
