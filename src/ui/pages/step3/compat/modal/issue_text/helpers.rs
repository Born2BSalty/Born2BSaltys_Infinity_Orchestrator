// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::CompatIssueDisplay;

pub(super) fn parse_games(issue: &CompatIssueDisplay) -> String {
    issue
        .related_mod
        .split('|')
        .map(|s| s.trim().to_ascii_uppercase())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn parse_or_targets_from_reason(reason: &str) -> Option<Vec<String>> {
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

pub(super) fn is_duplicate_selection_issue(issue: &CompatIssueDisplay) -> bool {
    issue.code.eq_ignore_ascii_case("RULE_HIT")
        && (issue.reason.to_ascii_lowercase().contains("selected multiple times")
            || issue
                .raw_evidence
                .as_deref()
                .unwrap_or_default()
                .eq_ignore_ascii_case("selected_set_duplicate"))
}
