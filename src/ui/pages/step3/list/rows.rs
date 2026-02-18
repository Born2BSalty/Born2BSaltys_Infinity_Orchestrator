// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::state::Step3ItemState;
use crate::ui::step3::{blocks, format, selection};

use super::history;

pub(super) struct RowRenderOutcome {
    pub visible_rows: Vec<(usize, egui::Rect)>,
    pub uncheck_requests: Vec<(String, String)>,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn render_rows(
    ui: &mut egui::Ui,
    tab_id: &str,
    visible_indices: &[usize],
    jump_to_selected_requested: &mut bool,
    items: &mut Vec<Step3ItemState>,
    selected: &mut Vec<usize>,
    drag_from: &mut Option<usize>,
    drag_over: &mut Option<usize>,
    drag_indices: &mut Vec<usize>,
    anchor: &mut Option<usize>,
    drag_grab_offset: &mut f32,
    drag_grab_pos_in_block: &mut usize,
    drag_row_h: &mut f32,
    last_insert_at: &mut Option<usize>,
    collapsed_blocks: &mut Vec<String>,
    clone_seq: &mut usize,
    locked_blocks: &mut Vec<String>,
    undo_stack: &mut Vec<Vec<Step3ItemState>>,
    redo_stack: &mut Vec<Vec<Step3ItemState>>,
) -> RowRenderOutcome {
    let mut visible_rows: Vec<(usize, egui::Rect)> = Vec::with_capacity(visible_indices.len());
    let mut uncheck_requests: Vec<(String, String)> = Vec::new();

    for &idx in visible_indices {
        let item = &items[idx];
        let label_response = if item.is_parent {
            let child_count = blocks::count_children_in_block(items, idx);
            let collapsed = collapsed_blocks.contains(&item.block_id);
            let mut is_locked = locked_blocks.contains(&item.block_id);
            let mut row_response: Option<egui::Response> = None;
            ui.horizontal(|ui| {
                if ui
                    .small_button(if is_locked { "L" } else { "-" })
                    .on_hover_text("Lock/unlock this parent block for drag operations.")
                    .clicked()
                {
                    if is_locked {
                        locked_blocks.retain(|v| v != &item.block_id);
                        is_locked = false;
                    } else {
                        locked_blocks.push(item.block_id.clone());
                        is_locked = true;
                    }
                }
                let symbol = if collapsed { "+" } else { "-" };
                if ui.small_button(symbol).clicked() {
                    if collapsed {
                        collapsed_blocks.retain(|v| v != &item.block_id);
                    } else {
                        collapsed_blocks.push(item.block_id.clone());
                    }
                }
                let title = if item.parent_placeholder {
                    format!("{} (split target)", item.mod_name)
                } else {
                    format!("{} ({child_count})", item.mod_name)
                };
                let title = if is_locked {
                    format!("{title} [locked]")
                } else {
                    title
                };
                let resp = ui.selectable_label(selected.contains(&idx), egui::RichText::new(title).strong());
                row_response = Some(resp);
            });
            let resp = row_response.expect("row response should exist");
            resp.on_hover_text("Drag to move parent block")
        } else {
            ui.horizontal(|ui| {
                ui.add_space(25.0);
                let text = format::format_step3_item(item);
                let row_text = format::weidu_colored_widget_text(ui, &text);
                ui.selectable_label(selected.contains(&idx), row_text)
            })
            .inner
            .on_hover_text("Drag to reorder")
        };
        let drag_id = ui.make_persistent_id(("step3_drag_row", tab_id, idx));
        let drag_response = ui.interact(label_response.rect, drag_id, egui::Sense::click_and_drag());
        if items[idx].is_parent {
            drag_response.context_menu(|ui| {
                if ui.button("Clone Parent (empty split target)").clicked() {
                    history::push_undo_snapshot(items, undo_stack, redo_stack);
                    blocks::clone_parent_empty_block(items, idx, clone_seq);
                    ui.close_menu();
                }
            });
        } else {
            let tp_file = items[idx].tp_file.clone();
            let component_id = items[idx].component_id.clone();
            drag_response.context_menu(|ui| {
                if ui.button("Uncheck In Step 2").clicked() {
                    uncheck_requests.push((tp_file.clone(), component_id.clone()));
                    ui.close_menu();
                }
            });
        }
        visible_rows.push((idx, label_response.rect));
        if *jump_to_selected_requested && selected.contains(&idx) {
            ui.scroll_to_rect(label_response.rect, Some(egui::Align::Center));
            *jump_to_selected_requested = false;
        }
        if label_response.clicked() || drag_response.clicked() {
            let modifiers = ui.input(|i| i.modifiers);
            selection::apply_row_selection(selected, anchor, items, visible_indices, idx, modifiers);
        }
        if drag_response.drag_started() {
            if locked_blocks.contains(&items[idx].block_id) {
                *drag_from = None;
                drag_indices.clear();
                continue;
            }
            history::push_undo_snapshot(items, undo_stack, redo_stack);
            *drag_from = Some(idx);
            if items[idx].is_parent {
                *drag_indices = blocks::block_indices(items, idx);
            } else if selected.contains(&idx) && selected.len() > 1 {
                *drag_indices = selected.clone();
            } else {
                selected.clear();
                selected.push(idx);
                drag_indices.clear();
                drag_indices.push(idx);
            }
            let mut sorted = drag_indices.clone();
            sorted.sort_unstable();
            sorted.dedup();
            *drag_grab_pos_in_block = sorted.iter().position(|v| *v == idx).unwrap_or(0);
            if let Some(pointer) = ui.input(|i| i.pointer.interact_pos())
                && let Some((_, grabbed_rect)) =
                    visible_rows.iter().find(|(row_idx, _)| *row_idx == idx)
            {
                *drag_grab_offset = pointer.y - grabbed_rect.top();
                let row_pitch = grabbed_rect.height() + ui.spacing().item_spacing.y.max(0.0);
                *drag_row_h = row_pitch.max(1.0);
            }
            *last_insert_at = None;
            *drag_over = Some(idx + 1);
        }
    }
    RowRenderOutcome {
        visible_rows,
        uncheck_requests,
    }
}
