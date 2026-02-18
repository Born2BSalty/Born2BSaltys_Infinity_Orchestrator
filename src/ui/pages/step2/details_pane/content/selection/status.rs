// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::step2::details::Step2Details;

pub(super) fn render_checked_row(
    ui: &mut egui::Ui,
    details: &Step2Details,
    label_w: f32,
    value_w: f32,
    row_h: f32,
) {
    ui.add_sized(
        [label_w, row_h],
        egui::Label::new(egui::RichText::new("Checked").strong()),
    );
    let checked_pill = match details.is_checked {
        Some(true) => egui::RichText::new("Checked")
            .color(egui::Color32::from_rgb(124, 196, 124))
            .strong(),
        Some(false) => egui::RichText::new("Unchecked")
            .color(egui::Color32::from_rgb(180, 180, 180))
            .strong(),
        None => egui::RichText::new("No data").strong(),
    };
    ui.add_sized([value_w, row_h], egui::Label::new(checked_pill));
    ui.label("");
    ui.end_row();
}

pub(super) fn render_state_row(
    ui: &mut egui::Ui,
    details: &Step2Details,
    label_w: f32,
    value_w: f32,
    row_h: f32,
) {
    ui.add_sized(
        [label_w, row_h],
        egui::Label::new(egui::RichText::new("State").strong()),
    );
    let state_text = if details.is_disabled == Some(true) {
        egui::RichText::new("Disabled")
            .color(egui::Color32::from_rgb(214, 168, 96))
            .strong()
    } else {
        egui::RichText::new("Selectable")
            .color(egui::Color32::from_rgb(124, 196, 124))
            .strong()
    };
    let state_resp = ui.add_sized([value_w, row_h], egui::Label::new(state_text));
    if let Some(reason) = details.disabled_reason.as_deref() {
        state_resp.on_hover_text(reason);
    }
    ui.label("");
    ui.end_row();
}
