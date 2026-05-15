// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `toggle_row` — label + on/off switch + optional hint.
//
// Per Phase 4 P4.T8 file inventory: simple binary toggle used in the
// General + Advanced sub-tabs.

// rationale: `f32 as u8` casts are colour-channel / pixel roundings of small
// positive values — correct by construction (Cat 2).
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    redesign_accent, redesign_border_strong, redesign_chrome_bg, redesign_text_faint,
    redesign_text_primary, ThemePalette, REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX,
};

pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    label: &str,
    on: &mut bool,
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

        let (rect, response) = ui.allocate_exact_size(egui::vec2(42.0, 22.0), egui::Sense::click());
        let painter = ui.painter();
        let radius = egui::CornerRadius::same((REDESIGN_BORDER_RADIUS_PX + 8.0) as u8);
        let track_fill = if *on {
            redesign_accent(palette)
        } else {
            redesign_chrome_bg(palette)
        };
        painter.rect_filled(rect, radius, track_fill);
        painter.rect_stroke(
            rect,
            radius,
            egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
            egui::StrokeKind::Inside,
        );
        // Knob.
        let knob_size = 16.0;
        let knob_center = if *on {
            egui::pos2(rect.right() - knob_size * 0.5 - 4.0, rect.center().y)
        } else {
            egui::pos2(rect.left() + knob_size * 0.5 + 4.0, rect.center().y)
        };
        painter.circle_filled(
            knob_center,
            knob_size * 0.5,
            egui::Color32::from_rgb(0xE6, 0xED, 0xF3),
        );
        painter.circle_stroke(
            knob_center,
            knob_size * 0.5,
            egui::Stroke::new(1.0, redesign_border_strong(palette)),
        );

        if response.clicked() {
            *on = !*on;
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
