// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::CompatIssueDisplay;

pub(super) fn matches_issue_filter(issue: &CompatIssueDisplay, filter: &str) -> bool {
    match filter.to_ascii_lowercase().as_str() {
        "conflicts" => {
            issue.code.eq_ignore_ascii_case("FORBID_HIT")
                || issue.code.eq_ignore_ascii_case("RULE_HIT")
                || issue.reason.to_ascii_lowercase().contains("incompatible")
                || issue.reason.to_ascii_lowercase().contains("conflict")
        }
        "dependencies" => issue.code.eq_ignore_ascii_case("REQ_MISSING"),
        "conditionals" => issue.code.eq_ignore_ascii_case("CONDITIONAL"),
        "warnings" => !issue.is_blocking || issue.code.eq_ignore_ascii_case("ORDER_WARN"),
        _ => true,
    }
}
