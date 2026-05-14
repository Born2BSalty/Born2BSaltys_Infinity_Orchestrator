// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `tab_general` — General sub-tab renderer.
//
// Per Phase 4 P4.T7: NameRow at the top, then 2-col grid of:
//   - Theme (segmented light/dark) — writes `RedesignSettings::theme_palette`
//     + `OrchestratorApp::theme_palette` so the change is live on the very
//     next frame.
//   - Language (ComboBox).
//   - Validate-all-paths-on-startup (Toggle, default on).
//   - Diagnostic mode (Toggle, default off). OR'd with the CLI `-d` flag at
//     app launch per M12; toggling at runtime updates `dev_mode` next frame.
//
// SPEC: §11.1.

use eframe::egui;

use crate::settings::redesign_fields::{ThemeChoice, UiLanguage};
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::settings::widgets::{name_row, segmented_toggle, toggle_row};
use crate::ui::shared::redesign_tokens::{
    ThemePalette, redesign_text_faint, redesign_text_muted, redesign_text_primary,
};

pub fn render(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp) {
    let palette = orchestrator.theme_palette;

    // NameRow.
    let user_name = &mut orchestrator.redesign_settings.user_name;
    let editing = &mut orchestrator.settings_screen_state.name_row_editing;
    let buffer = &mut orchestrator.settings_screen_state.name_row_buffer;
    let mut name_changed = false;
    name_row::render(
        ui,
        palette,
        "Your name",
        user_name,
        editing,
        buffer,
        || name_changed = true,
    );
    if name_changed {
        orchestrator.redesign_settings_dirty = true;
    }

    ui.add_space(16.0);
    section_divider(ui, palette);
    ui.add_space(12.0);

    // 2-column grid: row 1 = Theme | Language, row 2 = Validate | Diagnostic.
    // Each cell is a horizontal SettingsRow (label/hint stack on left, control
    // on right) — matches the wireframe `gridTemplateColumns: "1fr 1fr"`.
    ui.columns(2, |cols| {
        // ── Theme ──
        settings_row(
            &mut cols[0],
            palette,
            "Theme",
            "light parchment or warm dark",
            |ui| {
                let current = orchestrator.redesign_settings.theme_palette;
                let clicked = segmented_toggle::render(
                    ui,
                    palette,
                    [
                        ("light", current == ThemeChoice::Light),
                        ("dark", current == ThemeChoice::Dark),
                    ],
                );
                if let Some(i) = clicked {
                    let chosen = if i == 0 {
                        ThemeChoice::Light
                    } else {
                        ThemeChoice::Dark
                    };
                    if chosen != current {
                        orchestrator.redesign_settings.theme_palette = chosen;
                        orchestrator.theme_palette = match chosen {
                            ThemeChoice::Light => ThemePalette::Light,
                            ThemeChoice::Dark => ThemePalette::Dark,
                        };
                        orchestrator.redesign_settings_dirty = true;
                    }
                }
            },
        );
        // ── Language ──
        settings_row(
            &mut cols[1],
            palette,
            "Language",
            "language used across the BIO app",
            |ui| {
                // ComboBox pinned to right edge (right_to_left layout in the
                // settings_row control slot).
                let current_lang = orchestrator.redesign_settings.language;
                let mut new_lang = current_lang;
                egui::ComboBox::from_id_salt("settings_general_language")
                    .selected_text(
                        egui::RichText::new(current_lang.label())
                            .size(12.0)
                            .family(egui::FontFamily::Name("poppins_medium".into()))
                            .color(redesign_text_primary(palette)),
                    )
                    .show_ui(ui, |ui| {
                        for option in UiLanguage::all() {
                            ui.selectable_value(&mut new_lang, *option, option.label());
                        }
                    });
                if new_lang != current_lang {
                    orchestrator.redesign_settings.language = new_lang;
                    orchestrator.redesign_settings_dirty = true;
                }
                // Status indicator: the picker persists but BIO ships
                // Latin-only Poppins + has no i18n layer yet, so non-English
                // selections currently have no visible effect (SPEC §11.1
                // notes this is a v1-alpha visual stub).
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new("(coming soon)")
                        .size(11.0)
                        .family(egui::FontFamily::Proportional)
                        .color(redesign_text_faint(palette)),
                );
            },
        );
    });

    ui.columns(2, |cols| {
        // ── Validate on startup ──
        settings_row(
            &mut cols[0],
            palette,
            "Validate all paths on startup",
            "warns if game folders moved",
            |ui| {
                let mut on = orchestrator.redesign_settings.validate_paths_on_startup;
                let mut changed = false;
                toggle_row::render(ui, palette, "", &mut on, None, || changed = true);
                if changed {
                    orchestrator.redesign_settings.validate_paths_on_startup = on;
                    orchestrator.redesign_settings_dirty = true;
                    // Reflect the toggle's new state immediately rather than
                    // waiting for the next app launch: on → seed the inline
                    // status with a fresh pass; off → clear it so rows go
                    // neutral until the user edits.
                    orchestrator.settings_screen_state.path_validation_results = if on {
                        crate::ui::settings::validate_now::run_now(
                            &orchestrator.wizard_state.step1,
                        )
                    } else {
                        Default::default()
                    };
                }
            },
        );
        // ── Diagnostic mode ──
        settings_row(
            &mut cols[1],
            palette,
            "Diagnostic mode",
            "extra logging for bug reports",
            |ui| {
                let mut on = orchestrator.redesign_settings.diagnostic_mode;
                let mut changed = false;
                toggle_row::render(ui, palette, "", &mut on, None, || changed = true);
                if changed {
                    orchestrator.redesign_settings.diagnostic_mode = on;
                    // M12: dev_mode = cli_flag || persisted_toggle.
                    orchestrator.dev_mode = orchestrator.dev_mode_cli_flag || on;
                    orchestrator.redesign_settings_dirty = true;
                }
            },
        );
    });
}

