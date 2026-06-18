// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_strong,
    redesign_hover_overlay, redesign_shell_bg, redesign_text_muted, redesign_text_primary,
};

#[derive(Clone, Copy)]
pub(crate) enum ButtonIcon {
    Close,
    Copy,
    Details,
    Open,
}

const BUTTON_SIZE: egui::Vec2 = egui::vec2(24.0, 20.0);

pub(crate) fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    icon: ButtonIcon,
    tooltip: &str,
    visible: bool,
) -> egui::Response {
    let sense = if visible {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(BUTTON_SIZE, sense);

    if visible && ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let radius = egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8);
        painter.rect_filled(rect, radius, redesign_shell_bg(palette));
        if response.hovered() {
            painter.rect_filled(rect, radius, redesign_hover_overlay(palette));
        }
        painter.rect_stroke(
            rect,
            radius,
            egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
            egui::StrokeKind::Inside,
        );

        let color = if response.hovered() {
            redesign_text_primary(palette)
        } else {
            redesign_text_muted(palette)
        };
        match icon {
            ButtonIcon::Close => paint_close_icon(painter, rect, color),
            ButtonIcon::Copy => paint_copy_icon(painter, rect, color),
            ButtonIcon::Details => paint_details_icon(painter, rect, color),
            ButtonIcon::Open => paint_open_icon(painter, rect, color),
        }
    }

    if visible {
        response.on_hover_text(tooltip)
    } else {
        response
    }
}

pub(crate) fn paint_close_icon(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let stroke = egui::Stroke::new(1.5, color);
    let center = rect.center();
    painter.line_segment(
        [
            center + egui::vec2(-4.0, -4.0),
            center + egui::vec2(4.0, 4.0),
        ],
        stroke,
    );
    painter.line_segment(
        [
            center + egui::vec2(4.0, -4.0),
            center + egui::vec2(-4.0, 4.0),
        ],
        stroke,
    );
}

fn paint_copy_icon(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let stroke = egui::Stroke::new(1.4, color);
    let center = rect.center();
    let back = egui::Rect::from_min_size(center + egui::vec2(-6.0, -6.0), egui::vec2(9.0, 9.0));
    let front = egui::Rect::from_min_size(center + egui::vec2(-2.0, -2.0), egui::vec2(9.0, 9.0));
    painter.rect_stroke(
        back,
        egui::CornerRadius::same(2),
        stroke,
        egui::StrokeKind::Inside,
    );
    painter.rect_stroke(
        front,
        egui::CornerRadius::same(2),
        stroke,
        egui::StrokeKind::Inside,
    );
}

fn paint_details_icon(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "?",
        egui::FontId::new(12.0, egui::FontFamily::Name("poppins_medium".into())),
        color,
    );
}

fn paint_open_icon(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let stroke = egui::Stroke::new(1.4, color);
    let center = rect.center();
    let box_rect = egui::Rect::from_min_size(center + egui::vec2(-6.0, -2.0), egui::vec2(9.0, 9.0));
    painter.rect_stroke(
        box_rect,
        egui::CornerRadius::same(2),
        stroke,
        egui::StrokeKind::Inside,
    );
    painter.line_segment(
        [
            center + egui::vec2(-1.0, 1.0),
            center + egui::vec2(6.0, -6.0),
        ],
        stroke,
    );
    painter.line_segment(
        [
            center + egui::vec2(2.0, -6.0),
            center + egui::vec2(6.0, -6.0),
        ],
        stroke,
    );
    painter.line_segment(
        [
            center + egui::vec2(6.0, -6.0),
            center + egui::vec2(6.0, -2.0),
        ],
        stroke,
    );
}
