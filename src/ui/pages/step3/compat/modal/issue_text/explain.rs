// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::CompatIssueDisplay;

use super::helpers::{is_duplicate_selection_issue, parse_games};
use super::super::target::format_issue_target;

pub(crate) fn issue_verdict(issue: &CompatIssueDisplay) -> Option<String> {
    if is_duplicate_selection_issue(issue) {
        return Some("Same component appears multiple times in selection.".to_string());
    }
    if issue.code.eq_ignore_ascii_case("FORBID_HIT")
        || issue.code.eq_ignore_ascii_case("RULE_HIT")
    {
        return Some("Cannot install both sides at once.".to_string());
    }
    if issue.code.eq_ignore_ascii_case("REQ_MISSING") {
        return Some("Missing dependency blocks install until satisfied.".to_string());
    }
    if issue.code.eq_ignore_ascii_case("CONDITIONAL") {
        return Some("Optional behavior change; does not block install.".to_string());
    }
    if issue.code.eq_ignore_ascii_case("ORDER_WARN") {
        return Some("Order warning only; install can proceed.".to_string());
    }
    if issue.code.eq_ignore_ascii_case("GAME_MISMATCH") {
        let games = parse_games(issue);
        return Some(format!(
            "Not installable in current mode (allowed: {}).",
            if games.is_empty() { "N/A".to_string() } else { games }
        ));
    }
    None
}

pub(crate) fn issue_why_this_appears(issue: &CompatIssueDisplay) -> &'static str {
    if is_duplicate_selection_issue(issue) {
        return "The same mod component is present more than once in current selection/order.";
    }
    if issue.code.eq_ignore_ascii_case("REQ_MISSING") {
        "This component references another component that is not selected or not installed yet."
    } else if issue.code.eq_ignore_ascii_case("FORBID_HIT")
        || issue.code.eq_ignore_ascii_case("RULE_HIT")
    {
        "This component has a conflict rule against another selected component."
    } else if issue.code.eq_ignore_ascii_case("CONDITIONAL") {
        "A conditional TP2 branch was detected and is active."
    } else if issue.code.eq_ignore_ascii_case("GAME_MISMATCH") {
        "The component declares allowed game targets that do not match the current validation context."
    } else if issue.code.eq_ignore_ascii_case("ORDER_WARN") {
        "The dependency exists, but current order places it after this component."
    } else {
        "A compatibility rule was matched for this selection."
    }
}

pub(crate) fn issue_what_to_do(issue: &CompatIssueDisplay) -> &'static str {
    if is_duplicate_selection_issue(issue) {
        return "Remove duplicate instance(s) so each mod component appears once.";
    }
    if issue.code.eq_ignore_ascii_case("REQ_MISSING") {
        "Select/install the required related component first, then revalidate."
    } else if issue.code.eq_ignore_ascii_case("FORBID_HIT")
        || issue.code.eq_ignore_ascii_case("RULE_HIT")
    {
        "Keep one side and uncheck the other, then revalidate."
    } else if issue.code.eq_ignore_ascii_case("CONDITIONAL") {
        "No action required unless you want to change optional behavior."
    } else if issue.code.eq_ignore_ascii_case("GAME_MISMATCH") {
        "Use a compatible game mode for this component, or remove this component for current mode."
    } else if issue.code.eq_ignore_ascii_case("ORDER_WARN") {
        "Move dependency earlier (or this component later), then revalidate."
    } else {
        "Review the related target and source rule, then revalidate."
    }
}

pub(crate) fn issue_reason(issue: &CompatIssueDisplay) -> String {
    if is_duplicate_selection_issue(issue) {
        return issue.reason.clone();
    }
    if issue.code.eq_ignore_ascii_case("FORBID_HIT")
        || issue.code.eq_ignore_ascii_case("RULE_HIT")
    {
        return format!(
            "Conflicts with {} (currently selected).",
            format_issue_target(&issue.related_mod, issue.related_component)
        );
    }
    if issue.code.eq_ignore_ascii_case("REQ_MISSING") {
        return format!(
            "Requires {} which is not currently selected/installed.",
            format_issue_target(&issue.related_mod, issue.related_component)
        );
    }
    if issue.code.eq_ignore_ascii_case("CONDITIONAL") {
        return issue.reason.clone();
    }
    if issue.code.eq_ignore_ascii_case("GAME_MISMATCH") {
        let games = parse_games(issue);
        return format!(
            "This component is restricted to: {}.",
            if games.is_empty() { "N/A".to_string() } else { games }
        );
    }
    if issue.code.eq_ignore_ascii_case("ORDER_WARN") {
        return format!(
            "Order issue: {} should be installed before this component.",
            format_issue_target(&issue.related_mod, issue.related_component)
        );
    }
    issue.reason.clone()
}

pub(crate) fn human_related(issue: &CompatIssueDisplay, fallback_related: &str) -> String {
    if issue.code.eq_ignore_ascii_case("GAME_MISMATCH") {
        let games = parse_games(issue);
        return format!(
            "Allowed games: {}",
            if games.is_empty() { "N/A".to_string() } else { games }
        );
    }
    fallback_related.to_string()
}
