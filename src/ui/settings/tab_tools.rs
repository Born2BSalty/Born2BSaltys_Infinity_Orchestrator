// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::settings::state_settings::SettingsScreenState;
use crate::ui::settings::widgets::path_row;
use crate::ui::shared::redesign_tokens::{ThemePalette, redesign_text_muted};

pub fn render(ui: &mut egui::Ui, palette: ThemePalette, state: &mut SettingsScreenState) {
    section_label(
        ui,
        palette,
        "executable paths \u{b7} auto-detected when possible",
    );
    path_row::render(
        ui,
        palette,
        "weidu.exe",
        &mut state.weidu_executable_path,
        None,
    );
    path_row::render(
        ui,
        palette,
        "mod_installer.exe",
        &mut state.mod_installer_executable_path,
        None,
    );
    path_row::render(
        ui,
        palette,
        "7z executable",
        &mut state.seven_zip_executable_path,
        None,
    );
    path_row::render(
        ui,
        palette,
        "Git executable",
        &mut state.git_executable_path,
        None,
    );
}

fn section_label(ui: &mut egui::Ui, palette: ThemePalette, label: &str) {
    ui.label(
        egui::RichText::new(label)
            .size(13.0)
            .color(redesign_text_muted(palette))
            .strong(),
    );
    ui.add_space(4.0);
}
