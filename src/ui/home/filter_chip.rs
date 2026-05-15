// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_WIDTH_PX, REDESIGN_FILTER_CHIP_FONT_SIZE_PX, REDESIGN_FILTER_CHIP_LABEL_GAP_PX,
    REDESIGN_FILTER_CHIP_PADDING_X_PX, REDESIGN_FILTER_CHIP_PADDING_Y_PX,
    REDESIGN_FILTER_CHIP_RADIUS_PX, ThemePalette, redesign_accent, redesign_border_strong,
    redesign_font_light, redesign_font_medium, redesign_shell_bg, redesign_text_faint,
    redesign_text_on_accent, redesign_text_primary,
};

pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    label: &str,
    count: usize,
    active: bool,
) -> egui::Response {
    let fill = if active {
        redesign_accent(palette)
    } else {
        redesign_shell_bg(palette)
    };
    let label_color = if active {
        redesign_text_on_accent(palette)
    } else {
        redesign_text_primary(palette)
    };
    let count_color = if active {
        redesign_text_on_accent(palette)
    } else {
        redesign_text_faint(palette)
    };

    let response = egui::Frame::NONE
        .fill(fill)
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_border_strong(palette),
        ))
        .corner_radius(REDESIGN_FILTER_CHIP_RADIUS_PX)
        .inner_margin(egui::Margin::symmetric(
            crate::ui::shared::redesign_tokens::redesign_i8_px(REDESIGN_FILTER_CHIP_PADDING_X_PX),
            crate::ui::shared::redesign_tokens::redesign_i8_px(REDESIGN_FILTER_CHIP_PADDING_Y_PX),
        ))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = REDESIGN_FILTER_CHIP_LABEL_GAP_PX;
                ui.label(
                    egui::RichText::new(label)
                        .family(if active {
                            redesign_font_medium()
                        } else {
                            redesign_font_light()
                        })
                        .size(REDESIGN_FILTER_CHIP_FONT_SIZE_PX)
                        .color(label_color),
                );
                ui.label(
                    egui::RichText::new(format!("({count})"))
                        .family(redesign_font_light())
                        .size(REDESIGN_FILTER_CHIP_FONT_SIZE_PX)
                        .color(count_color),
                );
            });
        })
        .response;

    response.interact(egui::Sense::click())
}
