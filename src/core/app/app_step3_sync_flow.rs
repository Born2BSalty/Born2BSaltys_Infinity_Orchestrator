// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::{BTreeSet, HashMap};

use crate::app::controller::step3_sync::{build_step3_items, collect_parent_block_ids};
use crate::app::game_authority::GameSlot;
use crate::app::state::{Step2ModState, Step3ItemState, WizardState};

type ComponentPair = (String, String);

pub(crate) fn sync_step3_from_step2(state: &mut WizardState) {
    let placements = std::mem::take(&mut state.step3.pending_log_placements);
    let reconcile = |current: &[Step3ItemState], mods: &[Step2ModState], tree: GameSlot| {
        let mut children = reconcile_step3_children(current, build_step3_items(mods));
        for placement in placements.iter().filter(|placement| placement.tree == tree) {
            place_logged_children(&mut children, &placement.log_order, &placement.placed);
        }
        rebuild_parent_blocks(children)
    };

    state.step3.bgee_items = reconcile(
        &state.step3.bgee_items,
        &state.step2.bgee_mods,
        GameSlot::First,
    );
    state.step3.bg2ee_items = reconcile(
        &state.step3.bg2ee_items,
        &state.step2.bg2ee_mods,
        GameSlot::Second,
    );
    state.step3.bgee_collapsed_blocks = collect_parent_block_ids(&state.step3.bgee_items);
    state.step3.bg2ee_collapsed_blocks = collect_parent_block_ids(&state.step3.bg2ee_items);
    state.step3.bgee_clone_seq = 1;
    state.step3.bg2ee_clone_seq = 1;
    state.step3.bgee_selected.clear();
    state.step3.bg2ee_selected.clear();
    state.step3.bgee_drag_from = None;
    state.step3.bg2ee_drag_from = None;
    state.step3.bgee_drag_over = None;
    state.step3.bg2ee_drag_over = None;
    state.step3.bgee_drag_indices.clear();
    state.step3.bg2ee_drag_indices.clear();
    state.step3.bgee_anchor = None;
    state.step3.bg2ee_anchor = None;
    state.step3.jump_to_selected_requested = false;
}

fn reconcile_step3_children(
    current: &[Step3ItemState],
    fresh: Vec<Step3ItemState>,
) -> Vec<Step3ItemState> {
    let current_order: Vec<String> = current
        .iter()
        .filter(|item| !item.is_parent)
        .map(child_key)
        .collect();
    let mut fresh_by_key = HashMap::<String, Step3ItemState>::new();
    let mut fresh_order = Vec::<String>::new();
    for item in fresh.into_iter().filter(|item| !item.is_parent) {
        let key = child_key(&item);
        fresh_order.push(key.clone());
        fresh_by_key.insert(key, item);
    }

    let same_selected_set = current_order.len() == fresh_order.len()
        && current_order
            .iter()
            .all(|key| fresh_by_key.contains_key(key));
    if same_selected_set && current_order != fresh_order {
        let mut reordered_children = Vec::<Step3ItemState>::new();
        for key in fresh_order {
            if let Some(fresh_item) = fresh_by_key.remove(&key) {
                reordered_children.push(fresh_item);
            }
        }
        return reordered_children;
    }

    let mut ordered_children = Vec::<Step3ItemState>::new();
    for item in current.iter().filter(|item| !item.is_parent) {
        let key = child_key(item);
        if let Some(fresh_item) = fresh_by_key.remove(&key) {
            ordered_children.push(fresh_item);
        }
    }
    for key in fresh_order {
        if let Some(fresh_item) = fresh_by_key.remove(&key) {
            ordered_children.push(fresh_item);
        }
    }

    ordered_children
}

