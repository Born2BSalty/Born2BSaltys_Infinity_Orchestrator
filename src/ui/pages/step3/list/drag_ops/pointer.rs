// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::state::Step3ItemState;
use crate::ui::step3::drag;

#[allow(clippy::too_many_arguments)]
pub(in crate::ui::pages::step3::list) fn update_drag_target_from_pointer(
    ui: &egui::Ui,
    items: &[Step3ItemState],
    drag_from: &Option<usize>,
    drag_over: &mut Option<usize>,
    drag_indices: &[usize],
    drag_grab_offset: &f32,
    drag_grab_pos_in_block: &usize,
    drag_row_h: &f32,
    visible_rows: &[(usize, egui::Rect)],
) {
    if drag_from.is_none() {
        return;
    }
    if let Some(pointer) = ui.input(|i| i.pointer.interact_pos()) {
        let n = visible_rows.len();
        let k = visible_rows
            .iter()
            .filter(|(idx, _)| drag_indices.contains(idx))
            .count()
            .max(1);
        if n > 0 && k > 0 {
            let list_top_y = visible_rows.first().map(|(_, r)| r.top()).unwrap_or(pointer.y);
            let row_h = if *drag_row_h > 0.0 {
                *drag_row_h
            } else {
                (visible_rows.first().map(|(_, r)| r.height()).unwrap_or(20.0)
                    + ui.spacing().item_spacing.y.max(0.0))
                .max(1.0)
            };
            let desired_block_start = drag::compute_desired_block_start(
                pointer.y,
                list_top_y,
                row_h,
                *drag_grab_offset,
                *drag_grab_pos_in_block,
                n,
                k,
            );
            *drag_over = Some(desired_block_start.min(items.len()));
        }
    }
}
