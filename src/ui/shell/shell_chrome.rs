// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// Redesign shell chrome — combines the titlebar, body, and statusbar into the
// overall app frame, and paints invisible resize hit-zones around the window
// perimeter (required because `with_decorations(false)` removes the OS's
// native edge-resize handles).
//
// The dotted radial background per SPEC §12.3 is deferred to Phase 8; Phase 1
// ships a solid `page_bg` fill on the central panel.
//
// SPEC: §1.2 (overall shell shape).

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    REDESIGN_STATUSBAR_HEIGHT_PX, REDESIGN_TITLEBAR_HEIGHT_PX, ThemePalette, redesign_page_bg,
};
use crate::ui::shell::shell_statusbar::{self, RunningInstallStatus};
use crate::ui::shell::shell_titlebar;

/// Render the redesign shell around an arbitrary body callback.
///
/// Uses `egui::TopBottomPanel::top` for the titlebar (34px exact),
/// `egui::TopBottomPanel::bottom` for the statusbar (26px exact), and an
/// `egui::CentralPanel::default()` for the body. The body callback is given
/// the central-panel `Ui`; it owns its own internal layout (left rail + main
/// page in later phases).
pub fn render_shell<F: FnOnce(&mut egui::Ui)>(
    ctx: &egui::Context,
    palette: ThemePalette,
    modlist_count: usize,
    running_install: Option<&RunningInstallStatus>,
    body: F,
) {
    egui::TopBottomPanel::top("redesign_titlebar")
        .exact_height(REDESIGN_TITLEBAR_HEIGHT_PX)
        .show_separator_line(false)
        .frame(egui::Frame::NONE)
        .show(ctx, |ui| {
            shell_titlebar::render(ui, palette);
        });

    egui::TopBottomPanel::bottom("redesign_statusbar")
        .exact_height(REDESIGN_STATUSBAR_HEIGHT_PX)
        .show_separator_line(false)
        .frame(egui::Frame::NONE)
        .show(ctx, |ui| {
            shell_statusbar::render(ui, palette, modlist_count, running_install);
        });

    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(redesign_page_bg(palette)))
        .show(ctx, |ui| {
            body(ui);
        });

    // Resize hit-zones sit on top of everything so they win pointer interaction
    // at the window edges (the titlebar drag is suppressed when the press
    // origin is inside one of these zones).
    paint_resize_handles(ctx);
}

/// Paint 8 invisible interaction zones (4 corners + 4 edges) around the
/// window perimeter. Each one converts a drag into a `BeginResize` viewport
/// command and shows the matching resize cursor on hover.
///
/// We use thin zones (4px edges, 8px corners) so they only activate when the
/// pointer is genuinely on the window edge — not during normal interaction
/// with the body content.
fn paint_resize_handles(ctx: &egui::Context) {
    let screen = ctx.screen_rect();
    let edge = 4.0;
    let corner = 8.0;

    // (rect, direction, cursor)
    let zones: [(egui::Rect, egui::ResizeDirection, egui::CursorIcon); 8] = [
        // NW corner
        (
            egui::Rect::from_min_size(screen.min, egui::vec2(corner, corner)),
            egui::ResizeDirection::NorthWest,
            egui::CursorIcon::ResizeNwSe,
        ),
        // N edge (between the NW and NE corners)
        (
            egui::Rect::from_min_max(
                egui::pos2(screen.min.x + corner, screen.min.y),
                egui::pos2(screen.max.x - corner, screen.min.y + edge),
            ),
            egui::ResizeDirection::North,
            egui::CursorIcon::ResizeVertical,
        ),
        // NE corner
        (
            egui::Rect::from_min_size(
                egui::pos2(screen.max.x - corner, screen.min.y),
                egui::vec2(corner, corner),
            ),
            egui::ResizeDirection::NorthEast,
            egui::CursorIcon::ResizeNeSw,
        ),
        // E edge
        (
            egui::Rect::from_min_max(
                egui::pos2(screen.max.x - edge, screen.min.y + corner),
                egui::pos2(screen.max.x, screen.max.y - corner),
            ),
            egui::ResizeDirection::East,
            egui::CursorIcon::ResizeHorizontal,
        ),
        // SE corner
        (
            egui::Rect::from_min_size(
                egui::pos2(screen.max.x - corner, screen.max.y - corner),
                egui::vec2(corner, corner),
            ),
            egui::ResizeDirection::SouthEast,
            egui::CursorIcon::ResizeNwSe,
        ),
        // S edge
        (
            egui::Rect::from_min_max(
                egui::pos2(screen.min.x + corner, screen.max.y - edge),
                egui::pos2(screen.max.x - corner, screen.max.y),
            ),
            egui::ResizeDirection::South,
            egui::CursorIcon::ResizeVertical,
        ),
        // SW corner
        (
            egui::Rect::from_min_size(
                egui::pos2(screen.min.x, screen.max.y - corner),
                egui::vec2(corner, corner),
            ),
            egui::ResizeDirection::SouthWest,
            egui::CursorIcon::ResizeNeSw,
        ),
        // W edge
        (
            egui::Rect::from_min_max(
                egui::pos2(screen.min.x, screen.min.y + corner),
                egui::pos2(screen.min.x + edge, screen.max.y - corner),
            ),
            egui::ResizeDirection::West,
            egui::CursorIcon::ResizeHorizontal,
        ),
    ];

    for (i, (rect, direction, cursor)) in zones.iter().enumerate() {
        egui::Area::new(egui::Id::new(("redesign_resize", i)))
            .order(egui::Order::Foreground)
            .fixed_pos(rect.min)
            .interactable(true)
            .show(ctx, |ui| {
                let response = ui.allocate_response(rect.size(), egui::Sense::drag());
                if response.hovered() {
                    ui.ctx().set_cursor_icon(*cursor);
                }
                if response.drag_started() {
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::BeginResize(*direction));
                }
            });
    }
}
