// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::CompatIssueDisplay;

pub(crate) fn is_duplicate_selection_issue(issue: &CompatIssueDisplay) -> bool {
    issue.code.eq_ignore_ascii_case("RULE_HIT")
        && (issue.reason.to_ascii_lowercase().contains("selected multiple times")
            || issue
                .raw_evidence
                .as_deref()
                .unwrap_or_default()
                .eq_ignore_ascii_case("selected_set_duplicate"))
}

pub(crate) fn format_issue_target(mod_name: &str, component: Option<u32>) -> String {
    match component {
        Some(id) => format!("{mod_name} #{id}"),
        None => mod_name.to_string(),
    }
}

pub(crate) fn parse_games(issue: &CompatIssueDisplay) -> String {
    issue
        .related_mod
        .split('|')
        .map(|s| s.trim().to_ascii_uppercase())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(", ")
}
