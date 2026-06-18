// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::settings::state_settings::PathStatus;
use crate::ui::settings::validate_now::{
    FIELD_BG2EE_GAME_FOLDER, FIELD_BGEE_GAME_FOLDER, FIELD_IWDEE_GAME_FOLDER,
};
use crate::ui::shared::redesign_tokens::{
    ThemePalette, redesign_text_faint, redesign_text_primary,
};

const GAME_ROWS: [(&str, &str); 3] = [
    ("BGEE", FIELD_BGEE_GAME_FOLDER),
    ("BG2EE", FIELD_BG2EE_GAME_FOLDER),
    ("IWDEE", FIELD_IWDEE_GAME_FOLDER),
];

pub fn render(ui: &mut egui::Ui, palette: ThemePalette, orchestrator: &OrchestratorApp) {
    let report = &orchestrator.settings_screen_state.path_validation_results;

    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 4.0;
        for (name, field) in &GAME_ROWS {
            let found = matches!(report.fields.get(*field), Some(PathStatus::Ok { .. }));
            let (marker, text, color) = if found {
                ("\u{2713}", name.to_string(), redesign_text_primary(palette))
            } else {
                (
                    "?",
                    format!("{name} \u{00B7} not found"),
                    redesign_text_faint(palette),
                )
            };
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 5.0;
                ui.label(
                    egui::RichText::new(marker)
                        .size(14.0)
                        .family(egui::FontFamily::Name("firacode_nerd".into()))
                        .color(color),
                );
                ui.label(
                    egui::RichText::new(text)
                        .size(14.0)
                        .family(egui::FontFamily::Name("poppins_medium".into()))
                        .color(color),
                );
            });
        }
    });
}
