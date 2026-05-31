// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

#[cfg(not(target_os = "macos"))]
use crate::ui::shared::redesign_tokens::redesign_text_muted;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_WIDTH_PX, REDESIGN_TITLEBAR_HEIGHT_PX, ThemePalette, redesign_border_strong,
    redesign_chrome_bg, redesign_shell_bg, redesign_text_primary,
};

#[cfg(target_os = "macos")]
const TRAFFIC_CLOSE: egui::Color32 = egui::Color32::from_rgb(0xFF, 0x5F, 0x57);
#[cfg(target_os = "macos")]
const TRAFFIC_MIN: egui::Color32 = egui::Color32::from_rgb(0xFE, 0xBC, 0x2E);
#[cfg(target_os = "macos")]
const TRAFFIC_ZOOM: egui::Color32 = egui::Color32::from_rgb(0x28, 0xC8, 0x40);

#[derive(Clone, Copy)]
#[cfg(target_os = "macos")]
enum TrafficAction {
    Close,
    Minimize,
    Zoom,
}

#[cfg(target_os = "macos")]
impl TrafficAction {
    const fn color(self) -> egui::Color32 {
        match self {
            Self::Close => TRAFFIC_CLOSE,
            Self::Minimize => TRAFFIC_MIN,
            Self::Zoom => TRAFFIC_ZOOM,
        }
    }
}

