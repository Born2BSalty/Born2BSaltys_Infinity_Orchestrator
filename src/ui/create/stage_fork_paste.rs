// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::create::state_create::{CreateScreenState, CreateStage};
use crate::ui::orchestrator::widgets::btn::{BtnOpts, redesign_btn};
use crate::ui::orchestrator::widgets::r_box::redesign_box;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX, REDESIGN_HINT_FONT_SIZE_PX,
    REDESIGN_HOME_CARD_LIST_GAP_PX, REDESIGN_LABEL_FONT_SIZE_PX, ThemePalette,
    redesign_border_strong, redesign_input_bg, redesign_text_faint, redesign_text_primary,
};

pub fn render(ui: &mut egui::Ui, palette: ThemePalette, state: &mut CreateScreenState) {
    redesign_box(ui, palette, Some("paste share code"), |ui| {
        ui.label(
            egui::RichText::new("Paste a BIO-MODLIST-V1 share code to preview a fork.")
                .size(REDESIGN_LABEL_FONT_SIZE_PX)
                .color(redesign_text_primary(palette)),
        );
        ui.add_space(REDESIGN_HOME_CARD_LIST_GAP_PX);
        egui::Frame::NONE
            .fill(redesign_input_bg(palette))
            .stroke(egui::Stroke::new(
                REDESIGN_BORDER_WIDTH_PX,
                redesign_border_strong(palette),
            ))
            .corner_radius(REDESIGN_BORDER_RADIUS_PX)
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut state.fork_code)
                        .desired_rows(10)
                        .desired_width(ui.available_width())
                        .text_color(redesign_text_primary(palette))
                        .hint_text("BIO-MODLIST-V1 ...")
                        .frame(false),
                );
            });
        ui.add_space(REDESIGN_HOME_CARD_LIST_GAP_PX);
        ui.horizontal(|ui| {
            if redesign_btn(ui, palette, "← back", BtnOpts::default()).clicked() {
                state.stage = CreateStage::Choose;
            }
            if redesign_btn(
                ui,
                palette,
                "preview →",
                BtnOpts {
                    primary: true,
                    disabled: state.fork_code.trim().is_empty(),
                    ..Default::default()
                },
            )
            .clicked()
            {
                match crate::app::modlist_share::preview_modlist_share_code(&state.fork_code) {
                    Ok(_) => {
                        state.fork_preview_ready = true;
                        state.fork_preview_error = None;
                    }
                    Err(err) => {
                        state.fork_preview_ready = false;
                        state.fork_preview_error = Some(err);
                    }
                }
                state.stage = CreateStage::ForkPreview;
            }
        });
        if let Some(error) = &state.fork_preview_error {
            ui.label(
                egui::RichText::new(error)
                    .size(REDESIGN_HINT_FONT_SIZE_PX)
                    .color(redesign_text_faint(palette)),
            );
        }
    });
}
