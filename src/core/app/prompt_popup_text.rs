// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::prompt_eval_summary::{
    evaluate_component_prompt_summary, event_applies, normalize_prompt_blocks,
};
use crate::app::prompt_eval_summary_step3::evaluate_step3_item_prompt_summary;
use crate::app::state::{Step2ComponentState, Step2ModState, Step3ItemState};
use crate::parser::{PromptSummaryEvent, prompt_eval_expr::PromptEvalContext};

#[derive(Clone)]
pub(crate) struct PromptToolbarModEntry {
    pub(crate) mod_name: String,
    pub(crate) tp_file: String,
    pub(crate) component_ids: Vec<u32>,
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

pub(crate) fn build_mod_prompt_popup_text(
    mod_state: &Step2ModState,
    prompt_eval: &PromptEvalContext,
) -> Option<String> {
    let mut sections = Vec::<String>::new();
    for component in &mod_state.components {
        if !component.checked {
            continue;
        }
        let mut summary = evaluate_component_prompt_summary(component, prompt_eval);
        if summary.is_empty() {
            summary = format_prompt_event_blocks(&component.prompt_events, None);
        }
        if summary.is_empty() {
            summary = component
                .prompt_summary
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or_default()
                .to_string();
        }
        if summary.is_empty() {
            continue;
        }
        sections.push(format_component_prompt_popup_text_with_body(
            component, &summary,
        ));
    }
    if !sections.is_empty() {
        return Some(sections.join("\n\n----------------\n\n"));
    }
    if !mod_level_prompt_can_apply(mod_state) {
        return None;
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
        .filter(|value| !value.is_empty())
        .map(std::string::ToString::to_string)
}

fn component_has_prompt_data(component: &Step2ComponentState) -> bool {
    component
        .prompt_summary
        .as_deref()
        .map(str::trim)
        .is_some_and(|summary| !summary.is_empty())
        || !component.prompt_events.is_empty()
}

fn mod_level_prompt_can_apply(mod_state: &Step2ModState) -> bool {
    let any_checked = mod_state
        .components
        .iter()
        .any(|component| component.checked);
    let summary_is_component_aggregate = mod_state.components.iter().any(component_has_prompt_data);
    any_checked && !summary_is_component_aggregate
}

pub(crate) fn mod_has_any_prompt(
    mod_state: &Step2ModState,
    prompt_eval: &PromptEvalContext,
) -> bool {
    let checked_component_has_prompt = mod_state.components.iter().any(|component| {
        component.checked
            && (component_has_prompt_data(component)
                || !evaluate_component_prompt_summary(component, prompt_eval).is_empty())
    });
    if checked_component_has_prompt {
        return true;
    }
    mod_level_prompt_can_apply(mod_state)
        && (mod_state
            .mod_prompt_events
            .iter()
            .any(|event| event_applies(event, prompt_eval))
            || (mod_state.mod_prompt_events.is_empty()
                && mod_state
                    .mod_prompt_summary
                    .as_deref()
                    .map(str::trim)
                    .is_some_and(|summary| !summary.is_empty())))
}

pub(crate) fn collect_step2_prompt_toolbar_entries(
    mods: &[Step2ModState],
) -> Vec<PromptToolbarModEntry> {
    let mut entries = mods
        .iter()
        .filter_map(|mod_state| {
            let mut component_ids = mod_state
                .components
                .iter()
                .filter_map(|component| {
                    if !component.checked {
                        return None;
                    }
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
    entries.sort_by(|a, b| {
        a.mod_name
            .to_ascii_lowercase()
            .cmp(&b.mod_name.to_ascii_lowercase())
    });
    entries
}

pub(crate) fn collect_step3_prompt_toolbar_entries(
    items: &[Step3ItemState],
    prompt_eval: &PromptEvalContext,
) -> Vec<PromptToolbarModEntry> {
    let mut by_mod = std::collections::BTreeMap::<(String, String), Vec<u32>>::new();
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

pub(crate) fn format_step3_prompt_popup(item: &Step3ItemState, body: &str) -> (String, String) {
    (
        format!("{} #{}", item.tp_file, item.component_id),
        format!(
            "Component: {} - {}\n\n{}",
            item.component_id.trim(),
            item.component_label.trim(),
            body.trim()
        ),
    )
}

fn format_prompt_event_blocks(
    events: &[PromptSummaryEvent],
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
    normalize_prompt_event_blocks(&blocks)
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
        self.body.lines().next().map_or("", str::trim)
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

fn normalize_prompt_event_blocks(blocks: &[PromptDisplayBlock]) -> Vec<PromptDisplayBlock> {
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

    let lines = merged
        .iter()
        .map(PromptDisplayBlock::to_text)
        .collect::<Vec<_>>();
    let normalized = normalize_prompt_blocks(lines);
    normalized
        .into_iter()
        .map(|line| PromptDisplayBlock::from_summary_line(&line))
        .filter(|block| !block.is_empty())
        .collect()
}

fn should_merge_preface_block(current: &PromptDisplayBlock, next: &PromptDisplayBlock) -> bool {
    let current_is_preface = is_informational_preface_block(current);
    let next_is_question = is_real_question_block(next);
    current_is_preface && next_is_question && current.options == next.options
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

#[cfg(test)]
mod tests {
    use super::*;

    fn blank_component(
        id: &str,
        checked: bool,
        prompt_summary: Option<&str>,
    ) -> Step2ComponentState {
        Step2ComponentState {
            component_id: id.to_string(),
            label: id.to_string(),
            weidu_group: None,
            collapsible_group: None,
            collapsible_group_is_umbrella: false,
            collapsible_group_combinable: false,
            raw_line: String::new(),
            prompt_summary: prompt_summary.map(str::to_string),
            prompt_events: Vec::new(),
            is_meta_mode_component: false,
            disabled: false,
            compat_kind: None,
            compat_source: None,
            compat_related_mod: None,
            compat_related_component: None,
            compat_graph: None,
            compat_evidence: None,
            disabled_reason: None,
            checked,
            selected_order: None,
        }
    }

    fn blank_mod(components: Vec<Step2ComponentState>) -> Step2ModState {
        Step2ModState {
            name: "Mod".to_string(),
            tp_file: "mod.tp2".to_string(),
            tp2_path: String::new(),
            readme_path: None,
            ini_path: None,
            web_url: None,
            package_marker: None,
            latest_checked_version: None,
            update_locked: false,
            mod_prompt_summary: None,
            mod_prompt_events: Vec::new(),
            checked: false,
            hidden_components: Vec::new(),
            components,
        }
    }

    fn blank_mod_event() -> PromptSummaryEvent {
        PromptSummaryEvent {
            kind: "prompt".to_string(),
            node_id: String::new(),
            text: String::new(),
            summary_line: "Mod-level question?".to_string(),
            source_file: String::new(),
            line: None,
            branch_path: Vec::new(),
            condition: None,
            condition_id: None,
            game_allow: Vec::new(),
            game_deny: Vec::new(),
        }
    }

    #[test]
    fn mod_has_any_prompt_false_when_component_with_prompt_is_unticked() {
        let mod_state = blank_mod(vec![blank_component("1", false, Some("Pick a value"))]);
        let prompt_eval = PromptEvalContext::default();
        assert!(!mod_has_any_prompt(&mod_state, &prompt_eval));
    }

    #[test]
    fn mod_has_any_prompt_true_when_component_with_prompt_is_ticked() {
        let mod_state = blank_mod(vec![blank_component("1", true, Some("Pick a value"))]);
        let prompt_eval = PromptEvalContext::default();
        assert!(mod_has_any_prompt(&mod_state, &prompt_eval));
    }

    #[test]
    fn mod_level_event_counts_only_when_a_component_is_ticked() {
        let mut mod_state = blank_mod(vec![blank_component("1", false, None)]);
        mod_state.mod_prompt_events.push(blank_mod_event());
        let prompt_eval = PromptEvalContext::default();
        assert!(!mod_has_any_prompt(&mod_state, &prompt_eval));
        mod_state.components[0].checked = true;
        assert!(mod_has_any_prompt(&mod_state, &prompt_eval));
    }

    #[test]
    fn build_mod_prompt_popup_text_skips_unticked_components() {
        let mod_state = blank_mod(vec![blank_component("1", false, Some("Pick a value"))]);
        let prompt_eval = PromptEvalContext::default();
        assert_eq!(build_mod_prompt_popup_text(&mod_state, &prompt_eval), None);
    }

    #[test]
    fn build_mod_prompt_popup_text_includes_ticked_components() {
        let mod_state = blank_mod(vec![blank_component("1", true, Some("Pick a value"))]);
        let prompt_eval = PromptEvalContext::default();
        assert!(
            build_mod_prompt_popup_text(&mod_state, &prompt_eval)
                .is_some_and(|text| text.contains("Pick a value"))
        );
    }

    #[test]
    fn collect_step2_prompt_toolbar_entries_excludes_unticked_components() {
        let mods = vec![blank_mod(vec![blank_component(
            "1",
            false,
            Some("Pick a value"),
        )])];
        assert!(collect_step2_prompt_toolbar_entries(&mods).is_empty());
    }

    #[test]
    fn collect_step2_prompt_toolbar_entries_includes_ticked_components() {
        let mods = vec![blank_mod(vec![blank_component(
            "1",
            true,
            Some("Pick a value"),
        )])];
        let entries = collect_step2_prompt_toolbar_entries(&mods);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].component_ids, vec![1]);
    }

    #[test]
    fn mod_has_any_prompt_false_when_only_aggregate_summary_and_nothing_ticked() {
        let mut mod_state = blank_mod(vec![blank_component("1", false, Some("Pick a flavour"))]);
        mod_state.mod_prompt_summary = Some("Component 1:\nPick a flavour".to_string());
        assert!(!mod_has_any_prompt(
            &mod_state,
            &PromptEvalContext::default()
        ));
        assert!(build_mod_prompt_popup_text(&mod_state, &PromptEvalContext::default()).is_none());
    }

    #[test]
    fn mod_has_any_prompt_false_when_aggregate_summary_and_only_promptless_component_ticked() {
        let mut mod_state = blank_mod(vec![
            blank_component("1", false, Some("Pick a flavour")),
            blank_component("2", true, None),
        ]);
        mod_state.mod_prompt_summary = Some("Component 1:\nPick a flavour".to_string());
        assert!(!mod_has_any_prompt(
            &mod_state,
            &PromptEvalContext::default()
        ));
        assert!(build_mod_prompt_popup_text(&mod_state, &PromptEvalContext::default()).is_none());
    }

    #[test]
    fn mod_level_summary_applies_only_when_a_component_is_ticked() {
        let mut mod_state = blank_mod(vec![blank_component("1", false, None)]);
        mod_state.mod_prompt_summary = Some("Install language?".to_string());
        assert!(!mod_has_any_prompt(
            &mod_state,
            &PromptEvalContext::default()
        ));
        mod_state.components[0].checked = true;
        assert!(mod_has_any_prompt(
            &mod_state,
            &PromptEvalContext::default()
        ));
        assert_eq!(
            build_mod_prompt_popup_text(&mod_state, &PromptEvalContext::default()).as_deref(),
            Some("Install language?")
        );
    }
}
