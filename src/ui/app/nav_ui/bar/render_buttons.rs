// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::layout::{NAV_BUTTON_HEIGHT, NAV_BUTTON_WIDTH};
use crate::ui::step1::validation::run_path_check;

use crate::ui::app::WizardApp;

use super::actions;

pub(super) fn render_reset_button(ui: &mut egui::Ui, app: &mut WizardApp) {
    if app.state.current_step == 1
        && ui
            .add_sized(
                egui::vec2(NAV_BUTTON_WIDTH + 24.0, NAV_BUTTON_HEIGHT),
                egui::Button::new("Reset Wizard State"),
            )
            .on_hover_text("Clear scan/selection/order/install state and return to Step 1.")
            .clicked()
    {
        actions::handle_reset(app);
    }
}

pub(super) fn render_back_button(ui: &mut egui::Ui, app: &mut WizardApp, step5_install_running: bool) {
    if app.state.current_step > 0 {
        let back_enabled = app.state.can_go_back() && !step5_install_running;
        if back_enabled {
            let back_resp = ui
                .add_sized(
                    egui::vec2(NAV_BUTTON_WIDTH, NAV_BUTTON_HEIGHT),
                    egui::Button::new("Back"),
                )
                .on_hover_text("Go to previous step.");
            if back_resp.clicked() {
                actions::handle_back(app);
            }
        } else {
            ui.add_sized(
                egui::vec2(NAV_BUTTON_WIDTH, NAV_BUTTON_HEIGHT),
                egui::Button::new(egui::RichText::new("Back").color(egui::Color32::from_gray(120)))
                    .fill(egui::Color32::from_gray(45))
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(70))),
            )
            .on_hover_text("Disabled while install is running.");
        }
    }
}

pub(super) fn render_exit_button(ui: &mut egui::Ui, app: &mut WizardApp, step5_install_running: bool) {
    if step5_install_running {
        ui.add_sized(
            egui::vec2(NAV_BUTTON_WIDTH, NAV_BUTTON_HEIGHT),
            egui::Button::new(egui::RichText::new("Exit").color(egui::Color32::from_gray(120)))
                .fill(egui::Color32::from_gray(45))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(70))),
        )
        .on_hover_text("Disabled while install is running.");
    } else if ui
        .add_sized(
            egui::vec2(NAV_BUTTON_WIDTH, NAV_BUTTON_HEIGHT),
            egui::Button::new("Exit"),
        )
        .on_hover_text("Close the wizard.")
        .clicked()
    {
        app.shutdown_and_exit();
    }
}

pub(super) fn render_next_block(ui: &mut egui::Ui, app: &mut WizardApp, can_next: bool) {
    let mut next_clicked = false;
    ui.vertical(|ui| {
        if app.state.current_step == 0 {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add_sized(
                        egui::vec2(NAV_BUTTON_WIDTH, NAV_BUTTON_HEIGHT),
                        egui::Button::new("Test Paths"),
                    )
                    .on_hover_text("Validate required paths and files for the selected game mode.")
                    .clicked()
                {
                    app.state.step1_path_check = Some(run_path_check(&app.state.step1));
                }
            });
        }

        if app.state.current_step == 0 {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let next_resp = ui.add_enabled_ui(can_next, |ui| {
                    ui.add_sized(
                        egui::vec2(NAV_BUTTON_WIDTH, NAV_BUTTON_HEIGHT),
                        egui::Button::new("Next"),
                    )
                });
                next_resp.response.on_hover_text("Continue to the next step.");
                if next_resp.inner.clicked() {
                    next_clicked = true;
                }
            });
        } else {
            let next_resp = ui.add_enabled_ui(can_next, |ui| {
                ui.add_sized(
                    egui::vec2(NAV_BUTTON_WIDTH, NAV_BUTTON_HEIGHT),
                    egui::Button::new("Next"),
                )
            });
            next_resp.response.on_hover_text("Continue to the next step.");
            if next_resp.inner.clicked() {
                next_clicked = true;
            }
        }
    });

    if next_clicked {
        actions::handle_next(app);
    }
}
