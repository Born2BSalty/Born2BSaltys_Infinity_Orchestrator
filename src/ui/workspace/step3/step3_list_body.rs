// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;
use std::collections::HashMap;

use crate::app::compat_step3_rules::Step3CompatMarker;
use crate::app::game_authority::{self, GameSlot};
use crate::app::prompt_eval_summary_step3;
use crate::app::state::{Step2Selection, Step3ItemState, WizardState};
use crate::app::step3_history::{self, Step3HistoryEntry, Step3TouchedRows};
use crate::app::step3_prompt_edit::PromptActionRequest;
use crate::app::step3_separators::{SeparatorRename, move_section, section_indices};
use crate::parser::prompt_eval_expr::PromptEvalContext;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::shared::layout_tokens_global::BORDER_THIN;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_accent,
    redesign_border_strong, redesign_prompt_fill, redesign_prompt_stroke, redesign_prompt_text,
    redesign_rail_bg, redesign_shell_bg, redesign_text_disabled, redesign_text_faint,
    redesign_text_fainter, redesign_warning, redesign_with_alpha,
};
use crate::ui::shared::selected_row::selectable_row;
use crate::ui::shared::typography_global::{SIZE_PILL_TEXT, strong};
use crate::ui::step3::blocks;
use crate::ui::step3::format_step3;
use crate::ui::step3::installer_answers_step3::prompt_context_for_tab;
use crate::ui::step3::list_rows_step3::parent_display_name;
use crate::ui::step3::move_selection_step3::{
    MoveSelectionContext, MoveSelectionOutcome, MoveSelectionTarget, capture_identities,
    header_of_a_fully_selected_mod, is_locked, move_selection, moving_set,
};
use crate::ui::step3::service_step3;
use crate::ui::step3::state_step3;
use crate::ui::step3::toolbar_support_step3;
use crate::ui::workspace::widgets::weidu_line;

use super::step3_section_drag as section_drag;
use super::step3_separator_row::{
    RenameOutcome, SeparatorRowView, commit_separator_rename, grip_hit_rect, paint_grip,
    paint_separator_ghost, render_separator_row,
};

const MOVE_LOCKED_NOTICE: &str = "Locked mods cannot be moved. Unlock them first.";

const BOX_PADDING: f32 = 10.0;
const CHILD_INDENT: f32 = 18.0;
const LINENO_FONT_SIZE: f32 = 11.0;
const LINENO_DIGIT_PX: f32 = 7.0;
const LINENO_PAD_PX: f32 = 4.0;
const ROW_SEP_HEIGHT: f32 = 1.0;
const DOT_STEP_PX: f32 = 7.0;
const DOT_RADIUS: f32 = 0.5;
const GLYPH_ICON_PX: f32 = 14.0;
const GLYPH_FONT_SIZE: f32 = 12.0;
const GLYPH_GAP_PX: f32 = 4.0;
const GROUP_GAP_PX: f32 = 6.0;
const SCROLLBAR_RESERVE: f32 = 14.0;
const HEADER_BAR_VPAD_TOP: f32 = 6.0;
const HEADER_BAR_VPAD_BOT: f32 = 2.0;
const HEADER_DOT_STEP_PX: f32 = 7.0;
const HEADER_DOT_RADIUS: f32 = 0.7;
const HEADER_LEFT_PAD_PX: f32 = 6.0;
const HEADER_BAND_HEIGHT_KEY: &str = "step3_header_band_height";
const SEPARATOR_BAND_SCALE: f32 = 1.1;
const SECTION_DROP_LINE_PX: f32 = 2.0;

struct RenderCtx<'a> {
    palette: ThemePalette,
    tab_id: &'a str,
    tab_slot: GameSlot,
    separator_rename: &'a mut Option<SeparatorRename>,
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
    undo_stack: &'a mut Vec<Step3HistoryEntry>,
    redo_stack: &'a mut Vec<Step3HistoryEntry>,
    current_group_x_bounds: Option<(f32, f32)>,
    header_band_h: f32,
}

struct RowAccumulator {
    visible_rows: Vec<(usize, egui::Rect)>,
    uncheck_requests: Vec<(String, String)>,
    prompt_requests: Vec<PromptActionRequest>,
    open_installer_answers: Option<usize>,
    open_compat_popup: Option<(
        String,
        String,
        String,
        crate::app::compat_issue::CompatIssue,
    )>,
    move_request: Option<(usize, MoveSelectionTarget)>,
    drag_refused_locked: bool,
    separator_request: Option<SeparatorRequest>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SeparatorRequest {
    AddAbove(usize),
    Rename(usize),
    Remove(usize),
}

impl RowAccumulator {
    fn new(capacity: usize) -> Self {
        Self {
            visible_rows: Vec::with_capacity(capacity),
            uncheck_requests: Vec::new(),
            prompt_requests: Vec::new(),
            open_installer_answers: None,
            open_compat_popup: None,
            move_request: None,
            drag_refused_locked: false,
            separator_request: None,
        }
    }
}

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
        let radius = egui::CornerRadius {
            nw: 0,
            ne: 0,
            sw: REDESIGN_BORDER_RADIUS_U8,
            se: REDESIGN_BORDER_RADIUS_U8,
        };
        painter.rect_filled(box_rect, radius, redesign_shell_bg(palette));
        painter.rect_stroke(
            box_rect,
            radius,
            egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
            egui::StrokeKind::Inside,
        );
    }

    let border_w = REDESIGN_BORDER_WIDTH_PX;
    let inner = egui::Rect::from_min_max(
        egui::pos2(box_rect.left() + BOX_PADDING, box_rect.top() + BOX_PADDING),
        egui::pos2(box_rect.right() - border_w, box_rect.bottom() - BOX_PADDING),
    );
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    child.set_clip_rect(inner.intersect(ui.clip_rect()));
    child.add_space(3.0);

    let move_outcome = render_scroll_body(&mut child, state, palette, compat_markers);

    ui.allocate_rect(box_rect, egui::Sense::hover());

    service_step3::prompt_actions::render(ui, state);

    apply_move_outcome(orchestrator, move_outcome);
}

