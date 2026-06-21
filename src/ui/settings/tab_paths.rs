// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::settings::state_settings::{PathStatus, PathStatusTone};
use crate::ui::settings::validate_debounce;
use crate::ui::settings::validate_now;
use crate::ui::settings::widgets::path_row::{self, PathRow, PathRowMode};
use crate::ui::shared::redesign_tokens::{ThemePalette, redesign_text_faint, redesign_text_muted};

pub fn render(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp) {
    let palette = orchestrator.theme_palette;

    section_header(ui, palette, "GAME SOURCES");
    path_row_for_field(
        ui,
        palette,
        orchestrator,
        "BGEE source",
        validate_now::FIELD_BGEE_GAME_FOLDER,
    );
    path_row_for_field(
        ui,
        palette,
        orchestrator,
        "BG2EE source",
        validate_now::FIELD_BG2EE_GAME_FOLDER,
    );
    path_row_for_field(
        ui,
        palette,
        orchestrator,
        "IWDEE source",
        validate_now::FIELD_IWDEE_GAME_FOLDER,
    );

    ui.add_space(12.0);
    section_header(ui, palette, "WORKING FOLDERS");
    path_row_for_field(
        ui,
        palette,
        orchestrator,
        "Mods folder",
        validate_now::FIELD_GLOBAL_MODS_FOLDER,
    );
    path_row_for_field(
        ui,
        palette,
        orchestrator,
        "Mods archive",
        validate_now::FIELD_MODS_ARCHIVE_FOLDER,
    );
    path_row_for_field(
        ui,
        palette,
        orchestrator,
        "Mods backup",
        validate_now::FIELD_MODS_BACKUP_FOLDER,
    );
}

fn path_row_for_field(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    orchestrator: &mut OrchestratorApp,
    label: &str,
    field: &'static str,
) {
    let is_pending = orchestrator
        .settings_screen_state
        .path_edit_debounce
        .contains_key(field);

    let status = orchestrator
        .settings_screen_state
        .path_validation_results
        .fields
        .get(field);
    let (hint, tone) = if is_pending {
        (
            Some("checking\u{2026}".to_string()),
            PathStatusTone::Neutral,
        )
    } else {
        let hint = status.map(PathStatus::hint_text);
        let tone = status.map_or(PathStatusTone::Neutral, PathStatus::tone);
        (hint, tone)
    };

    let mut changed_field: Option<&'static str> = None;
    {
        let value_ref = field_mut(&mut orchestrator.wizard_state.step1, field);
        if let Some(v) = value_ref {
            path_row::render(
                ui,
                palette,
                PathRow {
                    label,
                    mono_value: v,
                    hint: hint.as_deref(),
                    tone,
                    mode: PathRowMode::Folder,
                },
                || changed_field = Some(field),
            );
        }
    }
    if let Some(f) = changed_field {
        validate_debounce::mark_dirty(orchestrator, f);
    }
}

fn field_mut<'a>(
    step1: &'a mut crate::app::state::Step1State,
    field: &'static str,
) -> Option<&'a mut String> {
    match field {
        f if f == validate_now::FIELD_BGEE_GAME_FOLDER => Some(&mut step1.bgee_game_folder),
        f if f == validate_now::FIELD_BG2EE_GAME_FOLDER => Some(&mut step1.bg2ee_game_folder),
        f if f == validate_now::FIELD_IWDEE_GAME_FOLDER => Some(&mut step1.iwdee_game_folder),
        f if f == validate_now::FIELD_EET_BGEE_GAME_FOLDER => Some(&mut step1.eet_bgee_game_folder),
        f if f == validate_now::FIELD_EET_BG2EE_GAME_FOLDER => {
            Some(&mut step1.eet_bg2ee_game_folder)
        }
        f if f == validate_now::FIELD_GLOBAL_MODS_FOLDER => Some(&mut step1.global_mods_folder),
        f if f == validate_now::FIELD_MODS_ARCHIVE_FOLDER => Some(&mut step1.mods_archive_folder),
        f if f == validate_now::FIELD_MODS_BACKUP_FOLDER => Some(&mut step1.mods_backup_folder),
        f if f == validate_now::FIELD_WEIDU_LOG_FOLDER => Some(&mut step1.weidu_log_folder),
        f if f == validate_now::FIELD_WEIDU_BINARY => Some(&mut step1.weidu_binary),
        f if f == validate_now::FIELD_MOD_INSTALLER_BINARY => Some(&mut step1.mod_installer_binary),
        _ => None,
    }
}

fn section_header(ui: &mut egui::Ui, palette: ThemePalette, label: &str) {
    ui.label(
        egui::RichText::new(label)
            .size(13.0)
            .family(egui::FontFamily::Name("poppins_medium".into()))
            .color(redesign_text_muted(palette)),
    );
    ui.add_space(4.0);
    let _ = redesign_text_faint(palette);
}
