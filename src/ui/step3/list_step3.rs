// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

pub(crate) mod list {
use eframe::egui;

use crate::ui::state::WizardState;
use crate::ui::step3::blocks;
use crate::ui::step3::state_step3;


pub(crate) fn render(
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
                    let prompt_eval = crate::ui::step2::state_step2::build_prompt_eval_context(state);
                    let (pending_unchecks, pending_prompt_actions, open_prompt_popup) = {
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
                        let row_outcome = rows::render_rows(
                            ui,
                            &prompt_eval,
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
                        crate::ui::step3::service_step3::drag_ops::update_drag_target_from_pointer(
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
                        crate::ui::step3::service_step3::drag_ops::draw_insert_marker(ui, items, drag_from, *drag_over, &visible_rows);
                        crate::ui::step3::service_step3::drag_ops::apply_live_reorder(
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
                        crate::ui::step3::service_step3::drag_ops::finalize_on_release(
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
                        (row_outcome.uncheck_requests, row_outcome.prompt_requests, row_outcome.open_prompt_popup)
                    };
                    if let Some((title, text)) = open_prompt_popup {
                        state.step2.prompt_popup_title = title;
                        state.step2.prompt_popup_text = text;
                        state.step2.prompt_popup_open = true;
                    }
                    if !pending_unchecks.is_empty() {
                        crate::ui::step3::service_step3::component_uncheck::apply_component_unchecks(state, &tab_id, &pending_unchecks);
                    }
                    if !pending_prompt_actions.is_empty() {
                        crate::ui::step3::service_step3::prompt_actions::apply_prompt_actions(state, &pending_prompt_actions);
                    }
                });
        });
    });

    crate::ui::step3::service_step3::prompt_actions::render(ui, state);
}

