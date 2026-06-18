// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    ThemePalette, redesign_text_muted, redesign_text_primary,
};

pub fn render(ui: &mut egui::Ui, palette: ThemePalette, title: &str, sub: Option<&str>) {
    ui.vertical(|ui| {
        ui.add_space(0.0);
        let title_rich = egui::RichText::new(title)
            .size(22.0)
            .family(egui::FontFamily::Name("poppins_medium".into()))
            .color(redesign_text_primary(palette));
        ui.label(title_rich);

        if let Some(sub) = sub {
            ui.add_space(4.0);
            let sub_rich = egui::RichText::new(sub)
                .size(13.0)
                .family(egui::FontFamily::Proportional)
                .color(redesign_text_muted(palette));
            ui.label(sub_rich);
        }

        ui.add_space(20.0);
    });
}
