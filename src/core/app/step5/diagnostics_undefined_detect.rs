// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

pub(super) fn looks_like_undefined_signal(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return false;
    }
    let lower = trimmed.to_ascii_lowercase();
    lower.contains("no translation provided")
        || lower.contains("cannot resolve string")
        || lower.contains("missing translation")
        || (lower.contains("undefined")
            && (lower.contains("string")
                || lower.contains("strref")
                || lower.contains("translation")))
}
