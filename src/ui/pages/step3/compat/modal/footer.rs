// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::state::WizardState;

use super::report;

pub(super) fn render_footer(ui: &mut egui::Ui, state: &mut WizardState) {
    ui.horizontal(|ui| {
        if ui.button("Export Compat Report").clicked() {
            match report::export_step3_compat_report(&state.compat.issues) {
                Ok(path) => {
                    state.step5.last_status_text =
                        format!("Step 3 compat report exported: {}", path.display());
                }
                Err(err) => {
                    state.step5.last_status_text = format!("Step 3 compat export failed: {err}");
                }
            }
        }
        if ui.button("Close").clicked() {
            state.step3.compat_modal_open = false;
        }
    });
}
