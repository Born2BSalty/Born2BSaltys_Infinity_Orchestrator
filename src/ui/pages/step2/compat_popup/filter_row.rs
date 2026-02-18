// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::state::WizardState;

pub(super) fn render_filter_row(ui: &mut egui::Ui, state: &mut WizardState) {
    ui.horizontal_wrapped(|ui| {
        ui.label(egui::RichText::new("Show").strong());
        for (id, label) in [
            ("all", "All"),
            ("conflicts", "Conflicts"),
            ("dependencies", "Missing deps"),
            ("conditionals", "Conditionals"),
            ("warnings", "Warnings"),
        ] {
            let is_selected = state.step2.compat_popup_filter.eq_ignore_ascii_case(id);
            if ui.selectable_label(is_selected, label).clicked() {
                state.step2.compat_popup_filter = id.to_string();
            }
        }
    });
}
