// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_accent,
    redesign_border_strong, redesign_chrome_bg, redesign_text_muted, redesign_text_primary,
};

pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    options: [(&str, bool); 2],
) -> Option<usize> {
    let segment_width = 90.0;
    let height = 26.0;
    let mut clicked: Option<usize> = None;
    let radius = egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8);
    let border = egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette));

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
        for (i, (label, selected)) in options.iter().enumerate() {
            let (rect, response) =
                ui.allocate_exact_size(egui::vec2(segment_width, height), egui::Sense::click());
            let painter = ui.painter();
            let fill = if *selected {
                redesign_accent(palette)
            } else {
                redesign_chrome_bg(palette)
            };
            painter.rect_filled(rect, radius, fill);
            painter.rect_stroke(rect, radius, border, egui::StrokeKind::Inside);

            let text_color = if *selected {
                egui::Color32::from_rgb(0x1a, 0x26, 0x38)
            } else if response.hovered() {
                redesign_text_primary(palette)
            } else {
                redesign_text_muted(palette)
            };
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                *label,
                egui::FontId::new(12.0, egui::FontFamily::Name("poppins_medium".into())),
                text_color,
            );

            if response.clicked() {
                clicked = Some(i);
            }
        }
    });

    clicked
}
