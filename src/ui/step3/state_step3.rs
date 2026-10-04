// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::game_authority::{self, GameSlot};
use crate::app::state::{Step3ItemState, WizardState};

pub type ActiveListMut<'a> = (
    &'a mut Vec<Step3ItemState>,
    &'a mut Vec<usize>,
    &'a mut Option<usize>,
    &'a mut Option<usize>,
    &'a mut Vec<usize>,
    &'a mut Option<usize>,
    &'a mut f32,
    &'a mut usize,
    &'a mut f32,
    &'a mut Option<usize>,
    &'a mut Vec<String>,
    &'a mut usize,
    &'a mut Vec<String>,
    &'a mut Vec<crate::app::step3_history::Step3HistoryEntry>,
    &'a mut Vec<crate::app::step3_history::Step3HistoryEntry>,
);

pub fn normalize_active_tab(state: &mut WizardState) {
    let normalized =
        game_authority::normalized_tab(&state.step1.game_install, &state.step3.active_game_tab);
    if normalized != state.step3.active_game_tab {
        state.step3.active_game_tab = normalized.to_string();
    }
}

pub fn active_list_mut(state: &mut WizardState) -> ActiveListMut<'_> {
    if game_authority::slot_for_tab(&state.step3.active_game_tab) == GameSlot::First {
        (
            &mut state.step3.bgee_items,
            &mut state.step3.bgee_selected,
            &mut state.step3.bgee_drag_from,
            &mut state.step3.bgee_drag_over,
            &mut state.step3.bgee_drag_indices,
            &mut state.step3.bgee_anchor,
            &mut state.step3.bgee_drag_grab_offset,
            &mut state.step3.bgee_drag_grab_pos_in_block,
            &mut state.step3.bgee_drag_row_h,
            &mut state.step3.bgee_last_insert_at,
            &mut state.step3.bgee_collapsed_blocks,
            &mut state.step3.bgee_clone_seq,
            &mut state.step3.bgee_locked_blocks,
            &mut state.step3.bgee_undo_stack,
            &mut state.step3.bgee_redo_stack,
        )
    } else {
        (
            &mut state.step3.bg2ee_items,
            &mut state.step3.bg2ee_selected,
            &mut state.step3.bg2ee_drag_from,
            &mut state.step3.bg2ee_drag_over,
            &mut state.step3.bg2ee_drag_indices,
            &mut state.step3.bg2ee_anchor,
            &mut state.step3.bg2ee_drag_grab_offset,
            &mut state.step3.bg2ee_drag_grab_pos_in_block,
            &mut state.step3.bg2ee_drag_row_h,
            &mut state.step3.bg2ee_last_insert_at,
            &mut state.step3.bg2ee_collapsed_blocks,
            &mut state.step3.bg2ee_clone_seq,
            &mut state.step3.bg2ee_locked_blocks,
            &mut state.step3.bg2ee_undo_stack,
            &mut state.step3.bg2ee_redo_stack,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{active_list_mut, normalize_active_tab};
    use crate::app::state::{Step3ItemState, WizardState};

    #[test]
    fn iwdee_step3_tab_normalises_and_reads_the_first_container() {
        let mut state = WizardState::default();
        state.step1.game_install = "IWDEE".to_string();

        state.step3.active_game_tab = "BG2EE".to_string();
        normalize_active_tab(&mut state);
        assert_eq!(state.step3.active_game_tab, "IWDEE");

        state.step3.bgee_items.push(Step3ItemState {
            tp_file: "MOD.TP2".to_string(),
            component_id: "0".to_string(),
            mod_name: "MOD".to_string(),
            component_label: "Component".to_string(),
            raw_line: String::new(),
            prompt_summary: None,
            prompt_events: Vec::new(),
            selected_order: 0,
            block_id: "0".to_string(),
            is_parent: false,
            parent_placeholder: false,
        });
        let (items, ..) = active_list_mut(&mut state);
        assert_eq!(items.len(), 1);
    }
}
