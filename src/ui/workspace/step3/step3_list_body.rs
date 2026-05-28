// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;
use std::collections::HashMap;

use crate::app::compat_step3_rules::Step3CompatMarker;
use crate::app::prompt_eval_context::build_prompt_eval_context;
use crate::app::prompt_eval_summary_step3;
use crate::app::prompt_popup_text::format_step3_prompt_popup;
use crate::app::state::{Step2Selection, Step3ItemState, WizardState};
use crate::app::step3_history;
use crate::app::step3_prompt_edit::PromptActionRequest;
use crate::parser::prompt_eval_expr::PromptEvalContext;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::shared::layout_tokens_global::BORDER_THIN;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_strong,
    redesign_prompt_fill, redesign_prompt_stroke, redesign_prompt_text, redesign_shell_bg,
    redesign_text_disabled, redesign_text_faint, redesign_text_fainter, redesign_warning,
};
use crate::ui::shared::typography_global::{SIZE_PILL_TEXT, strong};
use crate::ui::step3::block_selection_step3::{
    selected_full_main_parent_block_indices, single_child_main_parent_block_indices,
};
use crate::ui::step3::blocks;
use crate::ui::step3::format_step3;
use crate::ui::step3::service_step3;
use crate::ui::step3::state_step3;

const BOX_PADDING: f32 = 10.0;
const CHILD_INDENT: f32 = 18.0;
const DRAG_HANDLE_SIZE: f32 = 16.0;
const LINENO_FONT_SIZE: f32 = 11.0;
const LINENO_DIGIT_PX: f32 = 7.0;
const LINENO_PAD_PX: f32 = 4.0;
const ROW_SEP_HEIGHT: f32 = 1.0;
const DASH_STEP_PX: f32 = 6.0;
const DASH_LEN_PX: f32 = DASH_STEP_PX * 0.5;
const GLYPH_ICON_PX: f32 = 14.0;
const GLYPH_FONT_SIZE: f32 = 12.0;
const GLYPH_GAP_PX: f32 = 4.0;
const GROUP_BORDER_PADDING: f32 = 4.0;
const GROUP_GAP_PX: f32 = 12.0;

/// Per-frame rendering context bundling mutable state refs and read-only view data.
struct RenderCtx<'a> {
    palette: ThemePalette,
    tab_id: &'a str,
    prompt_eval: &'a PromptEvalContext,
    compat_markers: &'a HashMap<String, Step3CompatMarker>,
    visible_indices: &'a [usize],
    jump_to_selected_requested: &'a mut bool,
    items: &'a mut Vec<Step3ItemState>,
    selected: &'a mut Vec<usize>,
    drag_from: &'a mut Option<usize>,
    drag_over: &'a mut Option<usize>,
    drag_indices: &'a mut Vec<usize>,
    anchor: &'a mut Option<usize>,
    drag_grab_offset: &'a mut f32,
    drag_grab_pos_in_block: &'a mut usize,
    drag_row_h: &'a mut f32,
    last_insert_at: &'a mut Option<usize>,
    collapsed_blocks: &'a mut Vec<String>,
    clone_seq: &'a mut usize,
    locked_blocks: &'a mut Vec<String>,
    undo_stack: &'a mut Vec<Vec<Step3ItemState>>,
    redo_stack: &'a mut Vec<Vec<Step3ItemState>>,
}

struct RowAccumulator {
    visible_rows: Vec<(usize, egui::Rect)>,
    uncheck_requests: Vec<(String, String)>,
    prompt_requests: Vec<PromptActionRequest>,
    open_prompt_popup: Option<(String, String)>,
    open_compat_popup: Option<(
        String,
        String,
        String,
        crate::app::compat_issue::CompatIssue,
    )>,
}

impl RowAccumulator {
    fn new(capacity: usize) -> Self {
        Self {
            visible_rows: Vec::with_capacity(capacity),
            uncheck_requests: Vec::new(),
            prompt_requests: Vec::new(),
            open_prompt_popup: None,
            open_compat_popup: None,
        }
    }
}

