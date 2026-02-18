// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::layout::TOP_BOX_HEIGHT;
use crate::ui::state::Step1State;
use crate::ui::step1::widgets::{path_row_dir, path_row_file, section_title};

pub(super) fn render_mods_folder_section(ui: &mut egui::Ui, s: &mut Step1State) {
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
        ui.set_min_height(TOP_BOX_HEIGHT);
        section_title(ui, "Mods Folder");
        path_row_dir(ui, "Your Mods Folder", &mut s.mods_folder);
    });
}

pub(super) fn render_tools_section(ui: &mut egui::Ui, s: &mut Step1State) {
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
        ui.set_min_height(TOP_BOX_HEIGHT);
        section_title(ui, "Tools");
        path_row_file(ui, "WeiDU Binary", &mut s.weidu_binary);
        path_row_file(ui, "mod_installer Binary", &mut s.mod_installer_binary);
    });
}
