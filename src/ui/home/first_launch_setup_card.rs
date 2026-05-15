// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::home::page_home::HomeAction;
use crate::ui::orchestrator::widgets::btn::{BtnOpts, redesign_btn};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_HOME_SETUP_CARD_GAP_PX, REDESIGN_LABEL_FONT_SIZE_PX, ThemePalette,
    redesign_font_light, redesign_font_medium, redesign_text_faint, redesign_text_primary,
};

pub fn render(ui: &mut egui::Ui, palette: ThemePalette) -> Option<HomeAction> {
    let mut action = None;

    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = REDESIGN_HOME_SETUP_CARD_GAP_PX;
        ui.label(
            egui::RichText::new("Welcome to Infinity Orchestrator")
                .font(egui::FontId::new(
                    REDESIGN_LABEL_FONT_SIZE_PX,
                    redesign_font_medium(),
                ))
                .size(REDESIGN_LABEL_FONT_SIZE_PX)
                .color(redesign_text_primary(palette)),
        );
        ui.label(
            egui::RichText::new("Get set up first — point BIO at your games and tools.")
                .font(egui::FontId::new(
                    REDESIGN_LABEL_FONT_SIZE_PX,
                    redesign_font_light(),
                ))
                .size(REDESIGN_LABEL_FONT_SIZE_PX)
                .color(redesign_text_faint(palette)),
        );
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
            action = Some(HomeAction::OpenSettingsPaths);
        }
    });

    action
}