/// Renders the Step 3 list body with redesign chrome.
pub(crate) fn render(
    ui: &mut egui::Ui,
    orchestrator: &mut OrchestratorApp,
    compat_markers: &HashMap<String, Step3CompatMarker>,
) {
    let palette = orchestrator.theme_palette;
    let state = &mut orchestrator.wizard_state;

    let avail = ui.available_size();
    let (box_rect, _) = ui.allocate_exact_size(avail, egui::Sense::hover());

    if ui.is_rect_visible(box_rect) {
        let painter = ui.painter();
        let radius = egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8);
        painter.rect_filled(box_rect, radius, redesign_shell_bg(palette));
        painter.rect_stroke(
            box_rect,
            radius,
            egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
            egui::StrokeKind::Inside,
        );
    }

    let inner = box_rect.shrink(BOX_PADDING);
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    child.set_clip_rect(inner.intersect(ui.clip_rect()));

    render_scroll_body(&mut child, state, palette, compat_markers);

    ui.allocate_rect(box_rect, egui::Sense::hover());

    service_step3::prompt_actions::render(ui, state);
}

fn render_scroll_body(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    palette: ThemePalette,
    compat_markers: &HashMap<String, Step3CompatMarker>,
) {
    let tab_id = state.step3.active_game_tab.clone();
    let prompt_eval = build_prompt_eval_context(state);
    let initial_jump = state.step3.jump_to_selected_requested;
    state.step3.jump_to_selected_requested = false;

    let (acc_opt, final_jump) = run_row_pipeline(
        ui,
        state,
        palette,
        compat_markers,
        &tab_id,
        &prompt_eval,
        initial_jump,
    );

    state.step3.jump_to_selected_requested = state.step3.jump_to_selected_requested || final_jump;

    let Some(mut acc) = acc_opt else {
        return;
    };

    flush_row_outcome(state, &tab_id, &mut acc);
}

fn run_row_pipeline(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    palette: ThemePalette,
    compat_markers: &HashMap<String, Step3CompatMarker>,
    tab_id: &str,
    prompt_eval: &crate::parser::prompt_eval_expr::PromptEvalContext,
    initial_jump: bool,
) -> (Option<RowAccumulator>, bool) {
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
        ui.label(
            egui::RichText::new("No selected components from Step 2.")
                .size(13.0)
                .color(redesign_text_faint(palette)),
        );
        return (None, initial_jump);
    }

    let total_child_count = items.iter().filter(|i| !i.is_parent).count();
    let visible_indices = blocks::visible_indices(items, collapsed_blocks);
    let mut jump_flag = initial_jump;

    let mut ctx = RenderCtx {
        palette,
        tab_id,
        prompt_eval,
        compat_markers,
        visible_indices: &visible_indices,
        jump_to_selected_requested: &mut jump_flag,
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

    let lineno_w = lineno_col_width(total_child_count);

    configure_scroll_style(ui);
    let acc = egui::ScrollArea::both()
        .id_salt(("step3_body_scroll", tab_id))
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            render_rows(ui, &mut ctx, lineno_w)
        })
        .inner;

    run_drag_pipeline(ui, &mut ctx, &acc.visible_rows);

    (Some(acc), jump_flag)
}

fn configure_scroll_style(ui: &mut egui::Ui) {
    let mut scroll = egui::style::ScrollStyle::solid();
    scroll.bar_width = 12.0;
    scroll.bar_inner_margin = 0.0;
    scroll.bar_outer_margin = 2.0;
    ui.style_mut().spacing.scroll = scroll;
}

fn lineno_col_width(total_children: usize) -> f32 {
    let digits = total_children.to_string().len();
    let d = f32::from(u16::try_from(digits).unwrap_or(u16::MAX));
    d.mul_add(LINENO_DIGIT_PX, LINENO_PAD_PX)
}

