// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::state::WizardState;

pub(super) fn render(ui: &mut egui::Ui, state: &mut WizardState, dev_mode: bool) {
    ui.heading("Step 5: Install");
    ui.label("Final execution view.");
    if dev_mode {
        let has_rust_log = state.step1.rust_log_debug || state.step1.rust_log_trace;
        let level = if state.step1.rust_log_trace {
            "TRACE"
        } else if state.step1.rust_log_debug {
            "DEBUG"
        } else {
            "OFF"
        };
        let color = if has_rust_log {
            egui::Color32::from_rgb(124, 196, 124)
        } else {
            egui::Color32::from_rgb(224, 196, 156)
        };
        let msg = if has_rust_log {
            format!("Dev Mode: RUST_LOG={level} selected.")
        } else {
            "Dev Mode: open Diagnostics and choose RUST_LOG=DEBUG or TRACE before Install."
                .to_string()
        };
        ui.label(egui::RichText::new(msg).color(color).strong());
    }
    ui.add_space(10.0);
}
