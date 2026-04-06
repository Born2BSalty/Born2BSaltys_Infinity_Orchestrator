// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashMap;
use std::fs;
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

use crate::ui::step2::prompt_eval_expr_tokens_step2::{tokenize, Token};

use super::compat_rule_runtime::normalize_mod_key;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ComponentRequirement {
    pub(crate) raw_line: String,
    pub(crate) targets: Vec<ComponentRequirementTarget>,
    pub(crate) message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ComponentRequirementTarget {
    pub(crate) target_mod: String,
    pub(crate) target_component_id: String,
}

pub(crate) fn load_component_requirements(
    tp2_path: &str,
) -> HashMap<String, Vec<ComponentRequirement>> {
    if tp2_path.trim().is_empty() {
        return HashMap::new();
    }
    let cache = requirement_cache();
    let mut cache = cache.lock().expect("compat dependency cache lock poisoned");
    let stamp = cache_stamp(tp2_path);

    if let Some(entry) = cache.get(tp2_path)
        && entry.stamp == stamp
    {
        return entry.requirements.clone();
    }

    let requirements = load_component_requirements_uncached(tp2_path);
    cache.insert(
        tp2_path.to_string(),
        CachedRequirements {
            stamp,
            requirements: requirements.clone(),
        },
    );
    requirements
}

fn load_component_requirements_uncached(
    tp2_path: &str,
) -> HashMap<String, Vec<ComponentRequirement>> {
    let Ok(tp2_text) = fs::read_to_string(tp2_path) else {
        return HashMap::new();
    };

    let mut out = HashMap::<String, Vec<ComponentRequirement>>::new();
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

        let requirements = collect_component_requirements(block);
        if !requirements.is_empty() {
            out.insert(component_id, requirements);
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
struct CachedRequirements {
    stamp: FileCacheStamp,
    requirements: HashMap<String, Vec<ComponentRequirement>>,
}

fn requirement_cache() -> &'static Mutex<HashMap<String, CachedRequirements>> {
    static CACHE: OnceLock<Mutex<HashMap<String, CachedRequirements>>> = OnceLock::new();
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

fn collect_component_requirements(block: &[&str]) -> Vec<ComponentRequirement> {
    let mut out = Vec::<ComponentRequirement>::new();
    for line in block {
        let Some(requirement) = parse_requirement_line(line)
            .or_else(|| parse_predicate_requirement_line(line))
        else {
            continue;
        };
        out.push(requirement);
    }
    out
}

fn parse_requirement_line(line: &str) -> Option<ComponentRequirement> {
    let stripped = strip_inline_comments(line);
    let trimmed = stripped.trim();
    let upper = trimmed.to_ascii_uppercase();
    if !upper.starts_with("REQUIRE_COMPONENT") {
        return None;
    }

    let tail = trimmed["REQUIRE_COMPONENT".len()..].trim_start();
    if tail.is_empty() {
        return None;
    }

    let mut index = 0usize;
    let tp2_ref = read_statement_token(tail, &mut index)?;
    let component_ref = read_statement_token(tail, &mut index)?;
    let target_mod = normalize_mod_key(&tp2_ref);
    let target_component_id = normalize_component_id(&component_ref)?;
    let message = read_statement_token(tail, &mut index).and_then(|value| {
        let value = value.trim();
        if value.is_empty() || value.starts_with('@') {
            None
        } else {
            Some(value.to_string())
        }
    });

    if target_mod.is_empty() {
        return None;
    }

    Some(ComponentRequirement {
        raw_line: trimmed.to_string(),
        targets: vec![ComponentRequirementTarget {
            target_mod,
            target_component_id,
        }],
        message,
    })
}

fn parse_predicate_requirement_line(line: &str) -> Option<ComponentRequirement> {
    let stripped = strip_inline_comments(line);
    let trimmed = stripped.trim();
    let upper = trimmed.to_ascii_uppercase();
    if !upper.starts_with("REQUIRE_PREDICATE") {
        return None;
    }

    let tail = trimmed["REQUIRE_PREDICATE".len()..].trim_start();
    let targets = parse_mod_is_installed_dependency_targets(tail)?;
    if targets.is_empty() {
        return None;
    }

    Some(ComponentRequirement {
        raw_line: trimmed.to_string(),
        targets,
        message: None,
    })
}

pub(crate) fn parse_mod_is_installed_dependency_targets(
    input: &str,
) -> Option<Vec<ComponentRequirementTarget>> {
    let tokens = tokenize(input);
    let mut index = 0usize;
    let mut out = Vec::<ComponentRequirementTarget>::new();

    loop {
        while matches!(tokens.get(index), Some(Token::LParen)) {
            index += 1;
        }

        let Token::Ident(name) = tokens.get(index)? else {
            return None;
        };
        if !name.eq_ignore_ascii_case("MOD_IS_INSTALLED") {
            return None;
        }
        index += 1;

        let target_mod = normalize_mod_key(&token_value(tokens.get(index)?)?);
        index += 1;
        let target_component_id = normalize_component_id(&token_value(tokens.get(index)?)?)?;
        index += 1;

        while matches!(tokens.get(index), Some(Token::RParen)) {
            index += 1;
        }

        if target_mod.is_empty() {
            return None;
        }
        out.push(ComponentRequirementTarget {
            target_mod,
            target_component_id,
        });

        if !matches!(tokens.get(index), Some(Token::Or)) {
            break;
        }
        index += 1;
    }

    if out.is_empty() {
        return None;
    }

    if tokens[index..].iter().any(|token| {
        matches!(
            token,
            Token::LParen
                | Token::RParen
                | Token::Bang
                | Token::Eq
                | Token::Gt
                | Token::Lt
                | Token::And
                | Token::Or
                | Token::Not
        ) || matches!(token, Token::Ident(name) if name.eq_ignore_ascii_case("MOD_IS_INSTALLED"))
    }) {
        return None;
    }

    Some(out)
}

pub(crate) fn parse_negated_mod_is_installed_targets(
    input: &str,
) -> Option<Vec<ComponentRequirementTarget>> {
    let tokens = tokenize(input);
    let mut index = 0usize;
    let mut out = Vec::<ComponentRequirementTarget>::new();

    while index < tokens.len() {
        if !matches!(tokens.get(index), Some(Token::Bang) | Some(Token::Not)) {
            index += 1;
            continue;
        }

        let mut cursor = index + 1;
        let wrapped = matches!(tokens.get(cursor), Some(Token::LParen));
        if wrapped {
            cursor += 1;
        }

        let Some(Token::Ident(name)) = tokens.get(cursor) else {
            index += 1;
            continue;
        };
        if !name.eq_ignore_ascii_case("MOD_IS_INSTALLED") {
            index += 1;
            continue;
        }
        cursor += 1;

        let opened = matches!(tokens.get(cursor), Some(Token::LParen));
        if opened {
            cursor += 1;
        }

        let Some(target_mod) = tokens
            .get(cursor)
            .and_then(token_value)
            .map(|value| normalize_mod_key(&value))
        else {
            index += 1;
            continue;
        };
        cursor += 1;

        let Some(target_component_id) = tokens
            .get(cursor)
            .and_then(token_value)
            .and_then(|value| normalize_component_id(&value))
        else {
            index += 1;
            continue;
        };
        cursor += 1;

        if opened && !matches!(tokens.get(cursor), Some(Token::RParen)) {
            index += 1;
            continue;
        }
        if opened {
            cursor += 1;
        }

        if wrapped && !matches!(tokens.get(cursor), Some(Token::RParen)) {
            index += 1;
            continue;
        }
        if wrapped {
            cursor += 1;
        }

        if !target_mod.is_empty()
            && !out.iter().any(|target| {
                target.target_mod == target_mod && target.target_component_id == target_component_id
            })
        {
            out.push(ComponentRequirementTarget {
                target_mod,
                target_component_id,
            });
        }

        index = cursor;
    }

    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

#[cfg(test)]
pub(crate) fn parse_simple_mod_is_installed_predicate(input: &str) -> Option<(String, String)> {
    let mut targets = parse_mod_is_installed_dependency_targets(input)?;
    if targets.len() != 1 {
        return None;
    }
    let target = targets.pop()?;
    Some((target.target_mod, target.target_component_id))
}

fn token_value(token: &Token) -> Option<String> {
    match token {
        Token::Atom(value) | Token::Ident(value) => Some(value.trim().to_string()),
        _ => None,
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

fn read_statement_token(input: &str, index: &mut usize) -> Option<String> {
    let bytes = input.as_bytes();
    while *index < bytes.len() && bytes[*index].is_ascii_whitespace() {
        *index += 1;
    }
    if *index >= bytes.len() {
        return None;
    }

    let quote = bytes[*index];
    if quote == b'~' || quote == b'"' {
        *index += 1;
        let start = *index;
        while *index < bytes.len() && bytes[*index] != quote {
            *index += 1;
        }
        let value = input[start..(*index).min(bytes.len())].trim().to_string();
        if *index < bytes.len() {
            *index += 1;
        }
        return Some(value);
    }

    let start = *index;
    while *index < bytes.len() && !bytes[*index].is_ascii_whitespace() {
        *index += 1;
    }
    Some(input[start..*index].trim().to_string())
}

fn normalize_component_id(value: &str) -> Option<String> {
    let trimmed = value
        .trim()
        .trim_matches(|ch: char| matches!(ch, '~' | '"' | '\''));
    let digits: String = trimmed.chars().take_while(|ch| ch.is_ascii_digit()).collect();
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

#[cfg(test)]
#[path = "compat_dependency_parse_tests.rs"]
mod tests;