fn render_rows(ui: &mut egui::Ui, ctx: &mut RenderCtx<'_>, lineno_w: f32) -> RowAccumulator {
    let mut acc = RowAccumulator::new(ctx.visible_indices.len());
    let mut child_counter = 0usize;
    let mut first_group = true;

    let mut pos = 0;
    while pos < ctx.visible_indices.len() {
        let idx = ctx.visible_indices[pos];
        if !ctx.items[idx].is_parent {
            child_counter += 1;
            render_child_row(ui, ctx, idx, &mut acc, child_counter, lineno_w);
            pos += 1;
            continue;
        }

        if !first_group {
            ui.add_space(GROUP_GAP_PX);
        }
        first_group = false;

        let block_id = ctx.items[idx].block_id.clone();
        let palette = ctx.palette;

        let group_rect = {
            let top_cursor = ui.cursor().min;
            let avail_w = ui.available_width();

            let inner_rect = egui::Rect::from_min_size(
                top_cursor + egui::vec2(GROUP_BORDER_PADDING, GROUP_BORDER_PADDING),
                egui::vec2(GROUP_BORDER_PADDING.mul_add(-2.0, avail_w), 0.0),
            );

            let group_inner = ui.allocate_new_ui(
                egui::UiBuilder::new()
                    .max_rect(egui::Rect::from_min_size(
                        inner_rect.min,
                        egui::vec2(inner_rect.width(), f32::INFINITY),
                    ))
                    .layout(egui::Layout::top_down(egui::Align::Min)),
                |ui| {
                    render_header_row(ui, ctx, idx, &mut acc);
                    pos += 1;

                    while pos < ctx.visible_indices.len() {
                        let child_idx = ctx.visible_indices[pos];
                        if ctx.items[child_idx].is_parent
                            || ctx.items[child_idx].block_id != block_id
                        {
                            break;
                        }
                        child_counter += 1;
                        render_child_row(ui, ctx, child_idx, &mut acc, child_counter, lineno_w);
                        pos += 1;
                    }
                },
            );

            let inner_used = group_inner.response.rect;
            egui::Rect::from_min_max(
                top_cursor,
                egui::pos2(
                    top_cursor.x + avail_w,
                    inner_used.bottom() + GROUP_BORDER_PADDING,
                ),
            )
        };

        paint_dashed_rect(ui, palette, group_rect);
        ui.allocate_rect(group_rect, egui::Sense::hover());
    }

    acc
}

fn render_header_row(
    ui: &mut egui::Ui,
    ctx: &mut RenderCtx<'_>,
    idx: usize,
    acc: &mut RowAccumulator,
) {
    let block_id = ctx.items[idx].block_id.clone();
    let child_count = blocks::count_children_in_block(ctx.items, idx);
    let collapsed = ctx.collapsed_blocks.contains(&block_id);
    let mut is_locked = ctx.locked_blocks.contains(&block_id);

    let label_response = ui
        .scope(|ui| {
            let mut row_resp: Option<egui::Response> = None;
            ui.horizontal(|ui| {
                if paint_glyph_button(
                    ui,
                    if is_locked { "🔒" } else { "🔓" },
                    if is_locked {
                        redesign_warning(ctx.palette)
                    } else {
                        redesign_text_disabled(ctx.palette)
                    },
                )
                .on_hover_text(crate::ui::shared::tooltip_global::STEP3_LOCK_PARENT)
                .clicked()
                {
                    toggle_locked(ctx.locked_blocks, &block_id, &mut is_locked);
                }

                ui.add_space(GLYPH_GAP_PX);
                paint_glyph_static(ui, "🔗", redesign_text_faint(ctx.palette));

                ui.add_space(GLYPH_GAP_PX);
                if paint_glyph_button(
                    ui,
                    if collapsed { "▸" } else { "▾" },
                    redesign_text_faint(ctx.palette),
                )
                .clicked()
                {
                    if collapsed {
                        ctx.collapsed_blocks.retain(|v| v != block_id.as_str());
                    } else if !ctx.collapsed_blocks.contains(&block_id) {
                        ctx.collapsed_blocks.push(block_id.clone());
                    }
                }

                ui.add_space(GLYPH_GAP_PX);
                let mod_name = ctx.items[idx].mod_name.clone();
                let parent_placeholder = ctx.items[idx].parent_placeholder;
                let title =
                    build_parent_title(&mod_name, parent_placeholder, child_count, is_locked);
                row_resp = Some(ui.selectable_label(ctx.selected.contains(&idx), strong(title)));
            });
            row_resp.expect("row response required")
        })
        .inner;

    let drag_id = ui.make_persistent_id(("step3b_drag_parent", ctx.tab_id, idx));
    let drag_response = ui.interact(label_response.rect, drag_id, egui::Sense::click_and_drag());

    render_parent_context_menu(&drag_response, ctx, idx);
    acc.visible_rows.push((idx, label_response.rect));
    handle_jump_to_selected(ui, ctx, idx, label_response.rect);
    handle_row_selection(ui, ctx, idx, &label_response, &drag_response);
    handle_drag_start(ui, ctx, idx, &drag_response, &acc.visible_rows);
}

