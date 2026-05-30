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

    let mut active_rect: Option<egui::Rect> = None;

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
            if active {
                // Outline is drawn last (after the body box) so it stays crisp.
                active_rect = Some(rect);
            } else {
                if response.hovered() {
                    painter.rect_filled(rect, tab_corner, redesign_hover_overlay(palette));
                }
                painter.rect_stroke(rect, tab_corner, border, egui::StrokeKind::Inside);
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

    if let Some(rect) = active_rect {
        open_active_tab_into_body(ui, rect, body_rect, tab_corner, border, active_fill);
    }

    let inner_rect = body_rect.shrink(body_padding);
    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(inner_rect), |ui| {
        ui.set_clip_rect(inner_rect);
        ui.vertical(|ui| body(ui));
    });
}

/// Opens the active tab into the content body: erases the body's top border
/// directly under the tab, then re-strokes the tab (rounded top + sides) last,
/// clipped above the seam so its bottom edge is never drawn. Stroking last keeps
/// the side edges crisp and meeting the body border exactly at the tab's
/// left/right, independent of sub-pixel position.
fn open_active_tab_into_body(
    ui: &egui::Ui,
    tab_rect: egui::Rect,
    body_rect: egui::Rect,
    tab_corner: egui::CornerRadius,
    border: egui::Stroke,
    fill: egui::Color32,
) {
    let painter = ui.painter();
    let seam_y = body_rect.top();

    let cover = egui::Rect::from_min_max(
        egui::pos2(tab_rect.left(), seam_y - REDESIGN_BORDER_WIDTH_PX),
        egui::pos2(tab_rect.right(), seam_y + REDESIGN_BORDER_WIDTH_PX),
    );
    painter.rect_filled(cover, egui::CornerRadius::ZERO, fill);

    let stroke_rect = egui::Rect::from_min_max(
        tab_rect.min,
        egui::pos2(
            tab_rect.right(),
            seam_y + f32::from(REDESIGN_BORDER_RADIUS_U8) + 2.0,
        ),
    );
    let clip = egui::Rect::from_min_max(
        egui::pos2(tab_rect.left() - 4.0, tab_rect.top() - 4.0),
        egui::pos2(tab_rect.right() + 4.0, seam_y),
    );
    painter.with_clip_rect(clip).rect_stroke(
        stroke_rect,
        tab_corner,
        border,
        egui::StrokeKind::Inside,
    );

    // When the active tab is flush with the body's left edge, the full-width
    // cover also erased the top of the body's left border. Restore that short
    // segment so the left border stays continuous into the tab. Only the
    // left-most tab meets this condition; mid-strip tabs are untouched.
    if (tab_rect.left() - body_rect.left()).abs() < 1.0 {
        let x = border.width.mul_add(0.5, body_rect.left());
        painter.line_segment(
            [
                egui::pos2(x, seam_y - REDESIGN_BORDER_WIDTH_PX),
                egui::pos2(x, seam_y + REDESIGN_BORDER_WIDTH_PX),
            ],
            border,
        );
    }
}
