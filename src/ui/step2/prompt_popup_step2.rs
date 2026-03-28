// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::layout_tokens_global::{SPACE_MD, SPACE_SM, SPACE_XS};
use crate::ui::state::{Step2ModState, WizardState};
use crate::ui::step2::service_selection_step2::{
    jump_to_target, selection_normalize_mod_key,
};

pub fn render_prompt_popup(ui: &mut egui::Ui, state: &mut WizardState) {
    if !state.step2.prompt_popup_open {
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
