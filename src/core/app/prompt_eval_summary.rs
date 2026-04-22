// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{Mutex, OnceLock};

use crate::app::prompt_eval_vars::build_prompt_var_context;
use crate::app::state::Step2ComponentState;
use crate::parser::PromptSummaryEvent;
use crate::parser::prompt_eval_expr::{
    PromptComponentInput, PromptEvalContext, PromptVarContext, evaluate_condition_clause,
};

const PROMPT_SUMMARY_CACHE_LIMIT: usize = 8192;

pub(crate) fn evaluate_component_prompt_summary(
    component: &Step2ComponentState,
    prompt_eval: &PromptEvalContext,
) -> String {
    evaluate_prompt_summary_input(
        component.checked,
        &component.component_id,
        &component.raw_line,
        component.prompt_summary.as_deref(),
        &component.prompt_events,
        prompt_eval,
    )
}

pub(crate) fn evaluate_prompt_summary_input(
    checked: bool,
    component_id: &str,
    raw_line: &str,
    prompt_summary: Option<&str>,
    prompt_events: &[PromptSummaryEvent],
    prompt_eval: &PromptEvalContext,
) -> String {
    if !checked {
        return String::new();
    }
    let cache_key = prompt_summary_cache_key(
        checked,
        component_id,
        raw_line,
        prompt_summary,
        prompt_events,
        prompt_eval,
    );
    if let Some(cached) = prompt_summary_cache()
        .lock()
        .expect("prompt summary cache lock poisoned")
        .get(&cache_key)
        .cloned()
    {
        return cached;
    }

    let result = if prompt_events.is_empty() {
        prompt_summary.map(str::trim).unwrap_or("").to_string()
    } else {
        let prompt_vars = build_prompt_var_context(
            PromptComponentInput {
                raw_line,
                component_id,
                prompt_events,
            },
            prompt_eval,
        );
        let mut out = Vec::<String>::new();
        for event in prompt_events {
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
        out = normalize_prompt_blocks(out);
        if out.is_empty() {
            String::new()
        } else {
            out.into_iter().take(6).collect::<Vec<_>>().join("\n\n")
        }
    };

    let mut cache = prompt_summary_cache()
        .lock()
        .expect("prompt summary cache lock poisoned");
    if cache.len() >= PROMPT_SUMMARY_CACHE_LIMIT {
        cache.clear();
    }
    cache.insert(cache_key, result.clone());
    result
}

pub(crate) fn event_applies(event: &PromptSummaryEvent, prompt_eval: &PromptEvalContext) -> bool {
    event_applies_with_vars(event, prompt_eval, None)
}

fn event_applies_with_vars(
    event: &PromptSummaryEvent,
    prompt_eval: &PromptEvalContext,
    prompt_vars: Option<&PromptVarContext>,
) -> bool {
    if !event.game_deny.is_empty()
        && event.game_deny.iter().any(|g| {
            prompt_eval
                .active_games
                .iter()
                .any(|active| g.eq_ignore_ascii_case(active))
        })
    {
        return false;
    }
    if !event.game_allow.is_empty()
        && !event.game_allow.iter().any(|g| {
            prompt_eval
                .active_games
                .iter()
                .any(|active| g.eq_ignore_ascii_case(active))
        })
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

pub(crate) fn normalize_prompt_blocks(lines: Vec<String>) -> Vec<String> {
    if lines.is_empty() {
        return lines;
    }

    let mut merged = Vec::<String>::new();
    let mut idx = 0usize;
    while idx < lines.len() {
        let current = lines[idx].trim().to_string();
        if idx + 1 < lines.len() {
            let next = lines[idx + 1].trim().to_string();
            if should_merge_prompt_preface(&current, &next) {
                merged.push(format!("{}\n\n{}", strip_prompt_options(&current), next));
                idx += 2;
                continue;
            }
        }
        merged.push(current);
        idx += 1;
    }

    let has_yes_no_question = merged
        .iter()
        .any(|l| is_yes_no_prompt(l) && is_question_prompt(l));
    merged.retain(|line| !is_validation_retry_block(line));

    if !has_yes_no_question {
        return merged;
    }
    merged
        .into_iter()
        .filter(|line| !is_yes_no_prompt(line) || is_question_prompt(line))
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

fn should_merge_prompt_preface(current: &str, next: &str) -> bool {
    is_informational_preface(current)
        && is_real_question_block(next)
        && (!has_prompt_options(current) || prompt_options(current) == prompt_options(next))
}

fn is_informational_preface(block: &str) -> bool {
    let first = first_prompt_line(block);
    if first.is_empty() || is_real_question_line(first) {
        return false;
    }
    let lower = first.to_ascii_lowercase();
    !lower.starts_with("please enter")
        && !lower.starts_with("choose ")
        && !lower.starts_with("select ")
        && !lower.starts_with("accept ")
        && !lower.starts_with("do you wish")
}

fn is_real_question_block(block: &str) -> bool {
    let first = first_prompt_line(block);
    is_real_question_line(first)
}

fn is_real_question_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.contains('?') || trimmed.ends_with(':')
}

fn first_prompt_line(block: &str) -> &str {
    block.lines().next().map(str::trim).unwrap_or("")
}

fn has_prompt_options(block: &str) -> bool {
    !prompt_options(block).is_empty()
}

fn prompt_options(block: &str) -> String {
    block
        .lines()
        .filter(|line| line.trim_start().starts_with('-'))
        .map(str::trim)
        .collect::<Vec<_>>()
        .join("\n")
}

fn strip_prompt_options(block: &str) -> String {
    block
        .lines()
        .filter(|line| !line.trim_start().starts_with('-'))
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

fn is_validation_retry_block(block: &str) -> bool {
    if has_prompt_options(block) {
        return false;
    }
    let lower = block.to_ascii_lowercase();
    let mentions_retry = lower.contains("please try again") || lower.contains("try again");
    let mentions_validation = lower.contains("must be")
        || lower.contains("must be entered")
        || lower.contains("must be an integer")
        || lower.contains("invalid");
    mentions_retry && mentions_validation
}

fn prompt_summary_cache() -> &'static Mutex<HashMap<u64, String>> {
    static CACHE: OnceLock<Mutex<HashMap<u64, String>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn prompt_summary_cache_key(
    checked: bool,
    component_id: &str,
    raw_line: &str,
    prompt_summary: Option<&str>,
    prompt_events: &[PromptSummaryEvent],
    prompt_eval: &PromptEvalContext,
) -> u64 {
    let mut hasher = DefaultHasher::new();
    prompt_eval.signature.hash(&mut hasher);
    checked.hash(&mut hasher);
    component_id.hash(&mut hasher);
    raw_line.hash(&mut hasher);
    prompt_summary.hash(&mut hasher);
    hash_prompt_events(prompt_events, &mut hasher);
    hasher.finish()
}

fn hash_prompt_events(events: &[PromptSummaryEvent], state: &mut DefaultHasher) {
    events.len().hash(state);
    for event in events {
        event.kind.hash(state);
        event.node_id.hash(state);
        event.text.hash(state);
        event.summary_line.hash(state);
        event.source_file.hash(state);
        event.line.hash(state);
        event.branch_path.hash(state);
        event.condition.hash(state);
        event.condition_id.hash(state);
        event.game_allow.hash(state);
        event.game_deny.hash(state);
    }
}
