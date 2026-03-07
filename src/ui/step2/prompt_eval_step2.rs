// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::parser::PromptSummaryEvent;
use crate::ui::state::Step2ComponentState;
use crate::ui::step2::prompt_eval_expr_step2::evaluate_condition_clause;
use crate::ui::step2::prompt_eval_vars_step2::build_prompt_var_context;
use crate::ui::step2::state_step2::PromptEvalContext;

pub(crate) fn evaluate_component_prompt_summary(
    component: &Step2ComponentState,
    prompt_eval: &PromptEvalContext,
) -> String {
    if !component.checked {
        return String::new();
    }
    if component.prompt_events.is_empty() {
        return component
            .prompt_summary
            .as_deref()
            .map(str::trim)
            .unwrap_or("")
            .to_string();
    }
    let prompt_vars = build_prompt_var_context(component, prompt_eval);
    let mut out = Vec::<String>::new();
    for event in &component.prompt_events {
        if !event_applies_with_vars(event, prompt_eval, Some(&prompt_vars)) {
            continue;
        }
        let line = event.summary_line.trim();
        if line.is_empty() {
            continue;
        }
        if !out.iter().any(|existing| existing == line) {
            out.push(line.to_string());
        }
    }
    out = postprocess_prompt_lines(out);
    if out.is_empty() {
        return String::new();
    }
    out.into_iter().take(6).collect::<Vec<_>>().join("\n\n")
}

pub(crate) fn event_applies(event: &PromptSummaryEvent, prompt_eval: &PromptEvalContext) -> bool {
    event_applies_with_vars(event, prompt_eval, None)
}

fn event_applies_with_vars(
    event: &PromptSummaryEvent,
    prompt_eval: &PromptEvalContext,
    prompt_vars: Option<&crate::ui::step2::prompt_eval_vars_step2::PromptVarContext>,
) -> bool {
    if !event.game_deny.is_empty()
        && event
            .game_deny
            .iter()
            .any(|g| prompt_eval.active_games.iter().any(|active| g.eq_ignore_ascii_case(active)))
    {
        return false;
    }
    if !event.game_allow.is_empty()
        && !event
            .game_allow
            .iter()
            .any(|g| prompt_eval.active_games.iter().any(|active| g.eq_ignore_ascii_case(active)))
    {
        return false;
    }
    if let Some(condition) = event.condition.as_deref()
        && !evaluate_condition_clause(condition, prompt_eval, prompt_vars)
    {
        return false;
    }
    for branch in &event.branch_path {
        let upper = branch.to_ascii_uppercase();
        if !(upper.starts_with("if:ACTION_IF") || upper.starts_with("else:ACTION_IF")) {
            continue;
        }
        if !evaluate_condition_clause(branch, prompt_eval, prompt_vars) {
            return false;
        }
    }
    true
}

fn postprocess_prompt_lines(lines: Vec<String>) -> Vec<String> {
    if lines.is_empty() {
        return lines;
    }
    let has_yes_no_question = lines.iter().any(|l| is_yes_no_prompt(l) && is_question_prompt(l));
    if !has_yes_no_question {
        return lines;
    }
    lines
        .into_iter()
        .filter(|line| !(is_yes_no_prompt(line) && !is_question_prompt(line)))
        .collect()
}

fn is_question_prompt(line: &str) -> bool {
    line.lines()
        .next()
        .map(str::trim)
        .is_some_and(|first| first.ends_with('?'))
}

fn is_yes_no_prompt(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.contains("\n- y =") && lower.contains("\n- n =")
}