fn apply_move_outcome(orchestrator: &mut OrchestratorApp, outcome: Option<MoveSelectionOutcome>) {
    match outcome {
        Some(MoveSelectionOutcome::Moved) => orchestrator.mark_workspace_dirty(),
        Some(MoveSelectionOutcome::RefusedLocked) => {
            orchestrator.notification_manager.warn(MOVE_LOCKED_NOTICE);
        }
        Some(MoveSelectionOutcome::NothingToMove) | None => {}
    }
}

fn render_scroll_body(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    palette: ThemePalette,
    compat_markers: &HashMap<String, Step3CompatMarker>,
) -> Option<MoveSelectionOutcome> {
    let tab_id = state.step3.active_game_tab.clone();
    let prompt_eval = prompt_context_for_tab(state, &tab_id);
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

    let mut acc = acc_opt?;

    flush_row_outcome(state, &tab_id, &mut acc)
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
    let tab_slot = game_authority::slot_for_tab(tab_id);
    let rename_on_this_tab = state
        .step3
        .separator_rename
        .as_ref()
        .is_some_and(|rename| rename.tab == tab_slot);
    let mut separator_rename = if rename_on_this_tab {
        state.step3.separator_rename.take()
    } else {
        None
    };
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
    separator_rename.take_if(|rename| {
        !items
            .iter()
            .any(|item| item.is_separator() && item.block_id == rename.block_id)
    });

    let total_child_count = items.iter().filter(|i| !i.is_parent).count();
    let visible_indices = blocks::visible_indices(items, collapsed_blocks);
    let mut jump_flag = initial_jump;

    let mut ctx = RenderCtx {
        palette,
        tab_id,
        tab_slot,
        separator_rename: &mut separator_rename,
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
        current_group_x_bounds: None,
        header_band_h: remembered_header_band_height(ui),
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
    if rename_on_this_tab || separator_rename.is_some() {
        state.step3.separator_rename = separator_rename;
    }

    (Some(acc), jump_flag)
}

fn configure_scroll_style(ui: &mut egui::Ui) {
    let mut scroll = egui::style::ScrollStyle::floating();
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

    let list_left_x = ui.cursor().min.x;
    let viewport_w = (ui.clip_rect().width() - SCROLLBAR_RESERVE)
        .min(ui.available_width())
        .max(0.0);
    let tab_w_id = egui::Id::new(("step3_tab_w", ctx.tab_id));
    let remembered = ui
        .ctx()
        .data(|d| d.get_temp::<f32>(tab_w_id))
        .unwrap_or(0.0);
    let group_w = group_width(viewport_w, remembered);
    let mut measured: f32 = 0.0;

    let mut pos = 0;
    while pos < ctx.visible_indices.len() {
        let idx = ctx.visible_indices[pos];
        if !ctx.items[idx].is_parent {
            child_counter += 1;
            let right = render_child_row(ui, ctx, idx, &mut acc, child_counter, lineno_w, false);
            measured = measured.max(right - list_left_x + SCROLLBAR_RESERVE);
            pos += 1;
            continue;
        }

        if ctx.items[idx].is_separator() {
            if !first_group {
                ui.add_space(GROUP_GAP_PX * 2.0);
            }
            first_group = false;
            render_separator(ui, ctx, idx, &mut acc, list_left_x + group_w);
            pos += 1;
            continue;
        }

        if !first_group {
            ui.add_space(GROUP_GAP_PX);
        }
        first_group = false;

        let block_id = ctx.items[idx].block_id.clone();
        render_header_band(ui, ctx, idx, &mut acc, group_w);
        pos += 1;

        while pos < ctx.visible_indices.len() {
            let child_idx = ctx.visible_indices[pos];
            if ctx.items[child_idx].is_parent || ctx.items[child_idx].block_id != block_id {
                break;
            }
            child_counter += 1;
            let next_pos = pos + 1;
            let is_last_in_group = next_pos >= ctx.visible_indices.len()
                || ctx.items[ctx.visible_indices[next_pos]].is_parent
                || ctx.items[ctx.visible_indices[next_pos]].block_id != block_id;
            let right = render_child_row(
                ui,
                ctx,
                child_idx,
                &mut acc,
                child_counter,
                lineno_w,
                is_last_in_group,
            );
            measured = measured.max(right - list_left_x + SCROLLBAR_RESERVE);
            pos += 1;
        }

        ctx.current_group_x_bounds = None;
    }

    if width_changed(measured, remembered) {
        ui.ctx().data_mut(|d| d.insert_temp(tab_w_id, measured));
        ui.ctx().request_repaint();
    }
    remember_header_band_height(ui, ctx.header_band_h);

    acc
}

fn render_header_band(
    ui: &mut egui::Ui,
    ctx: &mut RenderCtx<'_>,
    idx: usize,
    acc: &mut RowAccumulator,
    group_w: f32,
) {
    let top_cursor = ui.cursor().min;
    let bg_shape_id = ui.painter().add(egui::Shape::Noop);

    let scope_resp = ui.scope(|ui| {
        ui.set_min_width(group_w + SCROLLBAR_RESERVE);
        ui.add_space(HEADER_BAR_VPAD_TOP);
        ui.horizontal(|ui| {
            ui.add_space(HEADER_LEFT_PAD_PX);
            render_header_row(ui, ctx, idx, acc);
        });
        ui.add_space(HEADER_BAR_VPAD_BOT);
    });
    let band_height = scope_resp.response.rect.height();
    ctx.header_band_h = band_height;

    let header_rect =
        egui::Rect::from_min_size(top_cursor, egui::vec2(group_w.max(0.0), band_height));

    ui.painter().set(
        bg_shape_id,
        egui::Shape::rect_filled(
            header_rect,
            egui::CornerRadius::ZERO,
            redesign_rail_bg(ctx.palette),
        ),
    );

    let dot_color = redesign_with_alpha(crate::ui::shared::theme_global::accent_path(), 1, 2);
    paint_dotted_rect(
        ui,
        header_rect,
        dot_color,
        HEADER_DOT_STEP_PX,
        HEADER_DOT_RADIUS,
    );
    render_header_grip(ui, ctx, idx, acc, header_rect);

    ctx.current_group_x_bounds = Some((header_rect.left(), header_rect.right()));
}

fn render_header_grip(
    ui: &egui::Ui,
    ctx: &mut RenderCtx<'_>,
    idx: usize,
    acc: &mut RowAccumulator,
    header_rect: egui::Rect,
) {
    let center_y = acc
        .visible_rows
        .iter()
        .rev()
        .find(|(row, _)| *row == idx)
        .map_or_else(|| header_rect.center().y, |(_, rect)| rect.center().y);
    paint_grip(
        ui.painter(),
        header_rect.right(),
        center_y,
        redesign_text_faint(ctx.palette),
    );
    let grip_id = ui.make_persistent_id(("step3b_drag_parent_grip", ctx.tab_id, idx));
    let grip = ui.interact(
        grip_hit_rect(header_rect),
        grip_id,
        egui::Sense::click_and_drag(),
    );
    if grip.hovered() && ctx.drag_from.is_none() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
    }
    handle_row_selection(ui, ctx, idx, &grip, &grip);
    if handle_drag_start(ui, ctx, idx, &grip, &acc.visible_rows) == DragStart::RefusedLocked {
        acc.drag_refused_locked = true;
    }
}

