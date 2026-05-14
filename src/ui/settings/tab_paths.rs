// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `tab_paths` — Paths sub-tab renderer.
//
// Per SPEC §11.2 (updated):
//   - Section 1 "Game sources": BGEE / BG2EE / IWDEE game folders. Validated
//     against the chitin.key + lang/ Infinity Engine marker.
//   - Section 2 "Working folders": Mods archive / Mods backup / Temp. Validated
//     to NOT look like a game install (no chitin.key); empty is fine
//     (auto-created on first install).
//   - Per-edit dirty marks via `validate_debounce::mark_dirty` feed the
//     debounce cycle which auto-validates 500ms after the user pauses typing.
//   - No "Validate now" button, no bottom aggregate summary — each row
//     carries its own inline status (border tint + reason text).

use eframe::egui;

use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::settings::state_settings::{PathStatus, PathStatusTone};
use crate::ui::settings::validate_debounce;
use crate::ui::settings::validate_now;
use crate::ui::settings::widgets::path_row::{self, PathRowMode};
use crate::ui::shared::redesign_tokens::{
    ThemePalette, redesign_text_faint, redesign_text_muted,
};

pub fn render(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp) {
    let palette = orchestrator.theme_palette;

    section_header(ui, palette, "Game sources");
    path_row_for_field(
        ui,
        palette,
        orchestrator,
        "BGEE game folder",
        validate_now::FIELD_BGEE_GAME_FOLDER,
    );
    path_row_for_field(
        ui,
        palette,
        orchestrator,
        "BG2EE game folder",
        validate_now::FIELD_BG2EE_GAME_FOLDER,
    );
    path_row_for_field(
        ui,
        palette,
        orchestrator,
        "IWDEE game folder",
        validate_now::FIELD_IWDEE_GAME_FOLDER,
    );

    ui.add_space(12.0);
    section_header(ui, palette, "Working folders");
    path_row_for_field(
        ui,
        palette,
        orchestrator,
        "Mods archive folder",
        validate_now::FIELD_MODS_ARCHIVE_FOLDER,
    );
    path_row_for_field(
        ui,
        palette,
        orchestrator,
        "Mods backup folder",
        validate_now::FIELD_MODS_BACKUP_FOLDER,
    );
    path_row_for_field(
        ui,
        palette,
        orchestrator,
        "Temp folder",
        validate_now::FIELD_MODS_FOLDER,
    );
}

fn path_row_for_field(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    orchestrator: &mut OrchestratorApp,
    label: &str,
    field: &'static str,
) {
    // If the field is mid-debounce (user just edited / picked), show a
    // transient "checking…" hint so the user sees that validation is
    // pending, rather than the stale previous result. The tone stays neutral
    // so the input border doesn't flash red/green during the brief wait.
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
        (Some("checking\u{2026}".to_string()), PathStatusTone::Neutral)
    } else {
        let hint = status.map(PathStatus::hint_text);
        let tone = status.map(PathStatus::tone).unwrap_or(PathStatusTone::Neutral);
        (hint, tone)
    };

    // Pull the &mut String out of `Step1State` based on the field name.
    let mut changed_field: Option<&'static str> = None;
    {
        let value_ref = field_mut(&mut orchestrator.wizard_state.step1, field);
        if let Some(v) = value_ref {
            path_row::render(
                ui,
                palette,
                label,
                v,
                hint.as_deref(),
                tone,
                PathRowMode::Folder,
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
        f if f == validate_now::FIELD_MODS_FOLDER => Some(&mut step1.mods_folder),
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
            .size(11.0)
            .family(egui::FontFamily::Name("poppins_medium".into()))
            .color(redesign_text_muted(palette)),
    );
    ui.add_space(4.0);
    let _ = redesign_text_faint(palette);
}
