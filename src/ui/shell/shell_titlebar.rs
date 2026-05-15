// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_WIDTH_PX, REDESIGN_TITLEBAR_CONTROL_GAP_PX,
    REDESIGN_TITLEBAR_CONTROL_GLYPH_SIZE_PX, REDESIGN_TITLEBAR_CONTROL_WIDTH_PX,
    REDESIGN_TITLEBAR_DOT_GAP_PX, REDESIGN_TITLEBAR_DOT_RADIUS_PX, REDESIGN_TITLEBAR_DOT_STROKE_PX,
    REDESIGN_TITLEBAR_FONT_SIZE_PX, REDESIGN_TITLEBAR_HEIGHT_PX, REDESIGN_TITLEBAR_PADDING_X_PX,
    ThemePalette, redesign_border_strong, redesign_chrome_bg, redesign_font_light,
    redesign_font_medium, redesign_hover_overlay, redesign_shell_bg, redesign_text_muted,
    redesign_text_primary,
};

pub fn render(ui: &mut egui::Ui, palette: ThemePalette) {
    let size = egui::vec2(ui.available_width(), REDESIGN_TITLEBAR_HEIGHT_PX);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());
    let painter = ui.painter_at(rect);

    painter.rect_filled(rect, 0.0, redesign_chrome_bg(palette));

    let border_y = rect.bottom() - (REDESIGN_BORDER_WIDTH_PX / 2.0);
    painter.line_segment(
        [
            egui::pos2(rect.left(), border_y),
            egui::pos2(rect.right(), border_y),
        ],
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
    );

    render_traffic_dots(&painter, rect, palette);
    render_title(&painter, rect, palette);
    let controls_rect = render_window_controls(ui, &painter, rect, palette);

    let pointer_on_controls = ui
        .input(|input| input.pointer.hover_pos())
        .is_some_and(|pos| controls_rect.contains(pos));
    if response.drag_started() && !pointer_on_controls {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }
}

fn render_traffic_dots(painter: &egui::Painter, rect: egui::Rect, palette: ThemePalette) {
    let center_y = rect.center().y;
    let radius = REDESIGN_TITLEBAR_DOT_RADIUS_PX;
    let dot_step = radius.mul_add(2.0, REDESIGN_TITLEBAR_DOT_GAP_PX);
    let stroke = egui::Stroke::new(
        REDESIGN_TITLEBAR_DOT_STROKE_PX,
        redesign_border_strong(palette),
    );

    let mut dot_x = rect.left() + REDESIGN_TITLEBAR_PADDING_X_PX + radius;
    for _ in 0..3 {
        let center = egui::pos2(dot_x, center_y);
        painter.circle_filled(center, radius, redesign_shell_bg(palette));
        painter.circle_stroke(center, radius, stroke);
        dot_x += dot_step;
    }
}

fn render_title(painter: &egui::Painter, rect: egui::Rect, palette: ThemePalette) {
    let title_font = egui::FontId::new(REDESIGN_TITLEBAR_FONT_SIZE_PX, redesign_font_medium());
    let version_font =
        egui::FontId::new(REDESIGN_TITLEBAR_FONT_SIZE_PX + 1.0, redesign_font_light());
    let title_center = egui::pos2(rect.center().x - 12.0, rect.center().y);
    painter.text(
        title_center,
        egui::Align2::CENTER_CENTER,
        "INFINITY ORCHESTRATOR",
        title_font,
        redesign_text_primary(palette),
    );
    painter.text(
        egui::pos2(title_center.x + 84.0, title_center.y),
        egui::Align2::LEFT_CENTER,
        "· v1",
        version_font,
        redesign_text_muted(palette),
    );
}

fn render_window_controls(
    ui: &egui::Ui,
    painter: &egui::Painter,
    rect: egui::Rect,
    palette: ThemePalette,
) -> egui::Rect {
    let center_y = rect.center().y;
    let color = redesign_text_muted(palette);
    let mut union_rect = egui::Rect::NOTHING;

    let mut control_x =
        rect.right() - REDESIGN_TITLEBAR_PADDING_X_PX - (REDESIGN_TITLEBAR_CONTROL_WIDTH_PX / 2.0);
    for (index, command) in [
        WindowCommand::Close,
        WindowCommand::ToggleMaximize,
        WindowCommand::Minimize,
    ]
    .iter()
    .enumerate()
    {
        let center = egui::pos2(control_x, center_y);
        let control_rect = egui::Rect::from_center_size(
            center,
            egui::vec2(
                REDESIGN_TITLEBAR_CONTROL_WIDTH_PX,
                REDESIGN_TITLEBAR_HEIGHT_PX,
            ),
        );
        union_rect = union_rect.union(control_rect);
        let response = ui.interact(
            control_rect,
            ui.id().with(("window-control", index)),
            egui::Sense::click(),
        );
        if response.hovered() {
            painter.rect_filled(control_rect, 0.0, redesign_hover_overlay(palette));
        }
        paint_window_control(painter, *command, center, color);
        if response.clicked() {
            match *command {
                WindowCommand::Close => ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close),
                WindowCommand::Minimize => ui
                    .ctx()
                    .send_viewport_cmd(egui::ViewportCommand::Minimized(true)),
                WindowCommand::ToggleMaximize => {
                    let maximized = ui
                        .ctx()
                        .input(|input| input.viewport().maximized.unwrap_or(false));
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                }
            }
        }
        control_x -= REDESIGN_TITLEBAR_CONTROL_GAP_PX + REDESIGN_TITLEBAR_CONTROL_WIDTH_PX;
    }

    union_rect
}

fn paint_window_control(
    painter: &egui::Painter,
    command: WindowCommand,
    center: egui::Pos2,
    color: egui::Color32,
) {
    let half = REDESIGN_TITLEBAR_CONTROL_GLYPH_SIZE_PX / 2.0;
    let stroke = egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, color);
    match command {
        WindowCommand::Close => {
            painter.line_segment(
                [
                    egui::pos2(center.x - half, center.y - half),
                    egui::pos2(center.x + half, center.y + half),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    egui::pos2(center.x + half, center.y - half),
                    egui::pos2(center.x - half, center.y + half),
                ],
                stroke,
            );
        }
        WindowCommand::Minimize => {
            painter.line_segment(
                [
                    egui::pos2(center.x - half, center.y),
                    egui::pos2(center.x + half, center.y),
                ],
                stroke,
            );
        }
        WindowCommand::ToggleMaximize => {
            let rect = egui::Rect::from_center_size(
                center,
                egui::Vec2::splat(REDESIGN_TITLEBAR_CONTROL_GLYPH_SIZE_PX),
            );
            painter.rect_stroke(rect, 0.0, stroke, egui::StrokeKind::Inside);
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum WindowCommand {
    Close,
    Minimize,
    ToggleMaximize,
}
