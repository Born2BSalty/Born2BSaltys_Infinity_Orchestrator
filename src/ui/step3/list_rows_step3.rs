// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;
use std::collections::HashMap;

use crate::app::compat_issue::CompatIssue;
use crate::app::compat_step3_rules::Step3CompatMarker;
use crate::app::prompt_eval_summary_step3;
use crate::app::prompt_popup_text::format_step3_prompt_popup;
use crate::app::state::Step3ItemState;
use crate::app::step3_history;
use crate::app::step3_prompt_edit::PromptActionRequest;
use crate::parser::PromptEvalContext;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BIO_PILL_HEIGHT_PX, REDESIGN_BIO_PILL_RADIUS_U8, REDESIGN_BIO_ROW_GAP_PX,
    REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_prompt_fill, redesign_prompt_stroke,
    redesign_prompt_text, redesign_text_disabled, redesign_warning,
};
use crate::ui::step3::block_selection_step3::{
    selected_full_main_parent_block_indices, single_child_main_parent_block_indices,
};
use crate::ui::step3::blocks;
use crate::ui::step3::format_step3;
use crate::ui::step3::service_step3;

pub(crate) struct RowRenderOutcome {
    pub visible_rows: Vec<(usize, egui::Rect)>,
    pub uncheck_requests: Vec<(String, String)>,
    pub prompt_requests: Vec<PromptActionRequest>,
    pub open_prompt_popup: Option<(String, String)>,
    pub open_compat_popup: Option<CompatPopupRequest>,
}

type CompatPopupRequest = (String, String, String, CompatIssue);

pub(crate) struct RowRenderContext<'a> {
    pub prompt_eval: &'a PromptEvalContext,
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
    pub palette: ThemePalette,
}

pub(crate) fn render_rows(ui: &mut egui::Ui, ctx: &mut RowRenderContext<'_>) -> RowRenderOutcome {
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
    let mut outcome = RowRenderOutcome {
        visible_rows: Vec::with_capacity(visible_indices.len()),
        uncheck_requests: Vec::new(),
        prompt_requests: Vec::new(),
        open_prompt_popup: None,
        open_compat_popup: None,
    };

    for &idx in visible_indices {
        let row_label = render_row_label(
            ui,
            idx,
            &mut RowLabelContext {
                prompt_eval,
                compat_markers,
                tab_id,
                items,
                selected,
                collapsed_blocks,
                locked_blocks,
                palette: ctx.palette,
            },
            &mut outcome.open_compat_popup,
        );
        let label_response = row_label.response;
        if let Some(row_prompt_popup) = row_label.prompt_popup {
            outcome.open_prompt_popup = Some(row_prompt_popup);
        }
        let drag_id = ui.make_persistent_id(("step3_drag_row", tab_id, idx));
        let drag_response =
            ui.interact(label_response.rect, drag_id, egui::Sense::click_and_drag());
        render_row_context_menu(
            &drag_response,
            idx,
            &mut RowMenuContext {
                items,
                undo_stack,
                redo_stack,
                clone_seq,
                uncheck_requests: &mut outcome.uncheck_requests,
                prompt_requests: &mut outcome.prompt_requests,
            },
        );
        outcome.visible_rows.push((idx, label_response.rect));
        handle_row_selection(
            ui,
            idx,
            &label_response,
            &drag_response,
            &mut RowSelectionContext {
                jump_to_selected_requested,
                selected,
                anchor,
                items,
                visible_indices,
            },
        );
        if drag_response.drag_started() {
            handle_drag_start(
                ui,
                idx,
                &outcome.visible_rows,
                &mut DragStartContext {
                    items,
                    selected,
                    drag_from,
                    drag_over,
                    drag_indices,
                    drag_grab_offset,
                    drag_grab_pos_in_block,
                    drag_row_h,
                    last_insert_at,
                    locked_blocks,
                    undo_stack,
                    redo_stack,
                },
            );
        }
    }

    outcome
}

struct RowLabelContext<'a> {
    prompt_eval: &'a PromptEvalContext,
    compat_markers: &'a HashMap<String, Step3CompatMarker>,
    tab_id: &'a str,
    items: &'a [Step3ItemState],
    selected: &'a [usize],
    collapsed_blocks: &'a mut Vec<String>,
    locked_blocks: &'a mut Vec<String>,
    palette: ThemePalette,
}

struct RowLabelOutcome {
    response: egui::Response,
    prompt_popup: Option<(String, String)>,
}

fn render_row_label(
    ui: &mut egui::Ui,
    idx: usize,
    ctx: &mut RowLabelContext<'_>,
    open_compat_popup: &mut Option<CompatPopupRequest>,
) -> RowLabelOutcome {
    if ctx.items[idx].is_parent {
        RowLabelOutcome {
            response: render_parent_label(ui, idx, ctx),
            prompt_popup: None,
        }
    } else {
        render_child_label(ui, idx, ctx, open_compat_popup)
    }
}

