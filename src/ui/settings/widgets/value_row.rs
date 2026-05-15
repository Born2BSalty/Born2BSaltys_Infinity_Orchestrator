// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_strong, redesign_input_bg,
    redesign_text_faint, redesign_text_primary,
};

pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    label: &str,
    value: &mut String,
    placeholder: &str,
    hint: Option<&str>,
) {
    ui.horizontal(|ui| {
        ui.add_sized(
            [190.0, 20.0],
            egui::Label::new(
                egui::RichText::new(label)
                    .size(14.0)
                    .color(redesign_text_primary(palette)),
            ),
        );
        egui::Frame::NONE
            .fill(redesign_input_bg(palette))
            .stroke(egui::Stroke::new(
                REDESIGN_BORDER_WIDTH_PX,
                redesign_border_strong(palette),
            ))
            .corner_radius(3.0)
            .inner_margin(egui::Margin::symmetric(8, 3))
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::singleline(value)
                        .desired_width(96.0)
                        .hint_text(placeholder)
                        .text_color(redesign_text_primary(palette))
                        .frame(false),
                );
            });
        if let Some(hint) = hint {
            ui.label(
                egui::RichText::new(hint)
                    .size(12.0)
                    .color(redesign_text_faint(palette)),
            );
        }
    });
}
