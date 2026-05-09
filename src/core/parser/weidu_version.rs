// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

pub(crate) fn parse_version(raw_line: &str) -> Option<String> {
    let comment = raw_line.split_once("//")?.1.trim();
    parse_version_text(comment)
}

pub(crate) fn parse_version_text(text: &str) -> Option<String> {
    text.split(':').rev().find_map(extract_version_from_segment)
}

pub(crate) fn normalize_version_text(value: &str) -> String {
    collapse_version_separators(&parse_version_text(value).unwrap_or_else(|| {
        value
            .trim()
            .trim_start_matches('v')
            .trim_start_matches('V')
            .trim()
            .to_ascii_lowercase()
    }))
}

fn extract_version_from_segment(segment: &str) -> Option<String> {
    segment
        .split_whitespace()
        .rev()
        .find_map(extract_version_token)
}

fn extract_version_token(token: &str) -> Option<String> {
    let token = token.trim_matches(|ch: char| {
        matches!(
            ch,
            '"' | '\'' | '(' | ')' | '[' | ']' | '{' | '}' | ',' | ';'
        )
    });
    if token.is_empty() {
        return None;
    }
    let lower = token.to_ascii_lowercase();
    let (stripped, had_version_prefix) = strip_version_prefix(&lower);
    let start = stripped.find(|ch: char| ch.is_ascii_digit())?;
    let tail = &stripped[start..];
    let end = tail
        .find(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '.' || ch == '-'))
        .unwrap_or(tail.len());
    let candidate = &tail[..end];
    (candidate.chars().any(|ch| ch.is_ascii_digit())
        && (candidate.contains('.') || had_version_prefix))
        .then(|| candidate.to_string())
}

fn strip_version_prefix(value: &str) -> (&str, bool) {
    if let Some(rest) = value.strip_prefix("version") {
        return (rest.trim_start_matches([' ', '-', '_', ':']), true);
    }
    if value.starts_with('v') && value.chars().nth(1).is_some_and(|ch| ch.is_ascii_digit()) {
        return (value[1..].trim_start_matches([' ', '-', '_', ':']), true);
    }
    (value, false)
}

fn collapse_version_separators(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if matches!(ch, ' ' | '_' | '-') {
                '.'
            } else {
                ch
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::normalize_version_text;

    #[test]
    fn normalizes_separator_variants_equally() {
        assert_eq!(
            normalize_version_text("Alpha 3"),
            normalize_version_text("Alpha_3")
        );
        assert_eq!(
            normalize_version_text("Alpha 3"),
            normalize_version_text("Alpha-3")
        );
        assert_eq!(
            normalize_version_text("v4.2"),
            normalize_version_text("v4-2")
        );
        assert_ne!(
            normalize_version_text("v1.10"),
            normalize_version_text("v11.0")
        );
    }
}
