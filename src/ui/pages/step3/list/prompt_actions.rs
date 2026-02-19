// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;
use serde::{Deserialize, Serialize};

use crate::ui::state::WizardState;
use crate::ui::step5::prompt_memory;

use super::rows::PromptActionRequest;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AdvancedPromptEntry {
    key: String,
    alias: String,
    answer: String,
    enabled: bool,
    preview: String,
    component_key: String,
    tp2_file: String,
    component_id: String,
    component_name: String,
    prompt_kind: String,
    source: String,
    captured_at: u64,
    last_used_at: u64,
    hit_count: u64,
}

pub(super) fn apply_prompt_actions(state: &mut WizardState, requests: &[PromptActionRequest]) {
    for req in requests {
        match req {
            PromptActionRequest::SetWlb {
                tp_file,
                component_id,
                component_label,
                mod_name,
            } => {
                let component_key = component_key(tp_file, component_id);
                state.step3.prompt_edit_key = component_entry_key(&component_key);
                state.step3.prompt_edit_component_key = component_key.clone();
                state.step3.prompt_edit_tp2_file = normalize_tp2_filename(tp_file);
                state.step3.prompt_edit_component_id = component_id.trim().to_string();
                state.step3.prompt_edit_component_name = component_label.clone();
                state.step3.prompt_edit_mod_name = mod_name.clone();
                state.step3.prompt_edit_answer = prompt_memory::get_component_sequence(&component_key)
                    .unwrap_or_default();
                state.step3.prompt_edit_status.clear();
                state.step3.prompt_edit_mode = "wlb".to_string();
                state.step3.prompt_edit_open = true;
            }
            PromptActionRequest::EditJson {
                tp_file,
                component_id,
                component_label,
                mod_name,
            } => {
                let component_key = component_key(tp_file, component_id);
                let key = component_entry_key(&component_key);
                let existing = prompt_memory::list_entries()
                    .into_iter()
                    .find(|(k, _)| k == &key)
                    .map(|(_, v)| v)
                    .unwrap_or_else(|| default_entry(&component_key, tp_file, component_id, component_label));
                state.step3.prompt_edit_key = key.clone();
                state.step3.prompt_edit_component_key = component_key;
                state.step3.prompt_edit_tp2_file = normalize_tp2_filename(tp_file);
                state.step3.prompt_edit_component_id = component_id.trim().to_string();
                state.step3.prompt_edit_component_name = component_label.clone();
                state.step3.prompt_edit_mod_name = mod_name.clone();
                state.step3.prompt_edit_json = advanced_entry_to_json(&key, &existing).unwrap_or_default();
                state.step3.prompt_edit_status.clear();
                state.step3.prompt_edit_mode = "json".to_string();
                state.step3.prompt_edit_open = true;
            }
            PromptActionRequest::Clear {
                tp_file,
                component_id,
            } => {
                let key = component_entry_key(&component_key(tp_file, component_id));
                prompt_memory::delete_entry(&key);
                state.step5.last_status_text = format!("Prompt data cleared for {key}");
            }
        }
    }
}

pub(super) fn render(ui: &mut egui::Ui, state: &mut WizardState) {
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
                            "Answer is empty. Use Clear Prompt Data to remove this entry.".to_string();
                    } else {
                        prompt_memory::upsert_component_sequence(
                            &state.step3.prompt_edit_component_key,
                            &state.step3.prompt_edit_tp2_file,
                            &state.step3.prompt_edit_component_id,
                            &state.step3.prompt_edit_component_name,
                            &answer,
                            "step3_context_menu",
                        );
                        state.step5.last_status_text = format!(
                            "Saved @wlb-inputs for {} #{}",
                            state.step3.prompt_edit_mod_name, state.step3.prompt_edit_component_id
                        );
                        state.step3.prompt_edit_open = false;
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
                    match serde_json::from_str::<AdvancedPromptEntry>(&state.step3.prompt_edit_json) {
                        Ok(parsed) => {
                            if parsed.key.trim().is_empty() {
                                state.step3.prompt_edit_status = "Save failed: key is required".to_string();
                            } else {
                                prompt_memory::upsert_entry(
                                    parsed.key.trim(),
                                    prompt_memory::PromptAnswerEntry {
                                        alias: parsed.alias,
                                        answer: parsed.answer,
                                        enabled: parsed.enabled,
                                        preview: parsed.preview,
                                        component_key: parsed.component_key,
                                        tp2_file: parsed.tp2_file,
                                        component_id: parsed.component_id,
                                        component_name: parsed.component_name,
                                        prompt_kind: parsed.prompt_kind,
                                        source: parsed.source,
                                        captured_at: parsed.captured_at,
                                        last_used_at: parsed.last_used_at,
                                        hit_count: parsed.hit_count,
                                    },
                                );
                                state.step3.prompt_edit_status = "Saved.".to_string();
                            }
                        }
                        Err(err) => {
                            state.step3.prompt_edit_status = format!("Save failed: {err}");
                        }
                    }
                }
                if ui.button("Delete Entry").clicked() {
                    match serde_json::from_str::<AdvancedPromptEntry>(&state.step3.prompt_edit_json) {
                        Ok(parsed) => {
                            let key = parsed.key.trim().to_string();
                            if key.is_empty() {
                                state.step3.prompt_edit_status = "Delete failed: key is required".to_string();
                            } else {
                                prompt_memory::delete_entry(&key);
                                state.step3.prompt_edit_status = "Deleted.".to_string();
                            }
                        }
                        Err(err) => {
                            state.step3.prompt_edit_status = format!("Delete failed: {err}");
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

fn normalize_tp2_filename(tp_file: &str) -> String {
    let replaced = tp_file.replace('\\', "/");
    let filename = replaced
        .rsplit('/')
        .next()
        .unwrap_or(replaced.as_str())
        .trim();
    filename.to_ascii_uppercase()
}

fn component_key(tp_file: &str, component_id: &str) -> String {
    format!("{}#{}", normalize_tp2_filename(tp_file), component_id.trim())
}

fn component_entry_key(component_key: &str) -> String {
    format!("ENTRY:COMPONENT:{component_key}")
}

fn default_entry(
    component_key: &str,
    tp_file: &str,
    component_id: &str,
    component_name: &str,
) -> prompt_memory::PromptAnswerEntry {
    prompt_memory::PromptAnswerEntry {
        alias: String::new(),
        answer: String::new(),
        enabled: false,
        preview: String::new(),
        component_key: component_key.to_string(),
        tp2_file: normalize_tp2_filename(tp_file),
        component_id: component_id.trim().to_string(),
        component_name: component_name.to_string(),
        prompt_kind: "component_sequence".to_string(),
        source: "step3_context_menu".to_string(),
        captured_at: 0,
        last_used_at: 0,
        hit_count: 0,
    }
}

fn advanced_entry_to_json(key: &str, entry: &prompt_memory::PromptAnswerEntry) -> Option<String> {
    let data = AdvancedPromptEntry {
        key: key.to_string(),
        alias: entry.alias.clone(),
        answer: entry.answer.clone(),
        enabled: entry.enabled,
        preview: entry.preview.clone(),
        component_key: entry.component_key.clone(),
        tp2_file: entry.tp2_file.clone(),
        component_id: entry.component_id.clone(),
        component_name: entry.component_name.clone(),
        prompt_kind: entry.prompt_kind.clone(),
        source: entry.source.clone(),
        captured_at: entry.captured_at,
        last_used_at: entry.last_used_at,
        hit_count: entry.hit_count,
    };
    serde_json::to_string_pretty(&data).ok()
}
