// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::orchestrator::widgets::btn::{BtnOpts, redesign_btn};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX, REDESIGN_HOME_ACTION_COLUMN_GAP_PX,
    REDESIGN_HOME_CONFIRM_WIDTH_PX, REDESIGN_LABEL_FONT_SIZE_PX,
    REDESIGN_SETTINGS_BOX_PADDING_X_PX, REDESIGN_SETTINGS_BOX_PADDING_Y_PX,
    REDESIGN_TAB_FONT_SIZE_PX, ThemePalette, redesign_border_strong, redesign_input_bg,
    redesign_shell_bg, redesign_success, redesign_text_faint, redesign_text_muted,
    redesign_text_primary,
};

pub fn render(ctx: &egui::Context, palette: ThemePalette, open: &mut bool, code: Option<&str>) {
    if !*open {
        return;
    }

    let mut close_requested = false;

    egui::Window::new("Share import code")
        .collapsible(true)
        .resizable(true)
        .frame(dialog_frame(palette))
        .default_width(REDESIGN_HOME_CONFIRM_WIDTH_PX)
        .open(open)
        .show(ctx, |ui| {
            render_header(ui, palette);
            ui.add_space(REDESIGN_HOME_ACTION_COLUMN_GAP_PX);

            code_frame(palette).show(ui, |ui| render_code(ui, palette, code));

            ui.add_space(REDESIGN_HOME_ACTION_COLUMN_GAP_PX);
            render_actions(ui, palette, code, &mut close_requested);
        });

    if close_requested {
        *open = false;
    }
}

fn dialog_frame(palette: ThemePalette) -> egui::Frame {
    egui::Frame::NONE
        .fill(redesign_shell_bg(palette))
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_border_strong(palette),
        ))
        .corner_radius(REDESIGN_BORDER_RADIUS_PX)
        .inner_margin(egui::Margin::symmetric(
            crate::ui::shared::redesign_tokens::redesign_i8_px(REDESIGN_SETTINGS_BOX_PADDING_X_PX),
            crate::ui::shared::redesign_tokens::redesign_i8_px(REDESIGN_SETTINGS_BOX_PADDING_Y_PX),
        ))
}

fn code_frame(palette: ThemePalette) -> egui::Frame {
    egui::Frame::NONE
        .fill(redesign_input_bg(palette))
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_border_strong(palette),
        ))
        .corner_radius(REDESIGN_BORDER_RADIUS_PX)
        .inner_margin(egui::Margin::symmetric(
            crate::ui::shared::redesign_tokens::redesign_i8_px(REDESIGN_SETTINGS_BOX_PADDING_X_PX),
            crate::ui::shared::redesign_tokens::redesign_i8_px(REDESIGN_SETTINGS_BOX_PADDING_Y_PX),
        ))
}

fn render_header(ui: &mut egui::Ui, palette: ThemePalette) {
    ui.label(
        egui::RichText::new("Share import code")
            .size(REDESIGN_TAB_FONT_SIZE_PX)
            .strong()
            .color(redesign_text_primary(palette)),
    );
    ui.label(
        egui::RichText::new("Anyone can paste this into BIO → Install to get the same modlist.")
            .size(REDESIGN_LABEL_FONT_SIZE_PX)
            .color(redesign_text_muted(palette)),
    );
}

fn render_code(ui: &mut egui::Ui, palette: ThemePalette, code: Option<&str>) {
    let text = code
        .unwrap_or("Import code is not available yet. Share-code generation lands in Batch 8.2.");
    ui.label(
        egui::RichText::new(text)
            .monospace()
            .size(REDESIGN_LABEL_FONT_SIZE_PX)
            .color(if code.is_some() {
                redesign_text_primary(palette)
            } else {
                redesign_text_faint(palette)
            }),
    );
}

fn render_actions(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    code: Option<&str>,
    close_requested: &mut bool,
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = REDESIGN_HOME_ACTION_COLUMN_GAP_PX;
        if code.is_none() {
            ui.label(
                egui::RichText::new("code unavailable")
                    .size(REDESIGN_LABEL_FONT_SIZE_PX)
                    .color(redesign_text_faint(palette)),
            );
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if redesign_btn(ui, palette, "Close", BtnOpts::default()).clicked() {
                *close_requested = true;
            }
            if redesign_btn(
                ui,
                palette,
                "Copy",
                BtnOpts {
                    primary: true,
                    disabled: code.is_none(),
                    ..Default::default()
                },
            )
            .clicked()
                && let Some(code) = code
            {
                ui.ctx().copy_text(code.to_string());
            }
        });
        if code.is_some() {
            ui.label(
                egui::RichText::new("ready to copy")
                    .size(REDESIGN_LABEL_FONT_SIZE_PX)
                    .color(redesign_success(palette)),
            );
        }
    });
}
