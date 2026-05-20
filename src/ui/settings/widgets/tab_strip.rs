// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_strong,
    redesign_chrome_bg, redesign_hover_overlay, redesign_shell_bg, redesign_text_muted,
    redesign_text_primary,
};

pub trait TabLabel: Copy + Eq {
    fn label(self) -> &'static str;
}

pub fn render<T: TabLabel>(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    tabs: &[T],
    current: &mut T,
    body: impl FnOnce(&mut egui::Ui),
) {
    let tab_height = 30.0;
    let body_padding = 14.0;

    let tab_corner = egui::CornerRadius {
        nw: REDESIGN_BORDER_RADIUS_U8,
        ne: REDESIGN_BORDER_RADIUS_U8,
        sw: 0,
        se: 0,
    };
    let body_corner = egui::CornerRadius {
        nw: 0,
        ne: 0,
        sw: REDESIGN_BORDER_RADIUS_U8,
        se: REDESIGN_BORDER_RADIUS_U8,
    };
    let border = egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette));
    let active_fill = redesign_shell_bg(palette);
    let idle_fill = redesign_chrome_bg(palette);

    let mut active_x_range: Option<(f32, f32)> = None;

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for &tab in tabs {
            let active = tab == *current;
            let label = tab.label();
            let font = egui::FontId::new(13.0, egui::FontFamily::Name("poppins_medium".into()));
            let galley = ui.painter().layout_no_wrap(
                label.to_string(),
                font.clone(),
                redesign_text_primary(palette),
            );
            let tab_w = galley.size().x + 26.0;
            let (rect, response) =
                ui.allocate_exact_size(egui::vec2(tab_w, tab_height), egui::Sense::click());
            let painter = ui.painter();

            let fill = if active { active_fill } else { idle_fill };
            painter.rect_filled(rect, tab_corner, fill);
            if !active && response.hovered() {
                painter.rect_filled(rect, tab_corner, redesign_hover_overlay(palette));
            }
            painter.rect_stroke(rect, tab_corner, border, egui::StrokeKind::Inside);

            if active {
                active_x_range = Some((rect.left(), rect.right()));
            }

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

            if response.clicked() {
                *current = tab;
            }
        }
    });

    let item_gap_y = ui.spacing().item_spacing.y;
    ui.add_space(-item_gap_y);

    let avail = ui.available_size();
    let (body_rect, _) = ui.allocate_exact_size(avail, egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(body_rect, body_corner, active_fill);
    painter.rect_stroke(body_rect, body_corner, border, egui::StrokeKind::Inside);

    if let Some((x0, x1)) = active_x_range {
        let seam_y = body_rect.top();
        let mask = egui::Rect::from_min_max(
            egui::pos2(
                x0 + REDESIGN_BORDER_WIDTH_PX,
                seam_y - REDESIGN_BORDER_WIDTH_PX,
            ),
            egui::pos2(
                x1 - REDESIGN_BORDER_WIDTH_PX,
                seam_y + REDESIGN_BORDER_WIDTH_PX,
            ),
        );
        painter.rect_filled(mask, 0.0, active_fill);
    }

    let inner_rect = body_rect.shrink(body_padding);
    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(inner_rect), |ui| {
        ui.set_clip_rect(inner_rect);
        ui.vertical(|ui| body(ui));
    });
}
