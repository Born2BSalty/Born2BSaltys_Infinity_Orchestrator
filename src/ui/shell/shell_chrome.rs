// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    REDESIGN_STATUSBAR_HEIGHT_PX, REDESIGN_TITLEBAR_HEIGHT_PX, ThemePalette, redesign_shell_bg,
};

use super::{shell_statusbar, shell_titlebar};

pub fn render_shell<F: FnOnce(&mut egui::Ui)>(
    ctx: &egui::Context,
    palette: ThemePalette,
    modlist_count: usize,
    jobs_running: usize,
    body: F,
) {
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(redesign_shell_bg(palette)))
        .show(ctx, |ui| {
            let content_rect = ui.max_rect();
            let painter = ui.painter();
            let titlebar_rect = egui::Rect::from_min_size(
                content_rect.min,
                egui::vec2(content_rect.width(), REDESIGN_TITLEBAR_HEIGHT_PX),
            );
            let statusbar_rect = egui::Rect::from_min_size(
                egui::pos2(
                    content_rect.left(),
                    content_rect.bottom() - REDESIGN_STATUSBAR_HEIGHT_PX,
                ),
                egui::vec2(content_rect.width(), REDESIGN_STATUSBAR_HEIGHT_PX),
            );
            let body_rect = egui::Rect::from_min_max(
                egui::pos2(content_rect.left(), titlebar_rect.bottom()),
                egui::pos2(content_rect.right(), statusbar_rect.top()),
            );

            painter.rect_filled(body_rect, 0.0, redesign_shell_bg(palette));
            ui.scope_builder(
                egui::UiBuilder::new()
                    .max_rect(titlebar_rect)
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
                |ui| shell_titlebar::render(ui, palette),
            );
            ui.scope_builder(
                egui::UiBuilder::new()
                    .max_rect(body_rect)
                    .layout(egui::Layout::left_to_right(egui::Align::Min)),
                |ui| body(ui),
            );
            ui.scope_builder(
                egui::UiBuilder::new()
                    .max_rect(statusbar_rect)
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
                |ui| shell_statusbar::render(ui, palette, modlist_count, jobs_running),
            );
        });
}
