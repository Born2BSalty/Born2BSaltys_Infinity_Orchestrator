// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::game_authority::{self, GameSlot};
use crate::app::state::{Step2ModState, WizardState};
use crate::ui::step2::tree_selection_rules_step2::{
    enforce_collapsible_group_umbrella_after_bulk, enforce_subcomponent_single_select_keep_first,
    enforce_tp2_same_mod_exclusive_after_bulk,
};

#[must_use]
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

pub fn clear_all(mods: &mut [Step2ModState], next_selection_order: &mut usize) {
    for mod_state in mods {
        mod_state.checked = false;
        for component in &mut mod_state.components {
            component.checked = false;
            component.selected_order = None;
        }
    }
    *next_selection_order = 1;
}

pub fn select_visible(mods: &mut [Step2ModState], filter: &str, next_selection_order: &mut usize) {
    for mod_state in mods {
        if !mod_matches_filter(mod_state, filter) {
            continue;
        }
        let mod_name_match = filter.is_empty() || mod_state.name.to_lowercase().contains(filter);
        for component in &mut mod_state.components {
            let is_visible = mod_name_match || component.label.to_lowercase().contains(filter);
            if !is_visible || component.disabled {
                continue;
            }
            component.checked = true;
            if component.selected_order.is_none() {
                component.selected_order = Some(*next_selection_order);
                *next_selection_order += 1;
            }
        }
        enforce_subcomponent_single_select_keep_first(mod_state);
        enforce_collapsible_group_umbrella_after_bulk(mod_state);
        enforce_meta_mode_after_bulk(mod_state);
        enforce_tp2_same_mod_exclusive_after_bulk(mod_state);
        mod_state.checked = mod_state
            .components
            .iter()
            .filter(|c| !c.disabled)
            .all(|c| c.checked);
    }
}

pub fn recompute_selection_counts(state: &mut WizardState) {
    let mods = if game_authority::slot_for_tab(&state.step2.active_game_tab) == GameSlot::First {
        &state.step2.bgee_mods
    } else {
        &state.step2.bg2ee_mods
    };
    let mut total = 0usize;
    let mut selected = 0usize;
    for mod_state in mods {
        for component in &mod_state.components {
            total += 1;
            if component.checked {
                selected += 1;
            }
        }
    }
    state.step2.total_count = total;
    state.step2.selected_count = selected;
}

fn enforce_meta_mode_after_bulk(mod_state: &mut Step2ModState) {
    let any_normal_checked = mod_state
        .components
        .iter()
        .any(|c| c.checked && !c.disabled && !c.is_meta_mode_component);
    if any_normal_checked {
        for comp in &mut mod_state.components {
            if comp.is_meta_mode_component {
                comp.checked = false;
                comp.selected_order = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::Step2ComponentState;

    fn mod_with_one_checked_component() -> Step2ModState {
        Step2ModState {
            name: "Mod".to_string(),
            tp_file: "mod.tp2".to_string(),
            tp2_path: String::new(),
            readme_path: None,
            ini_path: None,
            web_url: None,
            package_marker: None,
            latest_checked_version: None,
            update_locked: false,
            mod_prompt_summary: None,
            mod_prompt_events: Vec::new(),
            checked: false,
            hidden_components: Vec::new(),
            components: vec![Step2ComponentState {
                component_id: "0".to_string(),
                label: "Component".to_string(),
                weidu_group: None,
                collapsible_group: None,
                collapsible_group_is_umbrella: false,
                collapsible_group_combinable: false,
                raw_line: String::new(),
                prompt_summary: None,
                prompt_events: Vec::new(),
                is_meta_mode_component: false,
                disabled: false,
                compat_kind: None,
                compat_source: None,
                compat_related_mod: None,
                compat_related_component: None,
                compat_graph: None,
                compat_evidence: None,
                disabled_reason: None,
                checked: true,
                selected_order: Some(1),
            }],
        }
    }

    #[test]
    fn list_ops_route_iwdee_to_the_first_container() {
        let mut state = WizardState::default();
        state.step2.active_game_tab = "IWDEE".to_string();
        state.step2.bgee_mods = vec![mod_with_one_checked_component()];
        recompute_selection_counts(&mut state);
        assert_eq!(state.step2.total_count, 1);
        assert_eq!(state.step2.selected_count, 1);
    }
}
