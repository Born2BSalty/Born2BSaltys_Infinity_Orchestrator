// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

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
    &'a mut Vec<Vec<Step3ItemState>>,
    &'a mut Vec<Vec<Step3ItemState>>,
);

pub fn normalize_active_tab(state: &mut WizardState) {
    let show_primary_game = matches!(state.step1.game_install.as_str(), "BGEE" | "EET");
    let show_secondary_game = matches!(state.step1.game_install.as_str(), "BG2EE" | "EET");
    let active_is_visible = (state.step3.active_game_tab == "BGEE" && show_primary_game)
        || (state.step3.active_game_tab == "BG2EE" && show_secondary_game);
    if active_is_visible {
        return;
    }
    if show_primary_game {
        state.step3.active_game_tab = "BGEE".to_string();
    } else if show_secondary_game {
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

pub mod blocks {
    pub use crate::ui::step3::state_blocks_step3::*;
}

pub mod drag {
    pub use crate::ui::step3::state_drag_step3::*;
}