pub(crate) mod rows {
use eframe::egui;

use crate::ui::state::Step3ItemState;
use crate::ui::step3::blocks;
use crate::ui::step3::content_step3;
use crate::ui::step3::prompt_popup_step3;
use crate::ui::step3::service_step3;

#[derive(Debug, Clone)]
pub(crate) enum PromptActionRequest {
    SetWlb {
        tp_file: String,
        component_id: String,
        component_label: String,
        mod_name: String,
    },
    EditJson {
        tp_file: String,
        component_id: String,
        component_label: String,
        mod_name: String,
    },
    Clear {
        tp_file: String,
        component_id: String,
    },
}

pub(super) struct RowRenderOutcome {
    pub visible_rows: Vec<(usize, egui::Rect)>,
    pub uncheck_requests: Vec<(String, String)>,
    pub prompt_requests: Vec<PromptActionRequest>,
    pub open_prompt_popup: Option<(String, String)>,
}

const STEP3_HISTORY_LIMIT: usize = 100;

fn push_undo_snapshot(
    items: &[Step3ItemState],
    undo_stack: &mut Vec<Vec<Step3ItemState>>,
    redo_stack: &mut Vec<Vec<Step3ItemState>>,
) {
    undo_stack.push(items.to_vec());
    if undo_stack.len() > STEP3_HISTORY_LIMIT {
        undo_stack.remove(0);
    }
    redo_stack.clear();
}

pub(super) fn render_rows(
    ui: &mut egui::Ui,
    prompt_eval: &crate::ui::step2::state_step2::PromptEvalContext,
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
    let mut prompt_requests: Vec<PromptActionRequest> = Vec::new();
    let mut open_prompt_popup: Option<(String, String)> = None;

    for &idx in visible_indices {
        let item = &items[idx];
        let mut row_prompt_popup: Option<(String, String)> = None;
        let label_response = if item.is_parent {
            let child_count = blocks::count_children_in_block(items, idx);
            let collapsed = collapsed_blocks.contains(&item.block_id);
            let mut is_locked = locked_blocks.contains(&item.block_id);
            let mut row_response: Option<egui::Response> = None;
            ui.horizontal(|ui| {
                let lock_icon = if is_locked {
                    crate::ui::shared::typography_global::strong("🔒")
                        .color(crate::ui::shared::theme_global::warning())
                } else {
                    crate::ui::shared::typography_global::strong("🔓")
                        .color(crate::ui::shared::theme_global::text_disabled())
                };
                if ui
                    .small_button(lock_icon)
                    .on_hover_text(crate::ui::shared::tooltip_global::STEP3_LOCK_PARENT)
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
                let resp = ui.selectable_label(selected.contains(&idx), crate::ui::shared::typography_global::strong(title));
                row_response = Some(resp);
            });
            let resp = row_response.expect("row response should exist");
            resp.on_hover_text(crate::ui::shared::tooltip_global::STEP3_DRAG_PARENT)
        } else {
            let prompt_summary = prompt_popup_step3::evaluate_step3_item_prompt_summary(item, prompt_eval);
            ui.horizontal(|ui| {
                ui.add_space(25.0);
                let text = content_step3::format_step3_item(item);
                let row_text = content_step3::weidu_colored_widget_text(ui, &text);
                let resp = ui.selectable_label(selected.contains(&idx), row_text);
                if !prompt_summary.trim().is_empty() {
                    ui.add_space(6.0);
                    let prompt_text = crate::ui::shared::typography_global::strong("PROMPT")
                        .color(crate::ui::shared::theme_global::prompt_text())
                        .size(crate::ui::shared::typography_global::SIZE_PILL_TEXT);
                    let prompt_response = ui
                        .add(
                            egui::Button::new(prompt_text)
                                .fill(crate::ui::shared::theme_global::prompt_fill())
                                .stroke(egui::Stroke::new(
                                    crate::ui::shared::layout_tokens_global::BORDER_THIN,
                                    crate::ui::shared::theme_global::prompt_stroke(),
                                ))
                                .corner_radius(egui::CornerRadius::same(7))
                                .min_size(egui::vec2(0.0, 18.0)),
                        )
                        .on_hover_text(crate::ui::shared::tooltip_global::SHOW_PARSED_PROMPTS);
                    if prompt_response.clicked() {
                        row_prompt_popup = Some(prompt_popup_step3::format_step3_prompt_popup(item, &prompt_summary));
                    }
                }
                resp
            })
            .inner
            .on_hover_text(crate::ui::shared::tooltip_global::STEP3_DRAG_ROW)
        };
        if row_prompt_popup.is_some() {
            open_prompt_popup = row_prompt_popup;
        }
        let drag_id = ui.make_persistent_id(("step3_drag_row", tab_id, idx));
        let drag_response = ui.interact(label_response.rect, drag_id, egui::Sense::click_and_drag());
        if items[idx].is_parent {
            drag_response.context_menu(|ui| {
                if ui.button("Clone Parent (empty split target)").clicked() {
                    push_undo_snapshot(items, undo_stack, redo_stack);
                    blocks::clone_parent_empty_block(items, idx, clone_seq);
                    ui.close_menu();
                }
            });
        } else {
            let tp_file = items[idx].tp_file.clone();
            let component_id = items[idx].component_id.clone();
            let component_label = items[idx].component_label.clone();
            let mod_name = items[idx].mod_name.clone();
            drag_response.context_menu(|ui| {
                if ui.button("Uncheck In Step 2").clicked() {
                    uncheck_requests.push((tp_file.clone(), component_id.clone()));
                    ui.close_menu();
                }
                if ui.button("Set @wlb-inputs...").clicked() {
                    prompt_requests.push(PromptActionRequest::SetWlb {
                        tp_file: tp_file.clone(),
                        component_id: component_id.clone(),
                        component_label: component_label.clone(),
                        mod_name: mod_name.clone(),
                    });
                    ui.close_menu();
                }
                if ui.button("Edit Prompt JSON...").clicked() {
                    prompt_requests.push(PromptActionRequest::EditJson {
                        tp_file: tp_file.clone(),
                        component_id: component_id.clone(),
                        component_label: component_label.clone(),
                        mod_name: mod_name.clone(),
                    });
                    ui.close_menu();
                }
                if ui.button("Clear Prompt Data").clicked() {
                    prompt_requests.push(PromptActionRequest::Clear {
                        tp_file: tp_file.clone(),
                        component_id: component_id.clone(),
                    });
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
            service_step3::apply_row_selection(selected, anchor, items, visible_indices, idx, modifiers);
        }
        if drag_response.drag_started() {
            if locked_blocks.contains(&items[idx].block_id) {
                *drag_from = None;
                drag_indices.clear();
                continue;
            }
            push_undo_snapshot(items, undo_stack, redo_stack);
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
        prompt_requests,
        open_prompt_popup,
    }
}
}












}
