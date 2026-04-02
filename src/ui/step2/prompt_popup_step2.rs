// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;
use std::collections::BTreeMap;

use crate::ui::shared::layout_tokens_global::{SPACE_MD, SPACE_SM, SPACE_XS};
use crate::ui::state::{PromptPopupMode, Step2ModState, Step3ItemState, WizardState};
use crate::ui::step2::service_selection_step2::{
    jump_to_target, selection_normalize_mod_key,
};
use crate::ui::step2::state_step2::build_prompt_eval_context;
use crate::ui::step3::prompt_popup_step3::evaluate_step3_item_prompt_summary;
use crate::ui::step3::state_step3;

#[derive(Clone)]
pub(crate) struct PromptToolbarModEntry {
    pub(crate) mod_name: String,
    pub(crate) tp_file: String,
    pub(crate) component_ids: Vec<u32>,
}

pub fn render_prompt_popup(ui: &mut egui::Ui, state: &mut WizardState) {
    if !state.step2.prompt_popup_open {
        return;
    }
    if state.step2.prompt_popup_mode == PromptPopupMode::ToolbarIndex {
        render_prompt_toolbar_popup(ui, state);
        return;
    }
    let title = state.step2.prompt_popup_title.clone();
    let text = state.step2.prompt_popup_text.clone();
    let jump_ids = collect_prompt_jump_component_ids(active_mods_ref(state), &title, &text);
    let mut open = state.step2.prompt_popup_open;
    let mut jump_to_component_id: Option<u32> = None;
    egui::Window::new(format!("Parsed prompts - {}", title))
        .open(&mut open)
        .resizable(true)
        .collapsible(false)
        .default_width(700.0)
        .default_height(320.0)
        .show(ui.ctx(), |ui| {
            ui.label("Prompt summary from Lapdu parser:");
            ui.separator();
            let max_scroll_height = (ui.available_height() - 72.0).max(140.0);
            let scroll_width = ui.available_width();
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .max_height(max_scroll_height)
                .show(ui, |ui| {
                    ui.set_min_width(scroll_width);
                    ui.label(&text);
                });
            if !jump_ids.is_empty() {
                ui.add_space(SPACE_MD);
                ui.separator();
                ui.add_space(SPACE_SM);
                ui.label(crate::ui::shared::typography_global::strong("Jump to component"));
                ui.add_space(SPACE_XS);
                ui.horizontal_wrapped(|ui| {
                    for component_id in jump_ids {
                        let button_text =
                            crate::ui::shared::typography_global::monospace(component_id.to_string())
                                .color(crate::ui::shared::theme_global::accent_numbers());
                        if ui
                            .add(
                                egui::Button::new(button_text)
                                    .min_size(egui::vec2(42.0, 22.0))
                                    .fill(ui.visuals().widgets.inactive.bg_fill)
                                    .stroke(ui.visuals().widgets.inactive.bg_stroke),
                            )
                            .clicked()
                        {
                            jump_to_component_id = Some(component_id);
                        }
                    }
                });
            }
        });
    state.step2.prompt_popup_open = open;
    if let Some(component_id) = jump_to_component_id {
        let game_tab = state.step2.active_game_tab.clone();
        let mod_ref = parse_prompt_popup_mod_ref(&title);
        jump_to_target(state, &game_tab, &mod_ref, Some(component_id));
        state.step2.jump_to_selected_requested = true;
    }
}

pub(crate) fn open_text_prompt_popup(state: &mut WizardState, title: String, text: String) {
    state.step2.prompt_popup_mode = PromptPopupMode::Text;
    state.step2.prompt_popup_title = title;
    state.step2.prompt_popup_text = text;
    state.step2.prompt_popup_open = true;
}

pub(crate) fn open_toolbar_prompt_popup(state: &mut WizardState, title: &str) {
    state.step2.prompt_popup_mode = PromptPopupMode::ToolbarIndex;
    state.step2.prompt_popup_title = title.to_string();
    state.step2.prompt_popup_text.clear();
    state.step2.prompt_popup_open = true;
}

