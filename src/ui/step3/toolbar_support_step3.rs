// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

pub(crate) use crate::app::step3_toolbar::{
    Step3ToolbarSummary, build_toolbar_summary, open_toolbar_issue_popup, tab_has_conflict,
};

use crate::app::state::{Step3ItemState, WizardState};
use crate::app::step3_history::{Step3HistoryEntry, Step3TouchedRows};
use crate::ui::step3::move_selection_step3::{
    expand_blocks_hiding_lit_components, restore_selection_after_history,
};
use crate::ui::step3::state_step3::active_list_mut;
use crate::ui::step5::service_diagnostics_support_step5::export_diagnostics;

pub(crate) fn export_diagnostics_from_step3(
    state: &mut WizardState,
    dev_mode: bool,
    exe_fingerprint: &str,
) {
    match export_diagnostics(state, None, dev_mode, exe_fingerprint) {
        Ok(path) => {
            state.step5.last_status_text = format!("Diagnostics exported: {}", path.display());
        }
        Err(err) => {
            state.step5.last_status_text = format!("Diagnostics export failed: {err}");
        }
    }
}

pub(crate) fn expand_all_active(state: &mut WizardState) {
    let (_, selected, _, _, _, anchor, _, _, _, _, collapsed_blocks, _, _, _, _) =
        crate::ui::step3::state_step3::active_list_mut(state);
    crate::app::step3_history::expand_all(collapsed_blocks);
    selected.clear();
    *anchor = None;
}

pub(crate) fn collapse_all_active(state: &mut WizardState) {
    let (items, selected, _, _, _, anchor, _, _, _, _, collapsed_blocks, _, _, _, _) =
        crate::ui::step3::state_step3::active_list_mut(state);
    crate::app::step3_history::collapse_all(items, collapsed_blocks);
    selected.clear();
    *anchor = None;
}

fn step_history_active(
    state: &mut WizardState,
    change: impl FnOnce(
        &mut Vec<Step3ItemState>,
        &mut Vec<Step3HistoryEntry>,
        &mut Vec<Step3HistoryEntry>,
    ) -> Option<Step3TouchedRows>,
) {
    let (
        items,
        selected,
        _,
        _,
        _,
        anchor,
        _,
        _,
        _,
        _,
        collapsed_blocks,
        _,
        _,
        undo_stack,
        redo_stack,
    ) = active_list_mut(state);
    restore_selection_after_history(items, selected, anchor, |items| {
        change(items, undo_stack, redo_stack)
    });
    expand_blocks_hiding_lit_components(items, selected, collapsed_blocks);
    let anything_lit = !selected.is_empty();
    if anything_lit {
        state.step3.jump_to_selected_requested = true;
    }
}

pub(crate) fn redo_active(state: &mut WizardState) {
    step_history_active(state, crate::app::step3_history::redo);
}

pub(crate) fn undo_active(state: &mut WizardState) {
    step_history_active(state, crate::app::step3_history::undo);
}

#[cfg(test)]
mod tests {
    use super::{collapse_all_active, expand_all_active, redo_active, undo_active};
    use crate::app::state::{Step3ItemState, WizardState};
    use crate::app::step3_history::{self, Step3TouchedRows};
    use crate::ui::step3::move_selection_step3::capture_identities;
    use crate::ui::step3::state_step3::active_list_mut;

    fn row(mod_name: &str, component_id: &str, is_parent: bool) -> Step3ItemState {
        Step3ItemState {
            tp_file: format!("{mod_name}.tp2"),
            component_id: component_id.to_string(),
            mod_name: mod_name.to_string(),
            component_label: String::new(),
            raw_line: String::new(),
            prompt_summary: None,
            prompt_events: Vec::new(),
            selected_order: 0,
            block_id: format!("{mod_name}::block0"),
            is_parent,
            parent_placeholder: false,
        }
    }

    fn state_with_two_mods_and_a_selection(selection: &[usize]) -> WizardState {
        let mut state = WizardState::default();
        let (items, selected, _, _, _, anchor, ..) = active_list_mut(&mut state);
        *items = vec![
            row("A", "__PARENT__", true),
            row("A", "1", false),
            row("B", "__PARENT__", true),
            row("B", "1", false),
            row("B", "2", false),
        ];
        *selected = selection.to_vec();
        *anchor = selection.first().copied();
        state
    }

    fn selection_and_anchor(state: &mut WizardState) -> (Vec<usize>, Option<usize>) {
        let (_, selected, _, _, _, anchor, ..) = active_list_mut(state);
        (selected.clone(), *anchor)
    }

    #[test]
    fn collapse_all_clears_the_selection() {
        let mut state = state_with_two_mods_and_a_selection(&[1, 3]);

        collapse_all_active(&mut state);

        assert_eq!(selection_and_anchor(&mut state), (Vec::new(), None));
    }

    #[test]
    fn expand_all_clears_the_selection() {
        let mut state = state_with_two_mods_and_a_selection(&[1, 3]);

        expand_all_active(&mut state);

        assert_eq!(selection_and_anchor(&mut state), (Vec::new(), None));
    }

