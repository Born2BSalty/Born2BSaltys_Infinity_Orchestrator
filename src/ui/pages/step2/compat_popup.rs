// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::state::WizardState;

mod action_row;
mod actions;
mod details;
mod filter_row;
mod filters;
mod issue_text;
mod selection;

pub(super) fn render(ui: &mut egui::Ui, state: &mut WizardState) {
    if !state.step2.compat_popup_open {
        return;
    }

    let mut open = state.step2.compat_popup_open;
    egui::Window::new("Step 2 Compatibility")
        .open(&mut open)
        .collapsible(false)
        .resizable(true)
        .movable(true)
        .default_size(egui::vec2(620.0, 300.0))
        .min_width(420.0)
        .min_height(200.0)
        .show(ui.ctx(), |ui| {
            details::render_details(ui, state);

            ui.add_space(10.0);
            filter_row::render_filter_row(ui, state);
            ui.add_space(6.0);
            action_row::render_action_row(ui, state);
        });
    state.step2.compat_popup_open = open && state.step2.compat_popup_open;
}
