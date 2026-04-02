// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashMap;
use std::fs;
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

use super::compat_path_eval::{PathRequirementContext, PathTriState, evaluate_path_requirement};
use crate::ui::step2::prompt_eval_expr_tokens_step2::{Token, tokenize};

pub(crate) type ComponentPathGuardCache = HashMap<String, HashMap<String, Vec<PathGuard>>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PathRequirementHit {
    pub(crate) kind: &'static str,
    pub(crate) related_target: Option<String>,
    pub(crate) message: String,
    pub(crate) source: String,
    pub(crate) raw_evidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PathGuard {
    raw_line: String,
    eval_text: String,
}

pub(crate) fn scan_path_requirement_hit(
    tp2_path: &str,
    component_id: &str,
    context: &PathRequirementContext,
    path_guard_cache: &mut ComponentPathGuardCache,
) -> Option<PathRequirementHit> {
    if tp2_path.trim().is_empty() {
        return None;
    }

    let guards_by_component = path_guard_cache
        .entry(tp2_path.to_string())
        .or_insert_with(|| load_component_path_guards(tp2_path));
    let guards = guards_by_component.get(component_id.trim())?;

    let failing_guard = guards.iter().find(|guard| {
        let outcome = evaluate_path_requirement(&guard.eval_text, context);
        outcome.used_supported_predicate && outcome.value == PathTriState::False
    })?;

    let (kind, related_target, message) = classify_failing_guard(&failing_guard.eval_text);
    Some(PathRequirementHit {
        kind,
        related_target,
        message,
        source: tp2_path.to_string(),
        raw_evidence: failing_guard.raw_line.clone(),
    })
}

fn classify_failing_guard(eval_text: &str) -> (&'static str, Option<String>, String) {
    if let Some(dir_target) = first_predicate_target(eval_text, "DIRECTORY_EXISTS") {
        return (
            "path_requirement",
            None,
            format!("Required folder `{dir_target}` is missing from the current game folder."),
        );
    }
    (
        "path_requirement",
        None,
        "Required folder is missing from the current game folder.".to_string(),
    )
}

fn first_predicate_target(eval_text: &str, predicate: &str) -> Option<String> {
    let tokens = tokenize(eval_text);
    let mut index = 0usize;
    while index < tokens.len() {
        let Token::Ident(name) = &tokens[index] else {
            index += 1;
            continue;
        };
        if !name.eq_ignore_ascii_case(predicate) {
            index += 1;
            continue;
        }
        let mut value_index = index + 1;
        if matches!(tokens.get(value_index), Some(Token::LParen)) {
            value_index += 1;
        }
        match tokens.get(value_index) {
            Some(Token::Atom(value)) | Some(Token::Ident(value)) if !value.trim().is_empty() => {
                return Some(value.trim().to_string());
            }
            _ => return None,
        }
    }
    None
}

fn load_component_path_guards(tp2_path: &str) -> HashMap<String, Vec<PathGuard>> {
    if tp2_path.trim().is_empty() {
        return HashMap::new();
    }
    let cache = path_guard_file_cache();
    let mut cache = cache.lock().expect("compat path guard cache lock poisoned");
    let stamp = cache_stamp(tp2_path);

    if let Some(entry) = cache.get(tp2_path)
        && entry.stamp == stamp
    {
        return entry.guards.clone();
    }

    let guards = load_component_path_guards_uncached(tp2_path);
    cache.insert(
        tp2_path.to_string(),
        CachedPathGuards {
            stamp,
            guards: guards.clone(),
        },
    );
    guards
}

fn load_component_path_guards_uncached(tp2_path: &str) -> HashMap<String, Vec<PathGuard>> {
    let Ok(tp2_text) = fs::read_to_string(tp2_path) else {
        return HashMap::new();
    };

    let mut out = HashMap::<String, Vec<PathGuard>>::new();
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

        let guards = collect_path_guards(block);
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
struct CachedPathGuards {
    stamp: FileCacheStamp,
    guards: HashMap<String, Vec<PathGuard>>,
}

fn path_guard_file_cache() -> &'static Mutex<HashMap<String, CachedPathGuards>> {
    static CACHE: OnceLock<Mutex<HashMap<String, CachedPathGuards>>> = OnceLock::new();
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

fn collect_path_guards(block: &[&str]) -> Vec<PathGuard> {
    let mut out = Vec::<PathGuard>::new();
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

        let mut raw_line = trimmed.to_string();
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
            raw_line.push(' ');
            raw_line.push_str(candidate);
            clean_line.push(' ');
            clean_line.push_str(cleaned_candidate);
            eval_text = strip(&clean_line);
            next += 1;
        }

        if let Some(eval_text) = eval_text.filter(|text| !text.trim().is_empty()) {
            out.push(PathGuard { raw_line, eval_text });
        }
        index = next;
    }

    out
}

fn strip_requirement_prefix(line: &str) -> Option<String> {
    let trimmed = strip_inline_comments(line);
    let trimmed = trimmed.trim_start();
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
    let trimmed = strip_inline_comments(line);
    let trimmed = trimmed.trim_start();
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
        return None;
    }
    let normalized = digits.trim_start_matches('0');
    if normalized.is_empty() {
        Some("0".to_string())
    } else {
        Some(normalized.to_string())
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
