// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::settings::redesign_fields::{ThemeChoice, UiLanguage};
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::settings::widgets::{name_row, segmented_toggle, toggle_row};
use crate::ui::shared::redesign_tokens::{
    ThemePalette, redesign_text_faint, redesign_text_muted, redesign_text_primary,
};

pub fn render(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp) {
    let palette = orchestrator.theme_palette;

    render_name_row(ui, palette, orchestrator);
    ui.add_space(12.0);
    render_theme_language_rows(ui, palette, orchestrator);
    render_mode_rows(ui, palette, orchestrator);
}

fn render_name_row(ui: &mut egui::Ui, palette: ThemePalette, orchestrator: &mut OrchestratorApp) {
    let mut name_changed = false;
    settings_row(
        ui,
        palette,
        "Your name",
        "credited as the author on any modlists you create or share",
        |ui| {
            name_row::render(
                ui,
                palette,
                &mut orchestrator.redesign_settings.user_name,
                &mut orchestrator.settings_screen_state.name_row_editing,
                &mut orchestrator.settings_screen_state.name_row_buffer,
                || name_changed = true,
            );
        },
    );
    if name_changed {
        orchestrator.redesign_settings_dirty = true;
    }
}

fn render_theme_language_rows(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    orchestrator: &mut OrchestratorApp,
) {
    ui.columns(2, |cols| {
        render_theme_row(&mut cols[0], palette, orchestrator);
        render_language_row(&mut cols[1], palette, orchestrator);
    });
}

fn render_mode_rows(ui: &mut egui::Ui, palette: ThemePalette, orchestrator: &mut OrchestratorApp) {
    ui.columns(2, |cols| {
        render_validate_on_startup_row(&mut cols[0], palette, orchestrator);
        render_diagnostic_mode_row(&mut cols[1], palette, orchestrator);
    });
}

fn render_theme_row(ui: &mut egui::Ui, palette: ThemePalette, orchestrator: &mut OrchestratorApp) {
    settings_row(ui, palette, "Theme", "light parchment or warm dark", |ui| {
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
    });
}

fn render_language_row(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    orchestrator: &mut OrchestratorApp,
) {
    settings_row(
        ui,
        palette,
        "Language",
        "language used across the BIO app",
        |ui| {
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
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new("(coming soon)")
                    .size(11.0)
                    .family(egui::FontFamily::Proportional)
                    .color(redesign_text_faint(palette)),
            );
        },
    );
}

fn render_validate_on_startup_row(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    orchestrator: &mut OrchestratorApp,
) {
    settings_row(
        ui,
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
                orchestrator.settings_screen_state.path_validation_results = if on {
                    crate::ui::settings::validate_now::run_now(&orchestrator.wizard_state.step1)
                } else {
                    crate::ui::settings::state_settings::ValidationReport::default()
                };
            }
        },
    );
}

fn render_diagnostic_mode_row(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    orchestrator: &mut OrchestratorApp,
) {
    settings_row(
        ui,
        palette,
        "Diagnostic mode",
        "extra logging for bug reports",
        |ui| {
            let mut on = orchestrator.redesign_settings.diagnostic_mode;
            let mut changed = false;
            toggle_row::render(ui, palette, "", &mut on, None, || changed = true);
            if changed {
                orchestrator.redesign_settings.diagnostic_mode = on;
                orchestrator.dev_mode = orchestrator.dev_mode_cli_flag || on;
                orchestrator.redesign_settings_dirty = true;
            }
        },
    );
}

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
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            control(ui);
        });
    });
    ui.add_space(8.0);

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
    loop {
        if x >= right {
            break;
        }
        let x_end = (x + dash_w).min(right);
        painter.line_segment(
            [egui::pos2(x, y), egui::pos2(x_end, y)],
            egui::Stroke::new(1.0, color),
        );
        x += dash_w + gap_w;
    }
}