/// Allocates a small rect and paints a glyph at center with click semantics.
fn paint_glyph_button(ui: &mut egui::Ui, glyph: &str, color: egui::Color32) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(GLYPH_ICON_PX, GLYPH_ICON_PX), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            glyph,
            egui::FontId::new(GLYPH_FONT_SIZE, egui::FontFamily::Proportional),
            color,
        );
    }
    response
}

/// Paints a decorative glyph with no click semantics.
fn paint_glyph_static(ui: &mut egui::Ui, glyph: &str, color: egui::Color32) {
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(GLYPH_ICON_PX, GLYPH_ICON_PX), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            glyph,
            egui::FontId::new(GLYPH_FONT_SIZE, egui::FontFamily::Proportional),
            color,
        );
    }
}

fn build_parent_title(
    mod_name: &str,
    parent_placeholder: bool,
    child_count: usize,
    is_locked: bool,
) -> String {
    let base = if parent_placeholder {
        format!("{mod_name} (split target) ({child_count})")
    } else {
        format!("{mod_name} ({child_count})")
    };
    if is_locked {
        format!("{base} [locked]")
    } else {
        base
    }
}

fn toggle_locked(locked_blocks: &mut Vec<String>, block_id: &str, is_locked: &mut bool) {
    if *is_locked {
        locked_blocks.retain(|v| v != block_id);
        *is_locked = false;
    } else {
        locked_blocks.push(block_id.to_string());
        *is_locked = true;
    }
}

fn render_child_row(
    ui: &mut egui::Ui,
    ctx: &mut RenderCtx<'_>,
    idx: usize,
    acc: &mut RowAccumulator,
    child_counter: usize,
    lineno_w: f32,
) {
    let item = &ctx.items[idx];
    let prompt_summary =
        prompt_eval_summary_step3::evaluate_step3_item_prompt_summary(item, ctx.prompt_eval);
    let compat_marker = ctx
        .compat_markers
        .get(&crate::app::compat_step3_rules::marker_key(item));

    let label_response = ui
        .horizontal(|ui| {
            ui.add_space(CHILD_INDENT);

            render_drag_handle(ui, ctx.palette);
            render_lineno(ui, ctx.palette, child_counter, lineno_w);

            let text = format_step3::format_step3_item(&ctx.items[idx]);
            let row_text = format_step3::weidu_colored_widget_text(ui, &text);
            let resp = ui.selectable_label(ctx.selected.contains(&idx), row_text);

            if let Some(marker) = compat_marker {
                render_compat_pill(ui, &ctx.items[idx], marker, acc, ctx.palette);
            }
            if !prompt_summary.trim().is_empty() {
                render_prompt_pill(ui, &ctx.items[idx], &prompt_summary, acc, ctx.palette);
            }

            resp
        })
        .inner
        .on_hover_text(crate::ui::shared::tooltip_global::STEP3_DRAG_ROW);

    let drag_id = ui.make_persistent_id(("step3b_drag_child", ctx.tab_id, idx));
    let drag_response = ui.interact(label_response.rect, drag_id, egui::Sense::click_and_drag());

    render_child_context_menu(&drag_response, ctx, idx, acc);

    // Dashed bottom separator between child rows.
    paint_dashed_separator(ui, ctx.palette, label_response.rect);

    ui.add_space(ROW_SEP_HEIGHT);

    acc.visible_rows.push((idx, label_response.rect));
    handle_jump_to_selected(ui, ctx, idx, label_response.rect);
    handle_row_selection(ui, ctx, idx, &label_response, &drag_response);
    handle_drag_start(ui, ctx, idx, &drag_response, &acc.visible_rows);
}

