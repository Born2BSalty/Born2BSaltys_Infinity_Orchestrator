// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use super::WizardApp;

mod bar;
mod confirm;
mod logic;

pub(super) fn render_nav_buttons(app: &mut WizardApp, ctx: &egui::Context) {
    bar::render(app, ctx);
    confirm::render(app, ctx);
}
