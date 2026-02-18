// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::state::WizardState;

mod actions;
mod render_buttons;

use super::super::WizardApp;

pub(super) fn render(app: &mut WizardApp, ctx: &egui::Context) {
    let can_next = app.can_advance_from_current_step();
    let on_last_step = app.state.current_step + 1 == WizardState::STEP_COUNT;
    let step5_install_running = app.state.current_step == 4 && app.state.step5.install_running;
    let right_margin = -19.0;
    egui::Area::new("wizard_nav_buttons".into())
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(right_margin, -4.0))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                render_buttons::render_reset_button(ui, app);
                render_buttons::render_back_button(ui, app, step5_install_running);
                if on_last_step {
                    render_buttons::render_exit_button(ui, app, step5_install_running);
                } else {
                    render_buttons::render_next_block(ui, app, can_next);
                }
            });
        });
}
