// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::redesign_dot_background::paint_dot_background;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_STATUSBAR_HEIGHT_PX, REDESIGN_TITLEBAR_HEIGHT_PX, ThemePalette, redesign_page_bg,
};
use crate::ui::shell::shell_statusbar::{self, RunningInstallStatus};
use crate::ui::shell::shell_titlebar;

pub fn render_shell<F: FnOnce(&mut egui::Ui)>(
    ctx: &egui::Context,
    palette: ThemePalette,
    modlist_count: usize,
    running_install: Option<&RunningInstallStatus>,
    body: F,
) {
    let bg_painter = ctx.layer_painter(egui::LayerId::background());
    paint_dot_background(&bg_painter, ctx.screen_rect(), palette);

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

    paint_resize_handles(ctx);
}

fn paint_resize_handles(ctx: &egui::Context) {
    let screen = ctx.screen_rect();
    let edge = 4.0;
    let corner = 8.0;

    let zones: [(egui::Rect, egui::ResizeDirection, egui::CursorIcon); 8] = [
        (
            egui::Rect::from_min_size(screen.min, egui::vec2(corner, corner)),
            egui::ResizeDirection::NorthWest,
            egui::CursorIcon::ResizeNwSe,
        ),
        (
            egui::Rect::from_min_max(
                egui::pos2(screen.min.x + corner, screen.min.y),
                egui::pos2(screen.max.x - corner, screen.min.y + edge),
            ),
            egui::ResizeDirection::North,
            egui::CursorIcon::ResizeVertical,
        ),
        (
            egui::Rect::from_min_size(
                egui::pos2(screen.max.x - corner, screen.min.y),
                egui::vec2(corner, corner),
            ),
            egui::ResizeDirection::NorthEast,
            egui::CursorIcon::ResizeNeSw,
        ),
        (
            egui::Rect::from_min_max(
                egui::pos2(screen.max.x - edge, screen.min.y + corner),
                egui::pos2(screen.max.x, screen.max.y - corner),
            ),
            egui::ResizeDirection::East,
            egui::CursorIcon::ResizeHorizontal,
        ),
        (
            egui::Rect::from_min_size(
                egui::pos2(screen.max.x - corner, screen.max.y - corner),
                egui::vec2(corner, corner),
            ),
            egui::ResizeDirection::SouthEast,
            egui::CursorIcon::ResizeNwSe,
        ),
        (
            egui::Rect::from_min_max(
                egui::pos2(screen.min.x + corner, screen.max.y - edge),
                egui::pos2(screen.max.x - corner, screen.max.y),
            ),
            egui::ResizeDirection::South,
            egui::CursorIcon::ResizeVertical,
        ),
        (
            egui::Rect::from_min_size(
                egui::pos2(screen.min.x, screen.max.y - corner),
                egui::vec2(corner, corner),
            ),
            egui::ResizeDirection::SouthWest,
            egui::CursorIcon::ResizeNeSw,
        ),
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
