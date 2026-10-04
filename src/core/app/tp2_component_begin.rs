// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

pub(crate) fn code_part(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut in_tilde = false;
    let mut in_quote = false;
    let mut in_percent = false;
    let mut index = 0usize;
    while index < bytes.len() {
        match bytes[index] {
            b'~' if !in_quote && !in_percent => in_tilde = !in_tilde,
            b'"' if !in_tilde && !in_percent => in_quote = !in_quote,
            b'%' if !in_tilde && !in_quote => in_percent = !in_percent,
            b'/' if !in_tilde
                && !in_quote
                && !in_percent
                && bytes.get(index + 1) == Some(&b'/') =>
            {
                return &line[..index];
            }
            _ => {}
        }
        index += 1;
    }
    line
}

pub(crate) fn is_label_token_start(text: &str) -> bool {
    matches!(
        text.trim_start().chars().next(),
        Some('~' | '"' | '@' | '%')
    )
}

pub(crate) fn component_begin_at(line_code: &str, lines: &[&str], index: usize) -> Option<usize> {
    let code = code_part(line_code);
    let trimmed = code.trim();
    if trimmed.len() < 5 || !trimmed.as_bytes()[..5].eq_ignore_ascii_case(b"BEGIN") {
        return None;
    }
    let upper = trimmed.to_ascii_uppercase();
    let rest = upper.strip_prefix("BEGIN")?;
    if rest.is_empty() {
        return component_begin_label_on_next_line(lines, index);
    }
    let original_rest = &trimmed[5..];
    let first = original_rest.chars().next()?;
    if !first.is_whitespace() {
        return None;
    }
    let after_ws = original_rest.trim_start();
    is_label_token_start(after_ws).then_some(index)
}

fn component_begin_label_on_next_line(lines: &[&str], index: usize) -> Option<usize> {
    let mut cursor = index + 1;
    while cursor < lines.len() {
        let raw = lines[cursor];
        let trimmed = raw.trim_start();
        if trimmed.is_empty() {
            cursor += 1;
            continue;
        }
        if trimmed.starts_with("//") {
            cursor += 1;
            continue;
        }
        if trimmed.starts_with("/*") {
            return None;
        }
        return is_label_token_start(code_part(raw)).then_some(cursor);
    }
    None
}

pub(crate) fn designated_id_in_code(line: &str) -> Option<String> {
    let code = code_part(line).to_ascii_uppercase();
    let index = code.find("DESIGNATED")?;
    let tail = code[index + "DESIGNATED".len()..].trim_start();
    let digits: String = tail.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return None;
    }
    let normalized = digits.trim_start_matches('0');
    Some(if normalized.is_empty() {
        "0".to_string()
    } else {
        normalized.to_string()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn begin_with_tilde_label() {
        let lines = ["BEGIN ~Component~"];
        assert_eq!(component_begin_at(lines[0], &lines, 0), Some(0));
    }

    #[test]
    fn begin_with_quote_label() {
        let lines = ["BEGIN \"Component\""];
        assert_eq!(component_begin_at(lines[0], &lines, 0), Some(0));
    }

    #[test]
    fn begin_with_at_label() {
        let lines = ["BEGIN @12"];
        assert_eq!(component_begin_at(lines[0], &lines, 0), Some(0));
    }

    #[test]
    fn begin_with_percent_label() {
        let lines = ["BEGIN %var%"];
        assert_eq!(component_begin_at(lines[0], &lines, 0), Some(0));
    }

    #[test]
    fn bare_begin_with_label_on_next_line_returns_that_line() {
        let lines = ["BEGIN", "~Label~"];
        assert_eq!(component_begin_at(lines[0], &lines, 0), Some(1));
    }

    #[test]
    fn bare_begin_with_blank_line_then_label_returns_that_line() {
        let lines = ["BEGIN", "", "~Label~"];
        assert_eq!(component_begin_at(lines[0], &lines, 0), Some(2));
    }

    #[test]
    fn bare_begin_with_comment_line_then_label_still_counts() {
        let lines = ["BEGIN", "// header text", "~Label~"];
        assert_eq!(component_begin_at(lines[0], &lines, 0), Some(2));
    }

    #[test]
    fn bare_begin_followed_by_copy_is_not_a_component() {
        let lines = ["BEGIN", "COPY ~a~ ~b~"];
        assert_eq!(component_begin_at(lines[0], &lines, 0), None);
    }

    #[test]
    fn bare_begin_with_trailing_comment_then_label_line() {
        let lines = ["BEGIN // start", "~Label~"];
        assert_eq!(component_begin_at(lines[0], &lines, 0), Some(1));
    }

    #[test]
    fn bare_begin_followed_by_block_comment_stops_lookahead() {
        let lines = ["BEGIN", "/* note", "~Label~", "*/"];
        assert_eq!(component_begin_at(lines[0], &lines, 0), None);
    }

    #[test]
    fn begin_inside_line_comment_is_ignored() {
        let lines = ["// BEGIN ~x~"];
        assert_eq!(component_begin_at(lines[0], &lines, 0), None);
    }

    #[test]
    fn double_slash_inside_tilde_string_is_not_a_comment() {
        assert_eq!(code_part("BEGIN ~a//b~"), "BEGIN ~a//b~");
    }

    #[test]
    fn designated_in_trailing_comment_is_none() {
        assert_eq!(designated_id_in_code("ADD_COMPONENT // DESIGNATED 5"), None);
    }

    #[test]
    fn designated_with_leading_zeros() {
        assert_eq!(
            designated_id_in_code("DESIGNATED 007"),
            Some("7".to_string())
        );
    }
}
