// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::CompatIssueDisplay;

pub(crate) fn human_kind(code: &str) -> &'static str {
    if code.eq_ignore_ascii_case("GAME_MISMATCH") {
        "Game mismatch"
    } else if code.eq_ignore_ascii_case("REQ_MISSING") {
        "Missing dependency"
    } else if code.eq_ignore_ascii_case("CONDITIONAL") {
        "Conditional patch"
    } else if code.eq_ignore_ascii_case("FORBID_HIT") || code.eq_ignore_ascii_case("RULE_HIT") {
        "Conflict"
    } else if code.eq_ignore_ascii_case("ORDER_WARN") {
        "Order warning"
    } else {
        "Compatibility issue"
    }
}

pub(crate) fn human_severity(issue: &CompatIssueDisplay) -> &'static str {
    if issue.is_blocking {
        "Error"
    } else {
        "Warning"
    }
}
