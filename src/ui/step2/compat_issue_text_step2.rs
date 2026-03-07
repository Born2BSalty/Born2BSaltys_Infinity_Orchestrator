// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

pub(crate) mod compat_popup_issue_text_copy {
    use crate::ui::state::CompatIssueDisplay;

    use crate::ui::step2::compat_issue_text_step2::compat_popup_issue_text_helpers::format_issue_target;

    pub(crate) fn format_issue_for_copy(issue: &CompatIssueDisplay) -> String {
        let affected = format_issue_target(&issue.affected_mod, issue.affected_component);
        let related = format_issue_target(&issue.related_mod, issue.related_component);
        format!(
            "[{}] {} -> {}\\nBlocking: {}\\nReason: {}\\nSource: {}",
            issue.code,
            affected,
            related,
            if issue.is_blocking { "yes" } else { "no" },
            issue.reason,
            issue.source
        )
    }
}

pub(crate) mod compat_popup_issue_text_explain {
    use crate::ui::state::CompatIssueDisplay;

    use crate::ui::step2::compat_issue_text_step2::compat_popup_issue_text_helpers::{
        format_issue_target, is_duplicate_selection_issue, parse_games,
    };

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
}

pub(crate) mod compat_popup_issue_text_helpers {
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
}

pub(crate) mod compat_popup_issue_text_kind {
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
}
