// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use super::WizardApp;

mod bar {
use eframe::egui;

use crate::ui::state::WizardState;

mod actions {
use crate::ui::app::WizardApp;
use crate::ui::step5::prompt_memory;

use super::super::logic;
use crate::ui::scan::cache;

pub(super) fn handle_reset(app: &mut WizardApp) {
    super::super::super::step2_scan::cancel_step2_scan(app);
    app.step2_scan_rx = None;
    app.step2_cancel = None;
    app.step2_progress_queue.clear();
    if let Some(term) = app.step5_terminal.as_mut() {
        term.shutdown();
    }
    cache::clear_scan_cache_files();
    prompt_memory::clear_all();
    app.state.reset_workflow_keep_step1();
    app.last_step2_sync_signature = None;
    app.save_settings_best_effort();
}

pub(super) fn handle_back(app: &mut WizardApp) {
    let prev_step = app.state.current_step;
    if prev_step == 2 {
        app.sync_step2_from_step3();
    }
    if app.state.step1.have_weidu_logs && (app.state.current_step == 3 || app.state.current_step == 4) {
        app.state.current_step = 0;
    } else {
        app.state.go_back();
    }
    if prev_step == 1 && app.state.current_step == 0 {
        app.state.step1_path_check = None;
    }
    app.save_settings_best_effort();
}

pub(super) fn handle_next(app: &mut WizardApp) {
    if logic::should_show_step1_clean_confirm(app) {
        app.state.step1_clean_confirm_open = true;
    } else {
        logic::advance_after_next(app);
    }
}
}
mod render_buttons {
use eframe::egui;

use crate::ui::layout::{NAV_BUTTON_HEIGHT, NAV_BUTTON_WIDTH};
use crate::ui::step1::service_step1::run_path_check;

use crate::ui::app::WizardApp;

use super::actions;

pub(super) fn render_reset_button(ui: &mut egui::Ui, app: &mut WizardApp) {
    if app.state.current_step == 1
        && ui
            .add_sized(
                egui::vec2(NAV_BUTTON_WIDTH + 24.0, NAV_BUTTON_HEIGHT),
                egui::Button::new("Reset Wizard State"),
            )
            .on_hover_text("Clear scan/selection/order/install state, delete scan cache and prompt answers, and return to Step 1.")
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

pub(super) fn render_exit_button(ui: &mut egui::Ui, app: &mut WizardApp, step5_install_running: bool) {
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
}

use super::super::WizardApp;

pub(super) fn render(app: &mut WizardApp, ctx: &egui::Context) {
    let can_next = app.can_advance_from_current_step();
    let on_last_step = app.state.current_step + 1 == WizardState::STEP_COUNT;
    let step5_install_running =
        app.state.current_step == 4 && (app.state.step5.prep_running || app.state.step5.install_running);
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
}
mod confirm {
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
}
mod logic {
use crate::ui::state::Step2ModState;

use super::super::WizardApp;

pub(super) fn should_show_step1_clean_confirm(app: &WizardApp) -> bool {
    let uses_fresh_target = if app.state.step1.game_install == "EET" {
        app.state.step1.new_pre_eet_dir_enabled || app.state.step1.new_eet_dir_enabled
    } else {
        app.state.step1.generate_directory_enabled
    };
    app.state.current_step == 0
        && uses_fresh_target
        && app.state.step1.prepare_target_dirs_before_install
        && !app.state.step1.backup_targets_before_eet_copy
}

pub(super) fn advance_after_next(app: &mut WizardApp) {
    if !app.can_advance_from_current_step() {
        return;
    }
    if app.state.current_step == 0 && app.state.step1.have_weidu_logs {
        app.state.current_step = 3;
    } else {
        if app.state.current_step == 1 {
            let signature = step2_selection_signature(app);
            let step3_empty = step3_has_no_real_items(app);
            let should_sync = step3_empty
                || app
                    .last_step2_sync_signature
                    .as_ref()
                    .map(|s| s != &signature)
                    .unwrap_or(true);
            if should_sync {
                app.sync_step3_from_step2();
                app.last_step2_sync_signature = Some(signature);
            }
        }
        if app.state.current_step == 3
            && let Err(err) = app.auto_save_step4_weidu_logs()
        {
            let msg = format!("Step 4 save failed: {err}");
            app.state.step5.last_status_text = msg.clone();
            app.state.step4_save_error_text = msg;
            app.state.step4_save_error_open = true;
            app.save_settings_best_effort();
            return;
        }
        app.state.go_next();
    }
    app.save_settings_best_effort();
}

fn step3_has_no_real_items(app: &WizardApp) -> bool {
    let bgee_has = app.state.step3.bgee_items.iter().any(|i| !i.is_parent);
    let bg2_has = app.state.step3.bg2ee_items.iter().any(|i| !i.is_parent);
    !(bgee_has || bg2_has)
}

fn step2_selection_signature(app: &WizardApp) -> String {
    let mut entries: Vec<String> = Vec::new();
    let mut collect = |tag: &str, mods: &[Step2ModState]| {
        for m in mods {
            let tp = m.tp_file.to_ascii_uppercase();
            for c in &m.components {
                if c.checked {
                    entries.push(format!(
                        "{tag}|{tp}|{}|{}",
                        c.component_id,
                        c.selected_order.unwrap_or(usize::MAX)
                    ));
                }
            }
        }
    };
    collect("BGEE", &app.state.step2.bgee_mods);
    collect("BG2EE", &app.state.step2.bg2ee_mods);
    entries.sort_unstable();
    entries.join(";")
}
}

pub(super) fn render_nav_buttons(app: &mut WizardApp, ctx: &egui::Context) {
    bar::render(app, ctx);
    confirm::render(app, ctx);
}
