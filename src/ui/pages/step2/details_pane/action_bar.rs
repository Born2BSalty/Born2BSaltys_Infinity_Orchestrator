// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::step2::details::Step2Details;

use super::Step2Action;

pub(super) fn render(
    ui: &mut egui::Ui,
    details: &Step2Details,
    action: &mut Option<Step2Action>,
) {
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add_enabled(
                    details.readme_path.is_some(),
                    egui::Button::new("Readme").min_size(egui::vec2(64.0, 24.0)),
                )
                .on_hover_text("Open the selected mod readme file.")
                .clicked()
                && let Some(path) = &details.readme_path
            {
                *action = Some(Step2Action::OpenSelectedReadme(path.clone()));
            }
            if ui
                .add_enabled(
                    details.web_url.is_some(),
                    egui::Button::new("Web").min_size(egui::vec2(52.0, 24.0)),
                )
                .on_hover_text("Open the selected mod website.")
                .clicked()
                && let Some(url) = &details.web_url
            {
                *action = Some(Step2Action::OpenSelectedWeb(url.clone()));
            }
            if ui
                .add_enabled(
                    details.tp2_path.is_some(),
                    egui::Button::new("TP2").min_size(egui::vec2(52.0, 24.0)),
                )
                .on_hover_text("Open the selected TP2 file/folder.")
                .clicked()
                && let Some(path) = &details.tp2_path
            {
                *action = Some(Step2Action::OpenSelectedTp2(path.clone()));
            }
        });
    });
}
