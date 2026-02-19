// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;

use crate::ui::state::WizardState;
use crate::ui::state::Step2ModState;
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
            if state.step3.prompt_setup_scan_results.is_empty() && !items.is_empty() {
                state.step3.prompt_setup_scan_results = scan_selected_prompts(state, &items);
            }
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Rows").strong());
                for (id, label) in [
                    ("likely", "Likely prompts"),
                    ("configured", "Configured only"),
                    ("all", "All selected"),
                ] {
                    let selected = state.step3.prompt_setup_filter.eq_ignore_ascii_case(id);
                    if ui.selectable_label(selected, label).clicked() {
                        state.step3.prompt_setup_filter = id.to_string();
                    }
                }
                if ui.button("Rescan TP2 prompts").clicked() {
                    state.step3.prompt_setup_scan_results = scan_selected_prompts(state, &items);
                    state.step5.last_status_text = format!(
                        "Prompt scan detected {} likely prompt component(s).",
                        state.step3.prompt_setup_scan_results.len()
                    );
                }
            });
            ui.add_space(6.0);

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
            ui.label(format!("Selected components in tab: {}", items.len()));
            ui.add_space(6.0);

            let scan_map: BTreeMap<String, String> = state
                .step3
                .prompt_setup_scan_results
                .iter()
                .cloned()
                .collect();

            let mut open_advanced: Option<(String, String)> = None;
            ui.scope(|ui| {
                let mut scroll = egui::style::ScrollStyle::solid();
                scroll.bar_width = 12.0;
                scroll.bar_inner_margin = 0.0;
                scroll.bar_outer_margin = 2.0;
                ui.style_mut().spacing.scroll = scroll;
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                let entries = prompt_memory::list_entries();
                let filter_mode = state.step3.prompt_setup_filter.clone();
                let grouped = group_by_mod(
                    items
                        .iter()
                        .filter(|item| row_visible(item, &filter_mode, &entries, &scan_map))
                        .cloned()
                        .collect(),
                );
                for (mod_name, mod_items) in grouped {
                    egui::CollapsingHeader::new(format!("{mod_name} ({})", mod_items.len()))
                        .id_salt(("step3_prompt_setup_mod", mod_name.as_str()))
                        .default_open(false)
                        .show(ui, |ui| {
                            egui::Grid::new(("step3_prompt_setup_grid", mod_name.as_str()))
                                .num_columns(5)
                                .spacing([10.0, 6.0])
                                .striped(true)
                                .show(ui, |ui| {
                                    ui.strong("Auto");
                                    ui.strong("Component");
                                    ui.strong("Prompt");
                                    ui.strong("Answers");
                                    ui.strong("...");
                                    ui.end_row();

                                    for item in mod_items {
                                        let component_key = component_key(&item);
                                        let entry_key = component_entry_key(&component_key);
                                        let existing = entries
                                            .iter()
                                            .find(|(k, _)| k == &entry_key)
                                            .map(|(_, v)| v.clone());
                                        let mut enabled = existing.as_ref().map(|e| e.enabled).unwrap_or(false);
                                        let alias = existing
                                            .as_ref()
                                            .map(|e| e.alias.clone())
                                            .unwrap_or_default();
                                        let prompt_preview = existing
                                            .as_ref()
                                            .map(|e| e.preview.clone())
                                            .filter(|v| !v.trim().is_empty())
                                            .or_else(|| scan_map.get(&component_key).cloned())
                                            .or_else(|| {
                                                existing
                                                    .as_ref()
                                                    .map(|e| e.alias.clone())
                                                    .filter(|v| !v.trim().is_empty())
                                            })
                                            .unwrap_or_else(|| item.component_label.clone());
                                        let mut answer = existing
                                            .as_ref()
                                            .map(|e| e.answer.clone())
                                            .filter(|v| !v.trim().is_empty())
                                            .or_else(|| prompt_memory::get_component_sequence(&component_key))
                                            .unwrap_or_default();

                                        let enabled_changed = ui.checkbox(&mut enabled, "").changed();

                                        ui.label(format!("{}  #{}", item.mod_name, item.component_id));
                                        let prompt_short: String = if prompt_preview.chars().count() > 68 {
                                            let mut s: String = prompt_preview.chars().take(68).collect();
                                            s.push_str("...");
                                            s
                                        } else {
                                            prompt_preview.clone()
                                        };
                                        ui.add_sized(
                                            egui::vec2(338.0, 0.0),
                                            egui::Label::new(prompt_short),
                                        )
                                        .on_hover_text(prompt_preview.as_str());

                                        let answer_changed = ui
                                            .add_sized(
                                                egui::vec2(220.0, 0.0),
                                                egui::TextEdit::singleline(&mut answer),
                                            )
                                            .changed();
                                        if answer_changed || enabled_changed {
                                            prompt_memory::upsert_component_sequence(
                                                &component_key,
                                                &item.tp_file,
                                                &item.component_id,
                                                &item.component_label,
                                                &answer,
                                                "step3_prompt_setup",
                                            );
                                            prompt_memory::set_enabled(&entry_key, enabled);
                                        }
                                        if ui.button("...").clicked() {
                                            let entry = existing
                                                .clone()
                                                .unwrap_or_else(|| default_advanced_entry(&component_key, &item, &answer, enabled, &alias, &prompt_preview));
                                            if let Some(json) = advanced_entry_to_json(&entry_key, &entry) {
                                                open_advanced = Some((entry_key.clone(), json));
                                            }
                                        }
                                        ui.end_row();
                                    }
                                });
                        });
                    ui.add_space(4.0);
                }
                });
            });

            if let Some((key, json)) = open_advanced {
                state.step3.prompt_setup_advanced_key = key;
                state.step3.prompt_setup_advanced_json = json;
                state.step3.prompt_setup_advanced_status.clear();
                state.step3.prompt_setup_advanced_open = true;
            }
        });

    state.step3.prompt_setup_open = open;
    render_advanced_editor(ui, state);
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

