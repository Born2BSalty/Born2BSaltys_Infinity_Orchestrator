// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `tab_advanced` — Advanced sub-tab renderer.
//
// Per Phase 4 P4.T6 (SPEC §11.5):
//   - Left column "Timing & limits": ValueRows with absorb-the-gate pattern.
//     An empty input means "use BIO default" — internally maps to
//     `<field>_enabled = false` + `<field> = default`. A filled input maps to
//     `enabled = true` + parsed value. We do **not** remove the boolean
//     `_enabled` fields from `Step1Settings` (CRITICAL DIRECTIVE).
//   - Right column "Install behavior" + "WeiDU command-line flags":
//     ToggleRows.
//
// SPEC: §11.5.

use eframe::egui;

use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::settings::widgets::toggle_row;
use crate::ui::settings::widgets::value_row;
use crate::ui::shared::redesign_tokens::{
    ThemePalette, redesign_text_muted, redesign_text_primary,
};

pub fn render(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp) {
    let palette = orchestrator.theme_palette;

    egui::Grid::new("settings_advanced_grid")
        .num_columns(2)
        .spacing(egui::vec2(32.0, 12.0))
        .show(ui, |ui| {
            // ---------------- Left column ----------------
            ui.vertical(|ui| {
                col_header(ui, palette, "Timing & limits");
                value_row_gated(
                    ui,
                    palette,
                    "Custom scan depth",
                    &mut orchestrator.wizard_state.step1.depth,
                    &mut orchestrator.wizard_state.step1.custom_scan_depth,
                    5,
                    "default 5",
                );
                value_row_gated(
                    ui,
                    palette,
                    "Mod install timeout (s)",
                    &mut orchestrator.wizard_state.step1.timeout,
                    &mut orchestrator.wizard_state.step1.timeout_per_mod_enabled,
                    3600,
                    "default 3600",
                );
                value_row_gated_u64(
                    ui,
                    palette,
                    "Tick (dev)",
                    &mut orchestrator.wizard_state.step1.tick,
                    &mut orchestrator.wizard_state.step1.tick_dev_enabled,
                    500,
                    "default 500ms",
                );
                value_row_gated(
                    ui,
                    palette,
                    "Auto-answer initial delay (ms)",
                    &mut orchestrator
                        .wizard_state
                        .step1
                        .auto_answer_initial_delay_ms,
                    &mut orchestrator
                        .wizard_state
                        .step1
                        .auto_answer_initial_delay_enabled,
                    2000,
                    "default 2000",
                );
                value_row_gated(
                    ui,
                    palette,
                    "Auto-answer post-send delay (ms)",
                    &mut orchestrator
                        .wizard_state
                        .step1
                        .auto_answer_post_send_delay_ms,
                    &mut orchestrator
                        .wizard_state
                        .step1
                        .auto_answer_post_send_delay_enabled,
                    5000,
                    "default 5000",
                );
                value_row_gated(
                    ui,
                    palette,
                    "Prompt context lookback",
                    &mut orchestrator.wizard_state.step1.lookback,
                    &mut orchestrator.wizard_state.step1.lookback_enabled,
                    10,
                    "default 10",
                );
            });

            // ---------------- Right column ----------------
            ui.vertical(|ui| {
                col_header(ui, palette, "Install behavior");
                toggle_row::render(
                    ui,
                    palette,
                    "Prompt sound cue",
                    &mut orchestrator
                        .wizard_state
                        .step1
                        .prompt_required_sound_enabled,
                    Some("beep when a prompt needs you"),
                    || {},
                );
                toggle_row::render(
                    ui,
                    palette,
                    "Download missing mods",
                    &mut orchestrator.wizard_state.step1.download,
                    Some("fetch GitHub/Weasel/Morpheus during install"),
                    || {},
                );
                toggle_row::render(
                    ui,
                    palette,
                    "Casefold filename matching",
                    &mut orchestrator.wizard_state.step1.casefold,
                    Some("ASCII case-insensitive lookups"),
                    || {},
                );

                ui.add_space(10.0);
                col_header(ui, palette, "WeiDU command-line flags");
                toggle_row::render(
                    ui,
                    palette,
                    "-a abort on warnings",
                    &mut orchestrator.wizard_state.step1.abort_on_warnings,
                    None,
                    || {},
                );
                toggle_row::render(
                    ui,
                    palette,
                    "-x skip installed",
                    &mut orchestrator.wizard_state.step1.skip_installed,
                    None,
                    || {},
                );
                toggle_row::render(
                    ui,
                    palette,
                    "-o overwrite",
                    &mut orchestrator.wizard_state.step1.overwrite,
                    None,
                    || {},
                );
            });
            ui.end_row();
        });
}

fn col_header(ui: &mut egui::Ui, palette: ThemePalette, label: &str) {
    ui.label(
        egui::RichText::new(label)
            .size(12.0)
            .family(egui::FontFamily::Name("poppins_medium".into()))
            .color(redesign_text_muted(palette)),
    );
    ui.add_space(4.0);
    let _ = redesign_text_primary(palette);
}

/// Absorb-the-gate row for a `usize` field with an `enabled: bool` companion.
fn value_row_gated(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    label: &str,
    value: &mut usize,
    enabled: &mut bool,
    default_value: usize,
    placeholder: &str,
) {
    let mut buf = if *enabled {
        value.to_string()
    } else {
        String::new()
    };
    value_row::render(
        ui,
        palette,
        label,
        &mut buf,
        Some(placeholder),
        None,
        || {},
    );
    let trimmed = buf.trim();
    if trimmed.is_empty() {
        *enabled = false;
        *value = default_value;
    } else if let Ok(parsed) = trimmed.parse::<usize>() {
        *enabled = true;
        *value = parsed;
    }
}

/// Absorb-the-gate row for a `u64` field with an `enabled: bool` companion.
fn value_row_gated_u64(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    label: &str,
    value: &mut u64,
    enabled: &mut bool,
    default_value: u64,
    placeholder: &str,
) {
    let mut buf = if *enabled {
        value.to_string()
    } else {
        String::new()
    };
    value_row::render(
        ui,
        palette,
        label,
        &mut buf,
        Some(placeholder),
        None,
        || {},
    );
    let trimmed = buf.trim();
    if trimmed.is_empty() {
        *enabled = false;
        *value = default_value;
    } else if let Ok(parsed) = trimmed.parse::<u64>() {
        *enabled = true;
        *value = parsed;
    }
}