    fn state_with_three_one_component_mods() -> WizardState {
        let mut state = WizardState::default();
        let (items, ..) = active_list_mut(&mut state);
        *items = vec![
            row("A", "__PARENT__", true),
            row("A", "1", false),
            row("B", "__PARENT__", true),
            row("B", "1", false),
            row("C", "__PARENT__", true),
            row("C", "1", false),
        ];
        state
    }

    #[test]
    fn undo_and_redo_light_the_rows_each_step_moved() {
        let mut state = state_with_three_one_component_mods();
        {
            let (items, .., undo_stack, redo_stack) = active_list_mut(&mut state);
            let a_touched = capture_identities(items, &[0, 1]);
            step3_history::push_undo_snapshot(items, a_touched, undo_stack, redo_stack);
            let mod_a: Vec<Step3ItemState> = items.drain(0..2).collect();
            items.extend(mod_a);
        }
        {
            let (items, .., undo_stack, redo_stack) = active_list_mut(&mut state);
            let c_touched = capture_identities(items, &[2, 3]);
            step3_history::push_undo_snapshot(items, c_touched, undo_stack, redo_stack);
            let mod_c: Vec<Step3ItemState> = items.drain(2..4).collect();
            items.splice(0..0, mod_c);
        }
        {
            let (_, selected, _, _, _, anchor, ..) = active_list_mut(&mut state);
            *selected = vec![0, 1];
            *anchor = Some(0);
        }

        undo_active(&mut state);
        assert_eq!(selection_and_anchor(&mut state), (vec![2, 3], Some(2)));

        undo_active(&mut state);
        assert_eq!(selection_and_anchor(&mut state), (vec![0, 1], Some(0)));

        redo_active(&mut state);
        assert_eq!(selection_and_anchor(&mut state), (vec![4, 5], Some(4)));

        redo_active(&mut state);
        assert_eq!(selection_and_anchor(&mut state), (vec![0, 1], Some(0)));
    }

    #[test]
    fn undo_raises_the_scroll_flag_when_rows_light_up() {
        let mut state = state_with_three_one_component_mods();
        {
            let (items, _, _, _, _, _, _, _, _, _, _, _, _, undo_stack, redo_stack) =
                active_list_mut(&mut state);
            let a_touched = capture_identities(items, &[0, 1]);
            step3_history::push_undo_snapshot(items, a_touched, undo_stack, redo_stack);
            items.swap(0, 1);
        }
        state.step3.jump_to_selected_requested = false;

        undo_active(&mut state);
        assert!(state.step3.jump_to_selected_requested);

        state.step3.jump_to_selected_requested = false;
        redo_active(&mut state);
        assert!(state.step3.jump_to_selected_requested);
    }

    #[test]
    fn undo_that_lights_nothing_raises_no_scroll() {
        let mut state = state_with_three_one_component_mods();
        state.step3.jump_to_selected_requested = false;

        undo_active(&mut state);

        assert!(!state.step3.jump_to_selected_requested);
    }

    #[test]
    fn undo_expands_a_collapsed_mod_that_hides_a_lit_component() {
        let mut state = state_with_three_one_component_mods();
        {
            let (items, _, _, _, _, _, _, _, _, _, collapsed_blocks, _, _, undo_stack, redo_stack) =
                active_list_mut(&mut state);
            let touched = capture_identities(items, &[3]);
            step3_history::push_undo_snapshot(items, touched, undo_stack, redo_stack);
            items.swap(3, 5);
            collapsed_blocks.push("B::block0".to_string());
        }

        undo_active(&mut state);

        let (items, selected, _, _, _, _, _, _, _, _, collapsed_blocks, _, _, _, _) =
            active_list_mut(&mut state);
        assert!(!collapsed_blocks.contains(&"B::block0".to_string()));
        assert_eq!(selected.len(), 1);
        let idx = selected[0];
        assert_eq!(items[idx].mod_name, "B");
        assert_eq!(items[idx].component_id, "1");
    }

    #[test]
    fn undo_opens_a_whole_moved_mod_that_is_collapsed() {
        let mut state = state_with_three_one_component_mods();
        {
            let (items, _, _, _, _, _, _, _, _, _, collapsed_blocks, _, _, undo_stack, redo_stack) =
                active_list_mut(&mut state);
            let touched = capture_identities(items, &[0, 1]);
            step3_history::push_undo_snapshot(items, touched, undo_stack, redo_stack);
            items.swap(0, 2);
            items.swap(1, 3);
            collapsed_blocks.push("A::block0".to_string());
        }

        undo_active(&mut state);

        let (_, selected, _, _, _, _, _, _, _, _, collapsed_blocks, _, _, _, _) =
            active_list_mut(&mut state);
        assert!(!collapsed_blocks.contains(&"A::block0".to_string()));
        assert_eq!(selected.len(), 2);
    }

    #[test]
    fn a_step_that_moved_nothing_keeps_the_same_rows_selected() {
        let mut state = state_with_two_mods_and_a_selection(&[]);
        {
            let (items, selected, .., undo_stack, redo_stack) = active_list_mut(&mut state);
            step3_history::push_undo_snapshot(
                items,
                Step3TouchedRows::default(),
                undo_stack,
                redo_stack,
            );
            items.insert(0, row("Z", "__PARENT__", true));
            *selected = vec![4];
        }

        undo_active(&mut state);
        assert_eq!(selection_and_anchor(&mut state), (vec![3], Some(3)));
    }
}
