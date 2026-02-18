// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use super::WizardApp;

mod dispatch;
mod repaint;
mod terminal;

pub(super) fn run(app: &mut WizardApp, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    if ctx.input(|i| i.viewport().close_requested()) {
        app.shutdown_and_exit();
    }

    app.poll_step2_scan_events();
    terminal::poll_step5_terminal(app, ctx);
    dispatch::render_current_step(app, ctx);
    super::nav_ui::render_nav_buttons(app, ctx);

    if app.state.step1 != app.last_saved_step1 {
        app.save_settings_best_effort();
    }
    repaint::request_if_needed(app, ctx);
}