fn place_logged_children(
    children: &mut Vec<Step3ItemState>,
    log_order: &[ComponentPair],
    placed: &BTreeSet<ComponentPair>,
) {
    let (mut held, kept): (Vec<_>, Vec<_>) = std::mem::take(children)
        .into_iter()
        .partition(|child| placed.contains(&child_pair(child)));
    *children = kept;
    if held.is_empty() {
        return;
    }
    let mut anchor = 0;
    for pair in log_order {
        if held.iter().any(|child| has_pair(child, pair)) {
            let (matching, rest): (Vec<_>, Vec<_>) =
                held.into_iter().partition(|child| has_pair(child, pair));
            held = rest;
            for child in matching {
                children.insert(anchor, child);
                anchor += 1;
            }
        } else if let Some(index) = children.iter().position(|child| has_pair(child, pair)) {
            anchor = index + 1;
        }
    }
    children.extend(held);
}

fn child_pair(item: &Step3ItemState) -> ComponentPair {
    (item.tp_file.to_ascii_uppercase(), item.component_id.clone())
}

fn has_pair(item: &Step3ItemState, pair: &ComponentPair) -> bool {
    item.component_id == pair.1 && item.tp_file.eq_ignore_ascii_case(&pair.0)
}

fn rebuild_parent_blocks(children: Vec<Step3ItemState>) -> Vec<Step3ItemState> {
    let mut out = Vec::<Step3ItemState>::new();
    let mut block_seq = 0usize;
    let mut last_parent_key = String::new();
    let mut current_block_id = String::new();

    for mut child in children {
        let next_parent_key = parent_key(&child);
        if next_parent_key != last_parent_key {
            block_seq += 1;
            current_block_id = format!("{next_parent_key}::segment{block_seq}");
            out.push(Step3ItemState {
                tp_file: child.tp_file.clone(),
                component_id: "__PARENT__".to_string(),
                mod_name: child.mod_name.clone(),
                component_label: child.mod_name.clone(),
                raw_line: String::new(),
                prompt_summary: None,
                prompt_events: Vec::new(),
                selected_order: child.selected_order,
                block_id: current_block_id.clone(),
                is_parent: true,
                parent_placeholder: false,
            });
            last_parent_key = next_parent_key;
        }

        child.block_id.clone_from(&current_block_id);
        child.is_parent = false;
        child.parent_placeholder = false;
        out.push(child);
    }

    out
}

fn child_key(item: &Step3ItemState) -> String {
    format!(
        "{}|{}|{}|{}",
        item.tp_file.to_ascii_uppercase(),
        item.component_id,
        item.mod_name.to_ascii_uppercase(),
        item.raw_line
    )
}

