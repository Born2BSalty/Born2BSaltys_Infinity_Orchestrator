// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

pub fn configure_startup_visuals(ctx: &egui::Context) {
    crate::ui::shared::theme_global::apply_runtime_theme(ctx);
}
