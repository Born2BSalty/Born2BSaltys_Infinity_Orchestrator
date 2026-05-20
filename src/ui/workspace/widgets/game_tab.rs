// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_strong,
    redesign_chrome_bg, redesign_hover_overlay, redesign_shell_bg, redesign_text_muted,
    redesign_text_primary,
};

pub const TAB_H: f32 = 30.0;
pub const TAB_GAP: f32 = 4.0;
const TAB_PAD_X: f32 = 14.0;
const TAB_FONT_SIZE: f32 = 13.0;

pub fn game_tab(ui: &mut egui::Ui, palette: ThemePalette, label: &str, current: &mut String) {
    let active = current == label;
    let font = egui::FontId::new(
        TAB_FONT_SIZE,
        egui::FontFamily::Name("poppins_medium".into()),
    );
    let galley = ui.painter().layout_no_wrap(
        label.to_string(),
        font.clone(),
        redesign_text_primary(palette),
    );
    let tab_w = TAB_PAD_X.mul_add(2.0, galley.size().x);
    let (rect, response) = ui.allocate_exact_size(egui::vec2(tab_w, TAB_H), egui::Sense::click());

    let corner = egui::CornerRadius {
        nw: REDESIGN_BORDER_RADIUS_U8,
        ne: REDESIGN_BORDER_RADIUS_U8,
        sw: 0,
        se: 0,
    };

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let fill = if active {
            redesign_shell_bg(palette)
        } else {
            redesign_chrome_bg(palette)
        };

        let box_rect = egui::Rect::from_min_max(
            rect.min,
            egui::pos2(rect.max.x, rect.max.y + REDESIGN_BORDER_WIDTH_PX),
        );
        painter.rect_filled(box_rect, corner, fill);
        if !active && response.hovered() {
            painter.rect_filled(box_rect, corner, redesign_hover_overlay(palette));
        }

        let stroke = egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette));
        let border_clip = egui::Rect::from_min_max(
            egui::pos2(box_rect.min.x - 4.0, box_rect.min.y - 4.0),
            egui::pos2(
                box_rect.max.x + 4.0,
                REDESIGN_BORDER_WIDTH_PX.mul_add(-1.5, box_rect.max.y),
            ),
        );
        painter.with_clip_rect(border_clip).rect_stroke(
            box_rect,
            corner,
            stroke,
            egui::StrokeKind::Inside,
        );

        let text_color = if active {
            redesign_text_primary(palette)
        } else {
            redesign_text_muted(palette)
        };
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            font,
            text_color,
        );
    }

    if response.clicked() {
        *current = label.to_string();
    }
}
