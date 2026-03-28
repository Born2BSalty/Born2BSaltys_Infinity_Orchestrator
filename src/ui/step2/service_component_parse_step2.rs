// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

pub fn parse_lang(raw_line: &str) -> Option<String> {
    raw_line
        .split_whitespace()
        .find(|part| part.starts_with('#'))
        .map(|part| part.trim_start_matches('#').to_string())
        .filter(|value| !value.is_empty())
}

pub fn parse_version(raw_line: &str) -> Option<String> {
    let comment = raw_line.split_once("//")?.1.trim();
    let (_, version) = comment.rsplit_once(':')?;
    let version = version.trim();
    if version.is_empty() {
        None
    } else {
        Some(version.to_string())
    }
}
