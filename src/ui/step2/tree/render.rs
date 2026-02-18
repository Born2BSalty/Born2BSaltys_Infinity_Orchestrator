// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::state::{Step2ModState, Step2Selection};

use super::render_components::render_component_rows;
use super::render_filter::{finalize_mod_checked_state, mod_matches_filter};
use super::render_parent::render_parent_row;

pub struct ModTreeRenderResult {
    pub selected: Step2Selection,
    pub open_compat_for_component: Option<(String, String, String)>,
}

pub fn render_mod_tree(
    ui: &mut egui::Ui,
    filter: &str,
    active_tab: &str,
    selected: &Option<Step2Selection>,
    next_selection_order: &mut usize,
    collapse_epoch: u64,
    collapse_default_open: bool,
    jump_to_selected_requested: &mut bool,
    mod_state: &mut Step2ModState,
) -> Option<ModTreeRenderResult> {
    if !mod_matches_filter(mod_state, filter) {
        return None;
    }

    let header_id = egui::Id::new((
        "mod_header",
        collapse_epoch,
        &mod_state.tp_file,
        &mod_state.name,
        &mod_state.tp2_path,
    ));
    let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(
        ui.ctx(),
        header_id,
        collapse_default_open,
    );
    if *jump_to_selected_requested
        && selection_targets_mod(selected, active_tab, &mod_state.tp_file)
    {
        state.set_open(true);
    }

    let mut new_selection: Option<Step2Selection> = None;
    let mut open_compat_for_component: Option<(String, String, String)> = None;
    state
        .show_header(ui, |ui| {
            let parent_row = render_parent_row(
                ui,
                mod_state,
                active_tab,
                selected,
                next_selection_order,
                jump_to_selected_requested,
            );
            if parent_row.selection.is_some() {
                new_selection = parent_row.selection;
            }
            if parent_row.open_compat_for_component.is_some() {
                open_compat_for_component = parent_row.open_compat_for_component;
            }
        })
        .body(|ui| {
            let (selection_from_rows, compat_target) = render_component_rows(
                ui,
                filter,
                active_tab,
                selected,
                next_selection_order,
                jump_to_selected_requested,
                mod_state,
            );
            if selection_from_rows.is_some() {
                new_selection = selection_from_rows;
            }
            if compat_target.is_some() {
                open_compat_for_component = compat_target;
            }
        });

    finalize_mod_checked_state(mod_state);
    new_selection.map(|selected| ModTreeRenderResult {
        selected,
        open_compat_for_component,
    })
}

fn selection_targets_mod(
    selected: &Option<Step2Selection>,
    active_tab: &str,
    tp_file: &str,
) -> bool {
    match selected {
        Some(Step2Selection::Mod { game_tab, tp_file: selected_tp }) => {
            game_tab == active_tab && selected_tp == tp_file
        }
        Some(Step2Selection::Component {
            game_tab,
            tp_file: selected_tp,
            ..
        }) => game_tab == active_tab && selected_tp == tp_file,
        None => false,
    }
}
