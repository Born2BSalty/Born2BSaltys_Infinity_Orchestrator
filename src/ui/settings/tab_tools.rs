// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `tab_tools` — Tools sub-tab renderer.
//
// Per SPEC §11.3:
//   - `weidu`         → `Step1Settings::weidu_binary` (writable, validated)
//   - `mod_installer` → `Step1Settings::mod_installer_binary` (writable, validated)
//   - `7-Zip executable` → detection-only (no Step1Settings backing field;
//     system PATH is the only source)
//   - `git executable`   → detection-only (same)
//
// Detection runs once in `OrchestratorApp::new` via `validate_now::resolve_on_path`
// and is cached on `tool_version_cache.{sevenzip,git}_path`.

use std::path::Path;

use eframe::egui;

use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::settings::state_settings::{PathStatus, PathStatusTone};
use crate::ui::settings::validate_debounce;
use crate::ui::settings::validate_now;
use crate::ui::settings::widgets::path_row::{self, PathRowMode};
use crate::ui::shared::redesign_tokens::{
    ThemePalette, redesign_pill_danger, redesign_success_soft, redesign_text_faint,
    redesign_text_primary, redesign_warning_soft,
};

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

    ui.add_space(8.0);

    // Detection-only rows — no input, no browse, since these don't have
    // backing fields in Step1Settings. They show system-wide install status
    // so the user knows whether archive extraction (7z) and git-based mod
    // updates will work.
    detection_row(
        ui,
        palette,
        "7-Zip executable",
        orchestrator.tool_version_cache.sevenzip_path.as_deref(),
        "needed for archive extraction during install",
    );
    detection_row(
        ui,
        palette,
        "Git executable",
        orchestrator.tool_version_cache.git_path.as_deref(),
        "needed for git-based mod updates",
    );
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

/// Read-only detection row for tools we don't allow the user to override
/// (no Step1Settings backing field). Renders the label + a single line of
/// status text indicating whether the tool was found on `$PATH` at startup.
fn detection_row(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    label: &str,
    resolved: Option<&Path>,
    purpose: &str,
) {
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            // Label column (same fixed width as the binary rows so the two
            // sections line up visually).
            let (label_rect, _) = ui.allocate_exact_size(
                egui::vec2(160.0, 24.0),
                egui::Sense::hover(),
            );
            ui.painter().text(
                egui::pos2(label_rect.left(), label_rect.center().y),
                egui::Align2::LEFT_CENTER,
                label,
                egui::FontId::new(13.0, egui::FontFamily::Name("poppins_medium".into())),
                redesign_text_primary(palette),
            );

            let (text, color) = match resolved {
                Some(path) => (
                    format!("found at {}", path.display()),
                    redesign_success_soft(palette),
                ),
                None => (
                    format!("not installed \u{2014} {purpose}"),
                    redesign_warning_soft(palette),
                ),
            };
            ui.label(
                egui::RichText::new(text)
                    .size(12.0)
                    .family(egui::FontFamily::Proportional)
                    .color(color),
            );
        });
    });
    ui.add_space(6.0);
    // Suppress unused warning for danger token — reserved for a future
    // "found but broken" state once we shell out to `7z --help` / `git --version`.
    let _ = redesign_pill_danger;
    let _ = redesign_text_faint;
}
