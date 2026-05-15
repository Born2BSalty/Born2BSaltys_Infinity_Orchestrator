// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::orchestrator::widgets::btn::{self, BtnOpts};
use crate::ui::settings::state_settings::SettingsScreenState;
use crate::ui::settings::validate_debounce;
use crate::ui::settings::widgets::path_row;
use crate::ui::shared::redesign_tokens::{ThemePalette, redesign_text_muted};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathsAction {
    ValidatePathsNow,
}

pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    state: &mut SettingsScreenState,
) -> Option<PathsAction> {
    let now = std::time::Instant::now();

    section_label(ui, palette, "game sources");
    if path_row::render(
        ui,
        palette,
        "BGEE source",
        &mut state.bgee_source_path,
        None,
    ) {
        validate_debounce::mark_dirty(state, "bgee_source_path", now);
    }
    if path_row::render(
        ui,
        palette,
        "BG2EE source",
        &mut state.bg2ee_source_path,
        None,
    ) {
        validate_debounce::mark_dirty(state, "bg2ee_source_path", now);
    }
    if path_row::render(
        ui,
        palette,
        "IWDEE source",
        &mut state.iwdee_source_path,
        None,
    ) {
        validate_debounce::mark_dirty(state, "iwdee_source_path", now);
    }

    ui.add_space(14.0);
    section_label(ui, palette, "working folders");
    if path_row::render(
        ui,
        palette,
        "Mods archive",
        &mut state.mods_archive_path,
        None,
    ) {
        validate_debounce::mark_dirty(state, "mods_archive_path", now);
    }
    if path_row::render(
        ui,
        palette,
        "Mods backup",
        &mut state.mods_backup_path,
        None,
    ) {
        validate_debounce::mark_dirty(state, "mods_backup_path", now);
    }
    if path_row::render(ui, palette, "Tools", &mut state.tools_path, None) {
        validate_debounce::mark_dirty(state, "tools_path", now);
    }
    if path_row::render(ui, palette, "Temp", &mut state.temp_path, None) {
        validate_debounce::mark_dirty(state, "temp_path", now);
    }

    ui.add_space(14.0);
    let response = btn::redesign_btn(
        ui,
        palette,
        "Validate now",
        BtnOpts {
            primary: false,
            small: false,
            disabled: false,
        },
    );
    response.clicked().then_some(PathsAction::ValidatePathsNow)
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
