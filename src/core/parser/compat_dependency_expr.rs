// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::parser::prompt_eval_expr_tokens::{Token, tokenize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParsedComponentRequirement {
    pub(crate) raw_line: String,
    pub(crate) targets: Vec<ParsedDependencyTarget>,
    pub(crate) message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParsedDependencyTarget {
    pub(crate) target_mod: String,
    pub(crate) target_component_id: String,
}

pub(crate) fn collect_component_requirements(block: &[&str]) -> Vec<ParsedComponentRequirement> {
    let mut out = Vec::<ParsedComponentRequirement>::new();
    for line in block {
        let Some(requirement) =
            parse_requirement_line(line).or_else(|| parse_predicate_requirement_line(line))
        else {
            continue;
        };
        out.push(requirement);
    }
    out
}

pub(crate) fn parse_requirement_line(line: &str) -> Option<ParsedComponentRequirement> {
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

    Some(ParsedComponentRequirement {
        raw_line: trimmed.to_string(),
        targets: vec![ParsedDependencyTarget {
            target_mod,
            target_component_id,
        }],
        message,
    })
}

pub(crate) fn parse_predicate_requirement_line(line: &str) -> Option<ParsedComponentRequirement> {
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

    Some(ParsedComponentRequirement {
        raw_line: trimmed.to_string(),
        targets,
        message: None,
    })
}

pub(crate) fn parse_mod_is_installed_dependency_targets(
    input: &str,
) -> Option<Vec<ParsedDependencyTarget>> {
    let tokens = tokenize(input);
    let mut index = 0usize;
    let mut out = Vec::<ParsedDependencyTarget>::new();

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
        out.push(ParsedDependencyTarget {
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
) -> Option<Vec<ParsedDependencyTarget>> {
    let tokens = tokenize(input);
    let mut index = 0usize;
    let mut out = Vec::<ParsedDependencyTarget>::new();

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
            out.push(ParsedDependencyTarget {
                target_mod,
                target_component_id,
            });
        }

        index = cursor;
    }

    if out.is_empty() { None } else { Some(out) }
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

pub(crate) fn normalize_component_id(value: &str) -> Option<String> {
    let trimmed = value
        .trim()
        .trim_matches(|ch: char| matches!(ch, '~' | '"' | '\''));
    let digits: String = trimmed
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect();
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

fn normalize_mod_key(value: &str) -> String {
    let lower = value.to_ascii_lowercase();
    let file = if let Some(idx) = lower.rfind(['/', '\\']) {
        &lower[idx + 1..]
    } else {
        &lower
    };
    let without_ext = file.strip_suffix(".tp2").unwrap_or(file);
    without_ext
        .strip_prefix("setup-")
        .unwrap_or(without_ext)
        .to_string()
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