fn render_parent_label(
    ui: &mut egui::Ui,
    idx: usize,
    ctx: &mut RowLabelContext<'_>,
) -> egui::Response {
    let item = &ctx.items[idx];
    let child_count = blocks::count_children_in_block(ctx.items, idx);
    let collapsed = ctx.collapsed_blocks.contains(&item.block_id);
    let mut is_locked = ctx.locked_blocks.contains(&item.block_id);
    let mut row_response: Option<egui::Response> = None;
    ui.horizontal(|ui| {
        render_parent_lock_button(ui, item, ctx.locked_blocks, ctx.palette, &mut is_locked);
        render_parent_collapse_button(ui, item, ctx.tab_id, ctx.collapsed_blocks, collapsed);
        let title = parent_row_title(item, child_count, is_locked);
        let resp = ui.selectable_label(
            ctx.selected.contains(&idx),
            crate::ui::shared::typography_global::strong(title),
        );
        row_response = Some(resp);
    });
    let resp = row_response.expect("row response should exist");
    resp.on_hover_text(crate::ui::shared::tooltip_global::STEP3_DRAG_PARENT)
}

fn render_parent_lock_button(
    ui: &mut egui::Ui,
    item: &Step3ItemState,
    locked_blocks: &mut Vec<String>,
    palette: ThemePalette,
    is_locked: &mut bool,
) {
    let lock_icon = if *is_locked {
        crate::ui::shared::typography_global::strong("🔒").color(redesign_warning(palette))
    } else {
        crate::ui::shared::typography_global::strong("🔓").color(redesign_text_disabled(palette))
    };
    if ui
        .small_button(lock_icon)
        .on_hover_text(crate::ui::shared::tooltip_global::STEP3_LOCK_PARENT)
        .clicked()
    {
        if *is_locked {
            locked_blocks.retain(|value| value != &item.block_id);
            *is_locked = false;
        } else {
            locked_blocks.push(item.block_id.clone());
            *is_locked = true;
        }
    }
}

fn render_parent_collapse_button(
    ui: &mut egui::Ui,
    item: &Step3ItemState,
    tab_id: &str,
    collapsed_blocks: &mut Vec<String>,
    collapsed: bool,
) {
    let mut collapse_state = egui::collapsing_header::CollapsingState::load_with_default_open(
        ui.ctx(),
        egui::Id::new(("step3_parent_toggle", tab_id, &item.block_id)),
        !collapsed,
    );
    collapse_state.set_open(!collapsed);
    collapse_state.show_toggle_button(ui, egui::collapsing_header::paint_default_icon);
    let now_collapsed = !collapse_state.is_open();
    if now_collapsed != collapsed {
        if now_collapsed {
            collapsed_blocks.push(item.block_id.clone());
        } else {
            collapsed_blocks.retain(|value| value != &item.block_id);
        }
    }
}

fn parent_row_title(item: &Step3ItemState, child_count: usize, is_locked: bool) -> String {
    let title = if item.parent_placeholder {
        format!("{} (split target)", item.mod_name)
    } else {
        format!("{} ({child_count})", item.mod_name)
    };
    if is_locked {
        format!("{title} [locked]")
    } else {
        title
    }
}

fn render_child_label(
    ui: &mut egui::Ui,
    idx: usize,
    ctx: &RowLabelContext<'_>,
    open_compat_popup: &mut Option<CompatPopupRequest>,
) -> RowLabelOutcome {
    let item = &ctx.items[idx];
    let prompt_summary =
        prompt_eval_summary_step3::evaluate_step3_item_prompt_summary(item, ctx.prompt_eval);
    let compat_marker = ctx
        .compat_markers
        .get(&crate::app::compat_step3_rules::marker_key(item));
    let mut row_prompt_popup: Option<(String, String)> = None;
    let response = ui
        .horizontal(|ui| {
            ui.add_space(25.0);
            let text = format_step3::format_step3_item(item);
            let row_text = format_step3::weidu_colored_widget_text(ui, &text, ctx.palette);
            let resp = ui.selectable_label(ctx.selected.contains(&idx), row_text);
            render_compat_pill(ui, item, compat_marker, ctx.palette, open_compat_popup);
            render_prompt_pill(
                ui,
                item,
                &prompt_summary,
                ctx.palette,
                &mut row_prompt_popup,
            );
            resp
        })
        .inner
        .on_hover_text(crate::ui::shared::tooltip_global::STEP3_DRAG_ROW);
    RowLabelOutcome {
        response,
        prompt_popup: row_prompt_popup,
    }
}

