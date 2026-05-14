// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    REDESIGN_STATUSBAR_HEIGHT_PX, REDESIGN_TITLEBAR_HEIGHT_PX, ThemePalette, redesign_chrome_bg,
    redesign_shell_bg,
};

use super::{shell_statusbar, shell_titlebar};

pub fn render_shell<F: FnOnce(&mut egui::Ui)>(
    ctx: &egui::Context,
    palette: ThemePalette,
    modlist_count: usize,
    jobs_running: usize,
    body: F,
) {
    egui::TopBottomPanel::top("redesign_titlebar")
        .exact_height(REDESIGN_TITLEBAR_HEIGHT_PX)
        .frame(egui::Frame::NONE.fill(redesign_chrome_bg(palette)))
        .show(ctx, |ui| {
            shell_titlebar::render(ui, palette);
        });

    egui::TopBottomPanel::bottom("redesign_statusbar")
        .exact_height(REDESIGN_STATUSBAR_HEIGHT_PX)
        .frame(egui::Frame::NONE.fill(redesign_chrome_bg(palette)))
        .show(ctx, |ui| {
            shell_statusbar::render(ui, palette, modlist_count, jobs_running);
        });

    egui::CentralPanel::default()
        .frame(egui::Frame::default().fill(redesign_shell_bg(palette)))
        .show(ctx, |ui| {
            body(ui);
        });
}
