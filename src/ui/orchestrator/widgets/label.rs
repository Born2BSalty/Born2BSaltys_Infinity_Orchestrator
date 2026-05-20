// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    ThemePalette, redesign_accent_deep, redesign_text_primary,
};

pub fn redesign_label(ui: &mut egui::Ui, palette: ThemePalette, text: &str) -> egui::Response {
    let rich = egui::RichText::new(text)
        .size(13.0)
        .family(egui::FontFamily::Name("poppins_medium".into()))
        .color(redesign_text_primary(palette));
    ui.label(rich)
}

pub fn redesign_label_hand(ui: &mut egui::Ui, palette: ThemePalette, text: &str) -> egui::Response {
    let rich = egui::RichText::new(text)
        .size(14.0)
        .family(egui::FontFamily::Name("poppins_medium".into()))
        .color(redesign_accent_deep(palette));
    ui.label(rich)
}
