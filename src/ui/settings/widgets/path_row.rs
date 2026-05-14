// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::orchestrator::widgets::btn::{self, BtnOpts};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX, REDESIGN_DASHED_BORDER_WIDTH_PX,
    REDESIGN_HINT_FONT_SIZE_PX, REDESIGN_LABEL_FONT_SIZE_PX, REDESIGN_PATH_BUTTON_WIDTH_PX,
    REDESIGN_PATH_INPUT_FONT_SIZE_PX, REDESIGN_PATH_INPUT_PADDING_X_PX,
    REDESIGN_PATH_INPUT_PADDING_Y_PX, REDESIGN_PATH_ROW_GAP_PX, REDESIGN_PATH_ROW_HEIGHT_PX,
    REDESIGN_PATH_ROW_HINT_WIDTH_PX, REDESIGN_PATH_ROW_LABEL_WIDTH_PX, ThemePalette,
    redesign_border_dashed_light, redesign_border_strong, redesign_input_bg, redesign_text_faint,
    redesign_text_primary,
};

pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    label: &str,
    value: &mut String,
    hint: Option<&str>,
) -> bool {
    let mut changed = false;
    let row_width = ui.available_width();
    let hint_width = if hint.is_some() {
        REDESIGN_PATH_ROW_HINT_WIDTH_PX
    } else {
        0.0
    };
    let input_width = (row_width
        - REDESIGN_PATH_ROW_LABEL_WIDTH_PX
        - REDESIGN_PATH_BUTTON_WIDTH_PX
        - hint_width
        - (REDESIGN_PATH_ROW_GAP_PX * if hint.is_some() { 3.0 } else { 2.0 }))
    .max(120.0);

    let response = ui
        .allocate_ui_with_layout(
            egui::vec2(row_width, REDESIGN_PATH_ROW_HEIGHT_PX),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.spacing_mut().item_spacing.x = REDESIGN_PATH_ROW_GAP_PX;
                ui.add_sized(
                    [
                        REDESIGN_PATH_ROW_LABEL_WIDTH_PX,
                        REDESIGN_PATH_ROW_HEIGHT_PX,
                    ],
                    egui::Label::new(
                        egui::RichText::new(label)
                            .size(REDESIGN_LABEL_FONT_SIZE_PX)
                            .color(redesign_text_primary(palette)),
                    ),
                );
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
                        changed = ui
                            .add(
                                egui::TextEdit::singleline(value)
                                    .desired_width(input_width)
                                    .font(egui::FontId::proportional(
                                        REDESIGN_PATH_INPUT_FONT_SIZE_PX,
                                    ))
                                    .text_color(redesign_text_primary(palette))
                                    .frame(false),
                            )
                            .changed();
                    });
                if let Some(hint) = hint {
                    ui.add_sized(
                        [REDESIGN_PATH_ROW_HINT_WIDTH_PX, REDESIGN_PATH_ROW_HEIGHT_PX],
                        egui::Label::new(
                            egui::RichText::new(hint)
                                .size(REDESIGN_HINT_FONT_SIZE_PX)
                                .color(redesign_text_faint(palette)),
                        ),
                    );
                }
                ui.allocate_ui_with_layout(
                    egui::vec2(REDESIGN_PATH_BUTTON_WIDTH_PX, REDESIGN_PATH_ROW_HEIGHT_PX),
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        btn::redesign_btn(
                            ui,
                            palette,
                            "browse",
                            BtnOpts {
                                primary: false,
                                small: true,
                                disabled: true,
                            },
                        );
                    },
                );
            },
        )
        .response;

    ui.painter().line_segment(
        [response.rect.left_bottom(), response.rect.right_bottom()],
        egui::Stroke::new(
            REDESIGN_DASHED_BORDER_WIDTH_PX,
            redesign_border_dashed_light(palette),
        ),
    );
    changed
}
