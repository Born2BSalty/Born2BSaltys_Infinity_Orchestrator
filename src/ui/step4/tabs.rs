// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

pub fn draw_tab(ui: &mut egui::Ui, active: &mut String, value: &str) {
    let is_active = active == value;
    let fill = if is_active {
        ui.visuals().widgets.active.bg_fill
    } else {
        ui.visuals().widgets.inactive.bg_fill
    };
    let stroke = if is_active {
        ui.visuals().widgets.active.bg_stroke
    } else {
        ui.visuals().widgets.inactive.bg_stroke
    };
    let text_color = if is_active {
        ui.visuals().widgets.active.fg_stroke.color
    } else {
        ui.visuals().widgets.inactive.fg_stroke.color
    };
    let button = egui::Button::new(egui::RichText::new(value).color(text_color))
        .fill(fill)
        .stroke(stroke)
        .corner_radius(egui::CornerRadius::same(4));
    if ui.add_sized([58.0, 24.0], button).clicked() {
        *active = value.to_string();
    }
}
