// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::redesign_tokens::{REDESIGN_DOT_BG_SPACING_PX, ThemePalette, redesign_dot};

pub fn paint_dot_background(painter: &egui::Painter, rect: egui::Rect, palette: ThemePalette) {
    let color = redesign_dot(palette);
    let spacing = REDESIGN_DOT_BG_SPACING_PX;
    let radius = 1.0_f32;

    let first_x = (rect.min.x / spacing).floor() * spacing;
    let first_y = (rect.min.y / spacing).floor() * spacing;

    for x in grid_steps(first_x, rect.max.x, spacing) {
        for y in grid_steps(first_y, rect.max.y, spacing) {
            painter.circle_filled(egui::pos2(x, y), radius, color);
        }
    }
}

fn grid_steps(start: f32, end: f32, step: f32) -> impl Iterator<Item = f32> {
    std::iter::successors(Some(start), move |&prev| {
        let next = prev + step;
        (next <= end).then_some(next)
    })
}
