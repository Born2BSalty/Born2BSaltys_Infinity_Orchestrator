// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use super::super::super::format::ellipsize_end;

pub(super) fn render_value_row(
    ui: &mut egui::Ui,
    label: &str,
    value: Option<&str>,
    monospace: bool,
    label_w: f32,
    value_w: f32,
    row_h: f32,
    value_chars: usize,
) {
    ui.add_sized(
        [label_w, row_h],
        egui::Label::new(egui::RichText::new(label).strong()),
    );
    let raw = value.unwrap_or("No data");
    let display = ellipsize_end(raw, value_chars);
    let text = if monospace {
        egui::RichText::new(display).monospace()
    } else {
        egui::RichText::new(display)
    };
    ui.add_sized([value_w, row_h], egui::Label::new(text))
        .on_hover_text(raw);
    if let Some(copy_value) = value {
        if ui.small_button("C").on_hover_text("Copy").clicked() {
            ui.ctx().copy_text(copy_value.to_string());
        }
    } else {
        ui.label("");
    }
    ui.end_row();
}
