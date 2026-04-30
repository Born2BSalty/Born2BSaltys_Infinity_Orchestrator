// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::WizardState;

pub(crate) fn render_dev_header(ui: &mut egui::Ui, state: &mut WizardState, dev_mode: bool) {
    ui.heading("Step 5: Install, Logs, Diagnostics");
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
            crate::ui::shared::theme_global::success()
        } else {
            crate::ui::shared::theme_global::accent_path()
        };
        let msg = if has_rust_log {
            format!("Dev Mode: RUST_LOG={level} selected.")
        } else {
            "Dev Mode: open Diagnostics and choose RUST_LOG=DEBUG or TRACE before Install."
                .to_string()
        };
        ui.label(crate::ui::shared::typography_global::strong(msg).color(color));
    }
    ui.add_space(crate::ui::shared::layout_tokens_global::SPACE_LG);
}
