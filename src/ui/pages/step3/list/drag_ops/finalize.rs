// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::state::Step3ItemState;
use crate::ui::step3::blocks;

#[allow(clippy::too_many_arguments)]
pub(in crate::ui::pages::step3::list) fn finalize_on_release(
    ui: &egui::Ui,
    items: &mut Vec<Step3ItemState>,
    selected: &mut Vec<usize>,
    drag_from: &mut Option<usize>,
    drag_over: &mut Option<usize>,
    drag_indices: &mut Vec<usize>,
    drag_grab_offset: &mut f32,
    drag_grab_pos_in_block: &mut usize,
    drag_row_h: &mut f32,
    last_insert_at: &mut Option<usize>,
    clone_seq: &mut usize,
) {
    if !ui.input(|i| i.pointer.any_released()) {
        return;
    }
    let selected_keys: std::collections::HashSet<String> = selected
        .iter()
        .filter_map(|idx| items.get(*idx))
        .filter(|item| !item.is_parent)
        .map(blocks::step3_item_key)
        .collect();
    *drag_from = None;
    *drag_over = None;
    drag_indices.clear();
    *drag_grab_offset = 0.0;
    *drag_grab_pos_in_block = 0;
    *drag_row_h = 0.0;
    *last_insert_at = None;
    blocks::repair_orphan_children(items, selected, clone_seq);
    blocks::merge_adjacent_same_mod_blocks(items, selected);
    blocks::prune_empty_parent_blocks(items, selected);
    if !selected_keys.is_empty() {
        selected.clear();
        for (idx, item) in items.iter().enumerate() {
            if item.is_parent {
                continue;
            }
            if selected_keys.contains(&blocks::step3_item_key(item)) {
                selected.push(idx);
            }
        }
        selected.sort_unstable();
        selected.dedup();
    }
}
