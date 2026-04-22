// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::WizardState;

pub fn render(ui: &mut egui::Ui, state: &mut WizardState) {
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
            let details_h = (ui.available_height() - 58.0).max(40.0);
            egui::ScrollArea::vertical()
                .max_height(details_h)
                .show(ui, |ui| {
                    crate::ui::step2::content_step2::compat_popup_details::render_details(
                        ui, state,
                    );
                });

            ui.add_space(10.0);
            crate::ui::step2::content_step2::compat_popup_action_row::render_action_row(ui, state);
        });
    state.step2.compat_popup_open = open && state.step2.compat_popup_open;
    if !state.step2.compat_popup_open {
        state.step2.compat_popup_issue_override = None;
    }
}
