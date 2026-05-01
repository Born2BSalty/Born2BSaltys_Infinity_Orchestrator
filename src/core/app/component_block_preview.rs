// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;

const DETAILS_COMPONENT_PREVIEW_MAX_LINES: usize = 48;
const DETAILS_COMPONENT_PREVIEW_MAX_CHARS: usize = 3_000;
const DETAILS_COMPONENT_PREVIEW_SOFT_EXTEND_LINES: usize = 12;

pub(crate) fn load_component_block_preview(tp2_path: &str, component_id: &str) -> Option<String> {
    if tp2_path.trim().is_empty() || component_id.trim().is_empty() {
        return None;
    }
    let tp2_text = fs::read_to_string(tp2_path).ok()?;
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
        let Some(block_id) = block
            .iter()
            .find_map(|entry| parse_designated_id(&entry.to_ascii_uppercase()))
        else {
            continue;
        };

        if block_id.eq_ignore_ascii_case(component_id.trim()) {
            let display_start = component_block_start(&lines, start);
            let display_end = component_block_end(&lines, display_start, index);
            let mut preview = lines[display_start..display_end].join("\n");
            if display_end < index {
                preview.push_str("\n...");
            }
            return Some(preview);
        }
    }

    None
}

fn component_block_start(lines: &[&str], block_start: usize) -> usize {
    let mut start = block_start;
    let mut saw_comment = false;

    while start > 0 {
        let prev = lines[start - 1].trim();
        if prev.is_empty() {
            start -= 1;
            continue;
        }
        if is_component_header_comment(prev) {
            saw_comment = true;
            start -= 1;
            continue;
        }
        break;
    }

    if saw_comment { start } else { block_start }
}

fn component_block_end(lines: &[&str], block_start: usize, block_end: usize) -> usize {
    let mut end = block_start;
    let mut char_count = 0usize;

    while end < block_end {
        char_count += lines[end].len() + 1;
        end += 1;
        if end - block_start >= DETAILS_COMPONENT_PREVIEW_MAX_LINES
            || char_count >= DETAILS_COMPONENT_PREVIEW_MAX_CHARS
        {
            break;
        }
    }

    if end >= block_end {
        while end > block_start && lines[end - 1].trim().is_empty() {
            end -= 1;
        }
        return end;
    }

    let mut extended = end;
    let soft_limit = (end + DETAILS_COMPONENT_PREVIEW_SOFT_EXTEND_LINES).min(block_end);
    while extended < soft_limit {
        let line = lines[extended].trim();
        extended += 1;
        if line.is_empty() {
            end = extended;
            break;
        }
        if is_component_section_banner_line(line) {
            end = extended.saturating_sub(1);
            break;
        }
    }

    while end > block_start && lines[end - 1].trim().is_empty() {
        end -= 1;
    }

    if end == block_start {
        let mut fallback = block_start + 1;
        while fallback < block_end {
            let line = lines[fallback].trim();
            if line.is_empty()
                || is_component_header_comment(line)
                || is_component_header_line(line)
            {
                fallback += 1;
                continue;
            }
            break;
        }
        while fallback > block_start && lines[fallback - 1].trim().is_empty() {
            fallback -= 1;
        }
        return fallback.max((block_start + 1).min(block_end));
    }

    end
}

fn is_component_header_comment(line: &str) -> bool {
    line.starts_with("//")
        || line.starts_with("/*")
        || line.starts_with('*')
        || line.starts_with("*/")
}

fn is_component_section_banner_line(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return false;
    }
    let slash_count = trimmed.chars().filter(|&ch| ch == '/').count();
    let backslash_count = trimmed.chars().filter(|&ch| ch == '\\').count();
    slash_count >= 6 && backslash_count >= 6
}

fn is_component_header_line(line: &str) -> bool {
    let upper = line.trim_start().to_ascii_uppercase();
    upper.starts_with("REQUIRE_")
        || upper.starts_with("FORBID_")
        || upper.starts_with("DESIGNATED")
        || upper.starts_with("LABEL")
        || upper.starts_with("GROUP")
        || upper.starts_with("SUBCOMPONENT")
        || upper.starts_with("VERSION")
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
