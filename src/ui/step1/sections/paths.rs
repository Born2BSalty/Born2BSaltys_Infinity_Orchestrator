// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

mod install_paths;
mod layout;
mod log_mode;
mod mods_tools;

use eframe::egui;

use crate::ui::state::Step1State;

pub fn symmetric_boxes_layout(ui: &mut egui::Ui, s: &mut Step1State) {
    layout::render(ui, s);
}