pub fn render(ui: &mut egui::Ui, palette: ThemePalette) {
    let rect = ui.max_rect();

    let bar_response = ui.interact(
        rect,
        ui.id().with("titlebar_drag"),
        egui::Sense::click_and_drag(),
    );

    {
        let painter = ui.painter();
        painter.rect_filled(rect, 0.0, redesign_chrome_bg(palette));
        let bottom_y = REDESIGN_BORDER_WIDTH_PX.mul_add(-0.5, rect.bottom());
        painter.line_segment(
            [
                egui::pos2(rect.left(), bottom_y),
                egui::pos2(rect.right(), bottom_y),
            ],
            egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
        );
    }

    let mut interactive_rects: Vec<egui::Rect> = Vec::new();

    #[cfg(target_os = "macos")]
    render_traffic_lights(ui, rect, palette, &mut interactive_rects);
    #[cfg(not(target_os = "macos"))]
    render_traffic_lights(ui, rect, palette);

    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "Infinity Orchestrator",
        egui::FontId::new(10.0, egui::FontFamily::Name("poppins_medium".into())),
        redesign_text_primary(palette),
    );

    #[cfg(not(target_os = "macos"))]
    render_windows_controls(ui, rect, palette, &mut interactive_rects);

    if bar_response.drag_started() {
        let press_origin = ui.input(|i| i.pointer.press_origin());
        let on_control =
            press_origin.is_some_and(|p| interactive_rects.iter().any(|r| r.contains(p)));
        if !on_control {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }
    }

    if bar_response.double_clicked() {
        let click_pos = ui.input(|i| i.pointer.interact_pos());
        let on_control = click_pos.is_some_and(|p| interactive_rects.iter().any(|r| r.contains(p)));
        if !on_control {
            toggle_maximized(ui.ctx());
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn render_traffic_lights(ui: &egui::Ui, rect: egui::Rect, palette: ThemePalette) {
    let dot_d = 12.0;
    let dot_gap = 6.0;
    let dots_start_x = rect.left() + 12.0;
    let dot_y = rect.center().y;

    let mut cx = dots_start_x + dot_d * 0.5;
    for _ in 0..3 {
        let center = egui::pos2(cx, dot_y);
        let painter = ui.painter();
        painter.circle_filled(center, dot_d * 0.5, redesign_shell_bg(palette));
        painter.circle_stroke(
            center,
            dot_d * 0.5,
            egui::Stroke::new(1.2, redesign_border_strong(palette)),
        );
        cx += dot_d + dot_gap;
    }
}

#[cfg(target_os = "macos")]
fn render_traffic_lights(
    ui: &egui::Ui,
    rect: egui::Rect,
    palette: ThemePalette,
    interactive_rects: &mut Vec<egui::Rect>,
) {
    let dot_d = 12.0;
    let dot_gap = 6.0;
    let dots_start_x = rect.left() + 12.0;
    let dot_y = rect.center().y;
    let actions = [
        TrafficAction::Close,
        TrafficAction::Minimize,
        TrafficAction::Zoom,
    ];

    let dots_region = egui::Rect::from_min_max(
        egui::pos2(dots_start_x, dot_y - dot_d * 0.5),
        egui::pos2(
            2.0f32.mul_add(dot_gap, 3.0f32.mul_add(dot_d, dots_start_x)),
            dot_y + dot_d * 0.5,
        ),
    );
    let area_hovered = ui
        .input(|i| i.pointer.hover_pos())
        .is_some_and(|p| dots_region.contains(p));

    let mut cx = dots_start_x + dot_d * 0.5;
    for (i, action) in actions.iter().enumerate() {
        let center = egui::pos2(cx, dot_y);
        let dot_rect = egui::Rect::from_center_size(center, egui::vec2(dot_d, dot_d));
        let response = ui.interact(
            dot_rect,
            ui.id().with(("traffic_light", i)),
            egui::Sense::click(),
        );
        interactive_rects.push(dot_rect);
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if response.clicked() {
            match action {
                TrafficAction::Close => {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
                TrafficAction::Minimize => {
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                }
                TrafficAction::Zoom => toggle_maximized(ui.ctx()),
            }
        }

        let fill = if area_hovered {
            action.color()
        } else {
            redesign_shell_bg(palette)
        };
        let painter = ui.painter();
        painter.circle_filled(center, dot_d * 0.5, fill);
        painter.circle_stroke(
            center,
            dot_d * 0.5,
            egui::Stroke::new(1.2, redesign_border_strong(palette)),
        );
        cx += dot_d + dot_gap;
    }
}

#[cfg(not(target_os = "macos"))]
fn render_windows_controls(
    ui: &egui::Ui,
    rect: egui::Rect,
    palette: ThemePalette,
    interactive_rects: &mut Vec<egui::Rect>,
) {
    let controls = [
        (WindowsControl::Minimize, "—"),
        (WindowsControl::Maximize, "▢"),
        (WindowsControl::Close, "×"),
    ];
    let control_w = 32.0;
    let padding_right = 4.0;
    let total_w = control_w * 3.0;
    let mut x = rect.right() - padding_right - total_w;

    for (action, glyph) in &controls {
        let control_rect = egui::Rect::from_min_max(
            egui::pos2(x, rect.top()),
            egui::pos2(x + control_w, rect.bottom() - REDESIGN_BORDER_WIDTH_PX),
        );
        let response = ui.interact(
            control_rect,
            ui.id().with(("windows_ctrl", *glyph)),
            egui::Sense::click(),
        );
        interactive_rects.push(control_rect);

        let painter = ui.painter();
        if response.hovered() {
            let hover_fill = if matches!(action, WindowsControl::Close) {
                egui::Color32::from_rgb(0xC4, 0x2B, 0x1C)
            } else {
                redesign_shell_bg(palette)
            };
            painter.rect_filled(control_rect, 0.0, hover_fill);
        }

        painter.text(
            control_rect.center(),
            egui::Align2::CENTER_CENTER,
            *glyph,
            egui::FontId::new(12.0, egui::FontFamily::Proportional),
            redesign_text_muted(palette),
        );

        if response.clicked() {
            match action {
                WindowsControl::Minimize => {
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                }
                WindowsControl::Maximize => toggle_maximized(ui.ctx()),
                WindowsControl::Close => {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }
        x += control_w;
    }
}

#[cfg(not(target_os = "macos"))]
#[derive(Clone, Copy)]
enum WindowsControl {
    Minimize,
    Maximize,
    Close,
}

fn toggle_maximized(ctx: &egui::Context) {
    let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
    ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
}

pub const HEIGHT_PX: f32 = REDESIGN_TITLEBAR_HEIGHT_PX;