fn render_compat_pill(
    ui: &mut egui::Ui,
    item: &Step3ItemState,
    compat_marker: Option<&Step3CompatMarker>,
    palette: ThemePalette,
    open_compat_popup: &mut Option<CompatPopupRequest>,
) {
    if let Some(marker) = compat_marker
        && let Some((pill_text_color, pill_bg, pill_label)) =
            crate::ui::step2::tree_compat_display_step2::compat_colors(Some(&marker.kind), palette)
    {
        ui.add_space(REDESIGN_BIO_ROW_GAP_PX);
        let pill_text = crate::ui::shared::typography_global::strong(pill_label)
            .color(pill_text_color)
            .size(crate::ui::shared::typography_global::SIZE_PILL_TEXT);
        let pill_response = ui.add(
            egui::Button::new(pill_text)
                .fill(pill_bg)
                .stroke(egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, pill_bg))
                .corner_radius(egui::CornerRadius::same(REDESIGN_BIO_PILL_RADIUS_U8))
                .min_size(egui::vec2(0.0, REDESIGN_BIO_PILL_HEIGHT_PX)),
        );
        let pill_response = if let Some(message) = marker.message.as_deref() {
            pill_response.on_hover_text(message)
        } else {
            pill_response
        };
        if pill_response.clicked() {
            *open_compat_popup = Some((
                item.tp_file.clone(),
                item.component_id.clone(),
                item.raw_line.clone(),
                crate::app::compat_step3_rules::marker_issue(marker),
            ));
        }
    }
}

fn render_prompt_pill(
    ui: &mut egui::Ui,
    item: &Step3ItemState,
    prompt_summary: &str,
    palette: ThemePalette,
    row_prompt_popup: &mut Option<(String, String)>,
) {
    if prompt_summary.trim().is_empty() {
        return;
    }
    ui.add_space(REDESIGN_BIO_ROW_GAP_PX);
    let prompt_text = crate::ui::shared::typography_global::strong("PROMPT")
        .color(redesign_prompt_text(palette))
        .size(crate::ui::shared::typography_global::SIZE_PILL_TEXT);
    let prompt_response = ui
        .add(
            egui::Button::new(prompt_text)
                .fill(redesign_prompt_fill(palette))
                .stroke(egui::Stroke::new(
                    REDESIGN_BORDER_WIDTH_PX,
                    redesign_prompt_stroke(palette),
                ))
                .corner_radius(egui::CornerRadius::same(REDESIGN_BIO_PILL_RADIUS_U8))
                .min_size(egui::vec2(0.0, REDESIGN_BIO_PILL_HEIGHT_PX)),
        )
        .on_hover_text(crate::ui::shared::tooltip_global::SHOW_PARSED_PROMPTS);
    if prompt_response.clicked() {
        *row_prompt_popup = Some(format_step3_prompt_popup(item, prompt_summary));
    }
}

struct RowMenuContext<'a> {
    items: &'a mut Vec<Step3ItemState>,
    undo_stack: &'a mut Vec<Vec<Step3ItemState>>,
    redo_stack: &'a mut Vec<Vec<Step3ItemState>>,
    clone_seq: &'a mut usize,
    uncheck_requests: &'a mut Vec<(String, String)>,
    prompt_requests: &'a mut Vec<PromptActionRequest>,
}

struct RowSelectionContext<'a> {
    jump_to_selected_requested: &'a mut bool,
    selected: &'a mut Vec<usize>,
    anchor: &'a mut Option<usize>,
    items: &'a mut Vec<Step3ItemState>,
    visible_indices: &'a [usize],
}

fn handle_row_selection(
    ui: &egui::Ui,
    idx: usize,
    label_response: &egui::Response,
    drag_response: &egui::Response,
    ctx: &mut RowSelectionContext<'_>,
) {
    if *ctx.jump_to_selected_requested && ctx.selected.contains(&idx) {
        ui.scroll_to_rect(label_response.rect, Some(egui::Align::Center));
        *ctx.jump_to_selected_requested = false;
    }
    if label_response.clicked() || drag_response.clicked() {
        let modifiers = ui.input(|input| input.modifiers);
        service_step3::apply_row_selection(
            ctx.selected,
            ctx.anchor,
            ctx.items,
            ctx.visible_indices,
            idx,
            modifiers,
        );
    }
}

fn render_row_context_menu(response: &egui::Response, idx: usize, ctx: &mut RowMenuContext<'_>) {
    if ctx.items[idx].is_parent {
        response.context_menu(|ui| {
            if ui.button("Clone Parent (empty split target)").clicked() {
                step3_history::push_undo_snapshot(ctx.items, ctx.undo_stack, ctx.redo_stack);
                blocks::clone_parent_empty_block(ctx.items, idx, ctx.clone_seq);
                ui.close_menu();
            }
        });
    } else {
        render_child_context_menu(response, idx, ctx);
    }
}

