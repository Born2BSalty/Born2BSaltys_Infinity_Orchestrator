// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::{BTreeSet, HashMap};

use crate::app::controller::step3_sync::{build_step3_items, collect_parent_block_ids};
use crate::app::game_authority::GameSlot;
use crate::app::state::{Step2ModState, Step3ItemState, WizardState};
use crate::app::step3_separators::{
    SeparatorAnchor, capture_separator_anchors, insert_separators, resolve_separator_block_ids,
};

type ComponentPair = (String, String);
type TabPlacement<'a> = (&'a [ComponentPair], &'a BTreeSet<ComponentPair>);

struct TabSource<'a> {
    current: &'a [Step3ItemState],
    collapsed: &'a [String],
    mods: &'a [Step2ModState],
    tree: GameSlot,
    placements: Vec<TabPlacement<'a>>,
}

struct TabRebuild {
    items: Vec<Step3ItemState>,
    collapsed: Vec<String>,
}

fn reconcile_tab(
    source: &TabSource<'_>,
    pending: &mut Vec<(GameSlot, Vec<SeparatorAnchor>)>,
) -> TabRebuild {
    let mut children = reconcile_step3_children(source.current, build_step3_items(source.mods));
    for (log_order, placed) in &source.placements {
        place_logged_children(&mut children, log_order, placed);
    }
    let pending_idx = pending.iter().position(|(tree, _)| *tree == source.tree);
    if children.is_empty() {
        if pending_idx.is_none() {
            let captured = capture_separator_anchors(source.current, source.collapsed);
            if !captured.is_empty() {
                pending.push((source.tree, captured));
            }
        }
        return TabRebuild {
            items: Vec::new(),
            collapsed: Vec::new(),
        };
    }
    let anchors = pending_idx.map_or_else(
        || capture_separator_anchors(source.current, source.collapsed),
        |idx| pending.remove(idx).1,
    );
    let anchors = resolve_separator_block_ids(&children, &anchors);
    let items = rebuild_with_separators(children, &anchors);
    let mut collapsed = collect_parent_block_ids(&items);
    for anchor in anchors.iter().filter(|anchor| anchor.collapsed) {
        if let Some(block_id) = &anchor.block_id
            && !collapsed.contains(block_id)
        {
            collapsed.push(block_id.clone());
        }
    }
    TabRebuild { items, collapsed }
}

pub(crate) fn sync_step3_from_step2(state: &mut WizardState) {
    let placements = std::mem::take(&mut state.step3.pending_log_placements);
    let mut pending = std::mem::take(&mut state.step3.pending_separators);
    let placements_for = |tree: GameSlot| -> Vec<TabPlacement<'_>> {
        placements
            .iter()
            .filter(|placement| placement.tree == tree)
            .map(|placement| (placement.log_order.as_slice(), &placement.placed))
            .collect()
    };

    let first_tab = reconcile_tab(
        &TabSource {
            current: &state.step3.bgee_items,
            collapsed: &state.step3.bgee_collapsed_blocks,
            mods: &state.step2.bgee_mods,
            tree: GameSlot::First,
            placements: placements_for(GameSlot::First),
        },
        &mut pending,
    );
    let second_tab = reconcile_tab(
        &TabSource {
            current: &state.step3.bg2ee_items,
            collapsed: &state.step3.bg2ee_collapsed_blocks,
            mods: &state.step2.bg2ee_mods,
            tree: GameSlot::Second,
            placements: placements_for(GameSlot::Second),
        },
        &mut pending,
    );
    state.step3.pending_separators = pending;
    state.step3.bgee_items = first_tab.items;
    state.step3.bg2ee_items = second_tab.items;
    state.step3.bgee_collapsed_blocks = first_tab.collapsed;
    state.step3.bg2ee_collapsed_blocks = second_tab.collapsed;
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

#[must_use]
pub(crate) fn rebuild_with_separators(
    items: Vec<Step3ItemState>,
    anchors: &[SeparatorAnchor],
) -> Vec<Step3ItemState> {
    let mut children: Vec<Step3ItemState> =
        items.into_iter().filter(|item| !item.is_parent).collect();
    let anchors = resolve_separator_block_ids(&children, anchors);
    insert_separators(&mut children, &anchors);
    rebuild_parent_blocks(children)
}

