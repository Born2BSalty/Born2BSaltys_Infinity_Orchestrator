// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

mod controls;
mod tabs;
mod util;

use eframe::egui;

use crate::ui::state::WizardState;

use super::Step2Action;

pub(super) fn render_controls(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    action: &mut Option<Step2Action>,
    controls_rect: egui::Rect,
) {
    controls::render(ui, state, action, controls_rect);
}

pub(super) fn render_tabs(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    action: &mut Option<Step2Action>,
    tabs_rect: egui::Rect,
) {
    tabs::render(ui, state, action, tabs_rect);
}
