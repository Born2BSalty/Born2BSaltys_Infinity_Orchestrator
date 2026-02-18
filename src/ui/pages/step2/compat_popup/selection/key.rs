// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::controller::log_apply_match::parse_component_tp2_from_raw;

pub(crate) fn selected_mod_key(tp_file: &str, component_key: &str) -> String {
    if !component_key.trim().is_empty()
        && let Some(tp2) = parse_component_tp2_from_raw(component_key)
    {
        return normalize_mod_key(&tp2);
    }
    normalize_mod_key(tp_file)
}

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
