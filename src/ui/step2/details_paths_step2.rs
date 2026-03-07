// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::step2::action_step2::Step2Action;
use crate::ui::step2::state_step2::Step2Details;

pub(crate) fn render_paths_grid(
    ui: &mut egui::Ui,
    details: &Step2Details,
    action: &mut Option<Step2Action>,
    label_w: f32,
    value_w: f32,
    row_h: f32,
    value_chars: usize,
) {
    ui.label(crate::ui::shared::typography_global::small_strong("Paths / Links"));
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
                    egui::Label::new(crate::ui::shared::typography_global::strong(label)),
                );
                let raw = value.unwrap_or("No data");
                let display = ellipsize_end(raw, value_chars);
                let mut text = crate::ui::shared::typography_global::monospace(display);
                if value.is_none() && missing_amber {
                    text = text.color(crate::ui::shared::theme_global::warning());
                }
                ui.add_sized([value_w, row_h], egui::Label::new(text))
                    .on_hover_text(raw);
                if let Some(copy_value) = value {
                    ui.horizontal(|ui| {
                        if ui
                            .small_button("C")
                            .on_hover_text(crate::ui::shared::tooltip_global::COPY)
                            .clicked()
                        {
                            ui.ctx().copy_text(copy_value.to_string());
                        }
                        if ui
                            .small_button("O")
                            .on_hover_text(crate::ui::shared::tooltip_global::OPEN)
                            .clicked()
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

pub(crate) fn render_raw_line(ui: &mut egui::Ui, details: &Step2Details) {
    egui::CollapsingHeader::new("Raw line")
        .default_open(false)
        .show(ui, |ui| {
            let raw = details.raw_line.as_deref().unwrap_or("No data");
            ui.add(egui::Label::new(crate::ui::shared::typography_global::monospace(raw)).wrap());
        });
}

fn ellipsize_end(value: &str, max_chars: usize) -> String {
    let count = value.chars().count();
    if count <= max_chars {
        return value.to_string();
    }
    let keep = max_chars.saturating_sub(3);
    let prefix: String = value.chars().take(keep).collect();
    format!("{prefix}...")
}
