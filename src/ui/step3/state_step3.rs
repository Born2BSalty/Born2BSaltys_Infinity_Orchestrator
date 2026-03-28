// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::{Step3ItemState, WizardState};
use crate::ui::step2::service_selection_step2::selection_normalize_mod_key;

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
    &'a mut Vec<Vec<Step3ItemState>>,
    &'a mut Vec<Vec<Step3ItemState>>,
);

pub fn normalize_active_tab(state: &mut WizardState) {
    let show_bgee = matches!(state.step1.game_install.as_str(), "BGEE" | "EET");
    let show_bg2ee = matches!(state.step1.game_install.as_str(), "BG2EE" | "EET");
    let active_is_visible =
        (state.step3.active_game_tab == "BGEE" && show_bgee)
            || (state.step3.active_game_tab == "BG2EE" && show_bg2ee);
    if active_is_visible {
        return;
    }
    if show_bgee {
        state.step3.active_game_tab = "BGEE".to_string();
    } else if show_bg2ee {
        state.step3.active_game_tab = "BG2EE".to_string();
    }
}

pub fn active_list_mut(state: &mut WizardState) -> ActiveListMut<'_> {
    if state.step3.active_game_tab == "BGEE" {
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

pub fn jump_to_target(
    state: &mut WizardState,
    game_tab: &str,
    mod_ref: &str,
    component_ref: Option<u32>,
) -> bool {
    let target_key = selection_normalize_mod_key(mod_ref);
    if game_tab.eq_ignore_ascii_case("BGEE") {
        let mut target = Step3JumpTarget {
            active_game_tab: &mut state.step3.active_game_tab,
            jump_to_selected_requested: &mut state.step3.jump_to_selected_requested,
            items: &mut state.step3.bgee_items,
            selected: &mut state.step3.bgee_selected,
            anchor: &mut state.step3.bgee_anchor,
            collapsed_blocks: &mut state.step3.bgee_collapsed_blocks,
        };
        jump_to_target_in_tab(
            &mut target,
            game_tab,
            &target_key,
            component_ref,
        )
    } else {
        let mut target = Step3JumpTarget {
            active_game_tab: &mut state.step3.active_game_tab,
            jump_to_selected_requested: &mut state.step3.jump_to_selected_requested,
            items: &mut state.step3.bg2ee_items,
            selected: &mut state.step3.bg2ee_selected,
            anchor: &mut state.step3.bg2ee_anchor,
            collapsed_blocks: &mut state.step3.bg2ee_collapsed_blocks,
        };
        jump_to_target_in_tab(
            &mut target,
            game_tab,
            &target_key,
            component_ref,
        )
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
        let mod_matches = selection_normalize_mod_key(&item.tp_file) == target_key
            || selection_normalize_mod_key(&item.mod_name) == target_key;
        if !mod_matches {
            return false;
        }
        match component_ref {
            Some(target_component) => item.component_id.trim().parse::<u32>().ok() == Some(target_component),
            None => true,
        }
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

pub(crate) mod blocks {
    pub(crate) use crate::ui::step3::state_blocks_step3::*;
}

pub(crate) mod drag {
    pub(crate) use crate::ui::step3::state_drag_step3::*;
}
