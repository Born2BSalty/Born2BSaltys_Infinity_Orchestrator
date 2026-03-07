// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::state::{Step2ComponentState, Step2ModState, Step2Selection};
use crate::ui::step2::prompt_eval_step2::{evaluate_component_prompt_summary, event_applies};
use crate::ui::step2::state_step2::PromptEvalContext;
use crate::ui::step2::tree_step2::step2_tree::render_helpers::{
    enforce_meta_mode_after_bulk, enforce_subcomponent_single_select_keep_first,
    parent_compat_summary, set_component_checked_state,
};

pub(crate) struct ParentRowResult {
    pub selection: Option<Step2Selection>,
    pub open_compat_for_component: Option<(String, String, String)>,
    pub open_prompt_popup: Option<(String, String)>,
}

pub(crate) fn render_parent_row(
    ui: &mut egui::Ui,
    mod_state: &mut Step2ModState,
    active_tab: &str,
    selected: &Option<Step2Selection>,
    next_selection_order: &mut usize,
    prompt_eval: &PromptEvalContext,
    jump_to_selected_requested: &mut bool,
) -> ParentRowResult {
    let mod_name = mod_state.name.clone();
    let parent_summary = parent_compat_summary(mod_state);
    let enabled_count = mod_state.components.iter().filter(|c| !c.disabled).count();
    let all_selected = enabled_count > 0
        && mod_state
            .components
            .iter()
            .filter(|component| !component.disabled)
            .all(|component| component.checked);
    let any_selected = mod_state
        .components
        .iter()
        .filter(|component| !component.disabled)
        .any(|component| component.checked);
    let set_value = !any_selected;

    let mut new_selection: Option<Step2Selection> = None;
    let mut open_compat_for_component: Option<(String, String, String)> = None;
    let mut open_prompt_popup: Option<(String, String)> = None;
    let mut parent_checked = all_selected;
    let mut checkbox = egui::Checkbox::new(&mut parent_checked, "");
    if any_selected && !all_selected {
        checkbox = checkbox.indeterminate(true);
    }

    ui.horizontal(|ui| {
        let parent_clicked = ui
            .push_id(
                (
                    "mod_parent_checkbox",
                    &mod_state.tp_file,
                    &mod_state.name,
                    &mod_state.tp2_path,
                ),
                |ui| {
                    ui.add_enabled_ui(enabled_count > 0, |ui| ui.add(checkbox).clicked())
                        .inner
                },
            )
            .inner;
        if parent_clicked {
            for component in &mut mod_state.components {
                if component.disabled {
                    continue;
                }
                component.checked = set_value;
                set_component_checked_state(component, next_selection_order);
            }
            if set_value {
                enforce_subcomponent_single_select_keep_first(mod_state);
            }
            enforce_meta_mode_after_bulk(mod_state);
            mod_state.checked = enabled_count > 0
                && mod_state
                    .components
                    .iter()
                    .filter(|component| !component.disabled)
                    .all(|component| component.checked);
        }
        let is_selected = matches!(
            selected,
            Some(Step2Selection::Mod { game_tab, tp_file })
                if game_tab == active_tab && tp_file == &mod_state.tp_file
        );
        let row_w = ui.available_width().max(0.0);
        ui.allocate_ui_with_layout(
            egui::vec2(row_w, ui.spacing().interact_size.y),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.set_max_width(row_w);
                let row = ui.selectable_label(is_selected, mod_name.as_str());
                if *jump_to_selected_requested && is_selected {
                    ui.scroll_to_rect(row.rect, Some(egui::Align::Center));
                    *jump_to_selected_requested = false;
                }
                if row.clicked() {
                    new_selection = Some(Step2Selection::Mod {
                        game_tab: active_tab.to_string(),
                        tp_file: mod_state.tp_file.clone(),
                    });
                }
                if let Some((text_color, bg, label)) = &parent_summary {
                    ui.add_space(6.0);
                    let resp = ui.add(
                        egui::Button::new(
                            crate::ui::shared::typography_global::strong(label)
                                .color(*text_color)
                                .size(crate::ui::shared::typography_global::SIZE_PILL_TEXT),
                        )
                        .fill(*bg)
                        .stroke(egui::Stroke::new(
                            crate::ui::shared::layout_tokens_global::BORDER_THIN,
                            *bg,
                        ))
                        .corner_radius(egui::CornerRadius::same(7))
                        .min_size(egui::vec2(0.0, 18.0)),
                    );
                    if resp.clicked()
                        && let Some(first_compat) = mod_state
                            .components
                            .iter()
                            .find(|c| c.compat_kind.is_some())
                    {
                        open_compat_for_component = Some((
                            mod_state.tp_file.clone(),
                            first_compat.component_id.clone(),
                            first_compat.raw_line.clone(),
                        ));
                    }
                }
                if has_any_prompt(mod_state, prompt_eval) {
                    ui.add_space(6.0);
                    let prompt_resp = ui.add(
                        egui::Button::new(
                            crate::ui::shared::typography_global::strong("PROMPT")
                                .color(crate::ui::shared::theme_global::prompt_text())
                                .size(crate::ui::shared::typography_global::SIZE_PILL_TEXT),
                        )
                        .fill(crate::ui::shared::theme_global::prompt_fill())
                        .stroke(egui::Stroke::new(
                            crate::ui::shared::layout_tokens_global::BORDER_THIN,
                            crate::ui::shared::theme_global::prompt_stroke(),
                        ))
                        .corner_radius(egui::CornerRadius::same(7))
                        .min_size(egui::vec2(0.0, 18.0)),
                    );
                    let prompt_resp = prompt_resp
                        .on_hover_text(crate::ui::shared::tooltip_global::SHOW_PARSED_PROMPTS);
                    if prompt_resp.clicked() {
                        new_selection = Some(Step2Selection::Mod {
                            game_tab: active_tab.to_string(),
                            tp_file: mod_state.tp_file.clone(),
                        });
                        if let Some(text) = build_mod_prompt_popup_text(mod_state, prompt_eval) {
                            open_prompt_popup = Some((mod_state.tp_file.clone(), text));
                        }
                    }
                }
            },
        );
    });
    ParentRowResult {
        selection: new_selection,
        open_compat_for_component,
        open_prompt_popup,
    }
}

