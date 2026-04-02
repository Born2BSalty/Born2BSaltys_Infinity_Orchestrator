// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;
use std::collections::HashMap;

use crate::ui::state::Step2Selection;
use crate::ui::state::WizardState;
use crate::ui::compat_step3_rules::Step3CompatMarker;
use crate::ui::step3::blocks;
use crate::ui::step3::list_rows_step3::{render_rows, RowRenderContext};
use crate::ui::step3::state_step3;

pub(crate) fn render(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    jump_to_selected_requested: &mut bool,
    compat_markers: &HashMap<String, Step3CompatMarker>,
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
                    let prompt_eval = crate::ui::step2::state_step2::build_prompt_eval_context(state);
                    let (
                        pending_unchecks,
                        pending_prompt_actions,
                        open_prompt_popup,
                        open_compat_popup,
                        _revalidate_after_drag,
                    ) = {
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
                        ) = state_step3::active_list_mut(state);
                        if items.is_empty() {
                            ui.label("No selected components from Step 2.");
                            return;
                        }
                        let visible_indices = blocks::visible_indices(items, collapsed_blocks);
                        let mut row_ctx = RowRenderContext {
                            prompt_eval: &prompt_eval,
                            compat_markers,
                            tab_id: &tab_id,
                            visible_indices: &visible_indices,
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
                        };
                        let row_outcome = render_rows(ui, &mut row_ctx);
                        let mut visible_rows = row_outcome.visible_rows;
                        let mut pointer_ctx =
                            crate::ui::step3::service_step3::drag_ops::DragPointerContext {
                                items,
                                drag_from,
                                drag_over,
                                drag_indices,
                                drag_grab_offset,
                                drag_grab_pos_in_block,
                                drag_row_h,
                                visible_rows: &visible_rows,
                            };
                        crate::ui::step3::service_step3::drag_ops::update_drag_target_from_pointer(
                            ui,
                            &mut pointer_ctx,
                        );
                        crate::ui::step3::service_step3::drag_ops::draw_insert_marker(
                            ui,
                            items,
                            drag_from,
                            *drag_over,
                            &visible_rows,
                        );
                        let mut reorder_ctx =
                            crate::ui::step3::service_step3::drag_ops::LiveReorderContext {
                                items,
                                selected,
                                drag_from,
                                drag_over,
                                drag_indices,
                                drag_grab_pos_in_block,
                                last_insert_at,
                                locked_blocks,
                                visible_rows: &visible_rows,
                            };
                        crate::ui::step3::service_step3::drag_ops::apply_live_reorder(
                            ui,
                            &mut reorder_ctx,
                        );
                        let mut finalize_ctx =
                            crate::ui::step3::service_step3::drag_ops::DragFinalizeContext {
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
                            };
                        let finalized_drag =
                            crate::ui::step3::service_step3::drag_ops::finalize_on_release(
                                ui,
                                &mut finalize_ctx,
                            );
                        visible_rows.clear();
                        (
                            row_outcome.uncheck_requests,
                            row_outcome.prompt_requests,
                            row_outcome.open_prompt_popup,
                            row_outcome.open_compat_popup,
                            finalized_drag,
                        )
                    };
                    if let Some((title, text)) = open_prompt_popup {
                        crate::ui::step2::prompt_popup_step2::open_text_prompt_popup(
                            state, title, text,
                        );
                    }
                    if let Some((tp_file, component_id, component_key, issue)) = open_compat_popup {
                        state.step2.selected = Some(Step2Selection::Component {
                            game_tab: tab_id.clone(),
                            tp_file,
                            component_id,
                            component_key,
                        });
                        state.step2.compat_popup_issue_override = Some(issue);
                        state.step2.compat_popup_open = true;
                    }
                    if !pending_unchecks.is_empty() {
                        crate::ui::step3::service_step3::component_uncheck::apply_component_unchecks(
                            state,
                            &tab_id,
                            &pending_unchecks,
                        );
                    }
                    if !pending_prompt_actions.is_empty() {
                        crate::ui::step3::service_step3::prompt_actions::apply_prompt_actions(
                            state,
                            &pending_prompt_actions,
                        );
                    }
                });
        });
    });

    crate::ui::step3::service_step3::prompt_actions::render(ui, state);
}
