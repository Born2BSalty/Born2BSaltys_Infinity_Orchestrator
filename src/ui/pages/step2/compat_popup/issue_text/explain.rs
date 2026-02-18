// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::CompatIssueDisplay;

use super::helpers::{format_issue_target, is_duplicate_selection_issue, parse_games};

pub(crate) fn issue_verdict(issue: &CompatIssueDisplay) -> Option<String> {
    if is_duplicate_selection_issue(issue) {
        return Some("Same component appears multiple times in the selected set.".to_string());
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
        return Some("Order warning only; install can proceed but sequence is suboptimal.".to_string());
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

pub(crate) fn issue_reason(issue: &CompatIssueDisplay, fallback: &str) -> String {
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
        return format!(
            "Conditional patch branch is active because {} is present.",
            format_issue_target(&issue.related_mod, issue.related_component)
        );
    }
    if issue.code.eq_ignore_ascii_case("ORDER_WARN") {
        return format!(
            "Order issue: {} should be installed before this component.",
            format_issue_target(&issue.related_mod, issue.related_component)
        );
    }
    if issue.code.eq_ignore_ascii_case("GAME_MISMATCH") {
        let games = parse_games(issue);
        return format!(
            "This component is restricted to: {}.",
            if games.is_empty() { "N/A".to_string() } else { games }
        );
    }
    fallback.to_string()
}

pub(crate) fn issue_why_this_appears(issue: &CompatIssueDisplay) -> String {
    let t = issue.code.to_ascii_uppercase();
    if is_duplicate_selection_issue(issue) {
        return "The same mod component is present more than once in current selection/order."
            .to_string();
    }
    if t == "REQ_MISSING" {
        return "This component references another component that is not selected or not installed yet."
            .to_string();
    }
    if t == "FORBID_HIT" || t == "RULE_HIT" {
        return "This component has a conflict rule against another selected component.".to_string();
    }
    if t == "CONDITIONAL" {
        return "A conditional TP2 branch was detected and is active because the related mod/component is present."
            .to_string();
    }
    if t == "GAME_MISMATCH" {
        return "The component declares allowed game targets that do not match the current validation context."
            .to_string();
    }
    if t == "ORDER_WARN" {
        return "The dependency exists, but the current selection order places it after this component."
            .to_string();
    }
    "A compatibility rule was matched for this selection.".to_string()
}

pub(crate) fn issue_what_to_do(issue: &CompatIssueDisplay) -> String {
    let t = issue.code.to_ascii_uppercase();
    if is_duplicate_selection_issue(issue) {
        return "Remove duplicate instance(s) so each mod component appears once.".to_string();
    }
    if t == "REQ_MISSING" {
        return "Select/install the required related component first, then revalidate.".to_string();
    }
    if t == "FORBID_HIT" || t == "RULE_HIT" {
        return "Keep one side and uncheck the other, then revalidate.".to_string();
    }
    if t == "CONDITIONAL" {
        return "No action required unless you want to change optional behavior. Safe to keep."
            .to_string();
    }
    if t == "GAME_MISMATCH" {
        return "Use a compatible game mode for this component, or remove this component for current mode."
            .to_string();
    }
    if t == "ORDER_WARN" {
        return "Move the dependency earlier (or this component later), then revalidate.".to_string();
    }
    "Review the related target and source rule, then revalidate.".to_string()
}