fn estimated_header_band_height(ui: &egui::Ui) -> f32 {
    let spacing = ui.spacing();
    HEADER_BAR_VPAD_TOP + HEADER_BAR_VPAD_BOT + spacing.interact_size.y + spacing.item_spacing.y
}

fn remembered_header_band_height(ui: &egui::Ui) -> f32 {
    ui.ctx()
        .data(|d| d.get_temp::<f32>(egui::Id::new(HEADER_BAND_HEIGHT_KEY)))
        .unwrap_or_else(|| estimated_header_band_height(ui))
}

fn remember_header_band_height(ui: &egui::Ui, measured: f32) {
    let remembered = ui
        .ctx()
        .data(|d| d.get_temp::<f32>(egui::Id::new(HEADER_BAND_HEIGHT_KEY)));
    if remembered.is_none_or(|old| (measured - old).abs() > 0.5) {
        ui.ctx().data_mut(|d| {
            d.insert_temp(egui::Id::new(HEADER_BAND_HEIGHT_KEY), measured);
        });
        ui.ctx().request_repaint();
    }
}

const fn group_width(viewport_w: f32, remembered: f32) -> f32 {
    viewport_w.max(remembered)
}

fn width_changed(measured: f32, remembered: f32) -> bool {
    (measured - remembered).abs() > 0.5
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

    let mod_version = ctx
        .items
        .iter()
        .filter(|i| !i.is_parent && i.block_id == block_id)
        .find_map(|i| crate::parser::weidu_version::parse_version(&i.raw_line));

    let label_response = ui
        .scope(|ui| {
            let mut row_resp: Option<egui::Response> = None;
            ui.horizontal(|ui| {
                if paint_lock_button(
                    ui,
                    is_locked,
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
                if paint_chevron_button(ui, collapsed, redesign_text_faint(ctx.palette)).clicked() {
                    if collapsed {
                        ctx.collapsed_blocks.retain(|v| v != block_id.as_str());
                    } else if !ctx.collapsed_blocks.contains(&block_id) {
                        ctx.collapsed_blocks.push(block_id.clone());
                    }
                    ctx.selected.clear();
                    *ctx.anchor = None;
                }

                ui.add_space(GLYPH_GAP_PX);
                let mod_name = parent_display_name(&ctx.items[idx]);
                let parent_placeholder = ctx.items[idx].parent_placeholder;
                let title =
                    build_parent_title(&mod_name, parent_placeholder, child_count, is_locked);
                row_resp = Some(ui.selectable_label(ctx.selected.contains(&idx), strong(title)));

                if let Some(ref v) = mod_version {
                    ui.add_space(GLYPH_GAP_PX);
                    ui.label(
                        egui::RichText::new(format!("v{v}"))
                            .color(redesign_text_faint(ctx.palette)),
                    );
                }
            });
            row_resp.expect("row response required")
        })
        .inner;

    let drag_id = ui.make_persistent_id(("step3b_drag_parent", ctx.tab_id, idx));
    let drag_response = ui.interact(label_response.rect, drag_id, egui::Sense::click_and_drag());

    render_parent_context_menu(&drag_response, ctx, idx, acc);
    acc.visible_rows.push((idx, label_response.rect));
    handle_jump_to_selected(ui, ctx, idx, label_response.rect);
    handle_row_selection(ui, ctx, idx, &label_response, &drag_response);
    if handle_drag_start(ui, ctx, idx, &drag_response, &acc.visible_rows)
        == DragStart::RefusedLocked
    {
        acc.drag_refused_locked = true;
    }
}

fn paint_lock_button(ui: &mut egui::Ui, is_locked: bool, color: egui::Color32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(GLYPH_ICON_PX, GLYPH_ICON_PX),
        egui::Sense::click(),
    );
    if ui.is_rect_visible(rect) {
        let glyph = if is_locked { "\u{F023}" } else { "\u{F09C}" };
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            glyph,
            egui::FontId::new(
                GLYPH_FONT_SIZE,
                egui::FontFamily::Name("firacode_nerd".into()),
            ),
            color,
        );
    }
    response
}

