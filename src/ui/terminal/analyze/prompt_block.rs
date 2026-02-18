// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use super::model::{PromptInfo, PromptKind};
use super::prompt_detect::is_prompt_line;
use super::utils::{fnv1a64, normalize_prompt_key, recent_lines, strip_log_prefix};

pub(crate) fn current_prompt_info(output: &str) -> Option<PromptInfo> {
    let lines = recent_lines(output, 1200);
    let mut anchor_idx: Option<usize> = None;
    let mut legacy_key: Option<String> = None;
    for (idx, line) in lines.iter().enumerate().rev() {
        let stripped = strip_log_prefix(line).trim();
        if stripped.is_empty() {
            continue;
        }
        if is_prompt_line(stripped) && legacy_key.is_none() {
            let key = normalize_prompt_key(stripped);
            if !key.is_empty() {
                legacy_key = Some(key);
            }
        }
        if is_prompt_line(stripped) || is_numbered_option_line(stripped) {
            anchor_idx = Some(idx);
            break;
        }
    }

    let Some(anchor_idx) = anchor_idx else {
        return legacy_key.map(|key| PromptInfo {
            key,
            legacy_key: None,
            preview_line: String::new(),
            kind: PromptKind::FreeText,
            option_count: 0,
            line_count: 0,
            char_count: 0,
        });
    };

    let mut start = anchor_idx;
    let mut blank_budget = 2usize;
    while start > 0 {
        let prev = strip_log_prefix(lines[start - 1]).trim();
        if prev.is_empty() {
            if blank_budget == 0 {
                break;
            }
            blank_budget = blank_budget.saturating_sub(1);
            start -= 1;
            continue;
        }
        if is_prompt_block_line(prev) {
            start -= 1;
            blank_budget = 2;
            continue;
        }
        break;
    }

    let mut end = anchor_idx;
    blank_budget = 2;
    while end + 1 < lines.len() {
        let next = strip_log_prefix(lines[end + 1]).trim();
        if next.is_empty() {
            if blank_budget == 0 {
                break;
            }
            blank_budget = blank_budget.saturating_sub(1);
            end += 1;
            continue;
        }
        if is_prompt_block_line(next) {
            end += 1;
            blank_budget = 2;
            continue;
        }
        break;
    }

    let mut normalized_lines = Vec::new();
    let mut option_count = 0usize;
    let mut preview_line = String::new();
    let mut raw_lines = Vec::new();
    for line in &lines[start..=end] {
        let stripped = strip_log_prefix(line).trim();
        if stripped.is_empty() {
            continue;
        }
        raw_lines.push(stripped.to_string());
        if preview_line.is_empty() && !is_numbered_option_line(stripped) {
            preview_line = abbreviate_preview(stripped);
        }
        if is_numbered_option_line(stripped) {
            option_count = option_count.saturating_add(1);
        }
        let norm = normalize_prompt_key(stripped);
        if !norm.is_empty() {
            normalized_lines.push(norm);
        }
    }

    if normalized_lines.is_empty() {
        return legacy_key.map(|key| PromptInfo {
            key,
            legacy_key: None,
            preview_line: String::new(),
            kind: PromptKind::FreeText,
            option_count: 0,
            line_count: 0,
            char_count: 0,
        });
    }

    let block = normalized_lines.join("\n");
    let kind = classify_prompt_kind(&raw_lines, option_count);
    let line_count = raw_lines.len();
    let char_count = raw_lines.iter().map(|l| l.chars().count()).sum();
    Some(PromptInfo {
        key: format!("BLOCK:{:016X}", fnv1a64(&block)),
        legacy_key,
        preview_line,
        kind,
        option_count,
        line_count,
        char_count,
    })
}

fn classify_prompt_kind(raw_lines: &[String], option_count: usize) -> PromptKind {
    let joined_upper = raw_lines
        .iter()
        .map(|l| l.to_ascii_uppercase())
        .collect::<Vec<_>>()
        .join("\n");
    if joined_upper.contains("[Y]ES") && joined_upper.contains("[N]O") {
        return PromptKind::YesNo;
    }
    if joined_upper.contains("FULL PATH")
        || joined_upper.contains("DIRECTORY")
        || joined_upper.contains("PATH TO")
        || joined_upper.contains("INSTALLATION THEN PRESS ENTER")
    {
        return PromptKind::Path;
    }
    if option_count > 0
        || joined_upper.contains("PLEASE ENTER NUMBER")
        || joined_upper.contains("PLEASE CHOOSE")
        || joined_upper.contains("NUMBER OF")
    {
        return PromptKind::Number;
    }
    PromptKind::FreeText
}

fn abbreviate_preview(value: &str) -> String {
    const MAX: usize = 120;
    let mut out = String::new();
    for (i, ch) in value.chars().enumerate() {
        if i >= MAX {
            out.push_str("...");
            break;
        }
        out.push(ch);
    }
    out
}

fn is_prompt_block_line(line: &str) -> bool {
    let u = line.to_ascii_uppercase();
    is_prompt_line(&u)
        || is_numbered_option_line(&u)
        || u.contains("PLEASE CHOOSE")
        || u.contains("PLEASE ENTER")
        || u.contains("SELECT ONE")
        || u.contains("IS THIS CORRECT?")
        || u.contains("[I]NSTALL")
        || u.contains("[Q]UIT")
        || u.contains("[C]ANCEL")
        || u.contains("PROCE")
}

fn is_numbered_option_line(line: &str) -> bool {
    let t = line.trim_start();
    if let Some(rest) = t.strip_prefix('[') {
        let mut saw_digit = false;
        for ch in rest.chars() {
            if ch.is_ascii_digit() {
                saw_digit = true;
                continue;
            }
            return saw_digit && ch == ']';
        }
        return false;
    }

    let mut chars = t.chars().peekable();
    let mut saw_digit = false;
    while let Some(ch) = chars.peek() {
        if ch.is_ascii_digit() {
            saw_digit = true;
            let _ = chars.next();
        } else {
            break;
        }
    }
    if !saw_digit {
        return false;
    }
    matches!(chars.next(), Some('.') | Some(')'))
}
