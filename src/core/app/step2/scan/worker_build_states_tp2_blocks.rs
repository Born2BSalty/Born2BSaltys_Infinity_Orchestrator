// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashMap;

use crate::app::tp2_component_begin::{code_part, component_begin_at, designated_id_in_code};

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
    let mut in_block_comment = false;
    let mut next_start = find_next_begin_start(&lines, 0, &mut in_block_comment);
    while let Some((index, label_index)) = next_start {
        next_start = find_next_begin_start(&lines, index + 1, &mut in_block_comment);
        let end = next_start.map_or(lines.len(), |(found_index, _)| found_index);

        let block = &lines[index..end];
        let body_lines = build_block_body_lines(block, label_index - index);
        let component_id = block.iter().find_map(|line| designated_id_in_code(line));
        if let Some(id) = component_id {
            out.insert(
                id.clone(),
                Tp2ComponentBlock {
                    component_id: id.clone(),
                    begin_at_component_id: false,
                    group_key: block.iter().find_map(|line| parse_group_key(line)),
                    subcomponent_key: block.iter().find_map(|line| parse_subcomponent_key(line)),
                    body_lines,
                },
            );
        }
    }
    out
}

pub(super) fn parse_tp2_component_blocks_in_order(tp2_text: &str) -> Vec<Tp2ComponentBlock> {
    let mut out = Vec::<Tp2ComponentBlock>::new();
    let lines: Vec<&str> = tp2_text.lines().collect();
    let mut in_block_comment = false;
    let mut next_start = find_next_begin_start(&lines, 0, &mut in_block_comment);
    while let Some((index, label_index)) = next_start {
        next_start = find_next_begin_start(&lines, index + 1, &mut in_block_comment);
        let end = next_start.map_or(lines.len(), |(found_index, _)| found_index);

        let block = &lines[index..end];
        let body_lines = build_block_body_lines(block, label_index - index);
        let designated_component_id = block.iter().find_map(|line| designated_id_in_code(line));
        let begin_at_component_id = designated_component_id.is_none()
            && body_lines
                .first()
                .and_then(|line| parse_begin_at_component_id(line))
                .is_some();
        let component_id = designated_component_id
            .or_else(|| {
                body_lines
                    .first()
                    .and_then(|line| parse_begin_at_component_id(line))
            })
            .unwrap_or_default();
        out.push(Tp2ComponentBlock {
            component_id,
            begin_at_component_id,
            group_key: block.iter().find_map(|line| parse_group_key(line)),
            subcomponent_key: block.iter().find_map(|line| parse_subcomponent_key(line)),
            body_lines,
        });
    }
    out
}

fn build_block_body_lines(block: &[&str], label_offset: usize) -> Vec<String> {
    let mut body_lines: Vec<String> = block.iter().map(|line| (*line).to_string()).collect();
    if label_offset >= block.len() {
        return body_lines;
    }
    let first = begin_token_onward(block[0]);
    if label_offset == 0 {
        if first.len() != code_part(block[0]).trim().len() {
            body_lines[0] = first;
        }
        return body_lines;
    }
    body_lines[0] = format!("{first} {}", code_part(block[label_offset]).trim());
    body_lines
}

fn begin_token_onward(line: &str) -> String {
    let code = code_part(line).trim();
    let start = code.to_ascii_uppercase().find("BEGIN").unwrap_or(0);
    code[start..].to_string()
}

fn find_next_begin_start(
    lines: &[&str],
    from: usize,
    in_block_comment: &mut bool,
) -> Option<(usize, usize)> {
    let mut index = from;
    while index < lines.len() {
        if let Some(label_index) =
            line_starts_begin_outside_block_comment(lines, index, in_block_comment)
        {
            return Some((index, label_index));
        }
        index += 1;
    }
    None
}

fn line_starts_begin_outside_block_comment(
    lines: &[&str],
    index: usize,
    in_block_comment: &mut bool,
) -> Option<usize> {
    let line = lines[index];
    let trimmed = line.trim_start();
    if *in_block_comment {
        let pos = trimmed.find("*/")?;
        *in_block_comment = false;
        let remainder = &trimmed[pos + 2..];
        return component_begin_at(remainder, lines, index);
    }
    if trimmed.starts_with("/*") {
        if let Some(pos) = trimmed.find("*/") {
            let remainder = &trimmed[pos + 2..];
            return component_begin_at(remainder, lines, index);
        }
        *in_block_comment = true;
        return None;
    }
    component_begin_at(line, lines, index)
}

fn parse_begin_at_component_id(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    if trimmed.starts_with("//") || !trimmed.to_ascii_uppercase().starts_with("BEGIN ") {
        return None;
    }
    let tail = trimmed["BEGIN".len()..].trim_start();
    let rest = tail.strip_prefix('@')?;
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
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
    designated_id_in_code(upper_line)
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
        return (!value.is_empty()).then(|| format!("~{value}~"));
    }
    if let Some(rest) = tail.strip_prefix('"') {
        let end = rest.find('"')?;
        let value = rest[..end].trim();
        return (!value.is_empty()).then(|| format!("\"{value}\""));
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
    let content = trimmed.split("//").next().unwrap_or(trimmed).trim_end();
    let upper = content.to_ascii_uppercase();
    let keyword_index = if let Some(index) = upper.find("FORCED_SUBCOMPONENT ") {
        (index, "FORCED_SUBCOMPONENT")
    } else {
        let index = upper.find("SUBCOMPONENT ")?;
        (index, "SUBCOMPONENT")
    };
    let keyword = keyword_index.1;
    let tail = content[keyword_index.0 + keyword.len()..].trim_start();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_begin_followed_by_action_is_not_a_component_start() {
        let text = "BEGIN ~A~\nACTION_IF x THEN BEGIN\nCOPY ~a~ ~b~\nEND\n\nBEGIN ~B~\nbody\n";
        let blocks = parse_tp2_component_blocks_in_order(text);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].body_lines.len(), 5);
        assert_eq!(blocks[1].body_lines[0], "BEGIN ~B~");
    }

    #[test]
    fn label_on_next_line_is_the_block_label() {
        let text = "BEGIN\n~Widget~\nbody\n";
        let blocks = parse_tp2_component_blocks_in_order(text);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].body_lines[0], "BEGIN ~Widget~");
    }

    #[test]
    fn designated_in_comment_is_ignored_by_scan_splitter() {
        let text = "BEGIN ~A~ // DESIGNATED 99\nbody\n";
        let blocks = parse_tp2_component_blocks_in_order(text);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].component_id.len(), 0);
        assert!(!parse_tp2_component_blocks(text).contains_key("99"));
    }

    #[test]
    fn begin_after_block_comment_close_is_a_start() {
        let text = "BEGIN ~A~\nbody a\n/* note\n*/ BEGIN ~B~\nbody b\n";
        let blocks = parse_tp2_component_blocks_in_order(text);
        assert_eq!(blocks.len(), 2);
        assert_eq!(
            blocks[0].body_lines,
            vec![
                "BEGIN ~A~".to_string(),
                "body a".to_string(),
                "/* note".to_string(),
            ]
        );
        assert_eq!(blocks[1].body_lines[0], "BEGIN ~B~");
        assert_eq!(blocks[1].body_lines[1], "body b");
    }
}
