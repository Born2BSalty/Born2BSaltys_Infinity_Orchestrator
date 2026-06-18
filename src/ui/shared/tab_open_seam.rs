// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_shell_bg,
};

const COVER_TOP_MARGIN: f32 = 2.5;

const COVER_BOTTOM_MARGIN: f32 = 1.0;

pub fn paint_active_tab_seam_cover(
    painter: &egui::Painter,
    palette: ThemePalette,
    tab_rect: egui::Rect,
    panel_top_y: f32,
) {
    let ppp = painter.ctx().pixels_per_point();
    let snap = |v: f32| (v * ppp).round() / ppp;
    let cover = egui::Rect::from_min_max(
        egui::pos2(
            snap(tab_rect.left() + REDESIGN_BORDER_WIDTH_PX),
            snap(panel_top_y - COVER_TOP_MARGIN),
        ),
        egui::pos2(
            snap(tab_rect.right() - REDESIGN_BORDER_WIDTH_PX),
            snap(panel_top_y + REDESIGN_BORDER_WIDTH_PX + COVER_BOTTOM_MARGIN),
        ),
    );
    painter.rect_filled(cover, egui::CornerRadius::ZERO, redesign_shell_bg(palette));
}
