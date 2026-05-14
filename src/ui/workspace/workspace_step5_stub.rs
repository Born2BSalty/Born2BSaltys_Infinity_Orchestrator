// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::orchestrator::widgets::btn::{BtnOpts, redesign_btn};
use crate::ui::orchestrator::widgets::r_box::redesign_box;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_LABEL_FONT_SIZE_PX, ThemePalette, redesign_text_faint, redesign_text_primary,
};

pub fn render(ui: &mut egui::Ui, palette: ThemePalette) {
    redesign_box(ui, palette, Some("Step 5: Install"), |ui| {
        ui.label(
            egui::RichText::new("Step 5 install runtime arrives in Phase 7.")
                .size(REDESIGN_LABEL_FONT_SIZE_PX)
                .color(redesign_text_primary(palette)),
        );
        ui.label(
            egui::RichText::new("Install execution, console output, prompts, and diagnostics are intentionally not wired here.")
                .size(REDESIGN_LABEL_FONT_SIZE_PX)
                .color(redesign_text_faint(palette)),
        );
        let _ = redesign_btn(
            ui,
            palette,
            "Install",
            BtnOpts {
                primary: true,
                disabled: true,
                ..Default::default()
            },
        );
    });
}
