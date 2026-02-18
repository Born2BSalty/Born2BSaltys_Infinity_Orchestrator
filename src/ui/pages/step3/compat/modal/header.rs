// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::state::WizardState;

pub(super) fn render_header(ui: &mut egui::Ui, state: &WizardState) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!("{} errors", state.compat.error_count))
                .color(egui::Color32::from_rgb(220, 100, 100))
                .strong(),
        );
        ui.label(
            egui::RichText::new(format!("{} warnings", state.compat.warning_count))
                .color(egui::Color32::from_rgb(220, 180, 100))
                .strong(),
        );
    });
}
