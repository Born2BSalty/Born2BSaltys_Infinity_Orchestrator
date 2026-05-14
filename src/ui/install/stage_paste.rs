// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::install::state_install::{InstallPreviewTab, InstallScreenState, InstallStage};
use crate::ui::orchestrator::widgets::btn::{BtnOpts, redesign_btn};
use crate::ui::orchestrator::widgets::r_box::redesign_box;
use crate::ui::orchestrator::widgets::screen_title;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX, REDESIGN_HINT_FONT_SIZE_PX,
    REDESIGN_INPUT_MIN_HEIGHT_PX, REDESIGN_LABEL_FONT_SIZE_PX, REDESIGN_PATH_INPUT_PADDING_X_PX,
    REDESIGN_PATH_INPUT_PADDING_Y_PX, REDESIGN_SETTINGS_ROW_GAP_PX,
    REDESIGN_SUBFLOW_SECTION_GAP_PX, ThemePalette, redesign_border_strong, redesign_font_mono,
    redesign_input_bg, redesign_text_muted, redesign_text_primary,
};

pub(super) fn render(ui: &mut egui::Ui, palette: ThemePalette, state: &mut InstallScreenState) {
    screen_title::render(
        ui,
        palette,
        "Install shared modlist",
        Some("set destination + mods paths, paste a BIO share code, then preview before importing"),
    );

    redesign_box(ui, palette, Some("destination folder"), |ui| {
        ui.spacing_mut().item_spacing.y = REDESIGN_SETTINGS_ROW_GAP_PX;
        render_text_input(
            ui,
            palette,
            &mut state.destination,
            "D:\\BG2EE_install_test",
        );
        ui.label(
            egui::RichText::new("Browse and destination-not-empty choices are not wired yet.")
                .size(REDESIGN_HINT_FONT_SIZE_PX)
                .color(redesign_text_muted(palette)),
        );
    });

    ui.add_space(REDESIGN_SUBFLOW_SECTION_GAP_PX);
    redesign_box(ui, palette, Some("import code"), |ui| {
        ui.spacing_mut().item_spacing.y = REDESIGN_SETTINGS_ROW_GAP_PX;
        ui.label(
            egui::RichText::new("BIO-MODLIST-V1 share code")
                .size(REDESIGN_LABEL_FONT_SIZE_PX)
                .color(redesign_text_primary(palette)),
        );
        render_code_input(ui, palette, &mut state.import_code);
        if let Some(error) = state.preview_error.as_deref() {
            ui.label(
                egui::RichText::new(error)
                    .size(REDESIGN_HINT_FONT_SIZE_PX)
                    .color(redesign_text_muted(palette)),
            );
        }
    });

    ui.add_space(REDESIGN_SUBFLOW_SECTION_GAP_PX);
    if redesign_btn(
        ui,
        palette,
        "Preview ->",
        BtnOpts {
            primary: true,
            disabled: state.import_code.trim().is_empty(),
            ..Default::default()
        },
    )
    .clicked()
    {
        match crate::app::modlist_share::preview_modlist_share_code(&state.import_code) {
            Ok(preview) => {
                state.preview = Some(preview);
                state.preview_error = None;
                state.preview_tab = InstallPreviewTab::Summary;
                state.stage = InstallStage::Preview;
            }
            Err(err) => {
                state.preview = None;
                state.preview_error = Some(err);
            }
        }
    }
    ui.label(
        egui::RichText::new("no install starts until preview is accepted")
            .size(REDESIGN_HINT_FONT_SIZE_PX)
            .color(redesign_text_muted(palette)),
    );
}

fn render_text_input(ui: &mut egui::Ui, palette: ThemePalette, value: &mut String, hint: &str) {
    egui::Frame::NONE
        .fill(redesign_input_bg(palette))
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_border_strong(palette),
        ))
        .corner_radius(REDESIGN_BORDER_RADIUS_PX)
        .inner_margin(egui::Margin::symmetric(
            REDESIGN_PATH_INPUT_PADDING_X_PX as i8,
            REDESIGN_PATH_INPUT_PADDING_Y_PX as i8,
        ))
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::singleline(value)
                    .hint_text(hint)
                    .text_color(redesign_text_primary(palette))
                    .frame(false),
            );
        });
}

fn render_code_input(ui: &mut egui::Ui, palette: ThemePalette, value: &mut String) {
    egui::Frame::NONE
        .fill(redesign_input_bg(palette))
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_border_strong(palette),
        ))
        .corner_radius(REDESIGN_BORDER_RADIUS_PX)
        .inner_margin(egui::Margin::symmetric(
            REDESIGN_PATH_INPUT_PADDING_X_PX as i8,
            REDESIGN_PATH_INPUT_PADDING_Y_PX as i8,
        ))
        .show(ui, |ui| {
            ui.add_sized(
                [ui.available_width(), REDESIGN_INPUT_MIN_HEIGHT_PX],
                egui::TextEdit::multiline(value)
                    .hint_text(
                        "BIO-MODLIST-V1:eJyrVkrLz1eyUkpKLFKqBQA...\n\nPaste the full code here.",
                    )
                    .text_color(redesign_text_primary(palette))
                    .font(egui::FontId::new(
                        REDESIGN_LABEL_FONT_SIZE_PX,
                        redesign_font_mono(),
                    ))
                    .frame(false),
            );
        });
}