pub(crate) fn format_component_prompt_popup_text_with_body(
    component: &Step2ComponentState,
    body: &str,
) -> String {
    format!(
        "Component: {} - {}\n\n{}",
        component.component_id.trim(),
        component.label.trim(),
        body.trim()
    )
}

fn has_any_prompt(mod_state: &Step2ModState, prompt_eval: &PromptEvalContext) -> bool {
    let component_has_prompt_data = mod_state.components.iter().any(|component| {
        component
            .prompt_summary
            .as_deref()
            .map(str::trim)
            .is_some_and(|s| !s.is_empty())
            || !component.prompt_events.is_empty()
    });
    mod_state
        .components
        .iter()
        .any(|component| !evaluate_component_prompt_summary(component, prompt_eval).is_empty())
        || component_has_prompt_data
        || mod_state
            .mod_prompt_events
            .iter()
            .any(|event| event_applies(event, prompt_eval))
        || (mod_state.mod_prompt_events.is_empty()
            && mod_state
                .mod_prompt_summary
                .as_deref()
                .map(str::trim)
                .is_some_and(|s| !s.is_empty()))
}

fn build_mod_prompt_popup_text(
    mod_state: &Step2ModState,
    prompt_eval: &PromptEvalContext,
) -> Option<String> {
    let mut sections = Vec::<String>::new();
    for component in &mod_state.components {
        let mut summary = evaluate_component_prompt_summary(component, prompt_eval);
        if summary.is_empty() {
            summary = format_prompt_event_blocks(&component.prompt_events, None);
        }
        if summary.is_empty() {
            summary = component
                .prompt_summary
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or_default()
                .to_string();
        }
        if summary.is_empty() {
            continue;
        }
        sections.push(format_component_prompt_popup_text_with_body(component, &summary));
    }
    if !sections.is_empty() {
        return Some(sections.join("\n\n----------------\n\n"));
    }
    if !mod_state.mod_prompt_events.is_empty() {
        let summary = format_prompt_event_blocks(&mod_state.mod_prompt_events, Some(prompt_eval));
        if !summary.is_empty() {
            return Some(summary);
        }
        return None;
    }
    mod_state
        .mod_prompt_summary
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

fn format_prompt_event_blocks(
    events: &[crate::parser::PromptSummaryEvent],
    prompt_eval: Option<&PromptEvalContext>,
) -> String {
    let mut lines = Vec::<String>::new();
    for event in events {
        if prompt_eval.is_some_and(|ctx| !event_applies(event, ctx)) {
            continue;
        }
        let line = event.summary_line.trim();
        if line.is_empty() {
            continue;
        }
        if !lines.iter().any(|existing| existing == line) {
            lines.push(line.to_string());
        }
    }
    lines.join("\n\n")
}
