// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::{Step2ModState, Step2Selection};
use crate::parser::prompt_eval_expr::PromptEvalContext;
use crate::ui::shared::redesign_tokens::ThemePalette;
use crate::ui::step2::service_list_ops_step2::mod_matches_filter;
use crate::ui::step2::tree_component_types_step2::{ComponentRowsContext, ComponentRowsResult};
use crate::ui::step2::tree_components_step2::render_component_rows;
use crate::ui::step2::tree_parent_step2::{ParentRowInput, ParentRowResult, render_parent_row};

pub struct ModTreeRenderResult {
    pub selected: Step2Selection,
    pub open_compat_for_component: Option<(String, String, String)>,
    pub open_prompt_popup: Option<(String, String)>,
    pub open_details: bool,
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
    pub palette: ThemePalette,
}

#[derive(Default)]
struct ModTreePending {
    selection: Option<Step2Selection>,
    open_compat_for_component: Option<(String, String, String)>,
    open_prompt_popup: Option<(String, String)>,
    open_details: bool,
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
        && selection_targets_mod(ctx.selected.as_ref(), ctx.active_tab, &mod_state.tp_file)
    {
        state.set_open(true);
    }

    let mut pending = ModTreePending::default();
    state
        .show_header(ui, |ui| render_mod_header(ui, ctx, mod_state, &mut pending))
        .body(|ui| render_mod_body(ui, ctx, mod_state, &mut pending));

    finalize_mod_checked_state(mod_state);
    finish_mod_tree_result(ctx, mod_state, pending)
}

fn render_mod_header(
    ui: &mut egui::Ui,
    ctx: &mut ModTreeRenderContext<'_>,
    mod_state: &mut Step2ModState,
    pending: &mut ModTreePending,
) {
    let result = render_parent_row(
        ui,
        mod_state,
        ParentRowInput {
            active_tab: ctx.active_tab,
            selected: ctx.selected.as_ref(),
            next_selection_order: ctx.next_selection_order,
            prompt_eval: ctx.prompt_eval,
            jump_to_selected_requested: ctx.jump_to_selected_requested,
            palette: ctx.palette,
        },
    );
    pending.apply_parent(result);
}

fn render_mod_body(
    ui: &mut egui::Ui,
    ctx: &mut ModTreeRenderContext<'_>,
    mod_state: &mut Step2ModState,
    pending: &mut ModTreePending,
) {
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
        palette: ctx.palette,
    };
    let row_result = render_component_rows(ui, &mut row_ctx, mod_state);
    pending.apply_components(row_result);
}

fn finish_mod_tree_result(
    ctx: &ModTreeRenderContext<'_>,
    mod_state: &Step2ModState,
    pending: ModTreePending,
) -> Option<ModTreeRenderResult> {
    if !pending.has_action() {
        return None;
    }
    let selected = pending.selection.unwrap_or_else(|| Step2Selection::Mod {
        game_tab: ctx.active_tab.to_string(),
        tp_file: mod_state.tp_file.clone(),
    });
    Some(ModTreeRenderResult {
        selected,
        open_compat_for_component: pending.open_compat_for_component,
        open_prompt_popup: pending.open_prompt_popup,
        open_details: pending.open_details,
    })
}

impl ModTreePending {
    fn apply_parent(&mut self, result: ParentRowResult) {
        if result.selection.is_some() {
            self.selection = result.selection;
        }
        if result.open_compat_for_component.is_some() {
            self.open_compat_for_component = result.open_compat_for_component;
        }
        if result.open_prompt_popup.is_some() {
            self.open_prompt_popup = result.open_prompt_popup;
        }
        self.open_details |= result.open_details;
    }

    fn apply_components(&mut self, result: ComponentRowsResult) {
        if result.selection.is_some() {
            self.selection = result.selection;
        }
        if result.compat_popup.is_some() {
            self.open_compat_for_component = result.compat_popup;
        }
        if result.prompt_popup.is_some() {
            self.open_prompt_popup = result.prompt_popup;
        }
        self.open_details |= result.open_details;
    }

    const fn has_action(&self) -> bool {
        self.selection.is_some()
            || self.open_compat_for_component.is_some()
            || self.open_prompt_popup.is_some()
            || self.open_details
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
    selected: Option<&Step2Selection>,
    active_tab: &str,
    tp_file: &str,
) -> bool {
    match selected {
        Some(
            Step2Selection::Mod {
                game_tab,
                tp_file: selected_tp,
            }
            | Step2Selection::Component {
                game_tab,
                tp_file: selected_tp,
                ..
            },
        ) => game_tab == active_tab && selected_tp == tp_file,
        None => false,
    }
}
