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

pub(super) fn parse_component_u32(value: &str) -> Option<u32> {
    value.trim().parse::<u32>().ok()
}

pub(super) fn display_name_from_tp2(tp2_ref: &str) -> String {
    let file = if let Some(idx) = tp2_ref.rfind(['/', '\\']) {
        &tp2_ref[idx + 1..]
    } else {
        tp2_ref
    };
    let stem = file.strip_suffix(".tp2").unwrap_or(file);
    let stem = stem.strip_prefix("setup-").unwrap_or(stem);
    if stem.is_empty() {
        return tp2_ref.to_string();
    }
    stem.to_string()
}

pub(super) fn format_target(mod_name: &str, component: Option<u32>) -> String {
    match component {
        Some(id) => format!("{mod_name} #{id}"),
        None => mod_name.to_string(),
    }
}