pub(crate) fn draw_prompt_toolbar_badge(ui: &mut egui::Ui, count: usize) -> bool {
    if count == 0 {
        return false;
    }
    let prompt_text = crate::ui::shared::typography_global::strong(format!("PROMPT {count}"))
        .color(crate::ui::shared::theme_global::prompt_text())
        .size(crate::ui::shared::typography_global::SIZE_PILL_TEXT);
    ui.add(
        egui::Button::new(prompt_text)
            .fill(crate::ui::shared::theme_global::prompt_fill())
            .stroke(egui::Stroke::new(
                crate::ui::shared::layout_tokens_global::BORDER_THIN,
                crate::ui::shared::theme_global::prompt_stroke(),
            ))
            .corner_radius(egui::CornerRadius::same(7))
            .min_size(egui::vec2(0.0, 18.0)),
    )
    .on_hover_text(crate::ui::shared::tooltip_global::SHOW_PARSED_PROMPTS)
    .clicked()
}

pub(crate) fn collect_step2_prompt_toolbar_entries(state: &WizardState) -> Vec<PromptToolbarModEntry> {
    let mut entries = active_mods_ref(state)
        .iter()
        .filter_map(|mod_state| {
            let mut component_ids = mod_state
                .components
                .iter()
                .filter_map(|component| {
                    let has_prompt = component
                        .prompt_summary
                        .as_deref()
                        .map(str::trim)
                        .is_some_and(|summary| !summary.is_empty())
                        || !component.prompt_events.is_empty();
                    if !has_prompt {
                        return None;
                    }
                    component.component_id.trim().parse::<u32>().ok()
                })
                .collect::<Vec<_>>();
            component_ids.sort_unstable();
            component_ids.dedup();
            if component_ids.is_empty() {
                None
            } else {
                Some(PromptToolbarModEntry {
                    mod_name: mod_state.name.clone(),
                    tp_file: mod_state.tp_file.clone(),
                    component_ids,
                })
            }
        })
        .collect::<Vec<_>>();
    entries.sort_by(|a, b| a.mod_name.to_ascii_lowercase().cmp(&b.mod_name.to_ascii_lowercase()));
    entries
}

pub(crate) fn collect_step3_prompt_toolbar_entries(state: &WizardState) -> Vec<PromptToolbarModEntry> {
    let prompt_eval = build_prompt_eval_context(state);
    let items = if state.step3.active_game_tab == "BGEE" {
        &state.step3.bgee_items
    } else {
        &state.step3.bg2ee_items
    };
    collect_step3_prompt_toolbar_entries_from_items(items, &prompt_eval)
}

pub(crate) fn collect_step3_prompt_toolbar_entries_from_items(
    items: &[Step3ItemState],
    prompt_eval: &crate::ui::step2::state_step2::PromptEvalContext,
) -> Vec<PromptToolbarModEntry> {
    let mut by_mod = BTreeMap::<(String, String), Vec<u32>>::new();
    for item in items.iter().filter(|item| !item.is_parent) {
        let summary = evaluate_step3_item_prompt_summary(item, prompt_eval);
        if summary.trim().is_empty() {
            continue;
        }
        let Ok(component_id) = item.component_id.trim().parse::<u32>() else {
            continue;
        };
        by_mod
            .entry((item.mod_name.clone(), item.tp_file.clone()))
            .or_default()
            .push(component_id);
    }

    by_mod
        .into_iter()
        .map(|((mod_name, tp_file), mut component_ids)| {
            component_ids.sort_unstable();
            component_ids.dedup();
            PromptToolbarModEntry {
                mod_name,
                tp_file,
                component_ids,
            }
        })
        .collect()
}

