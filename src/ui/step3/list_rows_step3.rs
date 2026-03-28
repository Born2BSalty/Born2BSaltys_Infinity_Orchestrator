// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;
use std::collections::HashMap;

use crate::ui::step2::compat_types_step2::CompatIssueDisplay;
use crate::ui::state::Step3ItemState;
use crate::ui::compat_step3_rules::Step3CompatMarker;
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

pub(crate) struct RowRenderOutcome {
    pub visible_rows: Vec<(usize, egui::Rect)>,
    pub uncheck_requests: Vec<(String, String)>,
    pub prompt_requests: Vec<PromptActionRequest>,
    pub open_prompt_popup: Option<(String, String)>,
    pub open_compat_popup: Option<(String, String, String, CompatIssueDisplay)>,
}

pub(crate) struct RowRenderContext<'a> {
    pub prompt_eval: &'a crate::ui::step2::state_step2::PromptEvalContext,
    pub compat_markers: &'a HashMap<String, Step3CompatMarker>,
    pub tab_id: &'a str,
    pub visible_indices: &'a [usize],
    pub jump_to_selected_requested: &'a mut bool,
    pub items: &'a mut Vec<Step3ItemState>,
    pub selected: &'a mut Vec<usize>,
    pub drag_from: &'a mut Option<usize>,
    pub drag_over: &'a mut Option<usize>,
    pub drag_indices: &'a mut Vec<usize>,
    pub anchor: &'a mut Option<usize>,
    pub drag_grab_offset: &'a mut f32,
    pub drag_grab_pos_in_block: &'a mut usize,
    pub drag_row_h: &'a mut f32,
    pub last_insert_at: &'a mut Option<usize>,
    pub collapsed_blocks: &'a mut Vec<String>,
    pub clone_seq: &'a mut usize,
    pub locked_blocks: &'a mut Vec<String>,
    pub undo_stack: &'a mut Vec<Vec<Step3ItemState>>,
    pub redo_stack: &'a mut Vec<Vec<Step3ItemState>>,
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

fn single_child_main_parent_block_indices(items: &[Step3ItemState], idx: usize) -> Option<Vec<usize>> {
    let item = items.get(idx)?;
    if item.is_parent {
        return None;
    }
    let parent_idx = items
        .iter()
        .position(|it| it.is_parent && it.block_id == item.block_id)?;
    let parent = items.get(parent_idx)?;
    if parent.parent_placeholder || blocks::count_children_in_block(items, parent_idx) != 1 {
        return None;
    }
    Some(blocks::block_indices(items, parent_idx))
}

fn selected_full_main_parent_block_indices(
    items: &[Step3ItemState],
    selected: &[usize],
    idx: usize,
) -> Option<Vec<usize>> {
    let item = items.get(idx)?;
    if item.is_parent || !selected.contains(&idx) {
        return None;
    }
    let parent_idx = items
        .iter()
        .position(|it| it.is_parent && it.block_id == item.block_id)?;
    let parent = items.get(parent_idx)?;
    if parent.parent_placeholder {
        return None;
    }

    let child_indices: Vec<usize> = items
        .iter()
        .enumerate()
        .filter_map(|(i, it)| (!it.is_parent && it.block_id == item.block_id).then_some(i))
        .collect();
    if child_indices.is_empty() {
        return None;
    }

    let selected_non_parent: Vec<usize> = selected
        .iter()
        .copied()
        .filter(|i| items.get(*i).is_some_and(|it| !it.is_parent))
        .collect();
    if selected_non_parent.len() != child_indices.len() {
        return None;
    }
    if !child_indices.iter().all(|child_idx| selected_non_parent.contains(child_idx)) {
        return None;
    }

    Some(blocks::block_indices(items, parent_idx))
}