fn rebuild_parent_blocks(children: Vec<Step3ItemState>) -> Vec<Step3ItemState> {
    let mut out = Vec::<Step3ItemState>::new();
    let mut block_seq = 0usize;
    let mut last_parent_key = String::new();
    let mut current_block_id = String::new();

    for mut child in children {
        if child.is_separator() {
            out.push(child);
            last_parent_key.clear();
            continue;
        }
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
    use crate::app::step3_separators::separator_row;

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

    fn bgee_shape(state: &WizardState) -> Vec<String> {
        state
            .step3
            .bgee_items
            .iter()
            .map(|item| {
                if item.is_separator() {
                    format!("[{}]", item.mod_name)
                } else if item.is_parent {
                    format!("{}:", item.mod_name)
                } else {
                    format!("{}{}", item.mod_name, item.component_id)
                }
            })
            .collect()
    }

    fn separator_block(state: &WizardState, name: &str) -> String {
        state
            .step3
            .bgee_items
            .iter()
            .find(|item| item.is_separator() && item.mod_name == name)
            .map(|item| item.block_id.clone())
            .expect("separator present")
    }

    fn state_with_fixes_and_npcs_sections() -> WizardState {
        let mut state = state_with_a_b_c_ticked_and_d_unticked();
        let items = &mut state.step3.bgee_items;
        let b_header = items
            .iter()
            .position(|item| item.is_parent && item.mod_name == "B")
            .expect("B header");
        items.insert(b_header, separator_row("NPCs", "SEPARATOR::2".to_string()));
        items.insert(0, separator_row("Fixes", "SEPARATOR::1".to_string()));
        state
    }

    #[test]
    fn sync_keeps_separators_across_a_step2_change() {
        let mut state = state_with_fixes_and_npcs_sections();
        assert_eq!(
            bgee_shape(&state),
            ["[Fixes]", "A:", "A0", "[NPCs]", "B:", "B0", "C:", "C0"]
        );

        state.step2.bgee_mods[1].components[0].checked = false;
        state.step2.bgee_mods[1].components[0].selected_order = None;
        state.step2.bgee_mods[1].checked = false;
        let next_order = state.step2.next_selection_order;
        state.step2.bgee_mods[3].components[0].checked = true;
        state.step2.bgee_mods[3].components[0].selected_order = Some(next_order);
        state.step2.bgee_mods[3].checked = true;
        state.step2.next_selection_order = next_order + 1;
        sync_step3_from_step2(&mut state);

        assert_eq!(
            bgee_shape(&state),
            ["[Fixes]", "A:", "A0", "[NPCs]", "C:", "C0", "D:", "D0"]
        );
        assert_eq!(separator_block(&state, "Fixes"), "SEPARATOR::1");
        assert_eq!(separator_block(&state, "NPCs"), "SEPARATOR::2");
    }

    #[test]
    fn sync_keeps_separator_collapse_and_recollapses_mods() {
        let mut state = state_with_fixes_and_npcs_sections();
        state.step3.bgee_collapsed_blocks = vec!["SEPARATOR::2".to_string()];

        sync_step3_from_step2(&mut state);

        let collapsed = &state.step3.bgee_collapsed_blocks;
        assert!(collapsed.contains(&"SEPARATOR::2".to_string()));
        assert!(!collapsed.contains(&"SEPARATOR::1".to_string()));
        let mod_headers: Vec<&String> = state
            .step3
            .bgee_items
            .iter()
            .filter(|item| item.is_mod_header())
            .map(|item| &item.block_id)
            .collect();
        assert_eq!(mod_headers.len(), 3);
        assert!(mod_headers.iter().all(|block| collapsed.contains(block)));
    }

    #[test]
    fn sync_consumes_pending_separators_only_with_components() {
        let mut state = WizardState::default();
        state.step2.bgee_mods = vec![
            mod_with("A", vec![logged_component("A", None)]),
            mod_with("B", vec![logged_component("B", None)]),
        ];
        let pending = vec![SeparatorAnchor {
            name: "NPCs".to_string(),
            block_id: None,
            collapsed: true,
            following: vec![pair("B", "0")],
        }];
        state.step3.pending_separators = vec![(GameSlot::First, pending.clone())];

        sync_step3_from_step2(&mut state);
        assert_eq!(state.step3.bgee_items.len(), 0);
        assert_eq!(
            state.step3.pending_separators,
            vec![(GameSlot::First, pending)]
        );

        for (order, mod_state) in state.step2.bgee_mods.iter_mut().enumerate() {
            mod_state.components[0].checked = true;
            mod_state.components[0].selected_order = Some(order + 1);
            mod_state.checked = true;
        }
        state.step2.next_selection_order = 3;
        sync_step3_from_step2(&mut state);

        assert_eq!(state.step3.pending_separators.len(), 0);
        assert_eq!(bgee_shape(&state), ["A:", "A0", "[NPCs]", "B:", "B0"]);
        let npcs = separator_block(&state, "NPCs");
        assert!(state.step3.bgee_collapsed_blocks.contains(&npcs));
    }

    fn set_all_ticked(state: &mut WizardState, ticked: bool) {
        for (order, mod_state) in state.step2.bgee_mods.iter_mut().enumerate() {
            mod_state.components[0].checked = ticked;
            mod_state.components[0].selected_order = ticked.then_some(order + 1);
            mod_state.checked = ticked;
        }
        state.step2.next_selection_order = state.step2.bgee_mods.len() + 1;
    }

    #[test]
    fn untick_all_then_retick_restores_separator_positions() {
        let mut state = WizardState::default();
        state.step2.bgee_mods = ["A", "B", "C", "D"]
            .into_iter()
            .map(|name| mod_with(name, vec![logged_component(name, None)]))
            .collect();
        set_all_ticked(&mut state, true);
        sync_step3_from_step2(&mut state);
        let items = &mut state.step3.bgee_items;
        let c_header = items
            .iter()
            .position(|item| item.is_parent && item.mod_name == "C")
            .expect("C header");
        items.insert(c_header, separator_row("NPCs", "SEPARATOR::2".to_string()));
        items.insert(0, separator_row("Fixes", "SEPARATOR::1".to_string()));
        state
            .step3
            .bgee_collapsed_blocks
            .push("SEPARATOR::1".to_string());
        let original = [
            "[Fixes]", "A:", "A0", "B:", "B0", "[NPCs]", "C:", "C0", "D:", "D0",
        ];
        assert_eq!(bgee_shape(&state), original);

        set_all_ticked(&mut state, false);
        sync_step3_from_step2(&mut state);
        assert_eq!(state.step3.bgee_items.len(), 0);
        assert_eq!(state.step3.bgee_collapsed_blocks.len(), 0);
        assert_eq!(state.step3.pending_separators.len(), 1);
        assert_eq!(state.step3.pending_separators[0].0, GameSlot::First);
        assert_eq!(state.step3.pending_separators[0].1.len(), 2);

        set_all_ticked(&mut state, true);
        sync_step3_from_step2(&mut state);

        assert_eq!(bgee_shape(&state), original);
        assert_eq!(separator_block(&state, "Fixes"), "SEPARATOR::1");
        assert!(
            state
                .step3
                .bgee_collapsed_blocks
                .contains(&"SEPARATOR::1".to_string())
        );
        assert!(
            !state
                .step3
                .bgee_collapsed_blocks
                .contains(&"SEPARATOR::2".to_string())
        );
        assert_eq!(state.step3.pending_separators.len(), 0);
    }

    #[test]
    fn sync_without_separators_is_unchanged() {
        let mut state = state_with_a_b_c_ticked_and_d_unticked();
        assert_eq!(
            state.step3.bgee_items,
            build_step3_items(&state.step2.bgee_mods)
        );
        assert_eq!(
            state.step3.bgee_collapsed_blocks,
            collect_parent_block_ids(&state.step3.bgee_items)
        );

        untick_a_and_tick_d(&mut state);
        sync_step3_from_step2(&mut state);

        assert_eq!(bgee_shape(&state), ["B:", "B0", "C:", "C0", "D:", "D0"]);
        assert!(
            state
                .step3
                .bgee_items
                .iter()
                .all(|item| !item.is_separator())
        );
        assert_eq!(
            state.step3.bgee_collapsed_blocks,
            collect_parent_block_ids(&state.step3.bgee_items)
        );
        assert_eq!(state.step3.pending_separators.len(), 0);
        assert_eq!(state.step3.bg2ee_items.len(), 0);
    }
}
