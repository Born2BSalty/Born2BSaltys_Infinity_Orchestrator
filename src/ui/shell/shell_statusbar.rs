// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_WIDTH_PX, REDESIGN_STATUSBAR_FONT_SIZE_PX, REDESIGN_STATUSBAR_HEIGHT_PX,
    REDESIGN_STATUSBAR_PADDING_X_PX, ThemePalette, redesign_border_strong, redesign_chrome_bg,
    redesign_font_light, redesign_status_dot, redesign_text_muted,
};

const STATUS_DOT_RADIUS_PX: f32 = 4.0;
const STATUS_DOT_TEXT_GAP_PX: f32 = 8.0;

pub fn render(ui: &mut egui::Ui, palette: ThemePalette, modlist_count: usize, jobs_running: usize) {
    let size = egui::vec2(ui.available_width(), REDESIGN_STATUSBAR_HEIGHT_PX);
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let painter = ui.painter_at(rect);

    painter.rect_filled(rect, 0.0, redesign_chrome_bg(palette));

    let border_y = rect.top() + (REDESIGN_BORDER_WIDTH_PX / 2.0);
    painter.line_segment(
        [
            egui::pos2(rect.left(), border_y),
            egui::pos2(rect.right(), border_y),
        ],
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
    );

    let color = redesign_text_muted(palette);
    let font = egui::FontId::new(REDESIGN_STATUSBAR_FONT_SIZE_PX, redesign_font_light());
    let center_y = rect.center().y;
    let dot_center = egui::pos2(rect.left() + REDESIGN_STATUSBAR_PADDING_X_PX, center_y);
    painter.circle_filled(
        dot_center,
        STATUS_DOT_RADIUS_PX,
        redesign_status_dot(palette),
    );
    painter.circle_stroke(
        dot_center,
        STATUS_DOT_RADIUS_PX,
        egui::Stroke::new(1.0, redesign_border_strong(palette)),
    );

    painter.text(
        egui::pos2(
            dot_center.x + STATUS_DOT_RADIUS_PX + STATUS_DOT_TEXT_GAP_PX,
            center_y,
        ),
        egui::Align2::LEFT_CENTER,
        format!("connected   ·   {modlist_count} modlists   ·   {jobs_running} jobs running"),
        font.clone(),
        color,
    );

    painter.text(
        egui::pos2(rect.right() - REDESIGN_STATUSBAR_PADDING_X_PX, center_y),
        egui::Align2::RIGHT_CENTER,
        format!("v{}", env!("CARGO_PKG_VERSION")),
        font,
        color,
    );
}
