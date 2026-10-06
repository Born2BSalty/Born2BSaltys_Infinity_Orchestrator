// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_WIDTH_PX, REDESIGN_PANEL_RADIUS_U8, ThemePalette, redesign_accent,
    redesign_border_strong, redesign_chrome_bg,
};

pub fn toggle_switch(ui: &mut egui::Ui, palette: ThemePalette, on: &mut bool) -> bool {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(42.0, 22.0), egui::Sense::click());
    let painter = ui.painter();
    let radius = egui::CornerRadius::same(REDESIGN_PANEL_RADIUS_U8);
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
    let knob_size = 16.0;
    let knob_center = if *on {
        egui::pos2(
            f32::mul_add(knob_size, -0.5, rect.right()) - 4.0,
            rect.center().y,
        )
    } else {
        egui::pos2(
            f32::mul_add(knob_size, 0.5, rect.left()) + 4.0,
            rect.center().y,
        )
    };
    painter.circle_filled(
        knob_center,
        knob_size * 0.5,
        egui::Color32::from_rgb(0xE6, 0xED, 0xF3),
    );
    painter.circle_stroke(
        knob_center,
        knob_size * 0.5,
        egui::Stroke::new(1.0_f32, redesign_border_strong(palette)),
    );
    if response.clicked() {
        *on = !*on;
        true
    } else {
        false
    }
}
