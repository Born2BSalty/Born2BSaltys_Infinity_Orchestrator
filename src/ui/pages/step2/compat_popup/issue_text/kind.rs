// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

pub(crate) fn human_kind(kind: &str) -> &'static str {
    match kind.to_ascii_lowercase().as_str() {
        "game_mismatch" => "Game mismatch",
        "missing_dep" => "Missing dependency",
        "conflict" | "not_compatible" => "Conflict",
        "conditional" => "Conditional patch",
        "warning" => "Warning",
        _ => "Compatibility issue",
    }
}
