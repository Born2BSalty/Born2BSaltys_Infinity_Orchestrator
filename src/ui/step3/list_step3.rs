// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;
use std::collections::HashMap;

use crate::app::compat_step3_rules::Step3CompatMarker;
use crate::app::prompt_eval_context::build_prompt_eval_context;
use crate::app::state::{Step2Selection, Step3ItemState, WizardState};
use crate::ui::step3::blocks;
use crate::ui::step3::list_rows_step3::{RowRenderContext, RowRenderOutcome, render_rows};
use crate::ui::step3::state_step3;

pub(crate) fn render(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    jump_to_selected_requested: &mut bool,
    compat_markers: &HashMap<String, Step3CompatMarker>,
) {
    ui.group(|ui| {
        render_list_group(ui, state, jump_to_selected_requested, compat_markers);
    });

    crate::ui::step3::service_step3::prompt_actions::render(ui, state);
}

fn render_list_group(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    jump_to_selected_requested: &mut bool,
    compat_markers: &HashMap<String, Step3CompatMarker>,
) {
    ui.set_width(ui.available_width());
    let nav_clearance = 26.0;
    let list_height = (ui.available_height() - nav_clearance).max(180.0);
    let viewport_w = ui.available_width();
    ui.scope(|ui| {
        configure_scroll_style(ui);
        egui::ScrollArea::both()
            .id_salt("step3_list_scroll")
            .auto_shrink([false, false])
            .max_height(list_height)
            .show(ui, |ui| {
                render_scroll_contents(
                    ui,
                    state,
                    jump_to_selected_requested,
                    compat_markers,
                    viewport_w,
                );
            });
    });
}

fn configure_scroll_style(ui: &mut egui::Ui) {
    let mut scroll = egui::style::ScrollStyle::solid();
    scroll.bar_width = 12.0;
    scroll.bar_inner_margin = 0.0;
    scroll.bar_outer_margin = 2.0;
    ui.style_mut().spacing.scroll = scroll;
}

fn render_scroll_contents(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    jump_to_selected_requested: &mut bool,
    compat_markers: &HashMap<String, Step3CompatMarker>,
    viewport_w: f32,
) {
    ui.set_min_width(viewport_w);
    let tab_id = state.step3.active_game_tab.clone();
    let prompt_eval = build_prompt_eval_context(state);
    let Some(row_outcome) = render_active_rows(
        ui,
        state,
        jump_to_selected_requested,
        compat_markers,
        &tab_id,
        &prompt_eval,
    ) else {
        return;
    };
    apply_row_outcome(state, &tab_id, row_outcome);
}

fn render_active_rows(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    jump_to_selected_requested: &mut bool,
    compat_markers: &HashMap<String, Step3CompatMarker>,
    tab_id: &str,
    prompt_eval: &crate::parser::prompt_eval_expr::PromptEvalContext,
) -> Option<RowRenderOutcome> {
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
        return None;
    }
    let visible_indices = blocks::visible_indices(items, collapsed_blocks);
    let row_outcome = {
        let mut row_ctx = RowRenderContext {
            prompt_eval,
            compat_markers,
            tab_id,
            visible_indices: &visible_indices,
            jump_to_selected_requested,
            items: &mut *items,
            selected: &mut *selected,
            drag_from: &mut *drag_from,
            drag_over: &mut *drag_over,
            drag_indices: &mut *drag_indices,
            anchor: &mut *anchor,
            drag_grab_offset: &mut *drag_grab_offset,
            drag_grab_pos_in_block: &mut *drag_grab_pos_in_block,
            drag_row_h: &mut *drag_row_h,
            last_insert_at: &mut *last_insert_at,
            collapsed_blocks: &mut *collapsed_blocks,
            clone_seq: &mut *clone_seq,
            locked_blocks: &mut *locked_blocks,
            undo_stack: &mut *undo_stack,
            redo_stack: &mut *redo_stack,
        };
        render_rows(ui, &mut row_ctx)
    };
    let mut drag_pipeline_ctx = DragPipelineContext {
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
        locked_blocks,
        visible_rows: &row_outcome.visible_rows,
    };
    run_drag_pipeline(ui, &mut drag_pipeline_ctx);
    Some(row_outcome)
}

