// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_strong,
};

pub struct InputOpts<'a> {
    pub edit: egui::TextEdit<'a>,
    pub margin: egui::Margin,
    pub size: egui::Vec2,
    pub border: Option<egui::Color32>,
}

pub fn redesign_text_input(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    opts: InputOpts<'_>,
) -> egui::Response {
    let response = ui.add_sized(opts.size, opts.edit);

    let outer_rect = response.rect + opts.margin;

    let color = opts
        .border
        .unwrap_or_else(|| redesign_border_strong(palette));
    ui.painter().rect_stroke(
        outer_rect,
        egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8),
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, color),
        egui::StrokeKind::Inside,
    );

    response
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_f32_near {
        ($actual:expr, $expected:expr) => {
            assert!(($actual - $expected).abs() <= f32::EPSILON);
        };
    }

    #[test]
    fn outer_rect_is_inner_plus_margin() {
        let margin = egui::Margin::symmetric(12, 8);
        let inner = egui::Rect::from_min_size(egui::pos2(100.0, 50.0), egui::vec2(200.0, 14.0));
        let outer = inner + margin;
        assert_eq!(outer - margin, inner);
        assert_f32_near!(outer.width(), inner.width() + 24.0);
        assert_f32_near!(outer.height(), inner.height() + 16.0);
        assert_f32_near!(outer.left(), inner.left() - 12.0);
        assert_f32_near!(outer.top(), inner.top() - 8.0);
    }

    #[test]
    fn asymmetric_margin_recovers_each_side() {
        let margin = egui::Margin {
            left: 8,
            right: 4,
            top: 5,
            bottom: 5,
        };
        let inner = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(50.0, 20.0));
        let outer = inner + margin;
        assert_f32_near!(outer.left(), -8.0);
        assert_f32_near!(outer.right(), 54.0);
        assert_f32_near!(outer.top(), -5.0);
        assert_f32_near!(outer.bottom(), 25.0);
    }
}
