// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, ThemePalette, redesign_pill_danger, redesign_pill_info,
    redesign_pill_neutral, redesign_pill_text, redesign_pill_warn,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PillTone {
    Danger,
    Warn,
    Info,
    Neutral,
}

impl PillTone {
    const fn fill(self, palette: ThemePalette) -> egui::Color32 {
        match self {
            Self::Danger => redesign_pill_danger(palette),
            Self::Warn => redesign_pill_warn(palette),
            Self::Info => redesign_pill_info(palette),
            Self::Neutral => redesign_pill_neutral(palette),
        }
    }
}

pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    label: &str,
    tone: PillTone,
) -> egui::Response {
    let pad_x = 8.0;
    let pad_y = 2.0;
    let text_color = redesign_pill_text(palette);
    let font = egui::FontId::new(11.0, egui::FontFamily::Name("poppins_medium".into()));
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font.clone(), text_color);
    let desired = egui::vec2(galley.size().x + pad_x * 2.0, galley.size().y + pad_y * 2.0);
    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::hover());

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        painter.rect_filled(
            rect,
            egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8),
            tone.fill(palette),
        );
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            font,
            text_color,
        );
    }

    response
}