struct DragPipelineContext<'a> {
    items: &'a mut Vec<Step3ItemState>,
    selected: &'a mut Vec<usize>,
    drag_from: &'a mut Option<usize>,
    drag_over: &'a mut Option<usize>,
    drag_indices: &'a mut Vec<usize>,
    drag_grab_offset: &'a mut f32,
    drag_grab_pos_in_block: &'a mut usize,
    drag_row_h: &'a mut f32,
    last_insert_at: &'a mut Option<usize>,
    clone_seq: &'a mut usize,
    locked_blocks: &'a mut Vec<String>,
    visible_rows: &'a [(usize, egui::Rect)],
}

fn run_drag_pipeline(ui: &egui::Ui, ctx: &mut DragPipelineContext<'_>) {
    let mut pointer_ctx = crate::ui::step3::service_step3::drag_ops::DragPointerContext {
        items: &mut *ctx.items,
        drag_from: ctx.drag_from.as_ref(),
        drag_over: &mut *ctx.drag_over,
        drag_indices: &mut *ctx.drag_indices,
        drag_grab_offset: &mut *ctx.drag_grab_offset,
        drag_grab_pos_in_block: &mut *ctx.drag_grab_pos_in_block,
        drag_row_h: &mut *ctx.drag_row_h,
        visible_rows: ctx.visible_rows,
    };
    crate::ui::step3::service_step3::drag_ops::update_drag_target_from_pointer(
        ui,
        &mut pointer_ctx,
    );
    crate::ui::step3::service_step3::drag_ops::draw_insert_marker(
        ui,
        &*ctx.items,
        ctx.drag_from.is_some(),
        *ctx.drag_over,
        ctx.visible_rows,
    );
    let mut reorder_ctx = crate::ui::step3::service_step3::drag_ops::LiveReorderContext {
        items: &mut *ctx.items,
        selected: &mut *ctx.selected,
        drag_from: &mut *ctx.drag_from,
        drag_over: &mut *ctx.drag_over,
        drag_indices: &mut *ctx.drag_indices,
        drag_grab_pos_in_block: &mut *ctx.drag_grab_pos_in_block,
        last_insert_at: &mut *ctx.last_insert_at,
        locked_blocks: &mut *ctx.locked_blocks,
        visible_rows: ctx.visible_rows,
    };
    crate::ui::step3::service_step3::drag_ops::apply_live_reorder(ui, &mut reorder_ctx);
    let mut finalize_ctx = crate::ui::step3::service_step3::drag_ops::DragFinalizeContext {
        items: &mut *ctx.items,
        selected: &mut *ctx.selected,
        drag_from: &mut *ctx.drag_from,
        drag_over: &mut *ctx.drag_over,
        drag_indices: &mut *ctx.drag_indices,
        drag_grab_offset: &mut *ctx.drag_grab_offset,
        drag_grab_pos_in_block: &mut *ctx.drag_grab_pos_in_block,
        drag_row_h: &mut *ctx.drag_row_h,
        last_insert_at: &mut *ctx.last_insert_at,
        clone_seq: &mut *ctx.clone_seq,
    };
    crate::ui::step3::service_step3::drag_ops::finalize_on_release(ui, &mut finalize_ctx);
}

fn apply_row_outcome(state: &mut WizardState, tab_id: &str, row_outcome: RowRenderOutcome) {
    let RowRenderOutcome {
        visible_rows: _,
        uncheck_requests,
        prompt_requests,
        open_prompt_popup,
        open_compat_popup,
    } = row_outcome;
    if let Some((title, text)) = open_prompt_popup {
        crate::ui::step2::prompt_popup_step2::open_text_prompt_popup(state, title, text);
    }
    if let Some((tp_file, component_id, component_key, issue)) = open_compat_popup {
        state.step2.selected = Some(Step2Selection::Component {
            game_tab: tab_id.to_string(),
            tp_file,
            component_id,
            component_key,
        });
        state.step2.compat_popup_issue_override = Some(issue);
        state.step2.compat_popup_open = true;
    }
    if !uncheck_requests.is_empty() {
        crate::ui::step3::service_step3::component_uncheck::apply_component_unchecks(
            state,
            tab_id,
            &uncheck_requests,
        );
    }
    if !prompt_requests.is_empty() {
        crate::ui::step3::service_step3::prompt_actions::apply_prompt_actions(
            state,
            &prompt_requests,
        );
    }
}
