// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;
use serde::{Deserialize, Serialize};

use crate::ui::state::{Step3ItemState, WizardState};
use crate::ui::step3::access;

use super::rows::PromptActionRequest;

const WLB_MARKER: &str = "@wlb-inputs:";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AdvancedPromptEntry {
    tp2_file: String,
    component_id: String,
    component_name: String,
    mod_name: String,
    answer: String,
    raw_line: String,
}

pub(super) fn apply_prompt_actions(state: &mut WizardState, requests: &[PromptActionRequest]) {
    for req in requests {
        match req {
            PromptActionRequest::SetWlb {
                tp_file,
                component_id,
                component_label,
                mod_name,
            } => open_wlb_editor(state, tp_file, component_id, component_label, mod_name),
            PromptActionRequest::EditJson {
                tp_file,
                component_id,
                component_label,
                mod_name,
            } => open_json_editor(state, tp_file, component_id, component_label, mod_name),
            PromptActionRequest::Clear {
                tp_file,
                component_id,
            } => clear_prompt_data(state, tp_file, component_id),
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

fn open_wlb_editor(
    state: &mut WizardState,
    tp_file: &str,
    component_id: &str,
    component_label: &str,
    mod_name: &str,
) {
    let component_key = component_key(tp_file, component_id);
    state.step3.prompt_edit_key = component_key.clone();
    state.step3.prompt_edit_component_key = component_key;
    state.step3.prompt_edit_tp2_file = normalize_tp2_filename(tp_file);
    state.step3.prompt_edit_component_id = component_id.trim().to_string();
    state.step3.prompt_edit_component_name = component_label.to_string();
    state.step3.prompt_edit_mod_name = mod_name.to_string();

    state.step3.prompt_edit_answer = find_component_mut(state, tp_file, component_id)
        .and_then(|item| extract_wlb_inputs(&item.raw_line))
        .unwrap_or_default();

    state.step3.prompt_edit_status.clear();
    state.step3.prompt_edit_mode = "wlb".to_string();
    state.step3.prompt_edit_open = true;
}

fn open_json_editor(
    state: &mut WizardState,
    tp_file: &str,
    component_id: &str,
    component_label: &str,
    mod_name: &str,
) {
    let component_key = component_key(tp_file, component_id);
    state.step3.prompt_edit_key = component_key.clone();
    state.step3.prompt_edit_component_key = component_key;
    state.step3.prompt_edit_tp2_file = normalize_tp2_filename(tp_file);
    state.step3.prompt_edit_component_id = component_id.trim().to_string();
    state.step3.prompt_edit_component_name = component_label.to_string();
    state.step3.prompt_edit_mod_name = mod_name.to_string();

    let (answer, raw_line) = if let Some(item) = find_component_mut(state, tp_file, component_id) {
        (
            extract_wlb_inputs(&item.raw_line).unwrap_or_default(),
            effective_raw_line(item),
        )
    } else {
        (String::new(), String::new())
    };

    let entry = AdvancedPromptEntry {
        tp2_file: normalize_tp2_filename(tp_file),
        component_id: component_id.trim().to_string(),
        component_name: component_label.to_string(),
        mod_name: mod_name.to_string(),
        answer,
        raw_line,
    };

    state.step3.prompt_edit_json = serde_json::to_string_pretty(&entry).unwrap_or_default();
    state.step3.prompt_edit_status.clear();
    state.step3.prompt_edit_mode = "json".to_string();
    state.step3.prompt_edit_open = true;
}

fn clear_prompt_data(state: &mut WizardState, tp_file: &str, component_id: &str) {
    if let Some(item) = find_component_mut(state, tp_file, component_id) {
        item.raw_line = strip_wlb_marker(&effective_raw_line(item));
        state.step5.last_status_text = format!(
            "Cleared @wlb-inputs for {} #{}",
            item.mod_name, item.component_id
        );
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
                ui.add(egui::TextEdit::singleline(&mut state.step3.prompt_edit_answer).desired_width(360.0));
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    let answer = state.step3.prompt_edit_answer.trim().to_string();
                    if answer.is_empty() {
                        state.step3.prompt_edit_status =
                            "Answer is empty. Use Clear Prompt Data to remove this entry.".to_string();
                    } else {
                        let tp2_file = state.step3.prompt_edit_tp2_file.clone();
                        let component_id = state.step3.prompt_edit_component_id.clone();
                        if let Some(item) = find_component_mut(state, &tp2_file, &component_id) {
                        item.raw_line = set_wlb_inputs(&effective_raw_line(item), &answer);
                        state.step5.last_status_text = format!(
                            "Saved @wlb-inputs on weidu line for {} #{}",
                            item.mod_name, item.component_id
                        );
                            state.step3.prompt_edit_open = false;
                        } else {
                            state.step3.prompt_edit_status =
                                "Save failed: component not found in current Step 3 tab.".to_string();
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
                            if let Some(item) =
                                find_component_mut(state, &parsed.tp2_file, &parsed.component_id)
                            {
                                if !parsed.raw_line.trim().is_empty() {
                                    item.raw_line = parsed.raw_line.trim().to_string();
                                } else if !parsed.answer.trim().is_empty() {
                                    item.raw_line =
                                        set_wlb_inputs(&effective_raw_line(item), parsed.answer.trim());
                                } else {
                                    item.raw_line = strip_wlb_marker(&effective_raw_line(item));
                                }
                                state.step3.prompt_edit_status = "Saved to weidu line.".to_string();
                            } else {
                                state.step3.prompt_edit_status =
                                    "Save failed: component not found in current Step 3 tab."
                                        .to_string();
                            }
                        }
                        Err(err) => {
                            state.step3.prompt_edit_status = format!("Save failed: {err}");
                        }
                    }
                }
                if ui.button("Delete Entry").clicked() {
                    let tp2_file = state.step3.prompt_edit_tp2_file.clone();
                    let component_id = state.step3.prompt_edit_component_id.clone();
                    if let Some(item) = find_component_mut(state, &tp2_file, &component_id) {
                        item.raw_line = strip_wlb_marker(&effective_raw_line(item));
                        state.step3.prompt_edit_status = "Deleted from weidu line.".to_string();
                    } else {
                        state.step3.prompt_edit_status =
                            "Delete failed: component not found in current Step 3 tab."
                                .to_string();
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

fn find_component_mut<'a>(
    state: &'a mut WizardState,
    tp_file: &str,
    component_id: &str,
) -> Option<&'a mut Step3ItemState> {
    let tp_norm = normalize_tp2_filename(tp_file);
    let id_norm = component_id.trim();
    let (items, _, _, _, _, _, _, _, _, _, _, _, _, _, _) = access::active_list_mut(state);
    items.iter_mut().find(|i| {
        !i.is_parent
            && normalize_tp2_filename(&i.tp_file) == tp_norm
            && i.component_id.trim() == id_norm
    })
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

fn default_raw_line(item: &Step3ItemState) -> String {
    let folder = item.mod_name.replace('/', "\\");
    format!(
        "~{}\\{}~ #0 #{} // {}",
        folder, item.tp_file, item.component_id, item.component_label
    )
}

fn effective_raw_line(item: &Step3ItemState) -> String {
    if item.raw_line.trim().is_empty() {
        default_raw_line(item)
    } else {
        item.raw_line.trim().to_string()
    }
}

fn extract_wlb_inputs(raw_line: &str) -> Option<String> {
    let lower = raw_line.to_ascii_lowercase();
    let marker = WLB_MARKER;
    let start = lower.find(marker)?;
    let value = raw_line[start + marker.len()..].trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn strip_wlb_marker(raw_line: &str) -> String {
    let lower = raw_line.to_ascii_lowercase();
    let marker = WLB_MARKER;
    if let Some(start) = lower.find(marker) {
        let mut head = raw_line[..start].to_string();
        while head.ends_with(' ') || head.ends_with('\t') {
            head.pop();
        }
        if head.ends_with("//") {
            head.truncate(head.len().saturating_sub(2));
            while head.ends_with(' ') || head.ends_with('\t') {
                head.pop();
            }
        }
        head
    } else {
        raw_line.trim().to_string()
    }
}

fn set_wlb_inputs(raw_line: &str, answer: &str) -> String {
    let base = strip_wlb_marker(raw_line);
    if answer.trim().is_empty() {
        return base;
    }
    format!("{base} // @wlb-inputs: {}", answer.trim())
}
