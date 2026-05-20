// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::orchestrator::widgets::{InputOpts, redesign_text_input};
use crate::ui::shared::redesign_tokens::{
    ThemePalette, redesign_input_bg, redesign_text_faint, redesign_text_primary,
};

pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    label: &str,
    value: &mut String,
    placeholder: Option<&str>,
    hint: Option<&str>,
    mut on_change: impl FnMut(),
) {
    ui.horizontal(|ui| {
        let label_width = 200.0;
        let (label_rect, _) =
            ui.allocate_exact_size(egui::vec2(label_width, 26.0), egui::Sense::hover());
        ui.painter().text(
            egui::pos2(label_rect.left(), label_rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::new(13.0, egui::FontFamily::Name("poppins_medium".into())),
            redesign_text_primary(palette),
        );

        let pre = value.clone();
        let edit_width = 140.0;
        let margin = egui::Margin::symmetric(6, 4);
        let mut edit = egui::TextEdit::singleline(value)
            .font(egui::FontId::new(
                12.0,
                egui::FontFamily::Name("firacode_nerd".into()),
            ))
            .text_color(redesign_text_primary(palette))
            .background_color(redesign_input_bg(palette))
            .margin(margin);
        if let Some(ph) = placeholder {
            edit = edit.hint_text(
                egui::RichText::new(ph)
                    .size(12.0)
                    .family(egui::FontFamily::Name("firacode_nerd".into()))
                    .color(redesign_text_faint(palette)),
            );
        }
        let response = redesign_text_input(
            ui,
            palette,
            InputOpts {
                edit,
                margin,
                size: egui::vec2(edit_width, 24.0),
                border: None,
            },
        );
        if response.changed() || *value != pre {
            on_change();
        }

        if let Some(hint_text) = hint {
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new(hint_text)
                    .size(11.0)
                    .family(egui::FontFamily::Proportional)
                    .color(redesign_text_faint(palette)),
            );
        }
    });
    ui.add_space(4.0);
}