fn parent_key(item: &Step3ItemState) -> String {
    format!(
        "{}::{}",
        item.tp_file.to_ascii_uppercase(),
        item.mod_name.to_ascii_uppercase()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::Step2ComponentState;

    fn component(component_id: &str, selected_order: Option<usize>) -> Step2ComponentState {
        Step2ComponentState {
            component_id: component_id.to_string(),
            label: format!("Component {component_id}"),
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
            checked: selected_order.is_some(),
            selected_order,
        }
    }

    fn mod_with(name: &str, components: Vec<Step2ComponentState>) -> Step2ModState {
        Step2ModState {
            name: name.to_string(),
            tp_file: format!("SETUP-{name}.TP2"),
            tp2_path: format!("{name}/SETUP-{name}.TP2"),
            readme_path: None,
            ini_path: None,
            web_url: None,
            package_marker: None,
            latest_checked_version: None,
            update_locked: false,
            mod_prompt_summary: None,
            mod_prompt_events: Vec::new(),
            checked: components.iter().any(|component| component.checked),
            hidden_components: Vec::new(),
            components,
        }
    }

    fn children(state: &WizardState) -> Vec<ComponentPair> {
        state
            .step3
            .bgee_items
            .iter()
            .filter(|item| !item.is_parent)
            .map(|item| (item.tp_file.clone(), item.component_id.clone()))
            .collect()
    }

    fn pair(name: &str, component_id: &str) -> ComponentPair {
        (format!("SETUP-{name}.TP2"), component_id.to_string())
    }

    #[test]
    fn a_manual_tick_still_appends_at_the_end() {
        let mut state = WizardState::default();
        state.step2.bgee_mods = vec![
            mod_with("A", vec![component("0", Some(1))]),
            mod_with("B", vec![component("0", None)]),
            mod_with("C", vec![component("0", Some(2))]),
        ];
        state.step2.next_selection_order = 3;
        sync_step3_from_step2(&mut state);
        assert_eq!(children(&state), vec![pair("A", "0"), pair("C", "0")]);

        let next_order = state.step2.next_selection_order;
        state.step2.bgee_mods[1].components[0] = component("0", Some(next_order));
        state.step2.bgee_mods[1].checked = true;
        state.step2.next_selection_order = next_order + 1;
        sync_step3_from_step2(&mut state);

        assert_eq!(
            children(&state),
            vec![pair("A", "0"), pair("C", "0"), pair("B", "0")]
        );
    }

    fn logged_component(name: &str, selected_order: Option<usize>) -> Step2ComponentState {
        let mut logged = component("0", selected_order);
        logged.raw_line = format!("~{name}\\SETUP-{name}.TP2~ #0 #0 // Component 0");
        logged
    }

    fn state_with_a_b_c_ticked_and_d_unticked() -> WizardState {
        let mut state = WizardState::default();
        state.step2.bgee_mods = vec![
            mod_with("A", vec![logged_component("A", Some(1))]),
            mod_with("B", vec![logged_component("B", Some(2))]),
            mod_with("C", vec![logged_component("C", Some(3))]),
            mod_with("D", vec![logged_component("D", None)]),
        ];
        state.step2.next_selection_order = 4;
        sync_step3_from_step2(&mut state);
        state
    }

    fn untick_a_and_tick_d(state: &mut WizardState) {
        state.step2.bgee_mods[0].components[0].checked = false;
        state.step2.bgee_mods[0].components[0].selected_order = None;
        state.step2.bgee_mods[0].checked = false;
        let next_order = state.step2.next_selection_order;
        state.step2.bgee_mods[3].components[0].checked = true;
        state.step2.bgee_mods[3].components[0].selected_order = Some(next_order);
        state.step2.bgee_mods[3].checked = true;
        state.step2.next_selection_order = next_order + 1;
    }

    fn set_answer_on_b(state: &mut WizardState, answer: &str) {
        state.step3.prompt_edit_tp2_file = "SETUP-B.TP2".to_string();
        state.step3.prompt_edit_component_id = "0".to_string();
        crate::app::step3_prompt_edit::save_wlb_answer(state, answer).expect("answer saved");
    }

    fn b_raw_line(state: &WizardState) -> String {
        state
            .step3
            .bgee_items
            .iter()
            .find(|item| !item.is_parent && item.mod_name == "B")
            .map(|item| item.raw_line.clone())
            .expect("B stays in Step 3")
    }

    #[test]
    fn a_step3_prompt_answer_keeps_its_row_in_place_across_a_step2_change() {
        let mut state = state_with_a_b_c_ticked_and_d_unticked();
        set_answer_on_b(&mut state, "y,2");
        let edited_line = b_raw_line(&state);

        untick_a_and_tick_d(&mut state);
        sync_step3_from_step2(&mut state);

        assert_eq!(
            children(&state),
            vec![pair("B", "0"), pair("C", "0"), pair("D", "0")]
        );
        assert_eq!(b_raw_line(&state), edited_line);
        assert!(edited_line.contains("@wlb-inputs: y,2"));
    }

    #[test]
    fn clearing_a_step3_prompt_answer_keeps_its_row_in_place_across_a_step2_change() {
        let mut state = state_with_a_b_c_ticked_and_d_unticked();
        set_answer_on_b(&mut state, "y,2");
        sync_step3_from_step2(&mut state);
        crate::app::step3_prompt_edit::apply_prompt_actions(
            &mut state,
            &[crate::app::step3_prompt_edit::PromptActionRequest::Clear {
                tp_file: "SETUP-B.TP2".to_string(),
                component_id: "0".to_string(),
            }],
        );

        untick_a_and_tick_d(&mut state);
        sync_step3_from_step2(&mut state);

        assert_eq!(
            children(&state),
            vec![pair("B", "0"), pair("C", "0"), pair("D", "0")]
        );
        assert!(!b_raw_line(&state).contains("@wlb-inputs"));
    }
}