fn group_by_mod(items: Vec<PromptSetupItem>) -> Vec<(String, Vec<PromptSetupItem>)> {
    let mut grouped: BTreeMap<String, Vec<PromptSetupItem>> = BTreeMap::new();
    for item in items {
        grouped.entry(item.mod_name.clone()).or_default().push(item);
    }
    grouped.into_iter().collect()
}

fn component_key(item: &PromptSetupItem) -> String {
    let filename = normalize_tp2_filename(&item.tp_file);
    format!("{}#{}", filename, item.component_id.trim())
}

fn component_entry_key(component_key: &str) -> String {
    format!("ENTRY:COMPONENT:{component_key}")
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

fn row_visible(
    item: &PromptSetupItem,
    filter_mode: &str,
    entries: &[(String, prompt_memory::PromptAnswerEntry)],
    scan_map: &BTreeMap<String, String>,
) -> bool {
    let component_key = component_key(item);
    let entry_key = component_entry_key(&component_key);
    let existing = entries
        .iter()
        .find(|(k, _)| k == &entry_key)
        .map(|(_, v)| v);
    let configured = existing
        .map(|e| e.enabled || !e.answer.trim().is_empty())
        .unwrap_or(false);
    let likely = scan_map.contains_key(&component_key) || configured;
    match filter_mode.to_ascii_lowercase().as_str() {
        "configured" => configured,
        "all" => true,
        _ => likely,
    }
}

fn scan_selected_prompts(state: &WizardState, items: &[PromptSetupItem]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let index = step2_component_tp2_index(state);
    for item in items {
        let key = component_key(item);
        let Some(tp2_path) = index.get(&key) else {
            continue;
        };
        let Ok(content) = fs::read_to_string(tp2_path) else {
            continue;
        };
        if let Some(preview) = scan_tp2_component_prompt(&content, item.component_id.trim()) {
            out.push((key, preview));
        }
    }
    out
}

fn step2_component_tp2_index(state: &WizardState) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let mods = if state.step3.active_game_tab.eq_ignore_ascii_case("BG2EE") {
        &state.step2.bg2ee_mods
    } else {
        &state.step2.bgee_mods
    };
    index_step2_mods(mods, &mut out);
    out
}

fn index_step2_mods(mods: &[Step2ModState], out: &mut BTreeMap<String, String>) {
    for mod_state in mods {
        let file = normalize_tp2_filename(&mod_state.tp_file);
        for component in &mod_state.components {
            let key = format!("{}#{}", file, component.component_id.trim());
            out.entry(key).or_insert_with(|| mod_state.tp2_path.clone());
        }
    }
}

fn scan_tp2_component_prompt(tp2_content: &str, component_id: &str) -> Option<String> {
    let block = extract_component_block(tp2_content, component_id)?;
    let mut fallback_print: Option<String> = None;
    for line in block {
        let upper = line.to_ascii_uppercase();
        if upper.contains("READLN") || upper.contains("ACTION_READLN") {
            if let Some(text) = extract_inline_prompt_text(line) {
                return Some(text);
            }
            return Some("READLN prompt detected".to_string());
        }
        if upper.contains("ASK_EVERY_COMPONENT") {
            return Some("ASK_EVERY_COMPONENT prompt detected".to_string());
        }
        if (upper.contains("[Y]") && upper.contains("[N]")) || upper.contains("PLEASE CHOOSE ONE OF THE FOLLOWING")
        {
            if let Some(text) = extract_inline_prompt_text(line) {
                return Some(text);
            }
            return Some("Choice prompt detected".to_string());
        }
        if fallback_print.is_none()
            && (upper.contains("PRINT ") || upper.contains("SAY "))
            && let Some(text) = extract_inline_prompt_text(line)
        {
            fallback_print = Some(text);
        }
    }
    fallback_print
}

