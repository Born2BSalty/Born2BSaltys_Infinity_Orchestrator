// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::controller::util::open_in_shell;
use crate::ui::state::WizardState;

use super::actions;
use super::issue_text;
use super::selection;

pub(super) fn render_action_row(ui: &mut egui::Ui, state: &mut WizardState) {
    ui.horizontal(|ui| {
        if ui.button("Jump To Other").clicked() {
            actions::jump_to_related_component(state);
            state.step2.jump_to_selected_requested = true;
        }
        if ui.button("Jump To This").clicked() {
            actions::jump_to_affected_component(state);
            state.step2.jump_to_selected_requested = true;
        }
        if ui.button("Select Other").clicked() {
            actions::jump_to_related_component(state);
        }
        if ui.button("Next Issue").clicked() {
            actions::jump_to_next_conflict(state);
            state.step2.jump_to_selected_requested = true;
        }
        if ui.button("Copy Issue").clicked()
            && let Some(issue) = selection::current_issue_for_selection(state)
        {
            ui.ctx().copy_text(issue_text::format_issue_for_copy(&issue));
        }
        let source_path = selection::rule_source_open_path(state);
        let open_source_resp = ui.add_enabled(source_path.is_some(), egui::Button::new("Open Rule Source"));
        if let Some(path) = source_path {
            if open_source_resp.clicked()
                && let Err(err) = open_in_shell(&path)
            {
                state.step2.scan_status = format!("Open failed: {err}");
            }
        }
        if ui.button("Close").clicked() {
            state.step2.compat_popup_open = false;
        }
    });
}
