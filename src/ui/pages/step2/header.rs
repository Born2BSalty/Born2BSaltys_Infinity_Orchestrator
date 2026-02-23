// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::state::WizardState;
use crate::ui::step5::diagnostics::{DiagnosticsContext, export_diagnostics};

pub(super) fn render_header(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    title_rect: egui::Rect,
    subtitle_rect: egui::Rect,
    search_rect: egui::Rect,
    dev_mode: bool,
    exe_fingerprint: &str,
) {
    ui.scope_builder(egui::UiBuilder::new().max_rect(title_rect), |ui| {
        ui.horizontal(|ui| {
            ui.heading("Step2: Scan and Select");
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
    });
    ui.scope_builder(egui::UiBuilder::new().max_rect(subtitle_rect), |ui| {
        ui.label("Choose components to install.")
            .on_hover_text("Select the components you want BIO to install.");
    });
    ui.scope_builder(egui::UiBuilder::new().max_rect(search_rect), |ui| {
        let search_w = search_rect.width().min(517.0);
        let resp = ui.add_sized(
            [search_w, 30.0],
            egui::TextEdit::singleline(&mut state.step2.search_query)
                .hint_text("Search mods or components..."),
        );
        resp.on_hover_text("Filter the tree by mod name, component text, TP2, or id.");
    });
}
