// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::create::state_create::{CreateScreenState, CreateStage};
use crate::ui::orchestrator::widgets::btn::{BtnOpts, redesign_btn};
use crate::ui::orchestrator::widgets::r_box::redesign_box;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_HINT_FONT_SIZE_PX, REDESIGN_LABEL_FONT_SIZE_PX, ThemePalette, redesign_text_faint,
    redesign_text_primary,
};

pub fn render(ui: &mut egui::Ui, palette: ThemePalette, state: &mut CreateScreenState) {
    redesign_box(ui, palette, Some("downloading fork"), |ui| {
        ui.label(
            egui::RichText::new("Download/import screen not available yet")
                .size(REDESIGN_LABEL_FONT_SIZE_PX)
                .color(redesign_text_primary(palette)),
        );
        ui.label(
            egui::RichText::new(
                "Phase 5 import/download UI is not present in this repo, so this flow is stubbed.",
            )
            .size(REDESIGN_HINT_FONT_SIZE_PX)
            .color(redesign_text_faint(palette)),
        );
        if redesign_btn(ui, palette, "← back", BtnOpts::default()).clicked() {
            state.stage = CreateStage::ForkPreview;
        }
    });
}
