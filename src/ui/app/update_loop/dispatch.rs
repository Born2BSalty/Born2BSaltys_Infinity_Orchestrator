// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::pages::{step1, step2, step3, step4, step5};

use super::super::WizardApp;

pub(super) fn render_current_step(app: &mut WizardApp, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| match app.state.current_step {
        0 => step1::render(ui, &mut app.state, app.dev_mode),
        1 => {
            app.revalidate_compat_step2_checked_order();
            if let Some(action) = step2::render(ui, &mut app.state) {
                app.handle_step2_action(action);
            }
        }
        2 => {
            if let Some(action) = step3::render(ui, &mut app.state) {
                handle_step3_action(app, action);
            }
        }
        3 => {
            if let Some(action) = step4::render(ui, &mut app.state) {
                app.handle_step4_action(ctx, action);
            }
        }
        4 => {
            if let Some(action) = step5::render(
                ui,
                &mut app.state,
                app.step5_terminal.as_mut(),
                app.step5_terminal_error.as_deref(),
                app.dev_mode,
            ) {
                handle_step5_action(app, action);
            }
        }
        _ => {}
    });
}

fn handle_step3_action(app: &mut WizardApp, action: step3::Step3Action) {
    match action {
        step3::Step3Action::Revalidate => {
            app.revalidate_compat();
        }
    }
}

fn handle_step5_action(app: &mut WizardApp, action: step5::Step5Action) {
    match action {
        step5::Step5Action::CheckCompatBeforeInstall => {
            if app.check_compat_before_install() {
                app.state.compat.show_pre_install_modal = false;
                app.state.step5.start_install_requested = true;
            } else {
                app.state.step5.last_status_text =
                    "Blocking compatibility errors found. Fix them on Step 3.".to_string();
            }
        }
    }
}
