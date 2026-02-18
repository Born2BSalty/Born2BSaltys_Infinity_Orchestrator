// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::step2::details::Step2Details;

pub(super) fn render_raw_line(ui: &mut egui::Ui, details: &Step2Details) {
    egui::CollapsingHeader::new("Raw line")
        .default_open(false)
        .show(ui, |ui| {
            let raw = details.raw_line.as_deref().unwrap_or("No data");
            ui.add(egui::Label::new(egui::RichText::new(raw).monospace()).wrap());
        });
}