fn paint_dashed_separator(ui: &egui::Ui, palette: ThemePalette, row_rect: egui::Rect) {
    let y = row_rect.bottom();
    let x0 = row_rect.left();
    let x1 = row_rect.right();
    let color = redesign_text_fainter(palette);
    let painter = ui.painter();
    let stroke = egui::Stroke::new(ROW_SEP_HEIGHT, color);
    for x in std::iter::successors(Some(x0), |&prev| {
        let next = prev + DASH_STEP_PX;
        if next < x1 { Some(next) } else { None }
    }) {
        let end = (x + DASH_LEN_PX).min(x1);
        painter.line_segment([egui::pos2(x, y), egui::pos2(end, y)], stroke);
    }
}

/// Paints a dashed rectangle border around `rect`, walking each edge in dash-gap segments.
fn paint_dashed_rect(ui: &egui::Ui, palette: ThemePalette, rect: egui::Rect) {
    let color = redesign_text_faint(palette);
    let stroke = egui::Stroke::new(ROW_SEP_HEIGHT, color);
    let painter = ui.painter();

    let corners = [
        (rect.left_top(), rect.right_top()),
        (rect.right_top(), rect.right_bottom()),
        (rect.right_bottom(), rect.left_bottom()),
        (rect.left_bottom(), rect.left_top()),
    ];

    for (from, to) in corners {
        let dx = to.x - from.x;
        let dy = to.y - from.y;
        let edge_len = dx.hypot(dy);
        if edge_len < 1.0 {
            continue;
        }
        let ux = dx / edge_len;
        let uy = dy / edge_len;

        for t in std::iter::successors(Some(0.0_f32), |&prev| {
            let next = prev + DASH_STEP_PX;
            if next < edge_len { Some(next) } else { None }
        }) {
            let t_end = (t + DASH_LEN_PX).min(edge_len);
            let p0 = egui::pos2(ux.mul_add(t, from.x), uy.mul_add(t, from.y));
            let p1 = egui::pos2(ux.mul_add(t_end, from.x), uy.mul_add(t_end, from.y));
            painter.line_segment([p0, p1], stroke);
        }
    }
}

fn render_drag_handle(ui: &mut egui::Ui, palette: ThemePalette) {
    ui.add(
        egui::Label::new(
            egui::RichText::new("≡")
                .size(DRAG_HANDLE_SIZE)
                .color(redesign_text_faint(palette)),
        )
        .selectable(false),
    );
}

fn render_lineno(ui: &mut egui::Ui, palette: ThemePalette, n: usize, col_w: f32) {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(col_w, LINENO_FONT_SIZE + 4.0),
        egui::Sense::hover(),
    );
    if ui.is_rect_visible(rect) {
        ui.painter().text(
            egui::pos2(rect.right(), rect.center().y),
            egui::Align2::RIGHT_CENTER,
            n.to_string(),
            egui::FontId::new(
                LINENO_FONT_SIZE,
                egui::FontFamily::Name("firacode_nerd".into()),
            ),
            redesign_text_faint(palette),
        );
    }
}

