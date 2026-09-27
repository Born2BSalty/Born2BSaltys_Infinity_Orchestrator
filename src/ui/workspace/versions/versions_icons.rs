// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

fn icon_stroke(color: egui::Color32) -> egui::Stroke {
    egui::Stroke::new(1.6_f32, color)
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

pub(crate) fn paint_lock(
    painter: &egui::Painter,
    center: egui::Pos2,
    color: egui::Color32,
    locked: bool,
) {
    let stroke = icon_stroke(color);
    let body = egui::Rect::from_center_size(center + egui::vec2(0.0, 2.5), egui::vec2(11.0, 7.5));
    painter.rect_stroke(
        body,
        egui::CornerRadius::same(2),
        stroke,
        egui::StrokeKind::Inside,
    );

    let shackle_r = 4.0_f32;
    let left_leg_x = body.left() + 2.5;
    let right_leg_x = if locked {
        body.right() - 2.5
    } else {
        body.right() - 0.5
    };
    let shackle_center_x = left_leg_x.midpoint(right_leg_x);
    let shackle_center_y = body.top();

    let arc_points: Vec<egui::Pos2> = (0_u8..=12)
        .map(|i| {
            let t = std::f32::consts::PI * (1.0 - f32::from(i) / 12.0);
            egui::pos2(
                t.cos().mul_add(shackle_r, shackle_center_x),
                t.sin().mul_add(-shackle_r, shackle_center_y),
            )
        })
        .collect();
    painter.line(arc_points, stroke);
    painter.line_segment(
        [
            egui::pos2(left_leg_x, shackle_center_y),
            egui::pos2(left_leg_x, body.top()),
        ],
        stroke,
    );
    painter.line_segment(
        [
            egui::pos2(right_leg_x, shackle_center_y),
            egui::pos2(right_leg_x, body.top()),
        ],
        stroke,
    );
}

pub(crate) fn paint_external_link(
    painter: &egui::Painter,
    center: egui::Pos2,
    color: egui::Color32,
) {
    let stroke = icon_stroke(color);
    let box_rect = egui::Rect::from_min_size(center + egui::vec2(-6.0, -1.0), egui::vec2(9.0, 8.0));
    painter.rect_stroke(
        box_rect,
        egui::CornerRadius::same(2),
        stroke,
        egui::StrokeKind::Inside,
    );
    let arrow_from = center + egui::vec2(-1.0, 0.0);
    let arrow_to = center + egui::vec2(6.0, -7.0);
    painter.line_segment([arrow_from, arrow_to], stroke);
    painter.line_segment([arrow_to, egui::pos2(arrow_to.x - 5.0, arrow_to.y)], stroke);
    painter.line_segment([arrow_to, egui::pos2(arrow_to.x, arrow_to.y + 5.0)], stroke);
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

pub(crate) fn paint_search(painter: &egui::Painter, center: egui::Pos2, color: egui::Color32) {
    let stroke = icon_stroke(color);
    let lens_center = center + egui::vec2(-1.5, -1.5);
    painter.circle_stroke(lens_center, 4.0, stroke);
    let handle_start = lens_center + egui::vec2(2.8, 2.8);
    let handle_end = center + egui::vec2(4.5, 4.5);
    painter.line_segment([handle_start, handle_end], stroke);
}
