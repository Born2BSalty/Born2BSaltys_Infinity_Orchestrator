// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::state::{Step2ComponentState, Step2ModState, Step2Selection};
use crate::ui::step2::prompt_eval_step2::{
    evaluate_component_prompt_summary, event_applies, normalize_prompt_blocks,
};
use crate::ui::step2::state_step2::PromptEvalContext;
use crate::ui::step2::tree_compat_display_step2::{parent_compat_summary, parent_compat_target};
use crate::ui::step2::tree_selection_rules_step2::{
    enforce_collapsible_group_umbrella_after_bulk, enforce_meta_mode_after_bulk,
    enforce_subcomponent_single_select_keep_first, enforce_tp2_same_mod_exclusive_after_bulk,
    set_component_checked_state,
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
    let mod_visible_count = mod_state.components.len();
    let selected_visible_count = mod_state
        .components
        .iter()
        .filter(|component| component.checked)
        .count();
    let mod_header_label =
        format!("{mod_name} ({selected_visible_count}/{mod_visible_count})");
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
                enforce_collapsible_group_umbrella_after_bulk(mod_state);
                enforce_tp2_same_mod_exclusive_after_bulk(mod_state);
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
                let row = ui.selectable_label(is_selected, mod_header_label.as_str());
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
                        && let Some(target_compat) = parent_compat_target(mod_state)
                    {
                        open_compat_for_component = Some((
                            mod_state.tp_file.clone(),
                            target_compat.component_id.clone(),
                            target_compat.raw_line.clone(),
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
    let mut blocks = Vec::<PromptDisplayBlock>::new();
    for event in events {
        if prompt_eval.is_some_and(|ctx| !event_applies(event, ctx)) {
            continue;
        }
        let block = PromptDisplayBlock::from_summary_line(event.summary_line.trim());
        if block.is_empty() {
            continue;
        }
        if !blocks.iter().any(|existing| existing == &block) {
            blocks.push(block);
        }
    }
    normalize_prompt_event_blocks(blocks)
        .into_iter()
        .map(|block| block.to_text())
        .collect::<Vec<_>>()
        .join("\n\n")
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PromptDisplayBlock {
    body: String,
    options: Vec<String>,
}

impl PromptDisplayBlock {
    fn from_summary_line(line: &str) -> Self {
        let mut body_lines = Vec::<String>::new();
        let mut options = Vec::<String>::new();
        for raw in line.lines() {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                continue;
            }
            if trimmed.starts_with('-') {
                options.push(trimmed.to_string());
            } else {
                body_lines.push(trimmed.to_string());
            }
        }
        Self {
            body: body_lines.join("\n").trim().to_string(),
            options,
        }
    }

    fn is_empty(&self) -> bool {
        self.body.trim().is_empty() && self.options.is_empty()
    }

    fn first_line(&self) -> &str {
        self.body.lines().next().map(str::trim).unwrap_or("")
    }

    fn to_text(&self) -> String {
        if self.options.is_empty() {
            return self.body.clone();
        }
        format!("{}\n{}", self.body, self.options.join("\n"))
            .trim()
            .to_string()
    }
}

fn normalize_prompt_event_blocks(blocks: Vec<PromptDisplayBlock>) -> Vec<PromptDisplayBlock> {
    let mut merged = Vec::<PromptDisplayBlock>::new();
    let mut idx = 0usize;
    while idx < blocks.len() {
        let current = blocks[idx].clone();
        if idx + 1 < blocks.len() {
            let next = blocks[idx + 1].clone();
            if should_merge_preface_block(&current, &next) {
                merged.push(PromptDisplayBlock {
                    body: format!("{}\n\n{}", current.body.trim(), next.body.trim())
                        .trim()
                        .to_string(),
                    options: next.options,
                });
                idx += 2;
                continue;
            }
        }
        merged.push(current);
        idx += 1;
    }

    let lines = merged.iter().map(PromptDisplayBlock::to_text).collect::<Vec<_>>();
    let normalized = normalize_prompt_blocks(lines);
    normalized
        .into_iter()
        .map(|line| PromptDisplayBlock::from_summary_line(&line))
        .filter(|block| !block.is_empty())
        .collect()
}

fn should_merge_preface_block(current: &PromptDisplayBlock, next: &PromptDisplayBlock) -> bool {
    is_informational_preface_block(current)
        && is_real_question_block(next)
        && current.options == next.options
}

fn is_informational_preface_block(block: &PromptDisplayBlock) -> bool {
    let first = block.first_line().to_ascii_lowercase();
    !first.is_empty()
        && !is_real_question_line(block.first_line())
        && !first.starts_with("please enter")
        && !first.starts_with("choose ")
        && !first.starts_with("select ")
        && !first.starts_with("accept ")
        && !first.starts_with("do you wish")
}

fn is_real_question_block(block: &PromptDisplayBlock) -> bool {
    is_real_question_line(block.first_line())
}

fn is_real_question_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.contains('?') || trimmed.ends_with(':')
}
