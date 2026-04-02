// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::{HashMap, HashSet};
use std::fs;
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

use crate::ui::state::{Step1State, Step2ModState};

use super::compat_mismatch_eval::{
    build_mismatch_context, evaluate_requirement, render_requirement_evidence, TriState,
};
use super::compat_rule_runtime::normalize_mod_key;

pub(crate) fn apply_step2_scan_mismatch(
    step1: &Step1State,
    tab: &str,
    mods: &mut [Step2ModState],
) {
    let context = build_mismatch_context(step1, tab, collect_checked_components(mods));
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
                    .then(|| guard.display_line.clone())
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

fn collect_checked_components(mods: &[Step2ModState]) -> HashSet<(String, String)> {
    let mut checked_components = HashSet::<(String, String)>::new();
    for mod_state in mods {
        let mod_key = normalize_mod_key(&mod_state.tp_file);
        for component in &mod_state.components {
            if component.checked {
                checked_components.insert((mod_key.clone(), component.component_id.trim().to_string()));
            }
        }
    }
    checked_components
}

#[derive(Debug, Clone)]
struct RequirementGuard {
    display_line: String,
    eval_text: String,
}

fn load_component_guards(tp2_path: &str) -> HashMap<String, Vec<RequirementGuard>> {
    if tp2_path.trim().is_empty() {
        return HashMap::new();
    }
    let cache = requirement_guard_file_cache();
    let mut cache = cache.lock().expect("compat mismatch cache lock poisoned");
    let stamp = cache_stamp(tp2_path);
    if let Some(entry) = cache.get(tp2_path)
        && entry.stamp == stamp
    {
        return entry.guards.clone();
    }
    let guards = load_component_guards_uncached(tp2_path);
    cache.insert(
        tp2_path.to_string(),
        CachedRequirementGuards {
            stamp,
            guards: guards.clone(),
        },
    );
    guards
}

fn load_component_guards_uncached(tp2_path: &str) -> HashMap<String, Vec<RequirementGuard>> {
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileCacheStamp {
    modified: Option<SystemTime>,
    len: u64,
}

#[derive(Debug, Clone)]
struct CachedRequirementGuards {
    stamp: FileCacheStamp,
    guards: HashMap<String, Vec<RequirementGuard>>,
}

fn requirement_guard_file_cache() -> &'static Mutex<HashMap<String, CachedRequirementGuards>> {
    static CACHE: OnceLock<Mutex<HashMap<String, CachedRequirementGuards>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn cache_stamp(tp2_path: &str) -> FileCacheStamp {
    match fs::metadata(tp2_path) {
        Ok(meta) => FileCacheStamp {
            modified: meta.modified().ok(),
            len: meta.len(),
        },
        Err(_) => FileCacheStamp {
            modified: None,
            len: 0,
        },
    }
}

fn collect_requirement_guards(block: &[&str]) -> Vec<RequirementGuard> {
    let mut out = Vec::<RequirementGuard>::new();
    let mut index = 0usize;

    while index < block.len() {
        let trimmed = block[index].trim();
        let upper = trimmed.to_ascii_uppercase();
        let strip = if upper.starts_with("REQUIRE_PREDICATE") {
            strip_requirement_prefix as fn(&str) -> Option<String>
        } else if upper.starts_with("SUBCOMPONENT ") {
            strip_subcomponent_condition as fn(&str) -> Option<String>
        } else {
            index += 1;
            continue;
        };

        let mut clean_line = strip_inline_comments(trimmed);
        let mut eval_text = strip(&clean_line);
        let mut next = index + 1;
        while next < block.len() {
            let candidate = block[next].trim();
            let cleaned_candidate = strip_inline_comments(candidate);
            let cleaned_candidate = cleaned_candidate.trim();
            if cleaned_candidate.is_empty() {
                next += 1;
                continue;
            }
            if !should_extend_guard(eval_text.as_deref().unwrap_or(""), cleaned_candidate) {
                break;
            }
            clean_line.push(' ');
            clean_line.push_str(cleaned_candidate);
            eval_text = strip(&clean_line);
            next += 1;
        }

        if let Some(eval_text) = eval_text.filter(|text| !text.trim().is_empty()) {
            let display_line = build_display_line(trimmed, &clean_line, &eval_text);
            out.push(RequirementGuard {
                display_line,
                eval_text,
            });
        }
        index = next;
    }

    out
}

fn build_display_line(trimmed: &str, clean_line: &str, eval_text: &str) -> String {
    let display_expr = render_requirement_evidence(eval_text);
    let upper = trimmed.to_ascii_uppercase();
    if upper.starts_with("REQUIRE_PREDICATE") {
        return display_expr
            .map(|expr| format!("REQUIRE_PREDICATE {expr}"))
            .unwrap_or_else(|| "REQUIRE_PREDICATE".to_string());
    }
    if upper.starts_with("SUBCOMPONENT") {
        let prefix = clean_line
            .split_once('(')
            .map(|(head, _)| head.trim())
            .unwrap_or(trimmed.trim());
        return match display_expr {
            Some(expr) if expr.starts_with('(') => format!("{prefix} {expr}"),
            Some(expr) => format!("{prefix} ({expr})"),
            None => prefix.to_string(),
        };
    }
    display_expr.unwrap_or_else(|| clean_line.trim().to_string())
}

fn should_extend_guard(current: &str, next: &str) -> bool {
    if next.to_ascii_uppercase().starts_with("REQUIRE_PREDICATE")
        || next.to_ascii_uppercase().starts_with("SUBCOMPONENT ")
        || next.to_ascii_uppercase().starts_with("BEGIN ")
    {
        return false;
    }
    if current.trim().is_empty() {
        return next.starts_with('(');
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

fn strip_subcomponent_condition(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let upper = trimmed.to_ascii_uppercase();
    if !upper.starts_with("SUBCOMPONENT") {
        return None;
    }
    let tail = trimmed["SUBCOMPONENT".len()..].trim_start();
    if tail.is_empty() {
        return None;
    }
    let tail = if let Some(rest) = tail.strip_prefix('~') {
        let end = rest.find('~')?;
        &rest[end + 1..]
    } else if let Some(rest) = tail.strip_prefix('"') {
        let end = rest.find('"')?;
        &rest[end + 1..]
    } else {
        let split = tail
            .find(|ch: char| ch.is_whitespace() || ch == '/')
            .unwrap_or(tail.len());
        &tail[split..]
    };
    let tail = tail.trim_start();
    if tail.is_empty() {
        None
    } else {
        Some(tail.to_string())
    }
}

fn strip_inline_comments(line: &str) -> String {
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::new();
    let mut index = 0usize;
    let mut quote = None::<char>;

    while index < chars.len() {
        let ch = chars[index];
        if let Some(active) = quote {
            out.push(ch);
            if ch == active {
                quote = None;
            }
            index += 1;
            continue;
        }

        if ch == '~' || ch == '"' {
            quote = Some(ch);
            out.push(ch);
            index += 1;
            continue;
        }

        if ch == '/' && chars.get(index + 1) == Some(&'/') {
            break;
        }
        if ch == '/' && chars.get(index + 1) == Some(&'*') {
            break;
        }

        out.push(ch);
        index += 1;
    }

    out
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
