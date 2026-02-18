// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::state::WizardState;
use crate::ui::step3::{access, blocks};

mod drag_ops;
mod history;
mod rows;
mod component_uncheck;

pub(super) fn render(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    jump_to_selected_requested: &mut bool,
) {
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        let nav_clearance = 26.0;
        let list_height = (ui.available_height() - nav_clearance).max(180.0);
        let viewport_w = ui.available_width();
        ui.scope(|ui| {
            let mut scroll = egui::style::ScrollStyle::solid();
            scroll.bar_width = 12.0;
            scroll.bar_inner_margin = 0.0;
            scroll.bar_outer_margin = 2.0;
            ui.style_mut().spacing.scroll = scroll;
            egui::ScrollArea::both()
                .id_salt("step3_list_scroll")
                .auto_shrink([false, false])
                .max_height(list_height)
                .show(ui, |ui| {
                    ui.set_min_width(viewport_w);
                    let tab_id = state.step3.active_game_tab.clone();
                    let pending_unchecks = {
                        let (
                            items,
                            selected,
                            drag_from,
                            drag_over,
                            drag_indices,
                            anchor,
                            drag_grab_offset,
                            drag_grab_pos_in_block,
                            drag_row_h,
                            last_insert_at,
                            collapsed_blocks,
                            clone_seq,
                            locked_blocks,
                            undo_stack,
                            redo_stack,
                        ) = access::active_list_mut(state);
                        if items.is_empty() {
                            ui.label("No selected components from Step 2.");
                            return;
                        }
                        let visible_indices = blocks::visible_indices(items, collapsed_blocks);
                        let row_outcome = rows::render_rows(
                            ui,
                            &tab_id,
                            &visible_indices,
                            jump_to_selected_requested,
                            items,
                            selected,
                            drag_from,
                            drag_over,
                            drag_indices,
                            anchor,
                            drag_grab_offset,
                            drag_grab_pos_in_block,
                            drag_row_h,
                            last_insert_at,
                            collapsed_blocks,
                            clone_seq,
                            locked_blocks,
                            undo_stack,
                            redo_stack,
                        );
                        let mut visible_rows = row_outcome.visible_rows;
                        drag_ops::update_drag_target_from_pointer(
                            ui,
                            items,
                            drag_from,
                            drag_over,
                            drag_indices,
                            drag_grab_offset,
                            drag_grab_pos_in_block,
                            drag_row_h,
                            &visible_rows,
                        );
                        drag_ops::draw_insert_marker(ui, items, drag_from, *drag_over, &visible_rows);
                        drag_ops::apply_live_reorder(
                            ui,
                            items,
                            selected,
                            drag_from,
                            drag_over,
                            drag_indices,
                            drag_grab_pos_in_block,
                            last_insert_at,
                            locked_blocks,
                            &visible_rows,
                        );
                        drag_ops::finalize_on_release(
                            ui,
                            items,
                            selected,
                            drag_from,
                            drag_over,
                            drag_indices,
                            drag_grab_offset,
                            drag_grab_pos_in_block,
                            drag_row_h,
                            last_insert_at,
                            clone_seq,
                        );
                        visible_rows.clear();
                        row_outcome.uncheck_requests
                    };
                    if !pending_unchecks.is_empty() {
                        component_uncheck::apply_component_unchecks(state, &tab_id, &pending_unchecks);
                    }
                });
        });
    });
}
