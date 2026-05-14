// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_WIDTH_PX, REDESIGN_STATUSBAR_HEIGHT_PX, ThemePalette, redesign_border_strong,
    redesign_chrome_bg, redesign_text_muted,
};

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
    let font = egui::FontId::proportional(10.0);
    let center_y = rect.center().y;
    let padding_x = 12.0;

    painter.text(
        egui::pos2(rect.left() + padding_x, center_y),
        egui::Align2::LEFT_CENTER,
        format!("● connected · {modlist_count} modlists · {jobs_running} jobs running"),
        font.clone(),
        color,
    );

    painter.text(
        egui::pos2(rect.right() - padding_x, center_y),
        egui::Align2::RIGHT_CENTER,
        format!("v{}", env!("CARGO_PKG_VERSION")),
        font,
        color,
    );
}
