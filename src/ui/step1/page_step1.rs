// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::WizardState;
use crate::ui::layout::SECTION_GAP;
use crate::ui::step1::action_step1::Step1Action;
use crate::ui::step1::frame_step1::{render_bottom, render_top};
use crate::ui::step1::service_step1::{
    split_path_check_lines, sync_install_mode, sync_weidu_log_mode,
};
use crate::ui::step1::state_step1::clear_path_check_if_step1_changed;
use crate::ui::step5::service_diagnostics_support_step5::export_diagnostics;

pub fn render(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    dev_mode: bool,
    exe_fingerprint: &str,
) -> Option<Step1Action> {
    sync_install_mode(&mut state.step1);
    let before = state.step1.clone();
    sync_weidu_log_mode(&mut state.step1);
    let mut step1_action = None;
    let github_button_label = if state.github_auth_running {
        "GitHub: Waiting...".to_string()
    } else if state.github_auth_login.trim().is_empty() {
        "Connect GitHub".to_string()
    } else {
        format!("GitHub: {}", state.github_auth_login.trim())
    };

    ui.horizontal(|ui| {
        ui.heading("Step 1: Setup");
        if dev_mode {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Export diagnostics").clicked() {
                    match export_diagnostics(state, None, dev_mode, exe_fingerprint) {
                        Ok(path) => {
                            state.step5.last_status_text =
                                format!("Diagnostics exported: {}", path.display());
                        }
                        Err(err) => {
                            state.step5.last_status_text =
                                format!("Diagnostics export failed: {err}");
                        }
                    }
                }
            });
        }
    });
    ui.label("Choose game mode, paths, and installer options.");
    if let Some((ok, msg)) = state.step1_path_check.clone() {
        ui.add_space(4.0);
        ui.group(|ui| {
            ui.label(crate::ui::shared::typography_global::strong("Path Check"));
            if ok {
                ui.label(
                    crate::ui::shared::typography_global::plain(format!("- {msg}"))
                        .color(crate::ui::shared::theme_global::success_bright()),
                );
            } else {
                for line in split_path_check_lines(&msg) {
                    ui.label(
                        crate::ui::shared::typography_global::plain(format!("- {}", line))
                            .color(crate::ui::shared::theme_global::error()),
                    );
                }
            }
        });
    }
    ui.add_space(SECTION_GAP);

    egui::ScrollArea::vertical().show(ui, |ui| {
        render_top(
            ui,
            &mut state.step1,
            dev_mode,
            github_button_label.as_str(),
            &mut step1_action,
        );
        ui.add_space(SECTION_GAP);
        render_bottom(ui, &mut state.step1);
    });

    let step1_changed = state.step1 != before;
    clear_path_check_if_step1_changed(state, step1_changed);
    if step1_action.is_some() {
        step1_action
    } else if step1_changed {
        Some(Step1Action::PathsChanged)
    } else {
        None
    }
}
