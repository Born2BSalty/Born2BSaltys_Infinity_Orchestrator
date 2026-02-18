// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::state::WizardState;
use crate::ui::step5::prompt_memory;
use crate::ui::step5::scripted_inputs;

pub(super) fn render(ui: &mut egui::Ui, state: &mut WizardState) {
    if !state.step3.prompt_setup_open {
        return;
    }

    let mut open = state.step3.prompt_setup_open;
    egui::Window::new("Step 3 Prompt Setup")
        .open(&mut open)
        .resizable(true)
        .default_size(egui::vec2(980.0, 440.0))
        .min_width(640.0)
        .show(ui.ctx(), |ui| {
            ui.label("Configure per-component scripted answers (comma-separated). Example: 126,,a,y");
            ui.add_space(6.0);

            let items = active_items(state);
            if ui
                .button("Import @wlb-inputs from active source logs")
                .on_hover_text("Imports existing @wlb-inputs lines from current Step 1 log sources into this table.")
                .clicked()
            {
                let imported = scripted_inputs::load_from_step1(&state.step1);
                let mut imported_count = 0usize;
                for item in &items {
                    let key = component_key(item);
                    if let Some(tokens) = imported.get(&key) {
                        let answer = tokens.join(",");
                        prompt_memory::upsert_component_sequence(
                            &key,
                            &item.tp_file,
                            &item.component_id,
                            &item.component_label,
                            &answer,
                            "step3_prompt_import",
                        );
                        imported_count = imported_count.saturating_add(1);
                    }
                }
                state.step5.last_status_text = format!("Prompt setup imported {imported_count} component sequence(s).");
            }
            ui.add_space(6.0);

            if items.is_empty() {
                ui.label("No selected components in this tab.");
                return;
            }

            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("step3_prompt_setup_grid")
                    .num_columns(3)
                    .spacing([10.0, 6.0])
                    .striped(true)
                    .show(ui, |ui| {
                        ui.strong("Component");
                        ui.strong("Key");
                        ui.strong("Answers");
                        ui.end_row();

                        for item in items {
                            let component_key = component_key(&item);
                            let mut answer = prompt_memory::get_component_sequence(&component_key)
                                .unwrap_or_default();

                            ui.label(format!("{} #{}", item.mod_name, item.component_id));
                            ui.monospace(component_key.as_str());

                            let response = ui.text_edit_singleline(&mut answer);
                            if response.changed() {
                                prompt_memory::upsert_component_sequence(
                                    &component_key,
                                    &item.tp_file,
                                    &item.component_id,
                                    &item.component_label,
                                    &answer,
                                    "step3_prompt_setup",
                                );
                            }
                            ui.end_row();
                        }
                    });
            });
        });

    state.step3.prompt_setup_open = open;
}

#[derive(Debug, Clone)]
struct PromptSetupItem {
    tp_file: String,
    component_id: String,
    component_label: String,
    mod_name: String,
}

fn active_items(state: &WizardState) -> Vec<PromptSetupItem> {
    let items = if state.step3.active_game_tab.eq_ignore_ascii_case("BG2EE") {
        &state.step3.bg2ee_items
    } else {
        &state.step3.bgee_items
    };
    items
        .iter()
        .filter(|i| !i.is_parent && !i.parent_placeholder)
        .map(|i| PromptSetupItem {
            tp_file: i.tp_file.clone(),
            component_id: i.component_id.clone(),
            component_label: i.component_label.clone(),
            mod_name: i.mod_name.clone(),
        })
        .collect()
}

fn component_key(item: &PromptSetupItem) -> String {
    let filename = normalize_tp2_filename(&item.tp_file);
    format!("{}#{}", filename, item.component_id.trim())
}

fn normalize_tp2_filename(tp_file: &str) -> String {
    let replaced = tp_file.replace('\\', "/");
    let filename = replaced
        .rsplit('/')
        .next()
        .unwrap_or(replaced.as_str())
        .trim();
    filename.to_ascii_uppercase()
}
