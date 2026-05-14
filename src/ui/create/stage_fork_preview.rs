// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::create::state_create::{CreateScreenState, CreateStage};
use crate::ui::orchestrator::widgets::btn::{BtnOpts, redesign_btn};
use crate::ui::orchestrator::widgets::r_box::redesign_box;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_HINT_FONT_SIZE_PX, REDESIGN_LABEL_FONT_SIZE_PX, ThemePalette, redesign_success,
    redesign_text_faint, redesign_text_primary,
};

pub fn render(ui: &mut egui::Ui, palette: ThemePalette, state: &mut CreateScreenState) {
    redesign_box(ui, palette, Some("fork preview"), |ui| {
        if state.fork_preview_ready {
            ui.label(
                egui::RichText::new("Share code parsed successfully.")
                    .size(REDESIGN_LABEL_FONT_SIZE_PX)
                    .color(redesign_success(palette)),
            );
            ui.label(
                egui::RichText::new(
                    "Full preview tabs are blocked until the Install preview screen exists.",
                )
                .size(REDESIGN_HINT_FONT_SIZE_PX)
                .color(redesign_text_faint(palette)),
            );
        } else {
            ui.label(
                egui::RichText::new("Share code could not be previewed.")
                    .size(REDESIGN_LABEL_FONT_SIZE_PX)
                    .color(redesign_text_primary(palette)),
            );
            if let Some(error) = &state.fork_preview_error {
                ui.label(
                    egui::RichText::new(error)
                        .size(REDESIGN_HINT_FONT_SIZE_PX)
                        .color(redesign_text_faint(palette)),
                );
            }
        }

        ui.horizontal(|ui| {
            if redesign_btn(ui, palette, "← back", BtnOpts::default()).clicked() {
                state.stage = CreateStage::ForkPaste;
            }
            if redesign_btn(
                ui,
                palette,
                "begin import →",
                BtnOpts {
                    primary: true,
                    disabled: !state.fork_preview_ready,
                    ..Default::default()
                },
            )
            .clicked()
            {
                state.stage = CreateStage::ForkDownload;
            }
        });
    });
}
