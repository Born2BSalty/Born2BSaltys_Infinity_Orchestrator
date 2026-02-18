// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::layout::BOTTOM_BOX_HEIGHT;
use crate::ui::state::Step1State;
use crate::ui::step1::widgets::{path_row_dir, path_row_file, section_title};

pub(super) fn render_install_paths_section(ui: &mut egui::Ui, s: &mut Step1State) {
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
        ui.set_min_height(BOTTOM_BOX_HEIGHT);
        let title = match s.game_install.as_str() {
            "BG2EE" => "Install Paths BG2EE:",
            "EET" => "Install Paths EET:",
            _ => "Install Paths BGEE:",
        };
        section_title(ui, title);
        ui.add_space(10.0);
        egui::ScrollArea::vertical()
            .id_salt("install_paths_scroll")
            .max_height(BOTTOM_BOX_HEIGHT - 24.0)
            .show(ui, |ui| match s.game_install.as_str() {
                "BG2EE" => render_bg2ee_paths(ui, s),
                "EET" => render_eet_paths(ui, s),
                _ => render_bgee_paths(ui, s),
            });
    });
}

fn render_bg2ee_paths(ui: &mut egui::Ui, s: &mut Step1State) {
    if s.generate_directory_enabled {
        ui.label(egui::RichText::new("Using -g: source + generated target.").weak());
    }
    path_row_dir(ui, "BG2EE Game Folder", &mut s.bg2ee_game_folder);
    if s.have_weidu_logs {
        path_row_file(ui, "BG2EE WeiDU Log File", &mut s.bg2ee_log_file);
    } else {
        path_row_dir(ui, "BG2EE WeiDU Log Folder", &mut s.bg2ee_log_folder);
    }
    if s.generate_directory_enabled {
        path_row_dir(ui, "Generate Directory (-g)", &mut s.generate_directory);
    }
}

fn render_eet_paths(ui: &mut egui::Ui, s: &mut Step1State) {
    if s.new_pre_eet_dir_enabled {
        ui.label(egui::RichText::new("Using -p: source BGEE -> Pre-EET target.").weak());
        path_row_dir(ui, "Source BGEE Folder (-p)", &mut s.bgee_game_folder);
        path_row_dir(ui, "Pre-EET Directory", &mut s.eet_pre_dir);
    } else {
        path_row_dir(ui, "BGEE Game Folder", &mut s.eet_bgee_game_folder);
    }
    if s.have_weidu_logs {
        path_row_file(ui, "BGEE WeiDU Log File", &mut s.bgee_log_file);
    } else {
        path_row_dir(ui, "BGEE WeiDU Log Folder", &mut s.eet_bgee_log_folder);
    }
    if s.new_eet_dir_enabled {
        ui.label(egui::RichText::new("Using -n: source BG2EE -> New EET target.").weak());
        path_row_dir(ui, "Source BG2EE Folder (-n)", &mut s.bg2ee_game_folder);
        path_row_dir(ui, "New EET Directory", &mut s.eet_new_dir);
    } else {
        path_row_dir(ui, "BG2EE Game Folder", &mut s.eet_bg2ee_game_folder);
    }
    if s.have_weidu_logs {
        path_row_file(ui, "BG2EE WeiDU Log File", &mut s.bg2ee_log_file);
    } else {
        path_row_dir(ui, "BG2EE WeiDU Log Folder", &mut s.eet_bg2ee_log_folder);
    }
}

fn render_bgee_paths(ui: &mut egui::Ui, s: &mut Step1State) {
    if s.generate_directory_enabled {
        ui.label(egui::RichText::new("Using -g: source + generated target.").weak());
    }
    path_row_dir(ui, "BGEE Game Folder", &mut s.bgee_game_folder);
    if s.have_weidu_logs {
        path_row_file(ui, "BGEE WeiDU Log File", &mut s.bgee_log_file);
    } else {
        path_row_dir(ui, "BGEE WeiDU Log Folder", &mut s.bgee_log_folder);
    }
    if s.generate_directory_enabled {
        path_row_dir(ui, "Generate Directory (-g)", &mut s.generate_directory);
    }
}