fn render_separator(
    ui: &mut egui::Ui,
    ctx: &mut RenderCtx<'_>,
    idx: usize,
    acc: &mut RowAccumulator,
    right_x: f32,
) {
    let block_id = ctx.items[idx].block_id.clone();
    let name = ctx.items[idx].mod_name.clone();
    let collapsed = ctx.collapsed_blocks.contains(&block_id);
    let mod_count = section_indices(ctx.items, idx)
        .into_iter()
        .filter(|row| ctx.items[*row].is_mod_header())
        .count();
    let view = SeparatorRowView {
        palette: ctx.palette,
        name: &name,
        collapsed,
        mod_count,
        right_x,
        band_height: ctx.header_band_h * SEPARATOR_BAND_SCALE,
    };
    let rename = ctx
        .separator_rename
        .as_mut()
        .filter(|rename| rename.block_id == block_id);
    let renaming = rename.is_some();
    let row = render_separator_row(ui, &view, rename);

    if row.chevron_clicked {
        toggle_collapsed(ctx, &block_id, collapsed);
    }
    match row.rename {
        Some(RenameOutcome::Commit(text)) => {
            commit_separator_rename(ctx.items, idx, &text, ctx.undo_stack, ctx.redo_stack);
            *ctx.separator_rename = None;
        }
        Some(RenameOutcome::Cancel) => *ctx.separator_rename = None,
        Some(RenameOutcome::Editing) | None => {}
    }

    let drag_id = ui.make_persistent_id(("step3b_drag_separator", ctx.tab_id, idx));
    let drag_response = ui.interact(row.grab_rect, drag_id, egui::Sense::click_and_drag());
    if drag_response.hovered() && ctx.drag_from.is_none() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
    }
    let on_name = drag_response
        .interact_pointer_pos()
        .is_some_and(|pos| row.name_rect.contains(pos));
    if !renaming && on_name && drag_response.double_clicked() {
        *ctx.separator_rename = Some(SeparatorRename {
            tab: ctx.tab_slot,
            block_id,
            text: name,
            focus_pending: true,
        });
    }

    render_separator_context_menu(&drag_response, idx, acc);
    acc.visible_rows.push((idx, row.row_rect));
    handle_jump_to_selected(ui, ctx, idx, row.row_rect);
    handle_row_selection(ui, ctx, idx, &drag_response, &drag_response);
    if handle_drag_start(ui, ctx, idx, &drag_response, &acc.visible_rows)
        == DragStart::RefusedLocked
    {
        acc.drag_refused_locked = true;
    }
}

fn toggle_collapsed(ctx: &mut RenderCtx<'_>, block_id: &str, collapsed: bool) {
    if collapsed {
        ctx.collapsed_blocks.retain(|v| v != block_id);
    } else if !ctx.collapsed_blocks.iter().any(|v| v == block_id) {
        ctx.collapsed_blocks.push(block_id.to_string());
    }
    ctx.selected.clear();
    *ctx.anchor = None;
}

pub(super) fn paint_chevron_button(
    ui: &mut egui::Ui,
    collapsed: bool,
    color: egui::Color32,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(GLYPH_ICON_PX, GLYPH_ICON_PX),
        egui::Sense::click(),
    );
    if ui.is_rect_visible(rect) {
        let glyph = if collapsed { "▸" } else { "▾" };
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            glyph,
            egui::FontId::new(GLYPH_FONT_SIZE, egui::FontFamily::Monospace),
            color,
        );
    }
    response
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

