// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use super::super::WizardApp;
use super::logic;

pub(super) fn render(app: &mut WizardApp, ctx: &egui::Context) {
    if app.state.step1_clean_confirm_open {
        egui::Window::new("Confirm Clean Targets")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.label("Target dir(s) will be cleaned before fresh install.");
                ui.label("Continue?");
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Yes").clicked() {
                        app.state.step1_clean_confirm_open = false;
                        logic::advance_after_next(app);
                    }
                    if ui.button("No").clicked() {
                        app.state.step1_clean_confirm_open = false;
                    }
                });
            });
    }

    if app.state.step4_save_error_open {
        egui::Window::new("Could not continue to Step 5")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.label("BIO could not save weidu.log from Step 4.");
                ui.add_space(6.0);
                ui.label(app.state.step4_save_error_text.clone());
                ui.add_space(10.0);
                if ui.button("OK").clicked() {
                    app.state.step4_save_error_open = false;
                }
            });
    }
}
