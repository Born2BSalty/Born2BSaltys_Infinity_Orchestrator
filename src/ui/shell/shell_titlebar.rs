// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// Redesign shell titlebar (Infinity Orchestrator binary).
//
// Paints the 34px titlebar:
//   - 1.5px bottom border in `redesign_border_strong`,
//   - `redesign_chrome_bg` background,
//   - three 12×12px traffic-light dots (functional on macOS),
//   - centered title "INFINITY ORCHESTRATOR" in Poppins 10px weight 500
//     uppercase letter-spacing 1.5px,
//   - right-aligned Windows-style `— ▢ ×` controls (non-macOS only).
//
// Click-and-drag on empty bar area moves the window. Double-click maximizes.
// SPEC: §1.2 (custom titlebar), wireframe `index.html:89-108`, `app.jsx:80-92`.

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_WIDTH_PX, REDESIGN_TITLEBAR_HEIGHT_PX, ThemePalette,
    redesign_border_strong, redesign_chrome_bg, redesign_shell_bg, redesign_text_primary,
};
#[cfg(not(target_os = "macos"))]
use crate::ui::shared::redesign_tokens::redesign_text_muted;

// macOS traffic-light colors (approximations of the standard system tints).
const TRAFFIC_CLOSE: egui::Color32 = egui::Color32::from_rgb(0xFF, 0x5F, 0x57);
const TRAFFIC_MIN: egui::Color32 = egui::Color32::from_rgb(0xFE, 0xBC, 0x2E);
const TRAFFIC_ZOOM: egui::Color32 = egui::Color32::from_rgb(0x28, 0xC8, 0x40);

#[derive(Clone, Copy)]
enum TrafficAction {
    Close,
    Minimize,
    Zoom,
}

impl TrafficAction {
    fn color(self) -> egui::Color32 {
        match self {
            TrafficAction::Close => TRAFFIC_CLOSE,
            TrafficAction::Minimize => TRAFFIC_MIN,
            TrafficAction::Zoom => TRAFFIC_ZOOM,
        }
    }
}

/// Paint the redesign titlebar inside the given `ui` (caller is expected to
/// have allocated a 34px-tall strip — e.g., via `egui::TopBottomPanel::top`).
pub fn render(ui: &mut egui::Ui, palette: ThemePalette) {
    let rect = ui.max_rect();

    // Interact with the whole bar for drag + double-click detection. Sub-control
    // `ui.interact` calls below register their own (smaller) interactions; their
    // clicks/drags don't trigger this bar response's drag because we filter by
    // press_origin against the recorded interactive rects.
    let bar_response = ui.interact(
        rect,
        ui.id().with("titlebar_drag"),
        egui::Sense::click_and_drag(),
    );

    // Background fill + 1.5px bottom border (scoped so the painter borrow ends
    // before we re-borrow `ui` mutably for the control hit zones below).
    {
        let painter = ui.painter();
        painter.rect_filled(rect, 0.0, redesign_chrome_bg(palette));
        let bottom_y = rect.bottom() - REDESIGN_BORDER_WIDTH_PX * 0.5;
        painter.line_segment(
            [
                egui::pos2(rect.left(), bottom_y),
                egui::pos2(rect.right(), bottom_y),
            ],
            egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
        );
    }

    // Hit zones for non-drag interactions (traffic lights + Windows controls).
    // Used to filter out drag starts that originated on these.
    let mut interactive_rects: Vec<egui::Rect> = Vec::new();

    render_traffic_lights(ui, rect, palette, &mut interactive_rects);

    // Title (centered).
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "I N F I N I T Y   O R C H E S T R A T O R",
        egui::FontId::new(10.0, egui::FontFamily::Name("poppins_medium".into())),
        redesign_text_primary(palette),
    );

    #[cfg(not(target_os = "macos"))]
    render_windows_controls(ui, rect, palette, &mut interactive_rects);

    // Drag-to-move: trigger if the drag originated outside any control hit zone.
    if bar_response.drag_started() {
        let press_origin = ui.input(|i| i.pointer.press_origin());
        let on_control = press_origin
            .is_some_and(|p| interactive_rects.iter().any(|r| r.contains(p)));
        if !on_control {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }
    }

    // Double-click on empty bar area toggles maximize (standard convention).
    if bar_response.double_clicked() {
        let click_pos = ui.input(|i| i.pointer.interact_pos());
        let on_control = click_pos
            .is_some_and(|p| interactive_rects.iter().any(|r| r.contains(p)));
        if !on_control {
            toggle_maximized(ui.ctx());
        }
    }
}

/// Paint and (on macOS) wire the three traffic-light dots on the left.
/// On non-macOS the dots stay visual-only — the window controls on the right
/// handle close/minimize/maximize.
fn render_traffic_lights(
    ui: &mut egui::Ui,
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

    // When the pointer is anywhere over the dots region, all three "light up"
    // with their tint — mirroring the macOS Finder behavior.
    let dots_region = egui::Rect::from_min_max(
        egui::pos2(dots_start_x, dot_y - dot_d * 0.5),
        egui::pos2(
            dots_start_x + 3.0 * dot_d + 2.0 * dot_gap,
            dot_y + dot_d * 0.5,
        ),
    );
    let area_hovered = ui
        .input(|i| i.pointer.hover_pos())
        .is_some_and(|p| dots_region.contains(p));

    for (i, action) in actions.iter().enumerate() {
        let cx = dots_start_x + dot_d * 0.5 + (i as f32) * (dot_d + dot_gap);
        let center = egui::pos2(cx, dot_y);
        let dot_rect = egui::Rect::from_center_size(center, egui::vec2(dot_d, dot_d));

        #[cfg(target_os = "macos")]
        {
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
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = dot_rect;
        }

        let fill = if area_hovered && cfg!(target_os = "macos") {
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
    }

    #[cfg(not(target_os = "macos"))]
    let _ = interactive_rects;
}

#[cfg(not(target_os = "macos"))]
fn render_windows_controls(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    palette: ThemePalette,
    interactive_rects: &mut Vec<egui::Rect>,
) {
    // Order matches Windows: minimize, maximize, close — left to right.
    let controls = [
        (WindowsControl::Minimize, "—"),
        (WindowsControl::Maximize, "▢"),
        (WindowsControl::Close, "×"),
    ];
    let control_w = 32.0;
    let padding_right = 4.0;
    let total_w = control_w * controls.len() as f32;
    let mut x = rect.right() - padding_right - total_w;

    for (action, glyph) in controls.iter() {
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

/// Convenience: the natural pixel height of the titlebar strip
/// (matches the wireframe `.sk-titlebar` height: 34px).
pub const HEIGHT_PX: f32 = REDESIGN_TITLEBAR_HEIGHT_PX;