#[must_use]
fn render_child_row(
    ui: &mut egui::Ui,
    ctx: &mut RenderCtx<'_>,
    idx: usize,
    acc: &mut RowAccumulator,
    child_counter: usize,
    lineno_w: f32,
    is_last_in_group: bool,
) -> f32 {
    let item = &ctx.items[idx];
    let prompt_summary =
        prompt_eval_summary_step3::evaluate_step3_item_prompt_summary(item, ctx.prompt_eval);
    let compat_marker = ctx
        .compat_markers
        .get(&crate::app::compat_step3_rules::marker_key(item));

    let row_outer = ui.horizontal(|ui| {
        ui.add_space(CHILD_INDENT);

        render_lineno(ui, ctx.palette, child_counter, lineno_w);

        let text = format_step3::format_step3_item(&ctx.items[idx]);
        let row_text = weidu_line::weidu_widget_text(ui, ctx.palette, &text);
        let resp = selectable_row(ui, ctx.palette, ctx.selected.contains(&idx), row_text);

        if let Some(marker) = compat_marker {
            render_compat_pill(ui, &ctx.items[idx], marker, acc, ctx.palette);
        }
        if !prompt_summary.trim().is_empty() {
            render_prompt_pill(ui, idx, acc, ctx.palette);
        }

        resp
    });
    let row_right = row_outer.response.rect.right();
    let label_response = row_outer
        .inner
        .on_hover_text(crate::ui::shared::tooltip_global::STEP3_DRAG_ROW);

    let drag_id = ui.make_persistent_id(("step3b_drag_child", ctx.tab_id, idx));
    let drag_response = ui.interact(label_response.rect, drag_id, egui::Sense::click_and_drag());

    render_child_context_menu(&drag_response, ctx, idx, acc);

    if !is_last_in_group {
        let sep_x_bounds = ctx.current_group_x_bounds;
        paint_dashed_separator(ui, ctx.palette, label_response.rect, sep_x_bounds);
    }

    ui.add_space(ROW_SEP_HEIGHT);

    acc.visible_rows.push((idx, label_response.rect));
    handle_jump_to_selected(ui, ctx, idx, label_response.rect);
    handle_row_selection(ui, ctx, idx, &label_response, &drag_response);
    if handle_drag_start(ui, ctx, idx, &drag_response, &acc.visible_rows)
        == DragStart::RefusedLocked
    {
        acc.drag_refused_locked = true;
    }

    row_right
}

#[must_use]
fn visible_dot_range(
    from: f32,
    to: f32,
    step: f32,
    vis_min: f32,
    vis_max: f32,
) -> Option<(f32, f32)> {
    let start_k = ((vis_min - from) / step).ceil().max(0.0);
    let first = step.mul_add(start_k, from);
    let end = to.min(vis_max);
    if first > end {
        return None;
    }
    let steps_between = ((end - first) / step).floor();
    let last = step.mul_add(steps_between, first);
    Some((first, last))
}

struct DotRun {
    fixed: f32,
    start: f32,
    end: f32,
    step: f32,
    radius: f32,
    color: egui::Color32,
    horizontal: bool,
}

fn paint_dot_run(painter: &egui::Painter, run: &DotRun) {
    let tolerance = run.step.mul_add(0.001, run.end);
    for v in std::iter::successors(Some(run.start), |&prev| {
        let next = prev + run.step;
        if next <= tolerance { Some(next) } else { None }
    }) {
        let pt = if run.horizontal {
            egui::pos2(v, run.fixed)
        } else {
            egui::pos2(run.fixed, v)
        };
        painter.circle_filled(pt, run.radius, run.color);
    }
}

fn is_within(v: f32, min: f32, max: f32) -> bool {
    (min..=max).contains(&v)
}

fn paint_dashed_separator(
    ui: &egui::Ui,
    palette: ThemePalette,
    row_rect: egui::Rect,
    group_x_bounds: Option<(f32, f32)>,
) {
    let y = row_rect.bottom();
    let (x0, x1) = group_x_bounds.unwrap_or_else(|| (row_rect.left(), row_rect.right()));
    let base_color = redesign_text_fainter(palette);
    let color = redesign_with_alpha(base_color, 1, 8);
    let painter = ui.painter();
    let visible = ui.clip_rect();
    let Some((start, end)) =
        visible_dot_range(x0, x1, DOT_STEP_PX, visible.left(), visible.right())
    else {
        return;
    };
    paint_dot_run(
        painter,
        &DotRun {
            fixed: y,
            start,
            end,
            step: DOT_STEP_PX,
            radius: DOT_RADIUS,
            color,
            horizontal: true,
        },
    );
}

fn paint_dotted_rect(
    ui: &egui::Ui,
    rect: egui::Rect,
    color: egui::Color32,
    step_px: f32,
    radius: f32,
) {
    let painter = ui.painter();
    let visible = ui.clip_rect();

    if let Some((y0, y1)) = visible_dot_range(
        rect.top(),
        rect.bottom(),
        step_px,
        visible.top(),
        visible.bottom(),
    ) {
        if is_within(rect.left(), visible.left(), visible.right()) {
            paint_dot_run(
                painter,
                &DotRun {
                    fixed: rect.left(),
                    start: y0,
                    end: y1,
                    step: step_px,
                    radius,
                    color,
                    horizontal: false,
                },
            );
        }
        if is_within(rect.right(), visible.left(), visible.right()) {
            paint_dot_run(
                painter,
                &DotRun {
                    fixed: rect.right(),
                    start: y0,
                    end: y1,
                    step: step_px,
                    radius,
                    color,
                    horizontal: false,
                },
            );
        }
    }

    if let Some((x0, x1)) = visible_dot_range(
        rect.left(),
        rect.right(),
        step_px,
        visible.left(),
        visible.right(),
    ) {
        paint_dot_run(
            painter,
            &DotRun {
                fixed: rect.top(),
                start: x0,
                end: x1,
                step: step_px,
                radius,
                color,
                horizontal: true,
            },
        );
        paint_dot_run(
            painter,
            &DotRun {
                fixed: rect.bottom(),
                start: x0,
                end: x1,
                step: step_px,
                radius,
                color,
                horizontal: true,
            },
        );
    }
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
    row_index: usize,
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
        acc.open_installer_answers = Some(row_index);
    }
}

