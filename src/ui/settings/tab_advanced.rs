// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::settings::state_settings::SettingsScreenState;
use crate::ui::settings::widgets::{toggle_row, value_row};
use crate::ui::shared::redesign_tokens::{ThemePalette, redesign_text_muted};

pub fn render(ui: &mut egui::Ui, palette: ThemePalette, state: &mut SettingsScreenState) {
    egui::Grid::new("settings_advanced_grid")
        .num_columns(2)
        .spacing([28.0, 0.0])
        .show(ui, |ui| {
            ui.vertical(|ui| render_timing_limits(ui, palette, state));

            ui.vertical(|ui| render_install_behavior(ui, palette, state));
        });
}

fn render_timing_limits(ui: &mut egui::Ui, palette: ThemePalette, state: &mut SettingsScreenState) {
    section_label(ui, palette, "timing & limits");
    value_row::render(
        ui,
        palette,
        "Custom scan depth",
        &mut state.custom_scan_depth,
        "3",
        None,
    );
    value_row::render(
        ui,
        palette,
        "Mod install timeout",
        &mut state.mod_install_timeout,
        "7200",
        Some("sec"),
    );
    value_row::render(
        ui,
        palette,
        "Mod install timeout (per mod)",
        &mut state.mod_install_timeout_per_mod,
        "\u{2014}",
        Some("sec \u{b7} experimental"),
    );
    value_row::render(
        ui,
        palette,
        "Auto-answer initial delay",
        &mut state.auto_answer_initial_delay,
        "4000",
        Some("ms"),
    );
    value_row::render(
        ui,
        palette,
        "Auto-answer post-send delay",
        &mut state.auto_answer_post_send_delay,
        "5000",
        Some("ms"),
    );
    value_row::render(
        ui,
        palette,
        "Tick (dev)",
        &mut state.tick_dev,
        "500",
        Some("ms"),
    );
    value_row::render(
        ui,
        palette,
        "Prompt context lookback",
        &mut state.prompt_context_lookback,
        "1007",
        None,
    );
}

fn render_install_behavior(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    state: &mut SettingsScreenState,
) {
    section_label(ui, palette, "install behavior");
    toggle_row::render(
        ui,
        palette,
        "Sound cue when prompt input is required",
        &mut state.sound_cue_when_prompt_input_required,
        "",
    );
    toggle_row::render(
        ui,
        palette,
        "Download missing mods and keep archives",
        &mut state.download_missing_mods_and_keep_archives,
        "",
    );
    toggle_row::render(
        ui,
        palette,
        "Case-insensitive component matching",
        &mut state.case_insensitive_component_matching,
        "",
    );

    ui.add_space(14.0);
    section_label(ui, palette, "WeiDU command-line flags");
    toggle_row::render(
        ui,
        palette,
        "-a   Abort on warnings",
        &mut state.abort_on_warnings,
        "",
    );
    toggle_row::render(
        ui,
        palette,
        "-x   Strict matching",
        &mut state.strict_matching,
        "",
    );
    toggle_row::render(
        ui,
        palette,
        "-o   Overwrite mod folder",
        &mut state.overwrite_mod_folder,
        "",
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
