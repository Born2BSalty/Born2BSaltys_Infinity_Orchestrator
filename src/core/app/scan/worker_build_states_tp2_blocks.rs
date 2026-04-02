// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub(super) struct Tp2ComponentBlock {
    pub component_id: String,
    pub begin_at_component_id: bool,
    pub group_key: Option<String>,
    pub subcomponent_key: Option<String>,
    pub body_lines: Vec<String>,
}

pub(super) fn parse_tp2_component_blocks(tp2_text: &str) -> HashMap<String, Tp2ComponentBlock> {
    let mut out = HashMap::<String, Tp2ComponentBlock>::new();
    let lines: Vec<&str> = tp2_text.lines().collect();
    let mut index = 0usize;
    let mut in_block_comment = false;
    while index < lines.len() {
        let line = lines[index];
        if !line_starts_begin_outside_block_comment(line, &mut in_block_comment) {
            index += 1;
            continue;
        }

        let mut end = index + 1;
        while end < lines.len() {
            if line_starts_begin_outside_block_comment(lines[end], &mut in_block_comment) {
                break;
            }
            end += 1;
        }

        let block = &lines[index..end];
        let component_id = block
            .iter()
            .find_map(|line| parse_designated_id(&line.to_ascii_uppercase()));
        if let Some(id) = component_id {
            out.insert(
                id.clone(),
                Tp2ComponentBlock {
                    component_id: id.clone(),
                    begin_at_component_id: false,
                    group_key: block.iter().find_map(|line| parse_group_key(line)),
                    subcomponent_key: block.iter().find_map(|line| parse_subcomponent_key(line)),
                    body_lines: block.iter().map(|line| (*line).to_string()).collect(),
                },
            );
        }
        index = end;
    }
    out
}

pub(super) fn parse_tp2_component_blocks_in_order(tp2_text: &str) -> Vec<Tp2ComponentBlock> {
    let mut out = Vec::<Tp2ComponentBlock>::new();
    let lines: Vec<&str> = tp2_text.lines().collect();
    let mut index = 0usize;
    let mut in_block_comment = false;
    while index < lines.len() {
        let line = lines[index];
        if !line_starts_begin_outside_block_comment(line, &mut in_block_comment) {
            index += 1;
            continue;
        }

        let mut end = index + 1;
        while end < lines.len() {
            if line_starts_begin_outside_block_comment(lines[end], &mut in_block_comment) {
                break;
            }
            end += 1;
        }

        let block = &lines[index..end];
        let designated_component_id = block
            .iter()
            .find_map(|line| parse_designated_id(&line.to_ascii_uppercase()));
        let begin_at_component_id =
            designated_component_id.is_none()
                && block
                    .first()
                    .and_then(|line| parse_begin_at_component_id(line))
                    .is_some();
        let component_id = designated_component_id
            .or_else(|| block.first().and_then(|line| parse_begin_at_component_id(line)))
            .unwrap_or_default();
        out.push(Tp2ComponentBlock {
            component_id,
            begin_at_component_id,
            group_key: block.iter().find_map(|line| parse_group_key(line)),
            subcomponent_key: block.iter().find_map(|line| parse_subcomponent_key(line)),
            body_lines: block.iter().map(|line| (*line).to_string()).collect(),
        });
        index = end;
    }
    out
}

fn line_starts_begin_outside_block_comment(line: &str, in_block_comment: &mut bool) -> bool {
    let trimmed = line.trim_start();
    if *in_block_comment {
        if trimmed.contains("*/") {
            *in_block_comment = false;
        }
        return false;
    }
    if trimmed.starts_with("/*") {
        if !trimmed.contains("*/") {
            *in_block_comment = true;
        }
        return false;
    }
    trimmed.to_ascii_uppercase().starts_with("BEGIN ")
}

fn parse_begin_at_component_id(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    if trimmed.starts_with("//") || !trimmed.to_ascii_uppercase().starts_with("BEGIN ") {
        return None;
    }
    let tail = trimmed["BEGIN".len()..].trim_start();
    let rest = tail.strip_prefix('@')?;
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
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

pub(super) fn split_subcomponent_display_label(label: &str) -> Option<(String, String)> {
    let (base, choice) = label.split_once("->")?;
    let base = base.trim();
    let choice = choice.trim();
    if base.is_empty() || choice.is_empty() {
        None
    } else {
        Some((base.to_string(), choice.to_string()))
    }
}

pub(super) fn parse_designated_id(upper_line: &str) -> Option<String> {
    if upper_line.trim_start().starts_with("//") {
        return None;
    }
    let index = upper_line.find("DESIGNATED")?;
    let tail = upper_line[index + "DESIGNATED".len()..].trim_start();
    let digits: String = tail.chars().take_while(|c| c.is_ascii_digit()).collect();
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

pub(super) fn extract_tilde_or_quote_paths(line: &str) -> Vec<String> {
    let mut out = Vec::<String>::new();
    let bytes = line.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        let quote = bytes[index];
        if quote != b'~' && quote != b'"' {
            index += 1;
            continue;
        }
        index += 1;
        let start = index;
        while index < bytes.len() && bytes[index] != quote {
            index += 1;
        }
        if index <= bytes.len() {
            let value = line[start..index].trim();
            if !value.is_empty() {
                out.push(value.to_string());
            }
        }
        index += 1;
    }
    out
}

fn parse_group_key(line: &str) -> Option<String> {
    if line.trim_start().starts_with("//") {
        return None;
    }
    let upper = line.to_ascii_uppercase();
    let index = upper.find("GROUP")?;
    let tail = line[index + "GROUP".len()..].trim_start();
    if tail.is_empty() {
        return None;
    }
    if let Some(rest) = tail.strip_prefix('~') {
        let end = rest.find('~')?;
        let value = rest[..end].trim();
        return (!value.is_empty()).then(|| format!("~{}~", value));
    }
    if let Some(rest) = tail.strip_prefix('"') {
        let end = rest.find('"')?;
        let value = rest[..end].trim();
        return (!value.is_empty()).then(|| format!("\"{}\"", value));
    }
    let value: String = tail
        .chars()
        .take_while(|c| !c.is_whitespace() && *c != '/')
        .collect();
    (!value.is_empty()).then_some(value)
}

fn parse_subcomponent_key(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    if trimmed.starts_with("//") {
        return None;
    }
    let upper = trimmed.to_ascii_uppercase();
    let keyword = if upper.starts_with("FORCED_SUBCOMPONENT ") {
        "FORCED_SUBCOMPONENT"
    } else if upper.starts_with("SUBCOMPONENT ") {
        "SUBCOMPONENT"
    } else {
        return None;
    };
    let tail = trimmed[keyword.len()..].trim_start();
    if tail.is_empty() {
        return None;
    }
    if let Some(rest) = tail.strip_prefix('~') {
        let end = rest.find('~')?;
        let value = rest[..end].trim();
        return (!value.is_empty()).then(|| value.to_string());
    }
    if let Some(rest) = tail.strip_prefix('"') {
        let end = rest.find('"')?;
        let value = rest[..end].trim();
        return (!value.is_empty()).then(|| value.to_string());
    }
    let value: String = tail
        .chars()
        .take_while(|c| !c.is_whitespace() && *c != '/')
        .collect();
    (!value.is_empty()).then_some(value)
}
