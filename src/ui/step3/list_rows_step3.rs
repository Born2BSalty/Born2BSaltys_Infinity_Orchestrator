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
use crate::parser::prompt_eval_expr::PromptEvalContext;
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
    pub open_compat_popup: Option<(String, String, String, CompatIssue)>,
}

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
}

pub(crate) fn render_rows(ui: &mut egui::Ui, ctx: &mut RowRenderContext<'_>) -> RowRenderOutcome {
    let mut out = RowRenderAccumulator::new(ctx.visible_indices.len());
    for visible_pos in 0..ctx.visible_indices.len() {
        render_row(ui, ctx, ctx.visible_indices[visible_pos], &mut out);
    }
    out.finish()
}

struct RowRenderAccumulator {
    visible_rows: Vec<(usize, egui::Rect)>,
    uncheck_requests: Vec<(String, String)>,
    prompt_requests: Vec<PromptActionRequest>,
    open_prompt_popup: Option<(String, String)>,
    open_compat_popup: Option<(String, String, String, CompatIssue)>,
}

impl RowRenderAccumulator {
    fn new(row_capacity: usize) -> Self {
        Self {
            visible_rows: Vec::with_capacity(row_capacity),
            uncheck_requests: Vec::new(),
            prompt_requests: Vec::new(),
            open_prompt_popup: None,
            open_compat_popup: None,
        }
    }

    fn finish(self) -> RowRenderOutcome {
        let Self {
            visible_rows,
            uncheck_requests,
            prompt_requests,
            open_prompt_popup,
            open_compat_popup,
        } = self;
        RowRenderOutcome {
            visible_rows,
            uncheck_requests,
            prompt_requests,
            open_prompt_popup,
            open_compat_popup,
        }
    }
}

fn render_row(
    ui: &mut egui::Ui,
    ctx: &mut RowRenderContext<'_>,
    idx: usize,
    out: &mut RowRenderAccumulator,
) {
    let label_response = render_row_label(ui, ctx, idx, out);
    let drag_id = ui.make_persistent_id(("step3_drag_row", ctx.tab_id, idx));
    let drag_response = ui.interact(label_response.rect, drag_id, egui::Sense::click_and_drag());
    render_row_context_menu(&drag_response, ctx, idx, out);
    out.visible_rows.push((idx, label_response.rect));
    handle_jump_to_selected(ui, ctx, idx, label_response.rect);
    handle_row_selection(ui, ctx, idx, &label_response, &drag_response);
    handle_drag_start(ui, ctx, idx, &drag_response, &out.visible_rows);
}

fn render_row_label(
    ui: &mut egui::Ui,
    ctx: &mut RowRenderContext<'_>,
    idx: usize,
    out: &mut RowRenderAccumulator,
) -> egui::Response {
    if ctx.items[idx].is_parent {
        render_parent_label(ui, ctx, idx)
    } else {
        render_child_label(ui, ctx, idx, out)
    }
}

fn render_parent_label(
    ui: &mut egui::Ui,
    ctx: &mut RowRenderContext<'_>,
    idx: usize,
) -> egui::Response {
    let child_count = blocks::count_children_in_block(ctx.items, idx);
    let block_id = ctx.items[idx].block_id.clone();
    let mod_name = ctx.items[idx].mod_name.clone();
    let parent_placeholder = ctx.items[idx].parent_placeholder;
    let collapsed = ctx.collapsed_blocks.contains(&block_id);
    let mut is_locked = ctx.locked_blocks.contains(&block_id);
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
            update_locked_blocks(ctx.locked_blocks, &block_id, &mut is_locked);
        }
        render_parent_toggle(ui, ctx.tab_id, ctx.collapsed_blocks, &block_id, collapsed);
        let title = parent_row_title(&mod_name, parent_placeholder, child_count, is_locked);
        row_response = Some(ui.selectable_label(
            ctx.selected.contains(&idx),
            crate::ui::shared::typography_global::strong(title),
        ));
    });
    let resp = row_response.expect("row response should exist");
    resp.on_hover_text(crate::ui::shared::tooltip_global::STEP3_DRAG_PARENT)
}

