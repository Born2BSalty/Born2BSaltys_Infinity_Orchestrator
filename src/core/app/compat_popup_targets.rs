// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::compat_issue::CompatIssue;
use crate::app::selection_refs::normalize_mod_key;
use crate::parser::prompt_eval_expr_tokens::{Token, tokenize};

pub(crate) fn issue_related_target(issue: &CompatIssue) -> Option<(String, Option<u32>)> {
    explicit_related_target(Some(issue.related_mod.as_str()), issue.related_component)
        .or_else(|| extract_first_jump_target(issue.raw_evidence.as_deref()))
}

pub(crate) fn details_related_target(
    related_mod: Option<&str>,
    related_component: Option<&str>,
    raw_evidence: Option<&str>,
) -> Option<(String, Option<u32>)> {
    explicit_related_target(
        related_mod,
        related_component.and_then(|value| value.parse::<u32>().ok()),
    )
    .or_else(|| extract_first_jump_target(raw_evidence))
}

pub(crate) fn explicit_related_target(
    mod_ref: Option<&str>,
    component_ref: Option<u32>,
) -> Option<(String, Option<u32>)> {
    let mod_ref = mod_ref?.trim();
    if mod_ref.is_empty() || mod_ref.eq_ignore_ascii_case("unknown") {
        return None;
    }
    Some((mod_ref.to_string(), component_ref))
}

pub(crate) fn extract_first_jump_target(
    raw_evidence: Option<&str>,
) -> Option<(String, Option<u32>)> {
    let tokens = tokenize(raw_evidence?.trim());
    let mut index = 0usize;

    while index < tokens.len() {
        let Token::Ident(name) = &tokens[index] else {
            index += 1;
            continue;
        };
        if !name.eq_ignore_ascii_case("MOD_IS_INSTALLED") {
            index += 1;
            continue;
        }
        let Some(mod_ref) = token_value(tokens.get(index + 1)) else {
            index += 1;
            continue;
        };
        let Some(component_ref) = token_value(tokens.get(index + 2)).and_then(parse_component_id)
        else {
            index += 1;
            continue;
        };
        return Some((normalize_mod_key(&mod_ref), Some(component_ref)));
    }

    None
}

fn token_value(token: Option<&Token>) -> Option<String> {
    match token? {
        Token::Atom(value) | Token::Ident(value) => Some(value.trim().to_string()),
        _ => None,
    }
}

fn parse_component_id(value: String) -> Option<u32> {
    let trimmed = value
        .trim()
        .trim_matches(|ch: char| matches!(ch, '~' | '"' | '\''));
    let digits: String = trimmed
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect();
    digits.parse::<u32>().ok()
}
