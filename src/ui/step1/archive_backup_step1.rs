// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;
use rfd::FileDialog;

use crate::app::state::Step1State;
use crate::ui::layout::{
    BROWSE_BUTTON_WIDTH, PATH_FIELD_MIN_WIDTH, PATH_INPUT_HEIGHT, PATH_LABEL_WIDTH,
    PATH_ROW_INNER_GAP,
};
use crate::ui::shared::typography_global as typo;

pub fn render_archive_backup_content(ui: &mut egui::Ui, s: &mut Step1State) {
    ui.label(typo::section_title("Mods Archive / Backup"));
    path_row_dir(ui, "Mods Archive", &mut s.mods_archive_folder);
    path_row_dir(ui, "Backup", &mut s.mods_backup_folder);
}

fn path_row_dir(ui: &mut egui::Ui, label: &str, value: &mut String) {
    ui.horizontal(|ui| {
        right_label(ui, label);
        let text_width = (ui.available_width() - BROWSE_BUTTON_WIDTH - PATH_ROW_INNER_GAP)
            .max(PATH_FIELD_MIN_WIDTH);
        ui.add_sized(
            [text_width, PATH_INPUT_HEIGHT],
            egui::TextEdit::singleline(value).clip_text(true),
        );
        if ui
            .add_sized(
                [BROWSE_BUTTON_WIDTH, PATH_INPUT_HEIGHT],
                egui::Button::new("Browse"),
            )
            .clicked()
            && let Some(path) = FileDialog::new().pick_folder()
        {
            *value = path.display().to_string();
        }
    });
}

fn right_label(ui: &mut egui::Ui, label: &str) {
    ui.allocate_ui_with_layout(
        egui::vec2(PATH_LABEL_WIDTH, PATH_INPUT_HEIGHT),
        egui::Layout::right_to_left(egui::Align::Center),
        |ui| {
            ui.label(typo::strong(format!("{label}:")));
        },
    );
}
