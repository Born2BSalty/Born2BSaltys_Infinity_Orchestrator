// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    REDESIGN_SCREEN_SUBTITLE_FONT_SIZE_PX, REDESIGN_SCREEN_TITLE_BOTTOM_GAP_PX,
    REDESIGN_SCREEN_TITLE_FONT_SIZE_PX, REDESIGN_SCREEN_TITLE_SUBTITLE_GAP_PX, ThemePalette,
    redesign_font_light, redesign_font_medium, redesign_text_muted, redesign_text_primary,
};

pub fn render(ui: &mut egui::Ui, palette: ThemePalette, title: &str, sub: Option<&str>) {
    ui.label(
        egui::RichText::new(title)
            .family(redesign_font_medium())
            .size(REDESIGN_SCREEN_TITLE_FONT_SIZE_PX)
            .color(redesign_text_primary(palette)),
    );
    if let Some(sub) = sub {
        ui.add_space(REDESIGN_SCREEN_TITLE_SUBTITLE_GAP_PX);
        ui.label(
            egui::RichText::new(sub)
                .family(redesign_font_light())
                .size(REDESIGN_SCREEN_SUBTITLE_FONT_SIZE_PX)
                .color(redesign_text_muted(palette)),
        );
    }
    ui.add_space(REDESIGN_SCREEN_TITLE_BOTTOM_GAP_PX);
}
