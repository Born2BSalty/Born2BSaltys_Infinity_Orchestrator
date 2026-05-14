// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::create::state_create::CreateScreenState;
use crate::ui::orchestrator::widgets::btn::{BtnOpts, redesign_btn};
use crate::ui::orchestrator::widgets::r_box::redesign_box;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_HINT_FONT_SIZE_PX, REDESIGN_HOME_CARD_LIST_GAP_PX, REDESIGN_LABEL_FONT_SIZE_PX,
    ThemePalette, redesign_text_faint, redesign_text_primary,
};

pub fn render(ui: &mut egui::Ui, palette: ThemePalette, state: &mut CreateScreenState) {
    ui.add_space(REDESIGN_HOME_CARD_LIST_GAP_PX);
    redesign_box(ui, palette, Some("Load draft"), |ui| {
        ui.label(
            egui::RichText::new("Resume in-progress build")
                .size(REDESIGN_LABEL_FONT_SIZE_PX)
                .strong()
                .color(redesign_text_primary(palette)),
        );
        ui.label(
            egui::RichText::new("No in-progress builds. Start a new modlist from Create.")
                .size(REDESIGN_HINT_FONT_SIZE_PX)
                .color(redesign_text_faint(palette)),
        );
        if redesign_btn(
            ui,
            palette,
            "Cancel",
            BtnOpts {
                small: true,
                ..Default::default()
            },
        )
        .clicked()
        {
            state.load_draft_open = false;
        }
    });
}
