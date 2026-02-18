// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::state::Step3ItemState;

pub(crate) fn visible_slot_to_insert_at(
    items: &[Step3ItemState],
    block: &[usize],
    visible_rows: &[(usize, egui::Rect)],
    target_visible_slot: usize,
    remaining_len: usize,
) -> usize {
    let remaining_full_indices: Vec<usize> = items
        .iter()
        .enumerate()
        .filter_map(|(idx, _)| if block.contains(&idx) { None } else { Some(idx) })
        .collect();
    let visible_remaining_full: Vec<usize> = visible_rows
        .iter()
        .filter_map(|(idx, _)| if block.contains(idx) { None } else { Some(*idx) })
        .collect();
    if target_visible_slot >= visible_remaining_full.len() {
        return remaining_len;
    }
    let target_full = visible_remaining_full[target_visible_slot];
    remaining_full_indices
        .iter()
        .position(|idx| *idx == target_full)
        .unwrap_or(remaining_len)
}