fn render_move_selection_entries(ui: &mut egui::Ui, idx: usize, acc: &mut RowAccumulator) {
    if ui.button("Move selection to top").clicked() {
        acc.move_request = Some((idx, MoveSelectionTarget::Top));
        ui.close_menu();
    }
    if ui.button("Move selection to bottom").clicked() {
        acc.move_request = Some((idx, MoveSelectionTarget::Bottom));
        ui.close_menu();
    }
    ui.separator();
}

fn render_add_separator_entry(ui: &mut egui::Ui, idx: usize, acc: &mut RowAccumulator) {
    if ui.button("Add separator above").clicked() {
        acc.separator_request = Some(SeparatorRequest::AddAbove(idx));
        ui.close_menu();
    }
}

fn render_separator_context_menu(
    drag_response: &egui::Response,
    idx: usize,
    acc: &mut RowAccumulator,
) {
    drag_response.context_menu(|ui| {
        if ui.button("Move separator to top").clicked() {
            acc.move_request = Some((idx, MoveSelectionTarget::Top));
            ui.close_menu();
        }
        if ui.button("Move separator to bottom").clicked() {
            acc.move_request = Some((idx, MoveSelectionTarget::Bottom));
            ui.close_menu();
        }
        ui.separator();
        if ui.button("Rename\u{2026}").clicked() {
            acc.separator_request = Some(SeparatorRequest::Rename(idx));
            ui.close_menu();
        }
        if ui.button("Remove separator").clicked() {
            acc.separator_request = Some(SeparatorRequest::Remove(idx));
            ui.close_menu();
        }
    });
}

