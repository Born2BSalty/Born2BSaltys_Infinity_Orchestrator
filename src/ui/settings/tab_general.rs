// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

#[cfg(test)]
#[path = "tab_general_tests.rs"]
mod tests;

use crate::settings::installation_language::{
    INSTALLATION_LANGUAGES, installation_language_label, normalize_installation_locale,
};
use crate::settings::redesign_fields::ThemeChoice;
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
    settings_row(
        ui,
        palette,
        "Your name",
        "credited as the author on modlists you share",
        |ui| {
            name_row::render(
                ui,
                palette,
                &mut orchestrator.redesign_settings.user_name,
                &mut orchestrator.settings_screen_state.name_row_editing,
                &mut orchestrator.settings_screen_state.name_row_buffer,
                || {},
            );
        },
    );
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
            }
        }
    });
}

fn render_language_row(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    orchestrator: &mut OrchestratorApp,
) {
    let available = orchestrator.mod_installation_language_change_available();
    settings_row(
        ui,
        palette,
        "Mod installation language",
        "controls component scans and mod installation",
        |ui| {
            let current = orchestrator.wizard_state.step1.language.clone();
            let mut selected = normalize_installation_locale(&current);
            ui.add_enabled_ui(available, |ui| {
                if matches!(palette, ThemePalette::Light) {
                    let widgets = &mut ui.visuals_mut().widgets;
                    widgets.open.bg_fill = widgets.inactive.bg_fill;
                    widgets.open.weak_bg_fill = widgets.inactive.weak_bg_fill;
                }
                egui::ComboBox::from_id_salt("settings_general_language")
                    .selected_text(
                        egui::RichText::new(installation_language_label(&current))
                            .size(12.0)
                            .family(egui::FontFamily::Name("poppins_medium".into())),
                    )
                    .show_ui(ui, |ui| {
                        for (locale, label) in INSTALLATION_LANGUAGES {
                            if ui.selectable_label(selected == *locale, *label).clicked() {
                                selected = (*locale).to_string();
                                orchestrator.select_mod_installation_language(locale);
                            }
                        }
                    });
            });
        },
    );
    ui.add_space(8.0);
    let hint = if available {
        "Existing lists: use Rescan Mods after a change. Unavailable mod languages fall back to English or another available language."
    } else {
        "Available after installation finishes"
    };
    ui.add(
        egui::Label::new(
            egui::RichText::new(hint)
                .size(11.0)
                .color(redesign_text_faint(palette)),
        )
        .wrap(),
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
                orchestrator.settings_screen_state.path_validation_results = if on {
                    crate::app::compat_dlc_source::invalidate_source_check(
                        &mut orchestrator.wizard_state.step1,
                    );
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
            egui::Stroke::new(1.0_f32, color),
        );
        x += dash_w + gap_w;
    }
}