pub(crate) fn render_rows(
    ui: &mut egui::Ui,
    ctx: &mut RowRenderContext<'_>,
) -> RowRenderOutcome {
    let prompt_eval = ctx.prompt_eval;
    let compat_markers = ctx.compat_markers;
    let tab_id = ctx.tab_id;
    let visible_indices = ctx.visible_indices;
    let jump_to_selected_requested = &mut *ctx.jump_to_selected_requested;
    let items = &mut *ctx.items;
    let selected = &mut *ctx.selected;
    let drag_from = &mut *ctx.drag_from;
    let drag_over = &mut *ctx.drag_over;
    let drag_indices = &mut *ctx.drag_indices;
    let anchor = &mut *ctx.anchor;
    let drag_grab_offset = &mut *ctx.drag_grab_offset;
    let drag_grab_pos_in_block = &mut *ctx.drag_grab_pos_in_block;
    let drag_row_h = &mut *ctx.drag_row_h;
    let last_insert_at = &mut *ctx.last_insert_at;
    let collapsed_blocks = &mut *ctx.collapsed_blocks;
    let clone_seq = &mut *ctx.clone_seq;
    let locked_blocks = &mut *ctx.locked_blocks;
    let undo_stack = &mut *ctx.undo_stack;
    let redo_stack = &mut *ctx.redo_stack;
    let mut visible_rows: Vec<(usize, egui::Rect)> = Vec::with_capacity(visible_indices.len());
    let mut uncheck_requests: Vec<(String, String)> = Vec::new();
    let mut prompt_requests: Vec<PromptActionRequest> = Vec::new();
    let mut open_prompt_popup: Option<(String, String)> = None;
    let mut open_compat_popup: Option<(String, String, String, CompatIssueDisplay)> = None;

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
                        locked_blocks.retain(|value| value != &item.block_id);
                        is_locked = false;
                    } else {
                        locked_blocks.push(item.block_id.clone());
                        is_locked = true;
                    }
                }
                let symbol = if collapsed { "+" } else { "-" };
                if ui.small_button(symbol).clicked() {
                    if collapsed {
                        collapsed_blocks.retain(|value| value != &item.block_id);
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
                let resp = ui.selectable_label(
                    selected.contains(&idx),
                    crate::ui::shared::typography_global::strong(title),
                );
                row_response = Some(resp);
            });
            let resp = row_response.expect("row response should exist");
            resp.on_hover_text(crate::ui::shared::tooltip_global::STEP3_DRAG_PARENT)
        } else {
            let prompt_summary =
                prompt_popup_step3::evaluate_step3_item_prompt_summary(item, prompt_eval);
            let compat_marker =
                compat_markers.get(&crate::ui::compat_step3_rules::marker_key(item));
            ui.horizontal(|ui| {
                ui.add_space(25.0);
                let text = content_step3::format_step3_item(item);
                let row_text = content_step3::weidu_colored_widget_text(ui, &text);
                let resp = ui.selectable_label(selected.contains(&idx), row_text);
                if let Some(marker) = compat_marker
                    && let Some((pill_text_color, pill_bg, pill_label)) =
                        crate::ui::step2::tree_compat_display_step2::compat_colors(Some(&marker.kind))
                {
                    ui.add_space(6.0);
                    let pill_text = crate::ui::shared::typography_global::strong(pill_label)
                        .color(pill_text_color)
                        .size(crate::ui::shared::typography_global::SIZE_PILL_TEXT);
                    let pill_response = ui.add(
                        egui::Button::new(pill_text)
                            .fill(pill_bg)
                            .stroke(egui::Stroke::new(
                                crate::ui::shared::layout_tokens_global::BORDER_THIN,
                                pill_bg,
                            ))
                            .corner_radius(egui::CornerRadius::same(7))
                            .min_size(egui::vec2(0.0, 18.0)),
                    );
                    let pill_response = if let Some(message) = marker.message.as_deref() {
                        pill_response.on_hover_text(message)
                    } else {
                        pill_response
                    };
                    if pill_response.clicked() {
                        open_compat_popup = Some((
                            item.tp_file.clone(),
                            item.component_id.clone(),
                            item.raw_line.clone(),
                            crate::ui::compat_step3_rules::marker_issue(marker),
                        ));
                    }
                }
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
                        row_prompt_popup = Some(
                            prompt_popup_step3::format_step3_prompt_popup(item, &prompt_summary),
                        );
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
            let modifiers = ui.input(|input| input.modifiers);
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
            } else if let Some(block_indices) =
                selected_full_main_parent_block_indices(items, selected, idx)
            {
                *drag_indices = block_indices;
            } else if selected.contains(&idx) && selected.len() > 1 {
                *drag_indices = selected.clone();
            } else if let Some(block_indices) = single_child_main_parent_block_indices(items, idx) {
                selected.clear();
                selected.push(idx);
                *drag_indices = block_indices;
            } else {
                selected.clear();
                selected.push(idx);
                drag_indices.clear();
                drag_indices.push(idx);
            }
            let mut sorted = drag_indices.clone();
            sorted.sort_unstable();
            sorted.dedup();
            *drag_grab_pos_in_block = sorted.iter().position(|value| *value == idx).unwrap_or(0);
            if let Some(pointer) = ui.input(|input| input.pointer.interact_pos())
                && let Some((_, grabbed_rect)) = visible_rows.iter().find(|(row_idx, _)| *row_idx == idx)
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
        open_compat_popup,
    }
}