fn extract_component_block<'a>(content: &'a str, component_id: &str) -> Option<Vec<&'a str>> {
    let mut in_target = false;
    let mut lines_after_begin = 0usize;
    let mut out = Vec::new();
    let designated_token = format!("DESIGNATED {}", component_id);
    for line in content.lines() {
        let trimmed = line.trim_start();
        let upper = trimmed.to_ascii_uppercase();
        if upper.starts_with("BEGIN ") {
            if in_target {
                break;
            }
            in_target = upper.contains(&designated_token);
            lines_after_begin = 0;
            if in_target {
                out.push(line);
            }
            continue;
        }
        if in_target {
            out.push(line);
            continue;
        }
        if lines_after_begin < 8 {
            lines_after_begin = lines_after_begin.saturating_add(1);
            if upper.contains(&designated_token) {
                in_target = true;
                out.push(line);
            }
        }
    }
    if out.is_empty() { None } else { Some(out) }
}

fn extract_inline_prompt_text(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if let Some(start) = trimmed.find('~') {
        let rest = &trimmed[start + 1..];
        if let Some(end) = rest.find('~') {
            let text = rest[..end].trim();
            if !text.is_empty() {
                return Some(text.to_string());
            }
        }
    }
    if let Some(start) = trimmed.find('"') {
        let rest = &trimmed[start + 1..];
        if let Some(end) = rest.find('"') {
            let text = rest[..end].trim();
            if !text.is_empty() {
                return Some(text.to_string());
            }
        }
    }
    None
}

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

fn default_advanced_entry(
    component_key: &str,
    item: &PromptSetupItem,
    answer: &str,
    enabled: bool,
    alias: &str,
    preview: &str,
) -> prompt_memory::PromptAnswerEntry {
    prompt_memory::PromptAnswerEntry {
        alias: alias.to_string(),
        answer: answer.to_string(),
        enabled,
        preview: preview.to_string(),
        component_key: component_key.to_string(),
        tp2_file: normalize_tp2_filename(&item.tp_file),
        component_id: item.component_id.clone(),
        component_name: item.component_label.clone(),
        prompt_kind: String::new(),
        source: "step3_prompt_setup".to_string(),
        captured_at: 0,
        last_used_at: 0,
        hit_count: 0,
    }
}

fn render_advanced_editor(ui: &mut egui::Ui, state: &mut WizardState) {
    if !state.step3.prompt_setup_advanced_open {
        return;
    }
    let mut open = state.step3.prompt_setup_advanced_open;
    egui::Window::new("Prompt Entry Advanced Editor")
        .open(&mut open)
        .resizable(true)
        .default_size(egui::vec2(760.0, 520.0))
        .show(ui.ctx(), |ui| {
            ui.label("Edit full prompt entry JSON (advanced users).");
            ui.add_space(6.0);
            ui.add(
                egui::TextEdit::multiline(&mut state.step3.prompt_setup_advanced_json)
                    .desired_rows(22)
                    .desired_width(f32::INFINITY),
            );
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.button("Save JSON").clicked() {
                    match serde_json::from_str::<AdvancedPromptEntry>(
                        &state.step3.prompt_setup_advanced_json,
                    ) {
                        Ok(parsed) => {
                            let key = parsed.key.trim().to_string();
                            if key.is_empty() {
                                state.step3.prompt_setup_advanced_status =
                                    "Save failed: key is required".to_string();
                            } else {
                                prompt_memory::upsert_entry(
                                    &key,
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
                                state.step3.prompt_setup_advanced_status =
                                    "Saved.".to_string();
                            }
                        }
                        Err(err) => {
                            state.step3.prompt_setup_advanced_status =
                                format!("Save failed: {err}");
                        }
                    }
                }
                if ui.button("Close").clicked() {
                    state.step3.prompt_setup_advanced_open = false;
                }
            });
            if !state.step3.prompt_setup_advanced_status.trim().is_empty() {
                ui.label(state.step3.prompt_setup_advanced_status.clone());
            }
        });
    state.step3.prompt_setup_advanced_open = open && state.step3.prompt_setup_advanced_open;
}
