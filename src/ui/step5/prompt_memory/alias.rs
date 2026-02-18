// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

pub(super) fn suggest_alias_from_preview(preview: &str) -> String {
    let mut out = String::new();
    let mut last_was_sep = false;
    for ch in preview.chars().take(48) {
        let mapped = if ch.is_ascii_alphanumeric() {
            ch.to_ascii_lowercase()
        } else {
            '_'
        };
        if mapped == '_' {
            if !last_was_sep {
                out.push('_');
            }
            last_was_sep = true;
        } else {
            out.push(mapped);
            last_was_sep = false;
        }
    }
    let trimmed = out.trim_matches('_');
    if trimmed.is_empty() {
        "prompt_entry".to_string()
    } else {
        trimmed.to_string()
    }
}
