// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::CompatIssueDisplay;

use super::key::{format_target, normalize_mod_key};

pub(super) fn issue_matches_affected(issue: &CompatIssueDisplay, mod_key: &str, comp_id: Option<u32>) -> bool {
    if normalize_mod_key(&issue.affected_mod) != mod_key {
        return false;
    }
    match (issue.affected_component, comp_id) {
        (Some(a), Some(b)) => a == b,
        (None, _) => true,
        _ => false,
    }
}

pub(super) fn issue_matches_related(issue: &CompatIssueDisplay, mod_key: &str, comp_id: Option<u32>) -> bool {
    if normalize_mod_key(&issue.related_mod) != mod_key {
        return false;
    }
    match (issue.related_component, comp_id) {
        (Some(a), Some(b)) => a == b,
        (None, _) => true,
        _ => false,
    }
}

pub(super) fn issue_to_compat_kind(issue: &CompatIssueDisplay) -> String {
    if issue.code.eq_ignore_ascii_case("REQ_MISSING") {
        "missing_dep".to_string()
    } else if issue.code.eq_ignore_ascii_case("GAME_MISMATCH") {
        "game_mismatch".to_string()
    } else if issue.code.eq_ignore_ascii_case("CONDITIONAL") {
        "conditional".to_string()
    } else if issue.is_blocking {
        "conflict".to_string()
    } else {
        "warning".to_string()
    }
}

pub(super) fn issue_graph(issue: &CompatIssueDisplay) -> String {
    if is_duplicate_selection_issue(issue) {
        return format!(
            "{} appears multiple times in selection",
            format_target(&issue.affected_mod, issue.affected_component)
        );
    }
    if issue.code.eq_ignore_ascii_case("GAME_MISMATCH") {
        let games = issue
            .related_mod
            .split('|')
            .map(|s| s.trim().to_ascii_uppercase())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(", ");
        return format!(
            "{} allowed on: {}",
            format_target(&issue.affected_mod, issue.affected_component),
            if games.is_empty() { "N/A".to_string() } else { games }
        );
    }
    if issue.code.eq_ignore_ascii_case("FORBID_HIT")
        || issue.code.eq_ignore_ascii_case("RULE_HIT")
    {
        return format!(
            "{} conflicts with {}",
            format_target(&issue.affected_mod, issue.affected_component),
            format_target(&issue.related_mod, issue.related_component)
        );
    }
    if issue.code.eq_ignore_ascii_case("REQ_MISSING") {
        if let Some(or_targets) = parse_or_targets_from_reason(&issue.reason) {
            return format!(
                "{} requires one of: {}",
                format_target(&issue.affected_mod, issue.affected_component),
                or_targets.join(" | ")
            );
        }
        return format!(
            "{} requires {}",
            format_target(&issue.affected_mod, issue.affected_component),
            format_target(&issue.related_mod, issue.related_component)
        );
    }
    if issue.code.eq_ignore_ascii_case("CONDITIONAL") {
        return format!(
            "{} has optional patch for {}",
            format_target(&issue.affected_mod, issue.affected_component),
            format_target(&issue.related_mod, issue.related_component)
        );
    }
    if issue.code.eq_ignore_ascii_case("ORDER_WARN") {
        return format!(
            "{} should be installed after {}",
            format_target(&issue.affected_mod, issue.affected_component),
            format_target(&issue.related_mod, issue.related_component)
        );
    }
    format!(
        "{} -> {}",
        format_target(&issue.affected_mod, issue.affected_component),
        format_target(&issue.related_mod, issue.related_component)
    )
}

pub(super) fn issue_related_target(issue: &CompatIssueDisplay) -> String {
    if is_duplicate_selection_issue(issue) {
        return "Duplicate selection".to_string();
    }
    if issue.code.eq_ignore_ascii_case("GAME_MISMATCH") {
        let games = issue
            .related_mod
            .split('|')
            .map(|s| s.trim().to_ascii_uppercase())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(", ");
        return format!(
            "Allowed games: {}",
            if games.is_empty() { "N/A".to_string() } else { games }
        );
    }
    format_target(&issue.related_mod, issue.related_component)
}

fn parse_or_targets_from_reason(reason: &str) -> Option<Vec<String>> {
    let prefix = "Requires one of:";
    let body = reason.strip_prefix(prefix)?.trim();
    let parts = body
        .split(" OR ")
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if parts.len() > 1 {
        Some(parts)
    } else {
        None
    }
}

fn is_duplicate_selection_issue(issue: &CompatIssueDisplay) -> bool {
    issue.code.eq_ignore_ascii_case("RULE_HIT")
        && (issue.reason.to_ascii_lowercase().contains("selected multiple times")
            || issue
                .raw_evidence
                .as_deref()
                .unwrap_or_default()
                .eq_ignore_ascii_case("selected_set_duplicate"))
}
