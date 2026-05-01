// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::WizardState;
use crate::app::step3_prompt_edit::{self, PromptActionRequest};

pub(crate) fn apply_prompt_actions(state: &mut WizardState, requests: &[PromptActionRequest]) {
    step3_prompt_edit::apply_prompt_actions(state, requests);
}

pub(crate) fn render(ui: &mut egui::Ui, state: &mut WizardState) {
    if !state.step3.prompt_edit_open {
        return;
    }
    if state.step3.prompt_edit_mode == "json" {
        render_json_editor(ui, state);
    } else {
        render_wlb_editor(ui, state);
    }
}

fn render_wlb_editor(ui: &mut egui::Ui, state: &mut WizardState) {
    let mut open = state.step3.prompt_edit_open;
    egui::Window::new("Set @wlb-inputs")
        .open(&mut open)
        .resizable(false)
        .show(ui.ctx(), |ui| {
            ui.label(format!(
                "{} #{}",
                state.step3.prompt_edit_mod_name, state.step3.prompt_edit_component_id
            ));
            ui.label(state.step3.prompt_edit_component_name.clone());
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label("Answer");
                ui.add(
                    egui::TextEdit::singleline(&mut state.step3.prompt_edit_answer)
                        .desired_width(360.0),
                );
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    let answer = state.step3.prompt_edit_answer.trim().to_string();
                    if answer.is_empty() {
                        state.step3.prompt_edit_status =
                            "Answer is empty. Use Clear Prompt Data to remove this entry."
                                .to_string();
                    } else {
                        match step3_prompt_edit::save_wlb_answer(state, &answer) {
                            Ok(()) => {}
                            Err(err) => {
                                state.step3.prompt_edit_status = err;
                            }
                        }
                    }
                }
                if ui.button("Cancel").clicked() {
                    state.step3.prompt_edit_open = false;
                }
            });
            if !state.step3.prompt_edit_status.trim().is_empty() {
                ui.label(state.step3.prompt_edit_status.clone());
            }
        });
    state.step3.prompt_edit_open = open && state.step3.prompt_edit_open;
}

fn render_json_editor(ui: &mut egui::Ui, state: &mut WizardState) {
    let mut open = state.step3.prompt_edit_open;
    egui::Window::new("Prompt Entry JSON")
        .open(&mut open)
        .resizable(true)
        .default_size(egui::vec2(760.0, 520.0))
        .show(ui.ctx(), |ui| {
            ui.set_min_size(ui.available_size());
            ui.label(format!(
                "{} #{}",
                state.step3.prompt_edit_mod_name, state.step3.prompt_edit_component_id
            ));
            ui.add_space(6.0);
            ui.add(
                egui::TextEdit::multiline(&mut state.step3.prompt_edit_json)
                    .desired_rows(22)
                    .desired_width(f32::INFINITY),
            );
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.button("Save JSON").clicked() {
                    let prompt_json = state.step3.prompt_edit_json.clone();
                    match step3_prompt_edit::save_json_entry(state, &prompt_json) {
                        Ok(message) => {
                            state.step3.prompt_edit_status = message;
                        }
                        Err(err) => {
                            state.step3.prompt_edit_status = format!("Save failed: {err}");
                        }
                    }
                }
                if ui.button("Delete Entry").clicked() {
                    match step3_prompt_edit::delete_prompt_data_from_editor(state) {
                        Ok(message) => {
                            state.step3.prompt_edit_status = message;
                        }
                        Err(err) => {
                            state.step3.prompt_edit_status = err;
                        }
                    }
                }
                if ui.button("Close").clicked() {
                    state.step3.prompt_edit_open = false;
                }
            });
            if !state.step3.prompt_edit_status.trim().is_empty() {
                ui.label(state.step3.prompt_edit_status.clone());
            }
        });
    state.step3.prompt_edit_open = open && state.step3.prompt_edit_open;
}
