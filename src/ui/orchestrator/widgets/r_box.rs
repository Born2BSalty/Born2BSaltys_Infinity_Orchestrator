// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX, REDESIGN_BOX_LABEL_FONT_SIZE_PX,
    REDESIGN_BOX_LABEL_GAP_PX, REDESIGN_SETTINGS_BOX_PADDING_X_PX,
    REDESIGN_SETTINGS_BOX_PADDING_Y_PX, ThemePalette, redesign_border_strong, redesign_shell_bg,
    redesign_text_muted,
};

pub fn redesign_box<R>(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    label: Option<&str>,
    body: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
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
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            if let Some(label) = label {
                ui.label(
                    egui::RichText::new(label)
                        .size(REDESIGN_BOX_LABEL_FONT_SIZE_PX)
                        .color(redesign_text_muted(palette)),
                );
                ui.add_space(REDESIGN_BOX_LABEL_GAP_PX);
            }
            body(ui)
        })
}
