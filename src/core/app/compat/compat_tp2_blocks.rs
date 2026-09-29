// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::tp2_component_begin::{code_part, component_begin_at, designated_id_in_code};

pub(crate) fn collect_tp2_component_blocks<'a>(tp2_text: &'a str) -> Vec<(String, Vec<&'a str>)> {
    let lines: Vec<&'a str> = tp2_text.lines().collect();
    let mut in_block_comment = false;
    let starts: Vec<usize> = (0..lines.len())
        .filter(|&index| {
            line_starts_begin_outside_block_comment(&lines, index, &mut in_block_comment).is_some()
        })
        .collect();

    let mut out = Vec::<(String, Vec<&'a str>)>::with_capacity(starts.len());
    let mut next_number = 0u64;
    for (position, &start) in starts.iter().enumerate() {
        let end = starts.get(position + 1).copied().unwrap_or(lines.len());
        let block = lines[start..end].to_vec();
        let key = designated_in_block(&block).unwrap_or_else(|| next_number.to_string());
        next_number = key
            .parse::<u64>()
            .map_or(next_number, |number| number.saturating_add(1));
        out.push((key, block));
    }
    out
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

fn designated_in_block(block: &[&str]) -> Option<String> {
    let mut in_block_comment = false;
    block.iter().find_map(|line| {
        designated_id_in_code(&code_outside_block_comments(line, &mut in_block_comment))
    })
}

fn code_outside_block_comments(line: &str, in_block_comment: &mut bool) -> String {
    let mut code = String::new();
    let mut rest = line;
    loop {
        if *in_block_comment {
            let Some(close) = rest.find("*/") else {
                return code;
            };
            rest = &rest[close + 2..];
            *in_block_comment = false;
        }
        let Some(open) = block_comment_open(rest) else {
            code.push_str(rest);
            return code;
        };
        code.push_str(&rest[..open]);
        code.push(' ');
        rest = &rest[open + 2..];
        *in_block_comment = true;
    }
}

fn block_comment_open(text: &str) -> Option<usize> {
    let bytes = code_part(text).as_bytes();
    let mut in_tilde = false;
    let mut in_quote = false;
    let mut in_percent = false;
    for (index, &byte) in bytes.iter().enumerate() {
        match byte {
            b'~' if !in_quote && !in_percent => in_tilde = !in_tilde,
            b'"' if !in_tilde && !in_percent => in_quote = !in_quote,
            b'%' if !in_tilde && !in_quote => in_percent = !in_percent,
            b'/' if !in_tilde
                && !in_quote
                && !in_percent
                && bytes.get(index + 1) == Some(&b'*') =>
            {
                return Some(index);
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::collect_tp2_component_blocks;

    fn keys(lines: &[&str]) -> Vec<String> {
        collect_tp2_component_blocks(&lines.join("\n"))
            .into_iter()
            .map(|(key, _)| key)
            .collect()
    }

    #[test]
    fn unnumbered_components_continue_from_the_previous_number() {
        let fixture = [
            "BACKUP ~x~",
            "AUTHOR ~x~",
            "BEGIN ~A~",
            "BEGIN ~B~ DESIGNATED 50",
            "BEGIN ~C~",
            "/* BEGIN ~D~ */",
            "BEGIN ~E~",
        ];
        assert_eq!(keys(&fixture), ["0", "50", "51", "52"]);
    }

    #[test]
    fn a_begin_inside_a_block_comment_is_not_a_component() {
        let fixture = [
            "BEGIN ~A~",
            "COPY ~a~ ~b~",
            "/* disabled",
            "BEGIN ~X~",
            "COPY ~c~ ~d~",
            "*/",
            "BEGIN ~B~",
        ];
        assert_eq!(keys(&fixture), ["0", "1"]);
    }

    #[test]
    fn a_designated_inside_a_comment_does_not_number_the_block() {
        assert_eq!(keys(&["BEGIN ~A~ // DESIGNATED 99"]), ["0"]);
        assert_eq!(keys(&["BEGIN ~A~ /* DESIGNATED 99 */"]), ["0"]);
    }

    #[test]
    fn a_begin_with_its_label_on_the_next_line_is_a_component() {
        let fixture = ["BEGIN", "~A~", "COPY ~a~ ~b~", "BEGIN ~B~"];
        assert_eq!(keys(&fixture), ["0", "1"]);
    }

    #[test]
    fn every_numbered_block_keeps_its_designated_key() {
        let fixture = [
            "BEGIN ~A~ DESIGNATED 7",
            "BEGIN ~B~",
            "DESIGNATED 3",
            "BEGIN ~C~ DESIGNATED 9",
        ];
        assert_eq!(keys(&fixture), ["7", "3", "9"]);
    }
}