/// A single cell in the General-tab grid. Horizontal layout per wireframe
/// `SettingsRow` (`screens.jsx:3823`): label/hint stack on the left grows to
/// fill remaining width; control sits flush-right with a stable width.
/// Dashed bottom rule mirrors `borderBottom: "1px dashed #ccc"`.
fn settings_row(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    label: &str,
    hint: &str,
    control: impl FnOnce(&mut egui::Ui),
) {
    ui.add_space(8.0);
    let row_top = ui.cursor().top();
    ui.horizontal(|ui| {
        // Left: label + hint stack, grows.
        let cell_width = ui.available_width();
        ui.allocate_ui_with_layout(
            egui::vec2(cell_width, 36.0),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                ui.label(
                    egui::RichText::new(label)
                        .size(13.0)
                        .family(egui::FontFamily::Name("poppins_medium".into()))
                        .color(redesign_text_primary(palette)),
                );
                ui.label(
                    egui::RichText::new(hint)
                        .size(11.0)
                        .family(egui::FontFamily::Proportional)
                        .color(redesign_text_muted(palette)),
                );
            },
        );
        // Right: control, flush-right, fixed-fit width.
        ui.with_layout(
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| control(ui),
        );
    });
    ui.add_space(8.0);

    // Dashed bottom rule across the cell width.
    let row_bottom = ui.cursor().top();
    let rect = ui.max_rect();
    draw_dashed_horizontal(
        ui.painter(),
        row_bottom,
        rect.left(),
        rect.right(),
        redesign_text_faint(palette),
    );
    let _ = row_top;
}

fn draw_dashed_horizontal(
    painter: &egui::Painter,
    y: f32,
    left: f32,
    right: f32,
    color: egui::Color32,
) {
    let dash_w = 4.0;
    let gap_w = 4.0;
    let mut x = left;
    while x < right {
        let x_end = (x + dash_w).min(right);
        painter.line_segment(
            [egui::pos2(x, y), egui::pos2(x_end, y)],
            egui::Stroke::new(1.0, color),
        );
        x += dash_w + gap_w;
    }
}

fn section_divider(ui: &mut egui::Ui, palette: ThemePalette) {
    let rect = ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover()).0;
    let painter = ui.painter();
    let x0 = rect.left();
    let x1 = rect.right();
    let y = rect.center().y;
    let mut x = x0;
    while x < x1 {
        let xe = (x + 4.0).min(x1);
        painter.line_segment(
            [egui::pos2(x, y), egui::pos2(xe, y)],
            egui::Stroke::new(1.0, redesign_text_muted(palette)),
        );
        x += 8.0;
    }
}
