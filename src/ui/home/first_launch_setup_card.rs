// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::orchestrator::widgets::{BtnOpts, redesign_btn};
use crate::ui::shared::redesign_tokens::{
    ThemePalette, redesign_text_faint, redesign_text_primary,
};

pub fn render(ui: &mut egui::Ui, palette: ThemePalette) -> bool {
    let mut open_settings = false;

    ui.vertical(|ui| {
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new("Welcome to Infinity Orchestrator")
                .size(16.0)
                .family(egui::FontFamily::Name("poppins_bold".into()))
                .color(redesign_text_primary(palette)),
        );

        ui.add_space(6.0);
        ui.label(
            egui::RichText::new("Get set up first \u{2014} point BIO at your games and tools.")
                .size(13.0)
                .family(egui::FontFamily::Proportional)
                .color(redesign_text_faint(palette)),
        );

        ui.add_space(14.0);
        if redesign_btn(
            ui,
            palette,
            "Open Settings",
            BtnOpts {
                primary: true,
                ..Default::default()
            },
        )
        .clicked()
        {
            open_settings = true;
        }
        ui.add_space(4.0);
    });

    open_settings
}
