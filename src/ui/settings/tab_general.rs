// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::settings::state_settings::SettingsScreenState;
use crate::ui::settings::widgets::{name_row, segmented_toggle, toggle_row};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX, REDESIGN_BUTTON_SMALL_PADDING_X_PX,
    REDESIGN_BUTTON_SMALL_PADDING_Y_PX, REDESIGN_DASHED_BORDER_WIDTH_PX,
    REDESIGN_HINT_FONT_SIZE_PX, REDESIGN_LABEL_FONT_SIZE_PX, REDESIGN_SETTINGS_ROW_COLUMN_GAP_PX,
    REDESIGN_SETTINGS_ROW_GAP_PX, REDESIGN_SETTINGS_ROW_GRID_GAP_Y_PX,
    REDESIGN_SETTINGS_ROW_PADDING_Y_PX, ThemePalette, redesign_border_dashed_light,
    redesign_border_strong, redesign_input_bg, redesign_text_muted, redesign_text_primary,
};

const LANGUAGES: [&str; 10] = [
    "English",
    "German",
    "French",
    "Spanish",
    "Italian",
    "Polish",
    "Portuguese",
    "Czech",
    "Turkish",
    "Ukrainian",
];

pub fn render(ui: &mut egui::Ui, palette: ThemePalette, state: &mut SettingsScreenState) -> bool {
    let before = state.current_redesign_settings();

    name_row::render(ui, palette, &mut state.user_name);

    ui.spacing_mut().item_spacing.x = REDESIGN_SETTINGS_ROW_COLUMN_GAP_PX;
    ui.spacing_mut().item_spacing.y = REDESIGN_SETTINGS_ROW_GRID_GAP_Y_PX;
    ui.columns(2, |columns| {
        columns[0].vertical(|ui| {
            settings_row(ui, palette, "Theme", "light parchment or warm dark", |ui| {
                segmented_toggle::render_theme(ui, palette, &mut state.selected_theme);
            });
        });
        columns[1].vertical(|ui| {
            settings_row(
                ui,
                palette,
                "Language",
                "language used across the BIO app",
                |ui| {
                    render_language_combo(ui, palette, &mut state.language);
                },
            );
        });
    });
    ui.add_space(REDESIGN_SETTINGS_ROW_GRID_GAP_Y_PX);

    ui.columns(2, |columns| {
        columns[0].vertical(|ui| {
            settings_row(
                ui,
                palette,
                "Validate all paths on startup",
                "warns if game folders moved",
                |ui| {
                    toggle_row::render(ui, palette, "", &mut state.validate_paths_on_startup, "");
                },
            );
        });
        columns[1].vertical(|ui| {
            settings_row(
                ui,
                palette,
                "Diagnostic mode",
                "extra logging for bug reports",
                |ui| {
                    toggle_row::render(ui, palette, "", &mut state.diagnostic_mode, "");
                },
            );
        });
    });

    before != state.current_redesign_settings()
}

fn settings_row(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    label: &str,
    hint: &str,
    control: impl FnOnce(&mut egui::Ui),
) {
    let row_width = ui.available_width();
    let response = egui::Frame::NONE
        .inner_margin(egui::Margin::symmetric(
            0,
            crate::ui::shared::redesign_tokens::redesign_i8_px(REDESIGN_SETTINGS_ROW_PADDING_Y_PX),
        ))
        .show(ui, |ui| {
            ui.set_width(row_width);
            ui.horizontal(|ui| {
                ui.set_width(row_width);
                let control_width = row_width * 0.42;
                let label_width = (row_width - control_width - REDESIGN_SETTINGS_ROW_GAP_PX)
                    .max(row_width * 0.45);

                ui.allocate_ui_with_layout(
                    egui::vec2(label_width, 0.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.label(
                            egui::RichText::new(label)
                                .size(REDESIGN_LABEL_FONT_SIZE_PX)
                                .color(redesign_text_primary(palette)),
                        );
                        ui.label(
                            egui::RichText::new(hint)
                                .size(REDESIGN_HINT_FONT_SIZE_PX)
                                .color(redesign_text_muted(palette)),
                        );
                    },
                );
                ui.add_space(REDESIGN_SETTINGS_ROW_GAP_PX);
                ui.allocate_ui_with_layout(
                    egui::vec2(control_width, 0.0),
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        control(ui);
                    },
                );
            });
        })
        .response;

    let rect = response.rect;
    ui.painter().line_segment(
        [rect.left_bottom(), rect.right_bottom()],
        egui::Stroke::new(
            REDESIGN_DASHED_BORDER_WIDTH_PX,
            redesign_border_dashed_light(palette),
        ),
    );
    ui.add_space(REDESIGN_SETTINGS_ROW_GRID_GAP_Y_PX);
}

fn render_language_combo(ui: &mut egui::Ui, palette: ThemePalette, language: &mut String) {
    egui::Frame::NONE
        .fill(redesign_input_bg(palette))
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_border_strong(palette),
        ))
        .corner_radius(REDESIGN_BORDER_RADIUS_PX)
        .inner_margin(egui::Margin::symmetric(
            crate::ui::shared::redesign_tokens::redesign_i8_px(REDESIGN_BUTTON_SMALL_PADDING_X_PX),
            crate::ui::shared::redesign_tokens::redesign_i8_px(REDESIGN_BUTTON_SMALL_PADDING_Y_PX),
        ))
        .show(ui, |ui| {
            egui::ComboBox::from_id_salt("settings_general_language")
                .selected_text(
                    egui::RichText::new(language.as_str()).color(redesign_text_primary(palette)),
                )
                .show_ui(ui, |ui| {
                    for option in LANGUAGES {
                        ui.selectable_value(language, option.to_string(), option);
                    }
                });
        });
}
