// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::state::WizardState;
use crate::ui::step5::diagnostics::export_diagnostics;
use crate::ui::step5::log_files::{
    open_console_logs_folder, open_last_log_file, save_console_log,
};
use crate::ui::terminal::EmbeddedTerminal;

pub(super) fn render_actions_menu(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    mut terminal: Option<&mut EmbeddedTerminal>,
) {
    ui.menu_button("Actions", |ui| {
        if ui
            .add_enabled(terminal.is_some(), egui::Button::new("Copy Console"))
            .clicked()
        {
            if let Some(term) = terminal.as_ref() {
                ui.ctx().copy_text(term.console_text());
            }
            ui.close_menu();
        }
        if ui
            .add_enabled(terminal.is_some(), egui::Button::new("Save Console Log"))
            .clicked()
        {
            if let Some(term) = terminal.as_ref() {
                match save_console_log(&term.console_text()) {
                    Ok(path) => {
                        state.step5.last_status_text =
                            format!("Saved console log: {}", path.display());
                    }
                    Err(err) => {
                        state.step5.last_status_text = format!("Save console log failed: {err}");
                    }
                }
            }
            ui.close_menu();
        }
        if ui
            .add_enabled(terminal.is_some(), egui::Button::new("Open Logs Folder"))
            .clicked()
        {
            if let Err(err) = open_console_logs_folder() {
                state.step5.last_status_text = format!("Open logs folder failed: {err}");
            }
            ui.close_menu();
        }
        if ui
            .add_enabled(terminal.is_some(), egui::Button::new("Clear Console"))
            .clicked()
        {
            if let Some(term) = terminal.as_mut() {
                term.clear_console();
            }
            ui.close_menu();
        }
        if state.step5.last_install_failed
            && ui.button("Open last log file").clicked()
        {
            let _ = open_last_log_file(&state.step1);
            ui.close_menu();
        }
    });
}

pub(super) fn render_diagnostics_menu(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    terminal: Option<&EmbeddedTerminal>,
    dev_mode: bool,
) {
    if !dev_mode {
        return;
    }
    state.step1.bio_full_debug = true;
    state.step1.log_raw_output_dev = true;
    ui.menu_button("Diagnostics", |ui| {
        ui.label("Applies on next Install/Restart");
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if ui
                .selectable_label(
                    !state.step1.rust_log_debug && !state.step1.rust_log_trace,
                    "RUST_LOG Off",
                )
                .clicked()
            {
                set_rust_log_level(state, None);
            }
            if ui
                .selectable_label(state.step1.rust_log_debug, "RUST_LOG=DEBUG")
                .clicked()
            {
                set_rust_log_level(state, Some("debug"));
            }
            if ui
                .selectable_label(state.step1.rust_log_trace, "RUST_LOG=TRACE")
                .clicked()
            {
                set_rust_log_level(state, Some("trace"));
            }
        });
        if ui.button("Export diagnostics").clicked() {
            match export_diagnostics(state, terminal, dev_mode) {
                Ok(path) => {
                    state.step5.last_status_text =
                        format!("Diagnostics exported: {}", path.display());
                }
                Err(err) => {
                    state.step5.last_status_text = format!("Diagnostics export failed: {err}");
                }
            }
            ui.close_menu();
        }
    });
}

fn set_rust_log_level(state: &mut WizardState, level: Option<&str>) {
    match level {
        Some("trace") => {
            state.step1.rust_log_trace = true;
            state.step1.rust_log_debug = false;
        }
        Some("debug") => {
            state.step1.rust_log_debug = true;
            state.step1.rust_log_trace = false;
        }
        _ => {
            state.step1.rust_log_debug = false;
            state.step1.rust_log_trace = false;
        }
    }
}

pub(super) fn diagnostics_ready_for_dev(state: &WizardState) -> bool {
    state.step1.rust_log_debug || state.step1.rust_log_trace
}
