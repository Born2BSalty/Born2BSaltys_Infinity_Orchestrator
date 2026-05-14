// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    REDESIGN_HINT_FONT_SIZE_PX, REDESIGN_LABEL_FONT_SIZE_PX, ThemePalette, redesign_accent_deep,
    redesign_font_light, redesign_text_primary,
};

pub fn redesign_label(ui: &mut egui::Ui, palette: ThemePalette, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .family(redesign_font_light())
            .size(REDESIGN_HINT_FONT_SIZE_PX)
            .color(redesign_text_primary(palette)),
    );
}

pub fn redesign_label_hand(ui: &mut egui::Ui, palette: ThemePalette, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .family(redesign_font_light())
            .size(REDESIGN_LABEL_FONT_SIZE_PX)
            .color(redesign_accent_deep(palette)),
    );
}
