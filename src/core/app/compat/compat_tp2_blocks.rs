// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::tp2_component_begin::{code_part, component_begin_at, designated_id_in_code};

#[derive(Clone, Copy, PartialEq, Eq)]
enum LineState {
    Code,
    BlockComment,
    InlineFile,
}

pub(crate) fn collect_tp2_component_blocks<'a>(tp2_text: &'a str) -> Vec<(String, Vec<&'a str>)> {
    let lines: Vec<&'a str> = tp2_text.lines().collect();
    let mut state = LineState::Code;
    let mut blocks = Vec::<Vec<&'a str>>::new();
    for (index, &line) in lines.iter().enumerate() {
        let kept = kept_code(line, &mut state);
        if component_begin_at(kept, &lines, index).is_some() {
            blocks.push(Vec::new());
        }
        if let Some(block) = blocks.last_mut()
            && !kept.is_empty()
        {
            block.push(kept);
        }
    }

    let mut out = Vec::<(String, Vec<&'a str>)>::with_capacity(blocks.len());
    let mut next_number = 0u64;
    for block in blocks {
        let key = designated_in_block(&block).unwrap_or_else(|| next_number.to_string());
        next_number = key
            .parse::<u64>()
            .map_or(next_number, |number| number.saturating_add(1));
        out.push((key, block));
    }
    out
}

fn kept_code<'a>(line: &'a str, state: &mut LineState) -> &'a str {
    let trimmed = line.trim_start();
    match *state {
        LineState::InlineFile => {
            if trimmed.starts_with(">>>>>>>>") {
                *state = LineState::Code;
            }
            return "";
        }
        LineState::Code if trimmed.starts_with("<<<<<<<<") => {
            *state = LineState::InlineFile;
            return "";
        }
        LineState::Code | LineState::BlockComment => {}
    }
    let mut start = 0usize;
    if *state == LineState::BlockComment {
        let Some(close) = line.find("*/") else {
            return "";
        };
        start = close + 2;
        *state = LineState::Code;
    }
    let mut cursor = start;
    loop {
        let Some(open) = block_comment_open(&line[cursor..]).map(|offset| cursor + offset) else {
            return &line[start..];
        };
        let Some(close) = line[open + 2..].find("*/") else {
            *state = LineState::BlockComment;
            return &line[start..open];
        };
        cursor = open + 2 + close + 2;
    }
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

    #[test]
    fn lines_inside_a_commented_out_component_reach_no_block() {
        let fixture = [
            "BEGIN ~A~ DESIGNATED 1",
            "COPY ~a~ ~b~",
            "/*",
            "BEGIN ~Old~ DESIGNATED 2",
            "REQUIRE_COMPONENT ~foo.tp2~ ~3~ ~msg~",
            "*/",
            "BEGIN ~B~ DESIGNATED 3",
        ];
        let text = fixture.join("\n");
        let blocks = collect_tp2_component_blocks(&text);
        let block_keys: Vec<&str> = blocks.iter().map(|(key, _)| key.as_str()).collect();
        assert_eq!(block_keys, ["1", "3"]);
        assert!(
            blocks
                .iter()
                .flat_map(|(_, lines)| lines)
                .all(|line| !line.contains("REQUIRE_COMPONENT"))
        );
    }

    #[test]
    fn a_comment_opened_mid_line_keeps_the_code_before_it() {
        let fixture = [
            "BEGIN ~A~",
            "COPY ~a~ ~b~ /* start",
            "REQUIRE_COMPONENT ~x.tp2~ ~1~ ~m~",
            "end */ REQUIRE_PREDICATE (1) ~k~",
            "BEGIN ~B~",
        ];
        let text = fixture.join("\n");
        let blocks = collect_tp2_component_blocks(&text);
        let block_keys: Vec<&str> = blocks.iter().map(|(key, _)| key.as_str()).collect();
        assert_eq!(block_keys, ["0", "1"]);
        let first = &blocks[0].1;
        assert!(first.contains(&"COPY ~a~ ~b~ "));
        assert!(first.contains(&" REQUIRE_PREDICATE (1) ~k~"));
        assert!(first.iter().all(|line| !line.contains("REQUIRE_COMPONENT")));
    }

    #[test]
    fn an_inline_file_region_is_not_code() {
        let fixture = [
            "BACKUP ~x~",
            "AUTHOR ~x~",
            "ALWAYS",
            "<<<<<<<< .../x.d",
            "BEGIN ~DLG~",
            "IF ~~ THEN BEGIN 0 SAY ~hi~ END",
            ">>>>>>>>",
            "END",
            "BEGIN ~A~",
            "BEGIN ~B~",
        ];
        assert_eq!(keys(&fixture), ["0", "1"]);
    }

    #[test]
    fn the_counter_continues_from_the_last_component_not_the_maximum() {
        let fixture = [
            "BEGIN ~A~ DESIGNATED 50",
            "BEGIN ~B~ DESIGNATED 3",
            "BEGIN ~C~",
            "BEGIN ~D~",
        ];
        assert_eq!(keys(&fixture), ["50", "3", "4", "5"]);
    }

    #[test]
    fn a_same_line_block_comment_stays_in_the_line() {
        let line = "REQUIRE_COMPONENT ~x.tp2~ /* c */ ~1~ ~m~";
        let text = ["BEGIN ~A~", line].join("\n");
        let blocks = collect_tp2_component_blocks(&text);
        assert_eq!(blocks[0].0, "0");
        assert!(blocks[0].1.contains(&line));
    }
}
