// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

pub(super) fn parse_lang(raw_line: &str) -> Option<String> {
    raw_line
        .split_whitespace()
        .find(|p| p.starts_with('#'))
        .map(|p| p.trim_start_matches('#').to_string())
        .filter(|s| !s.is_empty())
}

pub(super) fn parse_version(raw_line: &str) -> Option<String> {
    let comment = raw_line.split_once("//")?.1.trim();
    let (_, v) = comment.rsplit_once(':')?;
    let version = v.trim();
    if version.is_empty() {
        None
    } else {
        Some(version.to_string())
    }
}
