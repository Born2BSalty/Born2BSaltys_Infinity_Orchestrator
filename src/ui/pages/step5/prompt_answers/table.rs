// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::step5::prompt_memory;

mod grouping;
mod labels;
mod rows;

pub(super) fn render(ui: &mut egui::Ui) {
    let entries = prompt_memory::list_entries();
    if entries.is_empty() {
        ui.label(egui::RichText::new("No saved prompt answers yet.").weak());
        return;
    }

    let groups = grouping::group_entries(entries);
    egui::ScrollArea::vertical()
        .id_salt("step5_prompt_answers_scroll")
        .show(ui, |ui| {
            for group in groups {
                egui::CollapsingHeader::new(format!("{} ({})", group.label, group.items.len()))
                    .id_salt(("step5_prompt_answers_group", group.label.as_str()))
                    .default_open(false)
                    .show(ui, |ui| {
                        egui::Grid::new(("step5_prompt_answers_grid", group.label.as_str()))
                            .num_columns(5)
                            .striped(true)
                            .spacing([10.0, 8.0])
                            .show(ui, |ui| {
                                ui.strong("Auto");
                                ui.strong("Alias");
                                ui.strong("Preview");
                                ui.strong("Answer");
                                ui.strong("Action");
                                ui.end_row();

                                for (key, entry) in group.items {
                                    rows::render_entry_row(ui, &key, entry);
                                }
                            });
                    });
                ui.add_space(6.0);
            }
        });
}