fn render_compat_pill(
    ui: &mut egui::Ui,
    item: &Step3ItemState,
    marker: &Step3CompatMarker,
    acc: &mut RowAccumulator,
    palette: ThemePalette,
) {
    let Some((pill_text_color, pill_bg, pill_label)) =
        crate::ui::step2::tree_compat_display_step2::compat_colors_redesign(
            Some(&marker.kind),
            palette,
        )
    else {
        return;
    };
    ui.add_space(6.0);
    let pill_text = strong(pill_label)
        .color(pill_text_color)
        .size(SIZE_PILL_TEXT);
    let pill_response = ui.add(
        egui::Button::new(pill_text)
            .fill(pill_bg)
            .stroke(egui::Stroke::new(BORDER_THIN, pill_bg))
            .corner_radius(egui::CornerRadius::same(7))
            .min_size(egui::vec2(0.0, 18.0)),
    );
    let pill_response = if let Some(message) = marker.message.as_deref() {
        pill_response.on_hover_text(message)
    } else {
        pill_response
    };
    if pill_response.clicked() {
        acc.open_compat_popup = Some((
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
    acc: &mut RowAccumulator,
    palette: ThemePalette,
) {
    ui.add_space(6.0);
    let prompt_text = strong("PROMPT")
        .color(redesign_prompt_text(palette))
        .size(SIZE_PILL_TEXT);
    let prompt_response = ui
        .add(
            egui::Button::new(prompt_text)
                .fill(redesign_prompt_fill(palette))
                .stroke(egui::Stroke::new(
                    BORDER_THIN,
                    redesign_prompt_stroke(palette),
                ))
                .corner_radius(egui::CornerRadius::same(7))
                .min_size(egui::vec2(0.0, 18.0)),
        )
        .on_hover_text(crate::ui::shared::tooltip_global::SHOW_PARSED_PROMPTS);
    if prompt_response.clicked() {
        acc.open_prompt_popup = Some(format_step3_prompt_popup(item, prompt_summary));
    }
}

fn render_parent_context_menu(drag_response: &egui::Response, ctx: &mut RenderCtx<'_>, idx: usize) {
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
    ctx: &RenderCtx<'_>,
    idx: usize,
    acc: &mut RowAccumulator,
) {
    let tp_file = ctx.items[idx].tp_file.clone();
    let component_id = ctx.items[idx].component_id.clone();
    let component_label = ctx.items[idx].component_label.clone();
    let mod_name = ctx.items[idx].mod_name.clone();
    drag_response.context_menu(|ui| {
        if ui.button("Uncheck In Step 2").clicked() {
            acc.uncheck_requests
                .push((tp_file.clone(), component_id.clone()));
            ui.close_menu();
        }
        if ui.button("Set @wlb-inputs...").clicked() {
            acc.prompt_requests.push(PromptActionRequest::SetWlb {
                tp_file: tp_file.clone(),
                component_id: component_id.clone(),
                component_label: component_label.clone(),
                mod_name: mod_name.clone(),
            });
            ui.close_menu();
        }
        if ui.button("Edit Prompt JSON...").clicked() {
            acc.prompt_requests.push(PromptActionRequest::EditJson {
                tp_file: tp_file.clone(),
                component_id: component_id.clone(),
                component_label: component_label.clone(),
                mod_name: mod_name.clone(),
            });
            ui.close_menu();
        }
        if ui.button("Clear Prompt Data").clicked() {
            acc.prompt_requests.push(PromptActionRequest::Clear {
                tp_file: tp_file.clone(),
                component_id: component_id.clone(),
            });
            ui.close_menu();
        }
    });
}

fn handle_jump_to_selected(
    ui: &egui::Ui,
    ctx: &mut RenderCtx<'_>,
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
    ctx: &mut RenderCtx<'_>,
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
    ctx: &mut RenderCtx<'_>,
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

fn update_drag_indices(ctx: &mut RenderCtx<'_>, idx: usize) {
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
    ctx: &mut RenderCtx<'_>,
    idx: usize,
    visible_rows: &[(usize, egui::Rect)],
) {
    let mut sorted = ctx.drag_indices.clone();
    sorted.sort_unstable();
    sorted.dedup();
    *ctx.drag_grab_pos_in_block = sorted.iter().position(|v| *v == idx).unwrap_or(0);
    if let Some(pointer) = ui.input(|input| input.pointer.interact_pos())
        && let Some((_, grabbed_rect)) = visible_rows.iter().find(|(row_idx, _)| *row_idx == idx)
    {
        *ctx.drag_grab_offset = pointer.y - grabbed_rect.top();
        let row_pitch = grabbed_rect.height() + ui.spacing().item_spacing.y.max(0.0);
        *ctx.drag_row_h = row_pitch.max(1.0);
    }
}

fn run_drag_pipeline(ui: &egui::Ui, ctx: &mut RenderCtx<'_>, visible_rows: &[(usize, egui::Rect)]) {
    let mut pointer_ctx = service_step3::drag_ops::DragPointerContext {
        items: ctx.items,
        drag_from: ctx.drag_from.as_ref(),
        drag_over: ctx.drag_over,
        drag_indices: ctx.drag_indices,
        drag_grab_offset: ctx.drag_grab_offset,
        drag_grab_pos_in_block: ctx.drag_grab_pos_in_block,
        drag_row_h: ctx.drag_row_h,
        visible_rows,
    };
    service_step3::drag_ops::update_drag_target_from_pointer(ui, &mut pointer_ctx);

    service_step3::drag_ops::draw_insert_marker(
        ui,
        ctx.items,
        ctx.drag_from.is_some(),
        *ctx.drag_over,
        visible_rows,
    );

    let mut reorder_ctx = service_step3::drag_ops::LiveReorderContext {
        items: ctx.items,
        selected: ctx.selected,
        drag_from: ctx.drag_from,
        drag_over: ctx.drag_over,
        drag_indices: ctx.drag_indices,
        drag_grab_pos_in_block: ctx.drag_grab_pos_in_block,
        last_insert_at: ctx.last_insert_at,
        locked_blocks: ctx.locked_blocks,
        visible_rows,
    };
    service_step3::drag_ops::apply_live_reorder(ui, &mut reorder_ctx);

    let mut finalize_ctx = service_step3::drag_ops::DragFinalizeContext {
        items: ctx.items,
        selected: ctx.selected,
        drag_from: ctx.drag_from,
        drag_over: ctx.drag_over,
        drag_indices: ctx.drag_indices,
        drag_grab_offset: ctx.drag_grab_offset,
        drag_grab_pos_in_block: ctx.drag_grab_pos_in_block,
        drag_row_h: ctx.drag_row_h,
        last_insert_at: ctx.last_insert_at,
        clone_seq: ctx.clone_seq,
    };
    service_step3::drag_ops::finalize_on_release(ui, &mut finalize_ctx);
}

fn flush_row_outcome(state: &mut WizardState, tab_id: &str, acc: &mut RowAccumulator) {
    if let Some((title, text)) = acc.open_prompt_popup.take() {
        crate::ui::step2::prompt_popup_step2::open_text_prompt_popup(state, title, text);
    }
    if let Some((tp_file, component_id, component_key, issue)) = acc.open_compat_popup.take() {
        state.step2.selected = Some(Step2Selection::Component {
            game_tab: tab_id.to_string(),
            tp_file,
            component_id,
            component_key,
        });
        state.step2.compat_popup_issue_override = Some(issue);
        state.step2.compat_popup_open = true;
    }
    if !acc.uncheck_requests.is_empty() {
        service_step3::component_uncheck::apply_component_unchecks(
            state,
            tab_id,
            &acc.uncheck_requests,
        );
    }
    if !acc.prompt_requests.is_empty() {
        service_step3::prompt_actions::apply_prompt_actions(state, &acc.prompt_requests);
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_item(mod_name: &str, id: &str, label: &str, is_parent: bool) -> Step3ItemState {
        Step3ItemState {
            tp_file: format!("{mod_name}.tp2"),
            component_id: id.to_string(),
            mod_name: mod_name.to_string(),
            component_label: label.to_string(),
            raw_line: String::new(),
            prompt_summary: None,
            prompt_events: Vec::new(),
            selected_order: 1,
            block_id: format!("{mod_name}::block0"),
            is_parent,
            parent_placeholder: false,
        }
    }

    fn mod_item(mod_name: &str) -> Step3ItemState {
        make_item(mod_name, "__PARENT__", "", true)
    }

    fn child_item(mod_name: &str, id: &str, label: &str) -> Step3ItemState {
        make_item(mod_name, id, label, false)
    }

    /// Verifies that `visible_indices` for a list with one parent and three children
    /// returns all four indices when no blocks are collapsed.
    #[test]
    fn visible_indices_all_visible_when_not_collapsed() {
        let items = vec![
            mod_item("ModA"),
            child_item("ModA", "1", "Component One"),
            child_item("ModA", "2", "Component Two"),
            child_item("ModA", "3", "Component Three"),
        ];
        let collapsed: Vec<String> = Vec::new();
        let vis = blocks::visible_indices(&items, &collapsed);
        assert_eq!(vis.len(), 4);
        assert_eq!(vis, vec![0, 1, 2, 3]);
    }

    /// Verifies that child rows are hidden when their block is in the collapsed set.
    #[test]
    fn visible_indices_children_hidden_when_collapsed() {
        let items = vec![
            mod_item("ModA"),
            child_item("ModA", "1", "Component One"),
            child_item("ModA", "2", "Component Two"),
            child_item("ModA", "3", "Component Three"),
        ];
        let collapsed = vec!["ModA::block0".to_string()];
        let vis = blocks::visible_indices(&items, &collapsed);
        assert_eq!(vis.len(), 1);
        assert_eq!(vis[0], 0);
    }

    /// Verifies that `count_children_in_block` counts only non-parent items
    /// belonging to the same block.
    #[test]
    fn count_children_in_block_correct() {
        let items = vec![
            mod_item("ModA"),
            child_item("ModA", "1", "Component One"),
            child_item("ModA", "2", "Component Two"),
            child_item("ModA", "3", "Component Three"),
        ];
        let count = blocks::count_children_in_block(&items, 0);
        assert_eq!(count, 3);
    }

    /// Verifies that `build_parent_title` produces the expected format for a locked block.
    #[test]
    fn build_parent_title_locked() {
        let title = build_parent_title("SomeMod", false, 3, true);
        assert_eq!(title, "SomeMod (3) [locked]");
    }

    /// Verifies that `build_parent_title` for a placeholder row includes "(split target)".
    #[test]
    fn build_parent_title_placeholder() {
        let title = build_parent_title("SomeMod", true, 2, false);
        assert_eq!(title, "SomeMod (split target) (2)");
    }

    /// Verifies that `toggle_locked` adds a block ID when not locked, and removes it when locked.
    #[test]
    fn toggle_locked_roundtrip() {
        let mut locked: Vec<String> = Vec::new();
        let block = "ModA::block0".to_string();
        let mut is_locked = false;

        toggle_locked(&mut locked, &block, &mut is_locked);
        assert!(is_locked);
        assert!(locked.contains(&block));

        toggle_locked(&mut locked, &block, &mut is_locked);
        assert!(!is_locked);
        assert!(!locked.contains(&block));
    }

    /// Verifies the lineno column width grows with digit count.
    #[test]
    fn lineno_col_width_grows_with_digits() {
        let w1 = lineno_col_width(9);
        let w2 = lineno_col_width(10);
        assert!(w2 > w1, "two-digit column must be wider than one-digit");
    }

    /// Verifies that `push_undo_snapshot` records a snapshot of the item list.
    #[test]
    fn undo_snapshot_records_state() {
        let items = vec![mod_item("ModA"), child_item("ModA", "1", "Component One")];
        let mut undo_stack: Vec<Vec<Step3ItemState>> = Vec::new();
        let mut redo_stack: Vec<Vec<Step3ItemState>> = Vec::new();
        step3_history::push_undo_snapshot(&items, &mut undo_stack, &mut redo_stack);
        assert_eq!(undo_stack.len(), 1);
        assert_eq!(undo_stack[0].len(), 2);
        assert!(redo_stack.is_empty());
    }

    /// Verifies that undo restores the previous snapshot and redo re-applies it.
    #[test]
    fn undo_redo_roundtrip() {
        let original = vec![
            mod_item("ModA"),
            child_item("ModA", "1", "Component One"),
            child_item("ModA", "2", "Component Two"),
            child_item("ModA", "3", "Component Three"),
        ];
        let mut items = original.clone();
        let mut undo_stack: Vec<Vec<Step3ItemState>> = Vec::new();
        let mut redo_stack: Vec<Vec<Step3ItemState>> = Vec::new();

        step3_history::push_undo_snapshot(&items, &mut undo_stack, &mut redo_stack);
        items.pop();

        step3_history::undo(&mut items, &mut undo_stack, &mut redo_stack);
        assert_eq!(items.len(), original.len(), "undo must restore four items");

        step3_history::redo(&mut items, &mut undo_stack, &mut redo_stack);
        assert_eq!(
            items.len(),
            original.len() - 1,
            "redo must re-remove the item"
        );
    }
}
