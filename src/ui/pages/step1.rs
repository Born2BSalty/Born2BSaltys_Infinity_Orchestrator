// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::layout::SECTION_GAP;
use crate::ui::state::WizardState;
use crate::ui::step1::sections::{symmetric_boxes_layout, top_row};
use crate::ui::step1::validation::sync_weidu_log_mode;
use crate::ui::step5::diagnostics::{DiagnosticsContext, export_diagnostics};

pub fn render(ui: &mut egui::Ui, state: &mut WizardState, dev_mode: bool, exe_fingerprint: &str) {
    let before = state.step1.clone();
    sync_weidu_log_mode(&mut state.step1);

    ui.horizontal(|ui| {
        ui.heading("Step 1: Setup");
        if dev_mode {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Export diagnostics").clicked() {
                    let diag_ctx = DiagnosticsContext {
                        dev_mode,
                        exe_fingerprint: exe_fingerprint.to_string(),
                    };
                    match export_diagnostics(state, None, &diag_ctx) {
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
            ui.label(egui::RichText::new("Path Check").strong());
            if ok {
                ui.label(egui::RichText::new(format!("- {msg}")).color(egui::Color32::LIGHT_GREEN));
            } else {
                let details = msg.strip_prefix("Path check failed: ").unwrap_or(msg.as_str());
                for line in details.split(" | ").filter(|v| !v.trim().is_empty()) {
                    ui.label(
                        egui::RichText::new(format!("- {}", line.trim()))
                            .color(egui::Color32::LIGHT_RED),
                    );
                }
            }
        });
    }
    ui.add_space(SECTION_GAP);

    egui::ScrollArea::vertical().show(ui, |ui| {
        top_row(ui, &mut state.step1, dev_mode);
        ui.add_space(SECTION_GAP);
        symmetric_boxes_layout(ui, &mut state.step1);
    });

    if state.step1 != before {
        state.step1_path_check = None;
    }
}
