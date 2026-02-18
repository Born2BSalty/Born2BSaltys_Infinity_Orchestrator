// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

pub(super) fn normalize_mod_key(value: &str) -> String {
    let lower = value.to_ascii_lowercase();
    let file = if let Some(idx) = lower.rfind(['/', '\\']) {
        &lower[idx + 1..]
    } else {
        &lower
    };
    let without_ext = file.strip_suffix(".tp2").unwrap_or(file);
    without_ext
        .strip_prefix("setup-")
        .unwrap_or(without_ext)
        .to_string()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum CompatJumpAction {
    Auto(String),
    Affected(String),
    Related(String),
}