fn render_parent_context_menu(
    drag_response: &egui::Response,
    ctx: &mut RenderCtx<'_>,
    idx: usize,
    acc: &mut RowAccumulator,
) {
    drag_response.context_menu(|ui| {
        render_move_selection_entries(ui, idx, acc);
        render_add_separator_entry(ui, idx, acc);
        if ui.button("Clone Parent (empty split target)").clicked() {
            step3_history::push_undo_snapshot(
                ctx.items,
                Step3TouchedRows::default(),
                ctx.undo_stack,
                ctx.redo_stack,
            );
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
        render_move_selection_entries(ui, idx, acc);
        render_add_separator_entry(ui, idx, acc);
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
) -> DragStart {
    if !drag_response.drag_started() {
        return DragStart::NotStarted;
    }
    let moving = moving_set(ctx.items, ctx.selected, ctx.collapsed_blocks, idx);
    if is_locked(ctx.items, &moving, ctx.locked_blocks) {
        *ctx.drag_from = None;
        ctx.drag_indices.clear();
        return DragStart::RefusedLocked;
    }
    step3_history::push_undo_snapshot(
        ctx.items,
        capture_identities(ctx.items, &moving),
        ctx.undo_stack,
        ctx.redo_stack,
    );
    *ctx.drag_from = Some(idx);
    if !ctx.selected.contains(&idx) {
        if header_of_a_fully_selected_mod(ctx.items, ctx.selected, idx) {
            ctx.selected.push(idx);
            ctx.selected.sort_unstable();
        } else {
            ctx.selected.clear();
            ctx.selected.push(idx);
        }
    }
    *ctx.drag_indices = moving;
    update_drag_grab_geometry(ui, ctx, idx, visible_rows);
    *ctx.last_insert_at = None;
    *ctx.drag_over = Some(idx + 1);
    DragStart::Started
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DragStart {
    NotStarted,
    Started,
    RefusedLocked,
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
    if section_drag::is_section_drag(ctx.items, *ctx.drag_from, ctx.drag_indices) {
        run_section_drag(ui, ctx, visible_rows);
    } else {
        run_row_drag(ui, ctx, visible_rows);
    }

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

fn run_section_drag(ui: &egui::Ui, ctx: &mut RenderCtx<'_>, visible_rows: &[(usize, egui::Rect)]) {
    let Some(separator_idx) = *ctx.drag_from else {
        return;
    };
    let Some(&(_, band)) = visible_rows.iter().find(|(idx, _)| *idx == separator_idx) else {
        return;
    };
    let ghost_and_drop = ui
        .input(|input| input.pointer.latest_pos())
        .and_then(|cursor| {
            let ghost = band.translate(egui::vec2(
                0.0,
                cursor.y - *ctx.drag_grab_offset - band.top(),
            ));
            section_drag::section_drop_target(
                ctx.items,
                visible_rows,
                ghost.center().y,
                GROUP_GAP_PX,
            )
            .map(|drop| (ghost, drop))
        });

    if ui.input(|input| input.pointer.any_released()) {
        let moved = ghost_and_drop
            .is_some_and(|(_, drop)| move_section(ctx.items, separator_idx, drop.before));
        if !moved
            && ctx
                .undo_stack
                .last()
                .is_some_and(|entry| entry.items == *ctx.items)
        {
            ctx.undo_stack.pop();
        }
        ctx.selected.clear();
        *ctx.anchor = None;
        return;
    }
    let Some((ghost, drop)) = ghost_and_drop else {
        return;
    };

    let clip = ui.clip_rect();
    let painter = ui
        .ctx()
        .layer_painter(egui::LayerId::new(
            egui::Order::Tooltip,
            egui::Id::new(("step3_section_drag", ctx.tab_id)),
        ))
        .with_clip_rect(clip);
    painter.rect_filled(
        band,
        egui::CornerRadius::ZERO,
        redesign_with_alpha(redesign_shell_bg(ctx.palette), 3, 5),
    );
    painter.line_segment(
        [
            egui::pos2(clip.left(), drop.line_y),
            egui::pos2(
                (clip.right() - SCROLLBAR_RESERVE).max(clip.left()),
                drop.line_y,
            ),
        ],
        egui::Stroke::new(SECTION_DROP_LINE_PX, redesign_accent(ctx.palette)),
    );
    let mod_count = section_indices(ctx.items, separator_idx)
        .into_iter()
        .filter(|row| ctx.items[*row].is_mod_header())
        .count();
    paint_separator_ghost(
        ui,
        &painter,
        ctx.palette,
        ghost,
        &ctx.items[separator_idx].mod_name,
        mod_count,
    );
    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
}

fn run_row_drag(ui: &egui::Ui, ctx: &mut RenderCtx<'_>, visible_rows: &[(usize, egui::Rect)]) {
    if ctx.drag_from.is_some() && ui.input(|input| input.pointer.primary_down()) {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    }
    let mut pointer_ctx = service_step3::drag_ops::DragPointerContext {
        items: ctx.items,
        drag_from: ctx.drag_from.as_ref(),
        drag_over: ctx.drag_over,
        drag_indices: ctx.drag_indices,
        drag_grab_offset: ctx.drag_grab_offset,
        drag_row_h: ctx.drag_row_h,
        visible_rows,
    };
    service_step3::drag_ops::update_drag_target_from_pointer(ui, &mut pointer_ctx);

    paint_insert_marker_full_width(
        ui,
        ctx.items,
        ctx.drag_from
            .and_then(|grabbed| ctx.items.get(grabbed))
            .is_some_and(|grabbed| !grabbed.is_separator()),
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
}

fn flush_row_outcome(
    state: &mut WizardState,
    tab_id: &str,
    acc: &mut RowAccumulator,
) -> Option<MoveSelectionOutcome> {
    if let Some(row_index) = acc.open_installer_answers.take() {
        crate::ui::step3::installer_answers_step3::open_component(state, tab_id, row_index);
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
    match acc.separator_request.take() {
        Some(SeparatorRequest::AddAbove(idx)) => {
            toolbar_support_step3::add_separator_active(state, Some(idx));
        }
        Some(SeparatorRequest::Rename(idx)) => {
            toolbar_support_step3::start_separator_rename_active(state, idx);
        }
        Some(SeparatorRequest::Remove(idx)) => {
            toolbar_support_step3::remove_separator_active(state, idx);
        }
        None => {}
    }
    let Some((clicked_idx, target)) = acc.move_request.take() else {
        return acc
            .drag_refused_locked
            .then_some(MoveSelectionOutcome::RefusedLocked);
    };
    let (
        items,
        selected,
        _drag_from,
        _drag_over,
        _drag_indices,
        anchor,
        _drag_grab_offset,
        _drag_grab_pos_in_block,
        _drag_row_h,
        _last_insert_at,
        collapsed_blocks,
        clone_seq,
        locked_blocks,
        undo_stack,
        redo_stack,
    ) = state_step3::active_list_mut(state);
    let mut move_ctx = MoveSelectionContext {
        items,
        selected,
        anchor,
        clone_seq,
        locked_blocks: locked_blocks.as_slice(),
        collapsed_blocks: collapsed_blocks.as_slice(),
        undo_stack,
        redo_stack,
    };
    Some(move_selection(&mut move_ctx, clicked_idx, target))
}

fn paint_insert_marker_full_width(
    ui: &egui::Ui,
    items: &[Step3ItemState],
    drag_active: bool,
    drag_over: Option<usize>,
    visible_rows: &[(usize, egui::Rect)],
) {
    if !drag_active {
        return;
    }
    let Some(insert_at) = drag_over else { return };
    let row_rects: Vec<egui::Rect> = visible_rows.iter().map(|(_, r)| *r).collect();
    if row_rects.is_empty() {
        return;
    }
    let clip = ui.clip_rect();
    let x0 = clip.left();
    let x1 = (clip.right() - SCROLLBAR_RESERVE).max(x0);
    let clamped = insert_at.min(items.len());
    let y = if clamped == 0 {
        row_rects[0].top() - 1.0
    } else if clamped >= row_rects.len() {
        row_rects[row_rects.len() - 1].bottom() + 1.0
    } else {
        row_rects[clamped].top() - 1.0
    };
    ui.painter().line_segment(
        [egui::pos2(x0, y), egui::pos2(x1, y)],
        egui::Stroke::new(1.5_f32, ui.visuals().selection.stroke.color),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completed_move_marks_the_draft_unsaved_without_a_toast() {
        let mut app = OrchestratorApp::new_isolated_for_test("step3movedirty");
        let clean_before = !app.workspace_state_dirty;

        apply_move_outcome(&mut app, Some(MoveSelectionOutcome::Moved));

        let clean_after = !app.workspace_state_dirty;
        assert!(clean_before);
        assert!(!clean_after);
        assert_eq!(app.notification_manager.history().len(), 0);
    }

    #[test]
    fn refused_move_warns_once_and_leaves_the_draft_clean() {
        let mut app = OrchestratorApp::new_isolated_for_test("step3movelocked");

        apply_move_outcome(&mut app, Some(MoveSelectionOutcome::RefusedLocked));
        apply_move_outcome(&mut app, Some(MoveSelectionOutcome::NothingToMove));
        apply_move_outcome(&mut app, None);

        let clean_after = !app.workspace_state_dirty;
        assert!(clean_after);
        let history = app.notification_manager.history();
        assert_eq!(history.len(), 1);
        assert_eq!(
            history.back().map(|record| record.text.as_str()),
            Some(MOVE_LOCKED_NOTICE)
        );
    }

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

    #[test]
    fn build_parent_title_locked() {
        let title = build_parent_title("SomeMod", false, 3, true);
        assert_eq!(title, "SomeMod (3) [locked]");
    }

    #[test]
    fn build_parent_title_placeholder() {
        let title = build_parent_title("SomeMod", true, 2, false);
        assert_eq!(title, "SomeMod (split target) (2)");
    }

    struct DisplayFormatReset;

    impl Drop for DisplayFormatReset {
        fn drop(&mut self) {
            crate::app::mod_display_name::set_display_mod_names(false);
        }
    }

    fn with_display_format(show_names: bool) -> DisplayFormatReset {
        crate::app::mod_display_name::set_display_mod_names(show_names);
        DisplayFormatReset
    }

    #[test]
    fn step3_separator_title_is_never_formatted() {
        let _reset = with_display_format(false);
        let separator = crate::app::step3_separators::separator_row(
            "My Setup-Fixes.tp2 Section",
            "SEPARATOR::1".to_string(),
        );
        assert_eq!(
            parent_display_name(&separator),
            "My Setup-Fixes.tp2 Section"
        );
    }

    #[test]
    fn step3_mod_header_title_follows_the_display_format() {
        let _reset = with_display_format(false);
        let mut header = mod_item("FolderName");
        header.tp_file = "FolderName/Setup-CdTweaks.tp2".to_string();
        let title = build_parent_title(&parent_display_name(&header), false, 4, false);
        assert_eq!(title, "cdtweaks (4)");
        assert_eq!(header.mod_name, "FolderName");
    }

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

    #[test]
    fn group_width_uses_viewport_when_nothing_remembered() {
        assert!((group_width(700.0, 0.0) - 700.0).abs() < f32::EPSILON);
    }

    #[test]
    fn group_width_uses_remembered_when_wider() {
        assert!((group_width(700.0, 900.0) - 900.0).abs() < f32::EPSILON);
    }

    #[test]
    fn width_changed_ignores_a_small_jitter() {
        assert!(!width_changed(900.0, 900.2));
    }

    #[test]
    fn width_changed_true_for_a_real_shift() {
        assert!(width_changed(900.0, 700.0));
    }

    #[test]
    fn visible_dot_range_clips_into_the_middle_of_a_long_run() {
        let (start, end) = visible_dot_range(0.0, 3000.0, 7.0, 1000.0, 1100.0).unwrap();
        assert!((start - 1001.0).abs() < f32::EPSILON);
        assert!((end - 1099.0).abs() < f32::EPSILON);
    }

    #[test]
    fn visible_dot_range_is_none_when_nothing_is_visible() {
        assert!(visible_dot_range(0.0, 100.0, 7.0, 200.0, 300.0).is_none());
    }

    #[test]
    fn visible_dot_range_clamps_to_the_run_when_visible_is_wider() {
        let (start, end) = visible_dot_range(0.0, 100.0, 7.0, -50.0, 500.0).unwrap();
        assert!((start - 0.0).abs() < f32::EPSILON);
        assert!((end - 98.0).abs() < f32::EPSILON);
    }

    #[test]
    fn lineno_col_width_grows_with_digits() {
        let w1 = lineno_col_width(9);
        let w2 = lineno_col_width(10);
        assert!(w2 > w1, "two-digit column must be wider than one-digit");
    }

    #[test]
    fn undo_snapshot_records_state() {
        let items = vec![mod_item("ModA"), child_item("ModA", "1", "Component One")];
        let mut undo_stack: Vec<Step3HistoryEntry> = Vec::new();
        let mut redo_stack: Vec<Step3HistoryEntry> = Vec::new();
        step3_history::push_undo_snapshot(
            &items,
            Step3TouchedRows::default(),
            &mut undo_stack,
            &mut redo_stack,
        );
        assert_eq!(undo_stack.len(), 1);
        assert_eq!(undo_stack[0].items.len(), 2);
        assert_eq!(redo_stack.len(), 0);
    }

    #[test]
    fn undo_redo_roundtrip() {
        let original = vec![
            mod_item("ModA"),
            child_item("ModA", "1", "Component One"),
            child_item("ModA", "2", "Component Two"),
            child_item("ModA", "3", "Component Three"),
        ];
        let mut items = original.clone();
        let mut undo_stack: Vec<Step3HistoryEntry> = Vec::new();
        let mut redo_stack: Vec<Step3HistoryEntry> = Vec::new();

        step3_history::push_undo_snapshot(
            &items,
            Step3TouchedRows::default(),
            &mut undo_stack,
            &mut redo_stack,
        );
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
