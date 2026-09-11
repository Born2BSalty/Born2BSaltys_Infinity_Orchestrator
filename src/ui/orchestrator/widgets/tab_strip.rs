// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_strong, redesign_chrome_bg,
    redesign_shell_bg, redesign_text_muted, redesign_text_primary,
};
use crate::ui::shared::tab_open_seam::paint_active_tab_seam_cover;

pub(crate) struct TabItem<'a> {
    pub(crate) label: &'a str,
    pub(crate) sub: Option<&'a str>,
}

pub(crate) fn render_tab_strip(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    tabs: &[TabItem<'_>],
    active: &mut usize,
) -> Option<egui::Rect> {
    let mut active_tab_rect: Option<egui::Rect> = None;

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.spacing_mut().item_spacing.y = 4.0;
        for (index, tab) in tabs.iter().enumerate() {
            let is_active = index == *active;
            let rect = render_one_tab(ui, palette, tab, is_active);
            if is_active {
                active_tab_rect = Some(rect);
            }
            if tab_clicked(ui, rect) {
                *active = index;
            }
        }
    });

    let strip_bottom = ui.cursor().top();
    active_tab_rect.filter(|r| {
        let item_gap = ui.spacing().item_spacing.y;
        r.bottom() + item_gap >= strip_bottom - 2.0
    })
}

pub(crate) fn paint_tab_seam_cover(
    painter: &egui::Painter,
    palette: ThemePalette,
    active_tab_rect: egui::Rect,
    panel_top_y: f32,
) {
    paint_active_tab_seam_cover(painter, palette, active_tab_rect, panel_top_y);
}

fn render_one_tab(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    tab: &TabItem<'_>,
    is_active: bool,
) -> egui::Rect {
    let pad_x = 14.0;
    let pad_y = 6.0;
    let font = egui::FontId::new(
        14.0,
        egui::FontFamily::Name(if is_active {
            "poppins_bold".into()
        } else {
            "poppins_light".into()
        }),
    );
    let sub_font = egui::FontId::new(11.0, egui::FontFamily::Name("poppins_light".into()));
    let text_color = if is_active {
        redesign_text_primary(palette)
    } else {
        redesign_text_muted(palette)
    };
    let sub_color = redesign_text_muted(palette);

    let galley = ui
        .painter()
        .layout_no_wrap(tab.label.to_string(), font.clone(), text_color);
    let sub_galley = tab.sub.map(|sub| {
        ui.painter()
            .layout_no_wrap(sub.to_string(), sub_font.clone(), sub_color)
    });

    let sub_w = sub_galley.as_ref().map_or(0.0, |g| g.size().x + 6.0);
    let content_w = galley.size().x + sub_w;
    let content_h = galley
        .size()
        .y
        .max(sub_galley.as_ref().map_or(0.0, |g| g.size().y));

    let desired = egui::vec2(content_w + pad_x * 2.0, content_h + pad_y * 2.0);
    let (rect, _resp) = ui.allocate_exact_size(desired, egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let radius = egui::CornerRadius {
            nw: 4,
            ne: 4,
            sw: 0,
            se: 0,
        };
        let fill = if is_active {
            redesign_shell_bg(palette)
        } else {
            redesign_chrome_bg(palette)
        };
        painter.rect_filled(rect, radius, fill);
        let stroke = egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette));
        painter.line_segment([rect.left_top(), rect.right_top()], stroke);
        painter.line_segment([rect.left_top(), rect.left_bottom()], stroke);
        painter.line_segment([rect.right_top(), rect.right_bottom()], stroke);

        let start_x = rect.left() + pad_x;
        let cy = rect.center().y;
        painter.text(
            egui::pos2(start_x, cy),
            egui::Align2::LEFT_CENTER,
            tab.label,
            font,
            text_color,
        );
        if let Some(sub) = tab.sub {
            painter.text(
                egui::pos2(start_x + galley.size().x + 6.0, cy),
                egui::Align2::LEFT_CENTER,
                sub,
                sub_font,
                sub_color,
            );
        }
    }

    rect
}

fn tab_clicked(ui: &egui::Ui, rect: egui::Rect) -> bool {
    ui.input(|i| {
        i.pointer.primary_clicked() && i.pointer.interact_pos().is_some_and(|p| rect.contains(p))
    })
}
