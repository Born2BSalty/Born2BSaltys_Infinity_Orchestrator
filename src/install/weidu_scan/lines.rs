// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

pub(super) fn is_tp2_component_line(line: &str) -> bool {
    line.starts_with('~') && line.contains('~') && line.contains("//")
}

pub(super) fn is_simple_component_line(line: &str) -> bool {
    let mut digits_len = 0usize;
    for ch in line.chars() {
        if ch.is_ascii_digit() {
            digits_len += 1;
            continue;
        }
        break;
    }
    if digits_len == 0 {
        return false;
    }
    let rest = line[digits_len..].trim_start();
    rest.starts_with(':')
        || rest.starts_with('-')
        || rest.chars().next().is_some_and(|c| c.is_whitespace())
}
