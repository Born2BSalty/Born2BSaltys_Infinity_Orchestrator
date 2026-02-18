// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

pub(super) fn maybe_last_two_parts(path_like: &str) -> Option<String> {
    let norm = normalize_path_key(path_like);
    let parts: Vec<&str> = norm.split('\\').filter(|p| !p.is_empty()).collect();
    if parts.len() >= 2 {
        Some(format!("{}\\{}", parts[parts.len() - 2], parts[parts.len() - 1]))
    } else {
        None
    }
}

pub(super) fn strip_setup_prefix(value: &str) -> String {
    let upper = value.to_ascii_uppercase();
    upper
        .strip_prefix("SETUP-")
        .or_else(|| upper.strip_prefix("SETUP_"))
        .unwrap_or(upper.as_str())
        .to_string()
}

pub(super) fn ensure_setup_prefix(value: &str) -> String {
    let upper = value.to_ascii_uppercase();
    if upper.starts_with("SETUP-") || upper.starts_with("SETUP_") {
        upper
    } else {
        format!("SETUP-{upper}")
    }
}

pub fn normalize_path_key(value: &str) -> String {
    let mut normalized = value.trim().trim_matches('"').replace('/', "\\");
    if normalized.starts_with(".\\") {
        normalized = normalized[2..].to_string();
    }
    normalized.to_ascii_uppercase()
}
