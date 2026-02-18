// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::step2::details::Step2Details;

use super::Step2Action;
use super::super::format::ellipsize_end;

pub(super) fn render_paths_grid(
    ui: &mut egui::Ui,
    details: &Step2Details,
    action: &mut Option<Step2Action>,
    label_w: f32,
    value_w: f32,
    row_h: f32,
    value_chars: usize,
) {
    ui.label(egui::RichText::new("Paths / Links").small().strong());
    egui::Grid::new("step2_details_paths_grid")
        .num_columns(3)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            let mut path_row = |ui: &mut egui::Ui,
                                label: &str,
                                value: Option<&str>,
                                missing_amber: bool,
                                open_action: Option<Step2Action>| {
                ui.add_sized(
                    [label_w, row_h],
                    egui::Label::new(egui::RichText::new(label).strong()),
                );
                let raw = value.unwrap_or("No data");
                let display = ellipsize_end(raw, value_chars);
                let mut text = egui::RichText::new(display).monospace();
                if value.is_none() && missing_amber {
                    text = text.color(egui::Color32::from_rgb(214, 168, 96));
                }
                ui.add_sized([value_w, row_h], egui::Label::new(text))
                    .on_hover_text(raw);
                if let Some(copy_value) = value {
                    ui.horizontal(|ui| {
                        if ui.small_button("C").on_hover_text("Copy").clicked() {
                            ui.ctx().copy_text(copy_value.to_string());
                        }
                        if ui.small_button("O").on_hover_text("Open").clicked()
                            && let Some(a) = open_action.clone()
                        {
                            *action = Some(a);
                        }
                    });
                } else {
                    ui.label("");
                }
                ui.end_row();
            };
            path_row(
                ui,
                "TP2 Path",
                details.tp2_path.as_deref(),
                true,
                details.tp2_path.clone().map(Step2Action::OpenSelectedTp2),
            );
            path_row(
                ui,
                "Readme",
                details.readme_path.as_deref(),
                true,
                details.readme_path.clone().map(Step2Action::OpenSelectedReadme),
            );
            path_row(
                ui,
                "Web",
                details.web_url.as_deref(),
                false,
                details.web_url.clone().map(Step2Action::OpenSelectedWeb),
            );
        });
}
