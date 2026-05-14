// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `tab_tools` — Tools sub-tab renderer.
//
// Per Phase 4 P4.T4: 4 PathRows for the tool binaries:
//   - `weidu`         → `Step1Settings::weidu_binary`
//   - `mod_installer` → `Step1Settings::mod_installer_binary`
//   - `7z executable` → no backing field in v1 alpha (visual stub; Phase 7
//                      can wire it when archive-extraction grows config).
//   - `git executable`→ no backing field in v1 alpha (visual stub).
//
// Each row shows a detected-version hint (cached on `OrchestratorApp`).
// Phase 4 ships a static stub for the version cache; the cache becomes live
// in Phase 7 when the install runner exercises `weidu --help`.

use eframe::egui;

use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::settings::state_settings::{PathStatus, PathStatusTone};
use crate::ui::settings::validate_debounce;
use crate::ui::settings::validate_now;
use crate::ui::settings::widgets::path_row::{self, PathRowMode};
use crate::ui::shared::redesign_tokens::ThemePalette;

pub fn render(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp) {
    let palette = orchestrator.theme_palette;

    bin_row_for_field(
        ui,
        palette,
        orchestrator,
        "WeiDU binary",
        validate_now::FIELD_WEIDU_BINARY,
        orchestrator
            .tool_version_cache
            .weidu_version
            .clone()
            .map(|v| format!("v{v}")),
    );
    bin_row_for_field(
        ui,
        palette,
        orchestrator,
        "Mod installer",
        validate_now::FIELD_MOD_INSTALLER_BINARY,
        orchestrator
            .tool_version_cache
            .mod_installer_version
            .clone(),
    );

    // 7z / git rows — visual stubs (no Step1Settings backing field).
    stub_bin_row(ui, palette, "7-Zip executable", "system \u{2713}");
    stub_bin_row(ui, palette, "Git executable", "system \u{2713}");
}

fn bin_row_for_field(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    orchestrator: &mut OrchestratorApp,
    label: &str,
    field: &'static str,
    version_hint: Option<String>,
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
        (Some("checking\u{2026}".to_string()), PathStatusTone::Neutral)
    } else {
        let validated_hint = status.map(PathStatus::hint_text);
        let hint = version_hint.or(validated_hint);
        let tone = status.map(PathStatus::tone).unwrap_or(PathStatusTone::Neutral);
        (hint, tone)
    };

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
                PathRowMode::File,
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
        f if f == validate_now::FIELD_WEIDU_BINARY => Some(&mut step1.weidu_binary),
        f if f == validate_now::FIELD_MOD_INSTALLER_BINARY => Some(&mut step1.mod_installer_binary),
        _ => None,
    }
}

fn stub_bin_row(ui: &mut egui::Ui, palette: ThemePalette, label: &str, hint: &str) {
    let mut placeholder = String::new();
    path_row::render(
        ui,
        palette,
        label,
        &mut placeholder,
        Some(hint),
        PathStatusTone::Neutral,
        PathRowMode::File,
        || {},
    );
}