fn update_locked_blocks(locked_blocks: &mut Vec<String>, block_id: &str, is_locked: &mut bool) {
    if *is_locked {
        locked_blocks.retain(|value| value != block_id);
        *is_locked = false;
    } else {
        locked_blocks.push(block_id.to_string());
        *is_locked = true;
    }
}

fn render_parent_toggle(
    ui: &mut egui::Ui,
    tab_id: &str,
    collapsed_blocks: &mut Vec<String>,
    block_id: &str,
    collapsed: bool,
) {
    let mut collapse_state = egui::collapsing_header::CollapsingState::load_with_default_open(
        ui.ctx(),
        egui::Id::new(("step3_parent_toggle", tab_id, block_id)),
        !collapsed,
    );
    collapse_state.set_open(!collapsed);
    collapse_state.show_toggle_button(ui, egui::collapsing_header::paint_default_icon);
    let now_collapsed = !collapse_state.is_open();
    if now_collapsed == collapsed {
        return;
    }
    if now_collapsed {
        collapsed_blocks.push(block_id.to_string());
    } else {
        collapsed_blocks.retain(|value| value != block_id);
    }
}

fn parent_row_title(
    mod_name: &str,
    parent_placeholder: bool,
    child_count: usize,
    is_locked: bool,
) -> String {
    let title = if parent_placeholder {
        format!("{mod_name} (split target)")
    } else {
        format!("{mod_name} ({child_count})")
    };
    if is_locked {
        format!("{title} [locked]")
    } else {
        title
    }
}

fn render_child_label(
    ui: &mut egui::Ui,
    ctx: &RowRenderContext<'_>,
    idx: usize,
    out: &mut RowRenderAccumulator,
) -> egui::Response {
    let item = &ctx.items[idx];
    let prompt_summary =
        prompt_eval_summary_step3::evaluate_step3_item_prompt_summary(item, ctx.prompt_eval);
    let compat_marker = ctx
        .compat_markers
        .get(&crate::app::compat_step3_rules::marker_key(item));
    ui.horizontal(|ui| {
        ui.add_space(25.0);
        let text = format_step3::format_step3_item(item);
        let row_text = format_step3::weidu_colored_widget_text(ui, &text);
        let resp = ui.selectable_label(ctx.selected.contains(&idx), row_text);
        if let Some(marker) = compat_marker {
            render_compat_marker_pill(ui, item, marker, out);
        }
        if !prompt_summary.trim().is_empty() {
            render_prompt_pill(ui, item, &prompt_summary, out);
        }
        resp
    })
    .inner
    .on_hover_text(crate::ui::shared::tooltip_global::STEP3_DRAG_ROW)
}

fn render_compat_marker_pill(
    ui: &mut egui::Ui,
    item: &Step3ItemState,
    marker: &Step3CompatMarker,
    out: &mut RowRenderAccumulator,
) {
    let Some((pill_text_color, pill_bg, pill_label)) =
        crate::ui::step2::tree_compat_display_step2::compat_colors(Some(&marker.kind))
    else {
        return;
    };
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
        out.open_compat_popup = Some((
            item.tp_file.clone(),
            item.component_id.clone(),
            item.raw_line.clone(),
            crate::app::compat_step3_rules::marker_issue(marker),
        ));
    }
}

fn render_prompt_pill(
    ui: &mut egui::Ui,
    item: &Step3ItemState,
    prompt_summary: &str,
    out: &mut RowRenderAccumulator,
) {
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
        out.open_prompt_popup = Some(format_step3_prompt_popup(item, prompt_summary));
    }
}

fn render_row_context_menu(
    drag_response: &egui::Response,
    ctx: &mut RowRenderContext<'_>,
    idx: usize,
    out: &mut RowRenderAccumulator,
) {
    if ctx.items[idx].is_parent {
        render_parent_context_menu(drag_response, ctx, idx);
    } else {
        render_child_context_menu(drag_response, ctx, idx, out);
    }
}

