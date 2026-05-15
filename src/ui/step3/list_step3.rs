// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;
use std::collections::HashMap;

use crate::app::compat_step3_rules::Step3CompatMarker;
use crate::app::prompt_eval_context::build_prompt_eval_context;
use crate::app::state::Step2Selection;
use crate::app::state::WizardState;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BIO_SCROLL_BAR_WIDTH_PX, REDESIGN_BIO_SCROLL_INNER_MARGIN_PX,
    REDESIGN_BIO_SCROLL_OUTER_MARGIN_PX, ThemePalette,
};
use crate::ui::step3::blocks;
use crate::ui::step3::list_rows_step3::{RowRenderContext, render_rows};
use crate::ui::step3::state_step3;

pub(crate) fn render(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    jump_to_selected_requested: &mut bool,
    compat_markers: &HashMap<String, Step3CompatMarker>,
    palette: ThemePalette,
) {
    ui.group(|ui| {
        render_group_content(
            ui,
            state,
            jump_to_selected_requested,
            compat_markers,
            palette,
        );
    });

    crate::ui::step3::service_step3::prompt_actions::render(ui, state);
}

fn render_group_content(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    jump_to_selected_requested: &mut bool,
    compat_markers: &HashMap<String, Step3CompatMarker>,
    palette: ThemePalette,
) {
    ui.set_width(ui.available_width());
    let nav_clearance = 26.0;
    let list_height = (ui.available_height() - nav_clearance).max(180.0);
    let viewport_w = ui.available_width();
    ui.scope(|ui| {
        let mut scroll = egui::style::ScrollStyle::solid();
        scroll.bar_width = REDESIGN_BIO_SCROLL_BAR_WIDTH_PX;
        scroll.bar_inner_margin = REDESIGN_BIO_SCROLL_INNER_MARGIN_PX;
        scroll.bar_outer_margin = REDESIGN_BIO_SCROLL_OUTER_MARGIN_PX;
        ui.style_mut().spacing.scroll = scroll;
        egui::ScrollArea::both()
            .id_salt("step3_list_scroll")
            .auto_shrink([false, false])
            .max_height(list_height)
            .show(ui, |ui| {
                ui.set_min_width(viewport_w);
                render_scroll_content(
                    ui,
                    state,
                    jump_to_selected_requested,
                    compat_markers,
                    palette,
                );
            });
    });
}

fn render_scroll_content(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    jump_to_selected_requested: &mut bool,
    compat_markers: &HashMap<String, Step3CompatMarker>,
    palette: ThemePalette,
) {
    let tab_id = state.step3.active_game_tab.clone();
    let prompt_eval = build_prompt_eval_context(state);
    let Some(outcome) = render_active_rows(
        ui,
        state,
        jump_to_selected_requested,
        compat_markers,
        palette,
        &tab_id,
        &prompt_eval,
    ) else {
        return;
    };
    apply_row_outcome(state, &tab_id, outcome);
}

struct ActiveRowOutcome {
    pending_unchecks: Vec<(String, String)>,
    pending_prompt_actions: Vec<crate::app::step3_prompt_edit::PromptActionRequest>,
    open_prompt_popup: Option<(String, String)>,
    open_compat_popup: Option<(
        String,
        String,
        String,
        crate::app::compat_issue::CompatIssue,
    )>,
}

fn render_active_rows(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    jump_to_selected_requested: &mut bool,
    compat_markers: &HashMap<String, Step3CompatMarker>,
    palette: ThemePalette,
    tab_id: &str,
    prompt_eval: &crate::parser::PromptEvalContext,
) -> Option<ActiveRowOutcome> {
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
    let mut row_ctx = RowRenderContext {
        prompt_eval,
        compat_markers,
        tab_id,
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
        palette,
    };
    let row_outcome = render_rows(ui, &mut row_ctx);
    let mut visible_rows = row_outcome.visible_rows;
    update_drag_state(ui, &mut row_ctx, &visible_rows, palette);
    visible_rows.clear();
    Some(ActiveRowOutcome {
        pending_unchecks: row_outcome.uncheck_requests,
        pending_prompt_actions: row_outcome.prompt_requests,
        open_prompt_popup: row_outcome.open_prompt_popup,
        open_compat_popup: row_outcome.open_compat_popup,
    })
}

