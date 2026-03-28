// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashMap;
use std::fs;

use crate::ui::state::{Step1State, Step2ModState};

use super::compat_mismatch_eval::{build_mismatch_context, evaluate_requirement, TriState};

pub(crate) fn apply_step2_scan_mismatch(
    step1: &Step1State,
    tab: &str,
    mods: &mut [Step2ModState],
) {
    let context = build_mismatch_context(step1, tab);
    let mut guard_cache = HashMap::<String, HashMap<String, Vec<RequirementGuard>>>::new();

    for mod_state in mods {
        let component_guards = guard_cache
            .entry(mod_state.tp2_path.clone())
            .or_insert_with(|| load_component_guards(&mod_state.tp2_path));

        for component in &mut mod_state.components {
            let Some(guards) = component_guards.get(component.component_id.trim()) else {
                continue;
            };
            let failing_guard = guards.iter().find_map(|guard| {
                (evaluate_requirement(&guard.eval_text, &context) == TriState::False)
                    .then(|| guard.raw_line.clone())
            });
            let Some(failing_guard) = failing_guard else {
                continue;
            };

            component.disabled = true;
            component.compat_kind = Some("mismatch".to_string());
            component.compat_source = Some(mismatch_source(&mod_state.tp2_path, &mod_state.tp_file));
            component.compat_related_mod = None;
            component.compat_related_component = None;
            component.compat_graph = None;
            component.compat_evidence = Some(failing_guard);
            component.disabled_reason =
                Some("TP2 REQUIRE_PREDICATE excludes this component for the current game tab.".to_string());
        }

    }
}

#[derive(Debug, Clone)]
struct RequirementGuard {
    raw_line: String,
    eval_text: String,
}

fn load_component_guards(tp2_path: &str) -> HashMap<String, Vec<RequirementGuard>> {
    if tp2_path.trim().is_empty() {
        return HashMap::new();
    }
    let Ok(tp2_text) = fs::read_to_string(tp2_path) else {
        return HashMap::new();
    };

    let mut out = HashMap::<String, Vec<RequirementGuard>>::new();
    let lines: Vec<&str> = tp2_text.lines().collect();
    let mut index = 0usize;
    while index < lines.len() {
        let line = lines[index].trim_start();
        if !line.to_ascii_uppercase().starts_with("BEGIN ") {
            index += 1;
            continue;
        }

        let start = index;
        index += 1;
        while index < lines.len() {
            let next = lines[index].trim_start().to_ascii_uppercase();
            if next.starts_with("BEGIN ") {
                break;
            }
            index += 1;
        }

        let block = &lines[start..index];
        let Some(component_id) = block
            .iter()
            .find_map(|entry| parse_designated_id(&entry.to_ascii_uppercase()))
        else {
            continue;
        };

        let guards = collect_requirement_guards(block);
        if !guards.is_empty() {
            out.insert(component_id, guards);
        }
    }

    out
}

fn collect_requirement_guards(block: &[&str]) -> Vec<RequirementGuard> {
    let mut out = Vec::<RequirementGuard>::new();
    let mut index = 0usize;

    while index < block.len() {
        let trimmed = block[index].trim();
        if !trimmed
            .to_ascii_uppercase()
            .starts_with("REQUIRE_PREDICATE")
        {
            index += 1;
            continue;
        }

        let mut raw_line = trimmed.to_string();
        let mut next = index + 1;
        while next < block.len() {
            let candidate = block[next].trim();
            if candidate.is_empty() {
                next += 1;
                continue;
            }
            if !should_extend_requirement(&raw_line, candidate) {
                break;
            }
            raw_line.push(' ');
            raw_line.push_str(candidate);
            next += 1;
        }

        if let Some(eval_text) = strip_requirement_prefix(&raw_line) {
            out.push(RequirementGuard { raw_line, eval_text });
        }
        index = next;
    }

    out
}

fn should_extend_requirement(current: &str, next: &str) -> bool {
    if next.to_ascii_uppercase().starts_with("REQUIRE_PREDICATE")
        || next.to_ascii_uppercase().starts_with("BEGIN ")
    {
        return false;
    }
    paren_balance(current) > 0
        || current.trim_end().ends_with("AND")
        || current.trim_end().ends_with("OR")
        || current.trim_end().ends_with("&&")
        || current.trim_end().ends_with("||")
        || current.trim_end().ends_with('(')
        || next.to_ascii_uppercase().starts_with("AND ")
        || next.to_ascii_uppercase().starts_with("OR ")
        || next.starts_with("&&")
        || next.starts_with("||")
        || next.starts_with(')')
}

fn strip_requirement_prefix(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let upper = trimmed.to_ascii_uppercase();
    if !upper.starts_with("REQUIRE_PREDICATE") {
        return None;
    }
    let tail = trimmed["REQUIRE_PREDICATE".len()..].trim_start();
    if tail.is_empty() {
        None
    } else {
        Some(tail.to_string())
    }
}

fn parse_designated_id(upper_line: &str) -> Option<String> {
    if upper_line.trim_start().starts_with("//") {
        return None;
    }
    let index = upper_line.find("DESIGNATED")?;
    let tail = upper_line[index + "DESIGNATED".len()..].trim_start();
    let digits: String = tail.chars().take_while(|ch| ch.is_ascii_digit()).collect();
    if digits.is_empty() {
        None
    } else {
        let normalized = digits.trim_start_matches('0');
        if normalized.is_empty() {
            Some("0".to_string())
        } else {
            Some(normalized.to_string())
        }
    }
}

fn mismatch_source(tp2_path: &str, tp_file: &str) -> String {
    let trimmed = tp2_path.trim();
    if trimmed.is_empty() {
        tp_file.to_string()
    } else {
        trimmed.to_string()
    }
}

fn paren_balance(input: &str) -> i32 {
    let mut balance = 0i32;
    let mut quote = None::<char>;
    for ch in input.chars() {
        if let Some(active) = quote {
            if ch == active {
                quote = None;
            }
            continue;
        }
        if ch == '~' || ch == '"' {
            quote = Some(ch);
            continue;
        }
        if ch == '(' {
            balance += 1;
        } else if ch == ')' {
            balance -= 1;
        }
    }
    balance
}