fn render_prompt_toolbar_popup(ui: &mut egui::Ui, state: &mut WizardState) {
    let title = state.step2.prompt_popup_title.clone();
    let entries = if state.current_step == 2 {
        collect_step3_prompt_toolbar_entries(state)
    } else {
        collect_step2_prompt_toolbar_entries(state)
    };
    let mut open = state.step2.prompt_popup_open;
    let mut jump_target: Option<(String, u32)> = None;
    egui::Window::new(title)
        .open(&mut open)
        .resizable(true)
        .collapsible(false)
        .default_width(420.0)
        .default_height(320.0)
        .show(ui.ctx(), |ui| {
            if entries.is_empty() {
                ui.label("No component prompts in the active tab.");
                return;
            }
            egui::ScrollArea::vertical().show(ui, |ui| {
                for entry in &entries {
                    let header = format!("{} ({})", entry.mod_name, entry.component_ids.len());
                    egui::CollapsingHeader::new(header)
                        .default_open(false)
                        .show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                for component_id in &entry.component_ids {
                                    let button_text =
                                        crate::ui::shared::typography_global::monospace(component_id.to_string())
                                            .color(crate::ui::shared::theme_global::accent_numbers());
                                    if ui
                                        .add(
                                            egui::Button::new(button_text)
                                                .min_size(egui::vec2(42.0, 22.0))
                                                .fill(ui.visuals().widgets.inactive.bg_fill)
                                                .stroke(ui.visuals().widgets.inactive.bg_stroke),
                                        )
                                        .clicked()
                                    {
                                        jump_target = Some((entry.tp_file.clone(), *component_id));
                                    }
                                }
                            });
                        });
                }
            });
        });
    state.step2.prompt_popup_open = open;
    if let Some((mod_ref, component_id)) = jump_target {
        let game_tab = if state.current_step == 2 {
            state.step3.active_game_tab.clone()
        } else {
            state.step2.active_game_tab.clone()
        };
        if state.current_step == 2 {
            let _ = state_step3::jump_to_target(state, &game_tab, &mod_ref, Some(component_id));
        } else {
            jump_to_target(state, &game_tab, &mod_ref, Some(component_id));
            state.step2.jump_to_selected_requested = true;
        }
    }
}

fn active_mods_ref(state: &WizardState) -> &[Step2ModState] {
    if state.step2.active_game_tab == "BGEE" {
        &state.step2.bgee_mods
    } else {
        &state.step2.bg2ee_mods
    }
}

fn collect_prompt_jump_component_ids(mods: &[Step2ModState], title: &str, text: &str) -> Vec<u32> {
    let mut ids = parse_prompt_jump_component_ids(text);
    let mod_ref = parse_prompt_popup_mod_ref(title);
    let target_mod_key = selection_normalize_mod_key(&mod_ref);
    for mod_state in mods {
        if selection_normalize_mod_key(&mod_state.tp_file) != target_mod_key {
            continue;
        }
        for component in &mod_state.components {
            let has_prompt = component
                .prompt_summary
                .as_ref()
                .map(|summary| !summary.trim().is_empty())
                .unwrap_or(false)
                || !component.prompt_events.is_empty();
            if !has_prompt {
                continue;
            }
            if let Ok(id) = component.component_id.trim().parse::<u32>()
                && !ids.contains(&id)
            {
                ids.push(id);
            }
        }
    }
    ids.sort_unstable();
    ids
}

fn parse_prompt_popup_mod_ref(title: &str) -> String {
    title
        .split(" #")
        .next()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| title.trim().to_string())
}

fn parse_prompt_jump_component_ids(text: &str) -> Vec<u32> {
    let mut ids = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix("Component:") else {
            continue;
        };
        let id_token = rest.split_whitespace().next().unwrap_or_default();
        if let Ok(id) = id_token.parse::<u32>()
            && !ids.contains(&id)
        {
            ids.push(id);
        }
    }
    ids
}
