// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::CompatIssueDisplay;

use super::helpers::{is_duplicate_selection_issue, parse_games, parse_or_targets_from_reason};
use super::super::target::format_issue_target;

pub(crate) fn issue_graph(issue: &CompatIssueDisplay) -> String {
    let affected = format_issue_target(&issue.affected_mod, issue.affected_component);
    let related = format_issue_target(&issue.related_mod, issue.related_component);
    if issue.code.eq_ignore_ascii_case("GAME_MISMATCH") {
        let games = parse_games(issue);
        return format!(
            "{affected} allowed on: {}",
            if games.is_empty() { "N/A".to_string() } else { games }
        );
    }
    if issue.code.eq_ignore_ascii_case("FORBID_HIT") || issue.code.eq_ignore_ascii_case("RULE_HIT")
    {
        if is_duplicate_selection_issue(issue) {
            return format!("{affected} appears multiple times in selection");
        }
        return format!("{affected} conflicts with {related}");
    }
    if issue.code.eq_ignore_ascii_case("REQ_MISSING") {
        if let Some(or_targets) = parse_or_targets_from_reason(&issue.reason) {
            return format!("{affected} requires one of: {}", or_targets.join(" | "));
        }
        return format!("{affected} requires {related}");
    }
    if issue.code.eq_ignore_ascii_case("CONDITIONAL") {
        return format!("{affected} has optional patch for {related}");
    }
    if issue.code.eq_ignore_ascii_case("ORDER_WARN") {
        return format!("{affected} should be installed after {related}");
    }
    format!("{affected} -> {related}")
}
