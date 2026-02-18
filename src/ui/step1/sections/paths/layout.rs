// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::layout::SECTION_GAP;
use crate::ui::state::Step1State;

use super::install_paths::render_install_paths_section;
use super::log_mode::render_weidu_log_mode_section;
use super::mods_tools::{render_mods_folder_section, render_tools_section};

pub(super) fn render(ui: &mut egui::Ui, s: &mut Step1State) {
    ui.columns(2, |cols| {
        cols[0].vertical(|ui| render_mods_folder_section(ui, s));
        cols[1].vertical(|ui| render_tools_section(ui, s));
    });
    ui.add_space(SECTION_GAP);
    if s.weidu_log_mode_enabled {
        ui.columns(2, |cols| {
            cols[0].vertical(|ui| render_install_paths_section(ui, s));
            cols[1].vertical(|ui| render_weidu_log_mode_section(ui, s));
        });
    } else {
        render_install_paths_section(ui, s);
    }
}