fn update_drag_state(
    ui: &egui::Ui,
    row_ctx: &mut RowRenderContext<'_>,
    visible_rows: &[(usize, egui::Rect)],
    palette: ThemePalette,
) {
    let mut pointer_ctx = crate::ui::step3::service_step3::drag_ops::DragPointerContext {
        items: &mut *row_ctx.items,
        drag_from: &mut *row_ctx.drag_from,
        drag_over: &mut *row_ctx.drag_over,
        drag_indices: &mut *row_ctx.drag_indices,
        drag_grab_offset: &mut *row_ctx.drag_grab_offset,
        drag_grab_pos_in_block: &mut *row_ctx.drag_grab_pos_in_block,
        drag_row_h: &mut *row_ctx.drag_row_h,
        visible_rows,
    };
    crate::ui::step3::service_step3::drag_ops::update_drag_target_from_pointer(
        ui,
        &mut pointer_ctx,
    );
    crate::ui::step3::service_step3::drag_ops::draw_insert_marker(
        ui,
        &*row_ctx.items,
        row_ctx.drag_from.as_ref(),
        *row_ctx.drag_over,
        visible_rows,
        palette,
    );
    let mut reorder_ctx = crate::ui::step3::service_step3::drag_ops::LiveReorderContext {
        items: &mut *row_ctx.items,
        selected: &mut *row_ctx.selected,
        drag_from: &mut *row_ctx.drag_from,
        drag_over: &mut *row_ctx.drag_over,
        drag_indices: &mut *row_ctx.drag_indices,
        drag_grab_pos_in_block: &mut *row_ctx.drag_grab_pos_in_block,
        last_insert_at: &mut *row_ctx.last_insert_at,
        locked_blocks: &mut *row_ctx.locked_blocks,
        visible_rows,
    };
    crate::ui::step3::service_step3::drag_ops::apply_live_reorder(ui, &mut reorder_ctx);
    let mut finalize_ctx = crate::ui::step3::service_step3::drag_ops::DragFinalizeContext {
        items: &mut *row_ctx.items,
        selected: &mut *row_ctx.selected,
        drag_from: &mut *row_ctx.drag_from,
        drag_over: &mut *row_ctx.drag_over,
        drag_indices: &mut *row_ctx.drag_indices,
        drag_grab_offset: &mut *row_ctx.drag_grab_offset,
        drag_grab_pos_in_block: &mut *row_ctx.drag_grab_pos_in_block,
        drag_row_h: &mut *row_ctx.drag_row_h,
        last_insert_at: &mut *row_ctx.last_insert_at,
        clone_seq: &mut *row_ctx.clone_seq,
    };
    let _finalized_drag =
        crate::ui::step3::service_step3::drag_ops::finalize_on_release(ui, &mut finalize_ctx);
}

fn apply_row_outcome(state: &mut WizardState, tab_id: &str, outcome: ActiveRowOutcome) {
    if let Some((title, text)) = outcome.open_prompt_popup {
        crate::ui::step2::prompt_popup_step2::open_text_prompt_popup(state, title, text);
    }
    if let Some((tp_file, component_id, component_key, issue)) = outcome.open_compat_popup {
        state.step2.selected = Some(Step2Selection::Component {
            game_tab: tab_id.to_string(),
            tp_file,
            component_id,
            component_key,
        });
        state.step2.compat_popup_issue_override = Some(issue);
        state.step2.compat_popup_open = true;
    }
    if !outcome.pending_unchecks.is_empty() {
        crate::ui::step3::service_step3::component_uncheck::apply_component_unchecks(
            state,
            tab_id,
            &outcome.pending_unchecks,
        );
    }
    if !outcome.pending_prompt_actions.is_empty() {
        crate::ui::step3::service_step3::prompt_actions::apply_prompt_actions(
            state,
            &outcome.pending_prompt_actions,
        );
    }
}
