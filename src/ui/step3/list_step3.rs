// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

pub(crate) mod list {
use eframe::egui;

use crate::ui::state::WizardState;
use crate::ui::step3::action_step3::Step3Action;
use crate::ui::step3::blocks;
use crate::ui::step3::state_step3;


pub(crate) fn render(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    action: &mut Option<Step3Action>,
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
                    let compat_issues = state.compat.issues.clone();
                    let (pending_unchecks, pending_prompt_actions, open_prompt_popup, open_compat_modal, revalidate_after_drag) = {
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
                        let mut row_ctx = rows::RowRenderContext {
                            prompt_eval: &prompt_eval,
                            compat_issues: &compat_issues,
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
                        let row_outcome = rows::render_rows(ui, &mut row_ctx);
                        let mut visible_rows = row_outcome.visible_rows;
                        let mut pointer_ctx = crate::ui::step3::service_step3::drag_ops::DragPointerContext {
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
                        crate::ui::step3::service_step3::drag_ops::draw_insert_marker(ui, items, drag_from, *drag_over, &visible_rows);
                        let mut reorder_ctx = crate::ui::step3::service_step3::drag_ops::LiveReorderContext {
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
                        let mut finalize_ctx = crate::ui::step3::service_step3::drag_ops::DragFinalizeContext {
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
                        let finalized_drag = crate::ui::step3::service_step3::drag_ops::finalize_on_release(
                            ui,
                            &mut finalize_ctx,
                        );
                        visible_rows.clear();
                        (
                            row_outcome.uncheck_requests,
                            row_outcome.prompt_requests,
                            row_outcome.open_prompt_popup,
                            row_outcome.open_compat_modal,
                            finalized_drag,
                        )
                    };
                    let should_revalidate =
                        revalidate_after_drag || !pending_unchecks.is_empty();
                    if should_revalidate {
                        *action = Some(Step3Action::Revalidate);
                    }
                    if open_compat_modal {
                        state.step3.compat_modal_open = true;
                    }
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

use crate::ui::state::CompatIssueDisplay;
use crate::ui::state::Step3ItemState;
use crate::ui::step2::tree_step2::step2_tree::render_helpers::compat_colors;
use crate::ui::step3::blocks;
use crate::ui::step3::compat_modal_step3::compat_model::normalize_mod_key;
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
    pub open_compat_modal: bool,
}

pub(super) struct RowRenderContext<'a> {
    pub prompt_eval: &'a crate::ui::step2::state_step2::PromptEvalContext,
    pub compat_issues: &'a [CompatIssueDisplay],
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

fn issue_to_compat_kind(issue: &CompatIssueDisplay) -> &'static str {
    if issue.code.eq_ignore_ascii_case("REQ_MISSING") {
        "missing_dep"
    } else if issue.code.eq_ignore_ascii_case("GAME_MISMATCH") {
        "game_mismatch"
    } else if issue.code.eq_ignore_ascii_case("CONDITIONAL") {
        "conditional"
    } else if issue.code.eq_ignore_ascii_case("INCLUDED") {
        "included"
    } else if issue.code.eq_ignore_ascii_case("ORDER_BLOCK") {
        "order_block"
    } else if issue.is_blocking {
        "conflict"
    } else {
        "warning"
    }
}

fn row_issue<'a>(item: &Step3ItemState, issues: &'a [CompatIssueDisplay]) -> Option<&'a CompatIssueDisplay> {
    let item_tp_key = normalize_mod_key(item.tp_file.as_str());
    let item_name_key = normalize_mod_key(item.mod_name.as_str());
    let comp_id = item.component_id.parse::<u32>().ok();

    issues.iter().find(|issue| {
        let affected_key = normalize_mod_key(issue.affected_mod.as_str());
        (item_tp_key == affected_key || item_name_key == affected_key)
            && match (issue.affected_component, comp_id) {
                (Some(a), Some(b)) => a == b,
                (None, _) => true,
                _ => false,
            }
    })
}

pub(super) fn render_rows(ui: &mut egui::Ui, ctx: &mut RowRenderContext<'_>) -> RowRenderOutcome {
    let prompt_eval = ctx.prompt_eval;
    let compat_issues = ctx.compat_issues;
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
    let mut open_compat_modal = false;
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
            let compat_issue = row_issue(item, compat_issues);
            ui.horizontal(|ui| {
                ui.add_space(25.0);
                let text = content_step3::format_step3_item(item);
                let row_text = content_step3::weidu_colored_widget_text(ui, &text);
                let resp = ui.selectable_label(selected.contains(&idx), row_text);
                if let Some(issue) = compat_issue
                    && let Some((pill_text_color, pill_bg, pill_label)) =
                        compat_colors(Some(issue_to_compat_kind(issue)))
                {
                    ui.add_space(6.0);
                    let pill_text = crate::ui::shared::typography_global::strong(pill_label)
                        .color(pill_text_color)
                        .size(crate::ui::shared::typography_global::SIZE_PILL_TEXT);
                    let pill_response = ui
                        .add(
                            egui::Button::new(pill_text)
                                .fill(pill_bg)
                                .stroke(egui::Stroke::new(
                                    crate::ui::shared::layout_tokens_global::BORDER_THIN,
                                    pill_bg,
                                ))
                                .corner_radius(egui::CornerRadius::same(7))
                                .min_size(egui::vec2(0.0, 18.0)),
                        )
                        .on_hover_text(issue.reason.as_str());
                    if pill_response.clicked() {
                        selected.clear();
                        selected.push(idx);
                        open_compat_modal = true;
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
        open_compat_modal,
    }
}
}












}
