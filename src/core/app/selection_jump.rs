// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::controller::log_apply_match::parse_component_tp2_from_raw;
use crate::app::selection_refs::normalize_mod_key;
use crate::app::state::{Step2Selection, Step3ItemState, WizardState};

pub(crate) fn step2_jump_to_target(
    state: &mut WizardState,
    game_tab: &str,
    mod_ref: &str,
    component_ref: Option<u32>,
) {
    let target_key = normalize_mod_key(mod_ref);
    let mods = if game_tab.eq_ignore_ascii_case("BGEE") {
        &state.step2.bgee_mods
    } else {
        &state.step2.bg2ee_mods
    };
    for mod_state in mods {
        if let Some(target_component) = component_ref {
            if let Some(component) = mod_state.components.iter().find(|component| {
                let component_key = parse_component_tp2_from_raw(&component.raw_line).map_or_else(
                    || normalize_mod_key(&mod_state.tp_file),
                    |tp2| normalize_mod_key(&tp2),
                );
                component.component_id.trim().parse::<u32>().ok() == Some(target_component)
                    && component_key == target_key
            }) {
                state.step2.selected = Some(Step2Selection::Component {
                    game_tab: game_tab.to_string(),
                    tp_file: mod_state.tp_file.clone(),
                    component_id: component.component_id.clone(),
                    component_key: component.raw_line.clone(),
                });
                return;
            }
        } else if let Some(component) = mod_state.components.iter().find(|component| {
            parse_component_tp2_from_raw(&component.raw_line).map_or_else(
                || normalize_mod_key(&mod_state.tp_file),
                |tp2| normalize_mod_key(&tp2),
            ) == target_key
        }) {
            state.step2.selected = Some(Step2Selection::Component {
                game_tab: game_tab.to_string(),
                tp_file: mod_state.tp_file.clone(),
                component_id: component.component_id.clone(),
                component_key: component.raw_line.clone(),
            });
            return;
        }
        if normalize_mod_key(&mod_state.tp_file) != target_key {
            continue;
        }
        state.step2.selected = Some(Step2Selection::Mod {
            game_tab: game_tab.to_string(),
            tp_file: mod_state.tp_file.clone(),
        });
        return;
    }
}

pub(crate) fn step3_jump_to_target(
    state: &mut WizardState,
    game_tab: &str,
    mod_ref: &str,
    component_ref: Option<u32>,
) -> bool {
    let target_key = normalize_mod_key(mod_ref);
    if game_tab.eq_ignore_ascii_case("BGEE") {
        let mut target = Step3JumpTarget {
            active_game_tab: &mut state.step3.active_game_tab,
            jump_to_selected_requested: &mut state.step3.jump_to_selected_requested,
            items: &mut state.step3.bgee_items,
            selected: &mut state.step3.bgee_selected,
            anchor: &mut state.step3.bgee_anchor,
            collapsed_blocks: &mut state.step3.bgee_collapsed_blocks,
        };
        jump_to_target_in_tab(&mut target, game_tab, &target_key, component_ref)
    } else {
        let mut target = Step3JumpTarget {
            active_game_tab: &mut state.step3.active_game_tab,
            jump_to_selected_requested: &mut state.step3.jump_to_selected_requested,
            items: &mut state.step3.bg2ee_items,
            selected: &mut state.step3.bg2ee_selected,
            anchor: &mut state.step3.bg2ee_anchor,
            collapsed_blocks: &mut state.step3.bg2ee_collapsed_blocks,
        };
        jump_to_target_in_tab(&mut target, game_tab, &target_key, component_ref)
    }
}

pub(crate) fn selected_step2_jump_target(
    state: &WizardState,
) -> Option<(String, String, Option<u32>)> {
    match state.step2.selected.as_ref()? {
        Step2Selection::Mod { game_tab, tp_file } => {
            Some((game_tab.clone(), tp_file.clone(), None))
        }
        Step2Selection::Component {
            game_tab,
            tp_file,
            component_id,
            ..
        } => Some((
            game_tab.clone(),
            tp_file.clone(),
            component_id.trim().parse::<u32>().ok(),
        )),
    }
}

struct Step3JumpTarget<'a> {
    active_game_tab: &'a mut String,
    jump_to_selected_requested: &'a mut bool,
    items: &'a mut [Step3ItemState],
    selected: &'a mut Vec<usize>,
    anchor: &'a mut Option<usize>,
    collapsed_blocks: &'a mut Vec<String>,
}

fn jump_to_target_in_tab(
    target: &mut Step3JumpTarget<'_>,
    game_tab: &str,
    target_key: &str,
    component_ref: Option<u32>,
) -> bool {
    let target_idx = target.items.iter().position(|item| {
        if item.is_parent {
            return false;
        }
        let mod_matches = normalize_mod_key(&item.tp_file) == target_key
            || normalize_mod_key(&item.mod_name) == target_key;
        if !mod_matches {
            return false;
        }
        component_ref.is_none_or(|target_component| {
            item.component_id.trim().parse::<u32>().ok() == Some(target_component)
        })
    });

    let Some(target_idx) = target_idx else {
        return false;
    };
    let block_id = target.items[target_idx].block_id.clone();
    target.collapsed_blocks.retain(|value| value != &block_id);
    target.selected.clear();
    target.selected.push(target_idx);
    *target.anchor = Some(target_idx);
    *target.active_game_tab = game_tab.to_string();
    *target.jump_to_selected_requested = true;
    true
}
