// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

pub(crate) fn parse_lang(raw_line: &str) -> Option<String> {
    raw_line
        .split_whitespace()
        .find(|part| part.starts_with('#'))
        .map(|part| part.trim_start_matches('#').to_string())
        .filter(|value| !value.is_empty())
}
