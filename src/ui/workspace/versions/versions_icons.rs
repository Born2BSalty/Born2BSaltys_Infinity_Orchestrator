// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

pub(crate) const GLYPH_LOCK: &str = "\u{F023}";
pub(crate) const GLYPH_UNLOCK: &str = "\u{F09C}";
pub(crate) const GLYPH_EXTERNAL_LINK: &str = "\u{F08E}";
const GLYPH_FONT_SIZE: f32 = 14.0;

fn icon_stroke(color: egui::Color32) -> egui::Stroke {
    egui::Stroke::new(1.6_f32, color)
}

pub(crate) fn paint_glyph(
    painter: &egui::Painter,
    center: egui::Pos2,
    glyph: &str,
    color: egui::Color32,
) {
    painter.text(
        center,
        egui::Align2::CENTER_CENTER,
        glyph,
        egui::FontId::new(
            GLYPH_FONT_SIZE,
            egui::FontFamily::Name("firacode_nerd".into()),
        ),
        color,
    );
}

pub(crate) fn paint_down_arrow(painter: &egui::Painter, center: egui::Pos2, color: egui::Color32) {
    let stroke = icon_stroke(color);
    let shaft_top = center.y - 6.0;
    let shaft_bottom = center.y + 1.5;
    painter.line_segment(
        [
            egui::pos2(center.x, shaft_top),
            egui::pos2(center.x, shaft_bottom),
        ],
        stroke,
    );
    painter.line_segment(
        [
            egui::pos2(center.x - 4.0, shaft_bottom - 4.0),
            egui::pos2(center.x, shaft_bottom),
        ],
        stroke,
    );
    painter.line_segment(
        [
            egui::pos2(center.x + 4.0, shaft_bottom - 4.0),
            egui::pos2(center.x, shaft_bottom),
        ],
        stroke,
    );
    painter.line_segment(
        [
            egui::pos2(center.x - 5.0, center.y + 6.0),
            egui::pos2(center.x + 5.0, center.y + 6.0),
        ],
        stroke,
    );
}

pub(crate) fn paint_kebab(painter: &egui::Painter, center: egui::Pos2, color: egui::Color32) {
    for dy in [-6.0_f32, 0.0, 6.0] {
        painter.circle_filled(center + egui::vec2(0.0, dy), 1.4, color);
    }
}

pub(crate) fn paint_caret_down(painter: &egui::Painter, center: egui::Pos2, color: egui::Color32) {
    let stroke = egui::Stroke::new(1.7_f32, color);
    painter.line_segment(
        [
            center + egui::vec2(-4.0, -2.0),
            center + egui::vec2(0.0, 2.0),
        ],
        stroke,
    );
    painter.line_segment(
        [
            center + egui::vec2(0.0, 2.0),
            center + egui::vec2(4.0, -2.0),
        ],
        stroke,
    );
}

pub(crate) fn paint_check(painter: &egui::Painter, center: egui::Pos2, color: egui::Color32) {
    let stroke = egui::Stroke::new(1.8_f32, color);
    painter.line_segment(
        [
            center + egui::vec2(-4.0, 0.0),
            center + egui::vec2(-1.0, 3.0),
        ],
        stroke,
    );
    painter.line_segment(
        [
            center + egui::vec2(-1.0, 3.0),
            center + egui::vec2(4.5, -3.5),
        ],
        stroke,
    );
}

pub(crate) fn elide(
    painter: &egui::Painter,
    text: &str,
    font: &egui::FontId,
    max_w: f32,
) -> String {
    let probe = egui::Color32::WHITE;
    if painter
        .layout_no_wrap(text.to_string(), font.clone(), probe)
        .size()
        .x
        <= max_w
    {
        return text.to_string();
    }
    let mut chars: Vec<char> = text.chars().collect();
    while !chars.is_empty() {
        chars.pop();
        let candidate: String = chars.iter().collect::<String>() + "\u{2026}";
        if painter
            .layout_no_wrap(candidate.clone(), font.clone(), probe)
            .size()
            .x
            <= max_w
        {
            return candidate;
        }
    }
    "\u{2026}".to_string()
}

pub(crate) fn paint_chevron_right(
    painter: &egui::Painter,
    center: egui::Pos2,
    color: egui::Color32,
) {
    let stroke = egui::Stroke::new(1.7_f32, color);
    painter.line_segment(
        [
            center + egui::vec2(-2.0, -4.0),
            center + egui::vec2(2.0, 0.0),
        ],
        stroke,
    );
    painter.line_segment(
        [
            center + egui::vec2(2.0, 0.0),
            center + egui::vec2(-2.0, 4.0),
        ],
        stroke,
    );
}

pub(crate) fn paint_note(painter: &egui::Painter, center: egui::Pos2, color: egui::Color32) {
    let stroke = egui::Stroke::new(1.4_f32, color);
    let page = egui::Rect::from_center_size(center, egui::vec2(10.0, 8.0));
    painter.rect_filled(
        page,
        egui::CornerRadius::same(1),
        color.gamma_multiply(0.18),
    );
    painter.rect_stroke(
        page,
        egui::CornerRadius::same(1),
        stroke,
        egui::StrokeKind::Inside,
    );
    for dy in [-1.5_f32, 1.5] {
        painter.line_segment(
            [
                egui::pos2(page.left() + 2.0, center.y + dy),
                egui::pos2(page.right() - 2.0, center.y + dy),
            ],
            egui::Stroke::new(1.0_f32, color),
        );
    }
}

pub(crate) fn paint_search(painter: &egui::Painter, center: egui::Pos2, color: egui::Color32) {
    let stroke = icon_stroke(color);
    let lens_center = center + egui::vec2(-1.5, -1.5);
    painter.circle_stroke(lens_center, 4.0, stroke);
    let handle_start = lens_center + egui::vec2(2.8, 2.8);
    let handle_end = center + egui::vec2(4.5, 4.5);
    painter.line_segment([handle_start, handle_end], stroke);
}
