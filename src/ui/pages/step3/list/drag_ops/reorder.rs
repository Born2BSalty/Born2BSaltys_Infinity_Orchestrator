// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::state::Step3ItemState;
use crate::ui::step3::drag;

#[allow(clippy::too_many_arguments)]
pub(in crate::ui::pages::step3::list) fn apply_live_reorder(
    ui: &egui::Ui,
    items: &mut Vec<Step3ItemState>,
    selected: &mut Vec<usize>,
    drag_from: &mut Option<usize>,
    drag_over: &Option<usize>,
    drag_indices: &mut Vec<usize>,
    drag_grab_pos_in_block: &usize,
    last_insert_at: &mut Option<usize>,
    locked_blocks: &[String],
    visible_rows: &[(usize, egui::Rect)],
) {
    if !ui.input(|i| i.pointer.primary_down()) || drag_from.is_none() || drag_indices.is_empty() {
        return;
    }
    let Some(target_slot) = *drag_over else {
        return;
    };
    if *last_insert_at == Some(target_slot) {
        return;
    }

    let mut block = drag_indices.clone();
    block.sort_unstable();
    block.dedup();
    if !block.iter().all(|idx| *idx < items.len()) {
        return;
    }
    let moving: Vec<_> = block.iter().map(|idx| items[*idx].clone()).collect();
    if moving.iter().any(|m| locked_blocks.contains(&m.block_id)) {
        return;
    }
    let mut remaining = Vec::with_capacity(items.len() - moving.len());
    for (idx, item) in items.iter().cloned().enumerate() {
        if !block.contains(&idx) {
            remaining.push(item);
        }
    }
    let mut insert_at = target_slot;
    insert_at = drag::visible_slot_to_insert_at(items, &block, visible_rows, insert_at, remaining.len());
    if block.first().is_some_and(|first| items[*first].is_parent) {
        insert_at = drag::snap_to_parent_boundary(&remaining, insert_at);
    } else {
        insert_at = drag::enforce_child_parent_constraint(&remaining, insert_at, &moving);
    }
    insert_at = drag::hard_clamp_insert_at(&remaining, insert_at, &moving);
    let mut reordered = remaining;
    reordered.splice(insert_at..insert_at, moving);
    if *items != reordered {
        *items = reordered;
        selected.clear();
        drag_indices.clear();
        for idx in insert_at..insert_at + block.len() {
            selected.push(idx);
            drag_indices.push(idx);
        }
        let grabbed = insert_at + (*drag_grab_pos_in_block).min(block.len() - 1);
        *drag_from = Some(grabbed);
    }
    *last_insert_at = Some(insert_at);
}