fn render_child_context_menu(response: &egui::Response, idx: usize, ctx: &mut RowMenuContext<'_>) {
    let tp_file = ctx.items[idx].tp_file.clone();
    let component_id = ctx.items[idx].component_id.clone();
    let component_label = ctx.items[idx].component_label.clone();
    let mod_name = ctx.items[idx].mod_name.clone();
    response.context_menu(|ui| {
        if ui.button("Uncheck In Step 2").clicked() {
            ctx.uncheck_requests
                .push((tp_file.clone(), component_id.clone()));
            ui.close_menu();
        }
        if ui.button("Set @wlb-inputs...").clicked() {
            ctx.prompt_requests.push(PromptActionRequest::SetWlb {
                tp_file: tp_file.clone(),
                component_id: component_id.clone(),
                component_label: component_label.clone(),
                mod_name: mod_name.clone(),
            });
            ui.close_menu();
        }
        if ui.button("Edit Prompt JSON...").clicked() {
            ctx.prompt_requests.push(PromptActionRequest::EditJson {
                tp_file: tp_file.clone(),
                component_id: component_id.clone(),
                component_label: component_label.clone(),
                mod_name: mod_name.clone(),
            });
            ui.close_menu();
        }
        if ui.button("Clear Prompt Data").clicked() {
            ctx.prompt_requests.push(PromptActionRequest::Clear {
                tp_file: tp_file.clone(),
                component_id: component_id.clone(),
            });
            ui.close_menu();
        }
    });
}

struct DragStartContext<'a> {
    items: &'a mut Vec<Step3ItemState>,
    selected: &'a mut Vec<usize>,
    drag_from: &'a mut Option<usize>,
    drag_over: &'a mut Option<usize>,
    drag_indices: &'a mut Vec<usize>,
    drag_grab_offset: &'a mut f32,
    drag_grab_pos_in_block: &'a mut usize,
    drag_row_h: &'a mut f32,
    last_insert_at: &'a mut Option<usize>,
    locked_blocks: &'a [String],
    undo_stack: &'a mut Vec<Vec<Step3ItemState>>,
    redo_stack: &'a mut Vec<Vec<Step3ItemState>>,
}

fn handle_drag_start(
    ui: &egui::Ui,
    idx: usize,
    visible_rows: &[(usize, egui::Rect)],
    ctx: &mut DragStartContext<'_>,
) {
    if ctx.locked_blocks.contains(&ctx.items[idx].block_id) {
        *ctx.drag_from = None;
        ctx.drag_indices.clear();
        return;
    }
    step3_history::push_undo_snapshot(ctx.items, ctx.undo_stack, ctx.redo_stack);
    *ctx.drag_from = Some(idx);
    set_drag_indices(idx, ctx);
    let mut sorted = ctx.drag_indices.clone();
    sorted.sort_unstable();
    sorted.dedup();
    *ctx.drag_grab_pos_in_block = sorted.iter().position(|value| *value == idx).unwrap_or(0);
    if let Some(pointer) = ui.input(|input| input.pointer.interact_pos())
        && let Some((_, grabbed_rect)) = visible_rows.iter().find(|(row_idx, _)| *row_idx == idx)
    {
        *ctx.drag_grab_offset = pointer.y - grabbed_rect.top();
        let row_pitch = grabbed_rect.height() + ui.spacing().item_spacing.y.max(0.0);
        *ctx.drag_row_h = row_pitch.max(1.0);
    }
    *ctx.last_insert_at = None;
    *ctx.drag_over = Some(idx + 1);
}

fn set_drag_indices(idx: usize, ctx: &mut DragStartContext<'_>) {
    if let Some(block_indices) =
        selected_full_main_parent_block_indices(ctx.items, ctx.selected, idx)
    {
        *ctx.drag_indices = block_indices;
    } else if ctx.items[idx].is_parent {
        *ctx.drag_indices = blocks::block_indices(ctx.items, idx);
    } else if ctx.selected.contains(&idx) && ctx.selected.len() > 1 {
        ctx.drag_indices.clone_from(ctx.selected);
    } else if let Some(block_indices) = single_child_main_parent_block_indices(ctx.items, idx) {
        ctx.selected.clear();
        ctx.selected.push(idx);
        *ctx.drag_indices = block_indices;
    } else {
        ctx.selected.clear();
        ctx.selected.push(idx);
        ctx.drag_indices.clear();
        ctx.drag_indices.push(idx);
    }
}
