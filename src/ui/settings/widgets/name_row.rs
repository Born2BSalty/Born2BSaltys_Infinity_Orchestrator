// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_WIDTH_PX, REDESIGN_DASHED_BORDER_WIDTH_PX, REDESIGN_HINT_FONT_SIZE_PX,
    REDESIGN_LABEL_FONT_SIZE_PX, REDESIGN_SETTINGS_ROW_GAP_PX, REDESIGN_SETTINGS_ROW_PADDING_Y_PX,
    ThemePalette, redesign_border_dashed_light, redesign_border_strong, redesign_input_bg,
    redesign_text_muted, redesign_text_primary,
};

pub fn render(ui: &mut egui::Ui, palette: ThemePalette, name: &mut String) {
    let row_width = ui.available_width();
    let response = egui::Frame::NONE
        .inner_margin(egui::Margin::symmetric(
            0,
            REDESIGN_SETTINGS_ROW_PADDING_Y_PX as i8,
        ))
        .show(ui, |ui| {
            ui.set_width(row_width);
            ui.horizontal(|ui| {
                ui.set_width(row_width);
                ui.allocate_ui_with_layout(
                    egui::vec2(
                        (row_width - 220.0 - REDESIGN_SETTINGS_ROW_GAP_PX).max(row_width * 0.45),
                        0.0,
                    ),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.label(
                            egui::RichText::new("Your name")
                                .size(REDESIGN_LABEL_FONT_SIZE_PX)
                                .color(redesign_text_primary(palette)),
                        );
                        ui.label(
                            egui::RichText::new(
                                "credited as the author on any modlists you create or share",
                            )
                            .size(REDESIGN_HINT_FONT_SIZE_PX)
                            .color(redesign_text_muted(palette)),
                        );
                    },
                );
                ui.add_space(REDESIGN_SETTINGS_ROW_GAP_PX);
                egui::Frame::NONE
                    .fill(redesign_input_bg(palette))
                    .stroke(egui::Stroke::new(
                        REDESIGN_BORDER_WIDTH_PX,
                        redesign_border_strong(palette),
                    ))
                    .corner_radius(3.0)
                    .inner_margin(egui::Margin::symmetric(10, 4))
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::singleline(name)
                                .desired_width(200.0)
                                .text_color(redesign_text_primary(palette))
                                .frame(false),
                        );
                    });
            });
        })
        .response;

    ui.painter().line_segment(
        [response.rect.left_bottom(), response.rect.right_bottom()],
        egui::Stroke::new(
            REDESIGN_DASHED_BORDER_WIDTH_PX,
            redesign_border_dashed_light(palette),
        ),
    );
    ui.add_space(4.0);
}
