// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use super::WizardApp;

mod dispatch {
    use eframe::egui;

    use crate::ui::{step1, step2, step3, step4, step5};

    use super::super::WizardApp;

    pub(super) fn render_current_step(app: &mut WizardApp, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| match app.state.current_step {
            0 => {
                if let Some(action) = step1::page_step1::render(
                    ui,
                    &mut app.state,
                    app.dev_mode,
                    app.exe_fingerprint.as_str(),
                ) {
                    app.handle_step1_action(action);
                }
            }
            1 => {
                if let Some(action) = step2::page_step2::render(
                    ui,
                    &mut app.state,
                    app.dev_mode,
                    app.exe_fingerprint.as_str(),
                ) {
                    app.handle_step2_action(action);
                }
            }
            2 => {
                step3::page_step3::render(
                    ui,
                    &mut app.state,
                    app.dev_mode,
                    app.exe_fingerprint.as_str(),
                );
            }
            3 => {
                if let Some(action) = step4::page_step4::render(
                    ui,
                    &mut app.state,
                    app.dev_mode,
                    app.exe_fingerprint.as_str(),
                ) {
                    app.handle_step4_action(ctx, action);
                }
            }
            4 => {
                if let Some(action) = step5::page_step5::render(
                    ui,
                    &mut app.state,
                    &mut app.step5_console_view,
                    app.step5_terminal.as_mut(),
                    app.step5_terminal_error.as_deref(),
                    app.dev_mode,
                    app.exe_fingerprint.as_str(),
                ) {
                    handle_step5_action(app, action);
                }
            }
            _ => {}
        });
    }

    pub(super) fn render_shared_popups(app: &mut WizardApp, ctx: &egui::Context) {
        let mut step1_action = None;
        crate::ui::step1::github_auth_popup_step1::render(
            ctx,
            &mut app.state,
            &mut step1_action,
        );
        if let Some(action) = step1_action {
            app.handle_step1_action(action);
        }
        let mut step2_action = None;
        crate::ui::step2::update_check_popup_step2::render(ctx, &mut app.state, &mut step2_action);
        if let Some(action) = step2_action {
            app.handle_step2_action(action);
        }
    }

    fn handle_step5_action(app: &mut WizardApp, action: step5::action_step5::Step5Action) {
        match action {
            step5::action_step5::Step5Action::StartInstall => {
                app.state.step5.start_install_requested = true;
            }
        }
    }
}
mod repaint {
    use eframe::egui;
    use std::time::Duration;

    use super::super::WizardApp;

    pub(super) fn request_if_needed(app: &WizardApp, ctx: &egui::Context) {
        if crate::app::app_update_cycle::needs_repaint(
            &app.step1_github_auth_rx,
            &app.step2_scan_rx,
            &app.step2_progress_queue,
            &app.step2_update_check_rx,
            &app.step2_update_download_rx,
            &app.step2_update_extract_rx,
            &app.step5_terminal,
            &app.step5_prep_rx,
            &app.state,
        ) {
            ctx.request_repaint_after(Duration::from_millis(16));
        }
    }
}

pub(super) fn run(app: &mut WizardApp, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    if ctx.input(|i| i.viewport().close_requested()) {
        app.shutdown_and_exit();
    }

    let install_was_running = app.state.step5.install_running;
    crate::app::app_step1_github_oauth::poll_github_oauth_flow(
        &mut app.state,
        &mut app.step1_github_auth_rx,
    );
    let mut step5_requested_repaint = crate::app::app_update_cycle::poll_before_render(
        &mut app.state,
        &mut app.step2_scan_rx,
        &mut app.step2_cancel,
        &mut app.step2_progress_queue,
        &mut app.step2_update_check_rx,
        &mut app.step2_update_download_rx,
        &mut app.step2_update_extract_rx,
        &mut app.step5_terminal,
        &mut app.step5_terminal_error,
        &mut app.step5_prep_rx,
        &mut app.step5_pending_start,
    );
    if !install_was_running && app.state.step5.install_running {
        app.step5_console_view.request_input_focus = true;
    }
    dispatch::render_current_step(app, ctx);
    dispatch::render_shared_popups(app, ctx);
    let install_was_running = app.state.step5.install_running;
    step5_requested_repaint |= crate::app::app_update_cycle::start_after_render(
        &mut app.state,
        &mut app.step5_terminal,
        &mut app.step5_terminal_error,
        &mut app.step5_prep_rx,
        &mut app.step5_pending_start,
    );
    if !install_was_running && app.state.step5.install_running {
        app.step5_console_view.request_input_focus = true;
    }
    if step5_requested_repaint {
        ctx.request_repaint();
    }
    crate::ui::frame::update_app::render_nav_buttons(app, ctx);

    crate::app::app_update_cycle::persist_step1_if_needed(
        &app.state,
        &app.settings_store,
        &mut app.last_saved_step1,
        app.dev_mode,
        app.exe_fingerprint.as_str(),
    );
    repaint::request_if_needed(app, ctx);
}
