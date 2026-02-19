// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::state::WizardState;

pub(super) fn render_header(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    title_rect: egui::Rect,
    subtitle_rect: egui::Rect,
    search_rect: egui::Rect,
) {
    ui.scope_builder(egui::UiBuilder::new().max_rect(title_rect), |ui| {
        ui.heading("Step2: Scan and Select");
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
