// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

pub(crate) fn normalize_mod_key(value: &str) -> String {
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

pub(crate) fn source_path_from_reference(src: &str) -> Option<String> {
    let mut path = src.trim();
    if let Some((lhs, rhs)) = path.rsplit_once(':')
        && rhs.trim().chars().all(|c| c.is_ascii_digit())
    {
        path = lhs.trim();
    }
    if path.is_empty() {
        None
    } else {
        Some(path.to_string())
    }
}
