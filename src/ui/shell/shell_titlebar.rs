// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_WIDTH_PX, REDESIGN_TITLEBAR_CONTROL_FONT_SIZE_PX,
    REDESIGN_TITLEBAR_CONTROL_GAP_PX, REDESIGN_TITLEBAR_CONTROL_WIDTH_PX,
    REDESIGN_TITLEBAR_DOT_GAP_PX, REDESIGN_TITLEBAR_DOT_RADIUS_PX, REDESIGN_TITLEBAR_DOT_STROKE_PX,
    REDESIGN_TITLEBAR_FONT_SIZE_PX, REDESIGN_TITLEBAR_HEIGHT_PX, REDESIGN_TITLEBAR_PADDING_X_PX,
    ThemePalette, redesign_border_strong, redesign_chrome_bg, redesign_hover_overlay,
    redesign_shell_bg, redesign_text_muted, redesign_text_primary,
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
    let dot_step = (radius * 2.0) + REDESIGN_TITLEBAR_DOT_GAP_PX;
    let stroke = egui::Stroke::new(
        REDESIGN_TITLEBAR_DOT_STROKE_PX,
        redesign_border_strong(palette),
    );

    for index in 0..3 {
        let center = egui::pos2(
            rect.left() + REDESIGN_TITLEBAR_PADDING_X_PX + radius + (index as f32 * dot_step),
            center_y,
        );
        painter.circle_filled(center, radius, redesign_shell_bg(palette));
        painter.circle_stroke(center, radius, stroke);
    }
}

fn render_title(painter: &egui::Painter, rect: egui::Rect, palette: ThemePalette) {
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "Infinity Orchestrator · v1",
        egui::FontId::proportional(REDESIGN_TITLEBAR_FONT_SIZE_PX),
        redesign_text_primary(palette),
    );
}

fn render_window_controls(
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    rect: egui::Rect,
    palette: ThemePalette,
) -> egui::Rect {
    let center_y = rect.center().y;
    let color = redesign_text_muted(palette);
    let font = egui::FontId::proportional(REDESIGN_TITLEBAR_CONTROL_FONT_SIZE_PX);
    let mut union_rect = egui::Rect::NOTHING;

    for (index, (glyph, command)) in [
        ("×", WindowCommand::Close),
        ("▢", WindowCommand::ToggleMaximize),
        ("—", WindowCommand::Minimize),
    ]
    .iter()
    .enumerate()
    {
        let center = egui::pos2(
            rect.right()
                - REDESIGN_TITLEBAR_PADDING_X_PX
                - (REDESIGN_TITLEBAR_CONTROL_WIDTH_PX / 2.0)
                - (index as f32
                    * (REDESIGN_TITLEBAR_CONTROL_GAP_PX + REDESIGN_TITLEBAR_CONTROL_WIDTH_PX)),
            center_y,
        );
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
        painter.text(
            center,
            egui::Align2::CENTER_CENTER,
            *glyph,
            font.clone(),
            color,
        );
        if response.clicked() {
            match command {
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
    }

    union_rect
}

#[derive(Debug, Clone, Copy)]
enum WindowCommand {
    Close,
    Minimize,
    ToggleMaximize,
}