fn render_parent_context_menu(
    drag_response: &egui::Response,
    ctx: &mut RowRenderContext<'_>,
    idx: usize,
) {
    drag_response.context_menu(|ui| {
        if ui.button("Clone Parent (empty split target)").clicked() {
            step3_history::push_undo_snapshot(ctx.items, ctx.undo_stack, ctx.redo_stack);
            blocks::clone_parent_empty_block(ctx.items, idx, ctx.clone_seq);
            ui.close_menu();
        }
    });
}

fn render_child_context_menu(
    drag_response: &egui::Response,
    ctx: &RowRenderContext<'_>,
    idx: usize,
    out: &mut RowRenderAccumulator,
) {
    let tp_file = ctx.items[idx].tp_file.clone();
    let component_id = ctx.items[idx].component_id.clone();
    let component_label = ctx.items[idx].component_label.clone();
    let mod_name = ctx.items[idx].mod_name.clone();
    drag_response.context_menu(|ui| {
        if ui.button("Uncheck In Step 2").clicked() {
            out.uncheck_requests
                .push((tp_file.clone(), component_id.clone()));
            ui.close_menu();
        }
        if ui.button("Set @wlb-inputs...").clicked() {
            out.prompt_requests.push(PromptActionRequest::SetWlb {
                tp_file: tp_file.clone(),
                component_id: component_id.clone(),
                component_label: component_label.clone(),
                mod_name: mod_name.clone(),
            });
            ui.close_menu();
        }
        if ui.button("Edit Prompt JSON...").clicked() {
            out.prompt_requests.push(PromptActionRequest::EditJson {
                tp_file: tp_file.clone(),
                component_id: component_id.clone(),
                component_label: component_label.clone(),
                mod_name: mod_name.clone(),
            });
            ui.close_menu();
        }
        if ui.button("Clear Prompt Data").clicked() {
            out.prompt_requests.push(PromptActionRequest::Clear {
                tp_file: tp_file.clone(),
                component_id: component_id.clone(),
            });
            ui.close_menu();
        }
    });
}

fn handle_jump_to_selected(
    ui: &egui::Ui,
    ctx: &mut RowRenderContext<'_>,
    idx: usize,
    row_rect: egui::Rect,
) {
    if *ctx.jump_to_selected_requested && ctx.selected.contains(&idx) {
        ui.scroll_to_rect(row_rect, Some(egui::Align::Center));
        *ctx.jump_to_selected_requested = false;
    }
}

fn handle_row_selection(
    ui: &egui::Ui,
    ctx: &mut RowRenderContext<'_>,
    idx: usize,
    label_response: &egui::Response,
    drag_response: &egui::Response,
) {
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

fn handle_drag_start(
    ui: &egui::Ui,
    ctx: &mut RowRenderContext<'_>,
    idx: usize,
    drag_response: &egui::Response,
    visible_rows: &[(usize, egui::Rect)],
) {
    if !drag_response.drag_started() {
        return;
    }
    if ctx.locked_blocks.contains(&ctx.items[idx].block_id) {
        *ctx.drag_from = None;
        ctx.drag_indices.clear();
        return;
    }
    step3_history::push_undo_snapshot(ctx.items, ctx.undo_stack, ctx.redo_stack);
    *ctx.drag_from = Some(idx);
    update_drag_indices(ctx, idx);
    update_drag_grab_geometry(ui, ctx, idx, visible_rows);
    *ctx.last_insert_at = None;
    *ctx.drag_over = Some(idx + 1);
}

fn update_drag_indices(ctx: &mut RowRenderContext<'_>, idx: usize) {
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

fn update_drag_grab_geometry(
    ui: &egui::Ui,
    ctx: &mut RowRenderContext<'_>,
    idx: usize,
    visible_rows: &[(usize, egui::Rect)],
) {
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
}
