// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::{Step2ModState, Step2Selection};
use crate::parser::prompt_eval_expr::PromptEvalContext;
use crate::ui::step2::service_list_ops_step2::mod_matches_filter;
use crate::ui::step2::tree_component_types_step2::ComponentRowsContext;
use crate::ui::step2::tree_components_step2::render_component_rows;
use crate::ui::step2::tree_parent_step2::{ParentRowResult, render_parent_row};

pub struct ModTreeRenderResult {
    pub selected: Step2Selection,
    pub open_compat_for_component: Option<(String, String, String)>,
    pub open_prompt_popup: Option<(String, String)>,
}

pub struct ModTreeRenderContext<'a> {
    pub filter: &'a str,
    pub active_tab: &'a str,
    pub selected: &'a Option<Step2Selection>,
    pub next_selection_order: &'a mut usize,
    pub prompt_eval: &'a PromptEvalContext,
    pub collapse_epoch: u64,
    pub collapse_default_open: bool,
    pub jump_to_selected_requested: &'a mut bool,
}

pub fn render_mod_tree(
    ui: &mut egui::Ui,
    ctx: &mut ModTreeRenderContext<'_>,
    mod_state: &mut Step2ModState,
) -> Option<ModTreeRenderResult> {
    if !mod_matches_filter(mod_state, ctx.filter) {
        return None;
    }

    let header_id = egui::Id::new((
        "mod_header",
        ctx.collapse_epoch,
        &mod_state.tp_file,
        &mod_state.name,
        &mod_state.tp2_path,
    ));
    let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(
        ui.ctx(),
        header_id,
        ctx.collapse_default_open,
    );
    if *ctx.jump_to_selected_requested
        && selection_targets_mod(ctx.selected, ctx.active_tab, &mod_state.tp_file)
    {
        state.set_open(true);
    }

    let mut new_selection: Option<Step2Selection> = None;
    let mut open_compat_for_component: Option<(String, String, String)> = None;
    let mut open_prompt_popup: Option<(String, String)> = None;
    state
        .show_header(ui, |ui| {
            let ParentRowResult {
                selection,
                open_compat_for_component: parent_compat,
                open_prompt_popup: parent_prompt,
            } = render_parent_row(
                ui,
                mod_state,
                ctx.active_tab,
                ctx.selected,
                ctx.next_selection_order,
                ctx.prompt_eval,
                ctx.jump_to_selected_requested,
            );
            if selection.is_some() {
                new_selection = selection;
            }
            if parent_compat.is_some() {
                open_compat_for_component = parent_compat;
            }
            if parent_prompt.is_some() {
                open_prompt_popup = parent_prompt;
            }
        })
        .body(|ui| {
            let tp_file = mod_state.tp_file.clone();
            let mod_name = mod_state.name.clone();
            let mut row_ctx = ComponentRowsContext {
                filter: ctx.filter,
                active_tab: ctx.active_tab,
                selected: ctx.selected,
                next_selection_order: ctx.next_selection_order,
                prompt_eval: ctx.prompt_eval,
                collapse_epoch: ctx.collapse_epoch,
                collapse_default_open: ctx.collapse_default_open,
                jump_to_selected_requested: ctx.jump_to_selected_requested,
                tp_file: &tp_file,
                mod_name: &mod_name,
            };
            let row_result = render_component_rows(ui, &mut row_ctx, mod_state);
            if row_result.selection.is_some() {
                new_selection = row_result.selection;
            }
            if row_result.compat_popup.is_some() {
                open_compat_for_component = row_result.compat_popup;
            }
            if row_result.prompt_popup.is_some() {
                open_prompt_popup = row_result.prompt_popup;
            }
        });

    finalize_mod_checked_state(mod_state);
    if new_selection.is_some() || open_compat_for_component.is_some() || open_prompt_popup.is_some()
    {
        let selected = new_selection.unwrap_or_else(|| Step2Selection::Mod {
            game_tab: ctx.active_tab.to_string(),
            tp_file: mod_state.tp_file.clone(),
        });
        Some(ModTreeRenderResult {
            selected,
            open_compat_for_component,
            open_prompt_popup,
        })
    } else {
        None
    }
}

fn finalize_mod_checked_state(mod_state: &mut Step2ModState) {
    let has_components = !mod_state.components.is_empty();
    mod_state.checked = has_components
        && mod_state.components.iter().filter(|c| !c.disabled).count() > 0
        && mod_state
            .components
            .iter()
            .filter(|component| !component.disabled)
            .all(|component| component.checked);
}

fn selection_targets_mod(
    selected: &Option<Step2Selection>,
    active_tab: &str,
    tp_file: &str,
) -> bool {
    match selected {
        Some(Step2Selection::Mod {
            game_tab,
            tp_file: selected_tp,
        }) => game_tab == active_tab && selected_tp == tp_file,
        Some(Step2Selection::Component {
            game_tab,
            tp_file: selected_tp,
            ..
        }) => game_tab == active_tab && selected_tp == tp_file,
        None => false,
    }
}
