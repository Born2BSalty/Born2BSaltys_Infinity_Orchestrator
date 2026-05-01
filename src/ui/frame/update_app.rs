// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::app::WizardApp;
use crate::ui::layout::{NAV_BUTTON_HEIGHT, NAV_BUTTON_WIDTH};

pub fn configure_startup_visuals(ctx: &egui::Context) {
    crate::ui::shared::theme_global::apply_runtime_theme(ctx);
}

pub fn render_nav_buttons(app: &mut WizardApp, ctx: &egui::Context) {
    let can_next = crate::ui::app::nav_ui::can_advance(app);
    let on_last_step = crate::ui::app::nav_ui::on_last_step(app);
    let step5_install_running = crate::ui::app::nav_ui::step5_install_running(app);
    let right_margin = -19.0;
    egui::Area::new("wizard_nav_buttons".into())
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(right_margin, -4.0))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                render_reset_button(ui, app);
                render_back_button(ui, app, step5_install_running);
                if on_last_step {
                    render_exit_button(ui, app, step5_install_running);
                } else {
                    render_next_block(ui, app, can_next);
                }
            });
        });

    render_confirm_windows(app, ctx);
}

fn render_reset_button(ui: &mut egui::Ui, app: &mut WizardApp) {
    if crate::ui::app::nav_ui::current_step(app) == 1
        && ui
            .add_sized(
                egui::vec2(NAV_BUTTON_WIDTH + 24.0, NAV_BUTTON_HEIGHT),
                egui::Button::new("Reset Wizard State"),
            )
            .on_hover_text("Clear scan/selection/order/install state, delete scan cache and prompt answers, and return to Step 1.")
            .clicked()
    {
        crate::ui::app::nav_ui::handle_reset(app);
    }
}

fn render_back_button(ui: &mut egui::Ui, app: &mut WizardApp, step5_install_running: bool) {
    if crate::ui::app::nav_ui::current_step(app) > 0 {
        let back_enabled = crate::ui::app::nav_ui::can_go_back(app) && !step5_install_running;
        if back_enabled {
            let back_resp = ui
                .add_sized(
                    egui::vec2(NAV_BUTTON_WIDTH, NAV_BUTTON_HEIGHT),
                    egui::Button::new("Back"),
                )
                .on_hover_text("Go to previous step.");
            if back_resp.clicked() {
                crate::ui::app::nav_ui::handle_back(app);
            }
        } else {
            ui.add_sized(
                egui::vec2(NAV_BUTTON_WIDTH, NAV_BUTTON_HEIGHT),
                egui::Button::new(
                    crate::ui::shared::typography_global::plain("Back")
                        .color(crate::ui::shared::theme_global::nav_disabled_text()),
                )
                .fill(crate::ui::shared::theme_global::nav_disabled_fill())
                .stroke(egui::Stroke::new(
                    crate::ui::shared::layout_tokens_global::BORDER_THIN,
                    crate::ui::shared::theme_global::nav_disabled_stroke(),
                )),
            )
            .on_hover_text("Disabled while install is running.");
        }
    }
}

fn render_exit_button(ui: &mut egui::Ui, app: &mut WizardApp, step5_install_running: bool) {
    if step5_install_running {
        ui.add_sized(
            egui::vec2(NAV_BUTTON_WIDTH, NAV_BUTTON_HEIGHT),
            egui::Button::new(
                crate::ui::shared::typography_global::plain("Exit")
                    .color(crate::ui::shared::theme_global::nav_disabled_text()),
            )
            .fill(crate::ui::shared::theme_global::nav_disabled_fill())
            .stroke(egui::Stroke::new(
                crate::ui::shared::layout_tokens_global::BORDER_THIN,
                crate::ui::shared::theme_global::nav_disabled_stroke(),
            )),
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
        crate::ui::app::nav_ui::handle_exit(app);
    }
}

fn render_next_block(ui: &mut egui::Ui, app: &mut WizardApp, can_next: bool) {
    let mut next_clicked = false;
    ui.vertical(|ui| {
        if crate::ui::app::nav_ui::current_step(app) == 0 {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add_sized(
                        egui::vec2(NAV_BUTTON_WIDTH, NAV_BUTTON_HEIGHT),
                        egui::Button::new("Test Paths"),
                    )
                    .on_hover_text("Validate required paths and files for the selected game mode.")
                    .clicked()
                {
                    crate::ui::app::nav_ui::handle_test_paths(app);
                }
            });
        }

        if crate::ui::app::nav_ui::current_step(app) == 0 {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let next_resp = ui.add_enabled_ui(can_next, |ui| {
                    ui.add_sized(
                        egui::vec2(NAV_BUTTON_WIDTH, NAV_BUTTON_HEIGHT),
                        egui::Button::new("Next"),
                    )
                });
                next_resp
                    .response
                    .on_hover_text("Continue to the next step.");
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
            next_resp
                .response
                .on_hover_text("Continue to the next step.");
            if next_resp.inner.clicked() {
                next_clicked = true;
            }
        }
    });

    if next_clicked {
        crate::ui::app::nav_ui::handle_next(app);
    }
}

fn render_confirm_windows(app: &mut WizardApp, ctx: &egui::Context) {
    if crate::ui::app::nav_ui::step1_clean_confirm_open(app) {
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
                        crate::ui::app::nav_ui::handle_clean_confirm_yes(app);
                    }
                    if ui.button("No").clicked() {
                        crate::ui::app::nav_ui::handle_clean_confirm_no(app);
                    }
                });
            });
    }

    if crate::ui::app::nav_ui::step4_save_error_open(app) {
        let error_text = crate::ui::app::nav_ui::step4_save_error_text(app).to_string();
        egui::Window::new("Could not continue to Step 5")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.label("BIO could not save weidu.log from Step 4.");
                ui.add_space(6.0);
                ui.label(error_text);
                ui.add_space(10.0);
                if ui.button("OK").clicked() {
                    crate::ui::app::nav_ui::dismiss_step4_save_error(app);
                }
            });
    }
}
