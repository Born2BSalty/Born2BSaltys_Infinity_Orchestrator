// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

pub(crate) mod compat_popup_issue_text_copy {
    use crate::ui::state::CompatIssueDisplay;

    use crate::ui::step2::compat_issue_text_step2::compat_popup_issue_text_helpers::format_issue_target;

    pub(crate) fn format_issue_for_copy(issue: &CompatIssueDisplay) -> String {
        let affected = format_issue_target(&issue.affected_mod, issue.affected_component);
        let related = format_issue_target(&issue.related_mod, issue.related_component);
        format!(
            "[{}] {} -> {}\nBlocking: {}\nReason: {}\nSource: {}",
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
        format_issue_target, is_duplicate_selection_issue, is_require_order_issue,
        parse_games, parse_or_targets_from_reason,
    };

    pub(crate) fn issue_summary(issue: &CompatIssueDisplay) -> String {
        if is_duplicate_selection_issue(issue) {
            return "Duplicate selection".to_string();
        }
        if issue.code.eq_ignore_ascii_case("GAME_MISMATCH") {
            let games = parse_games(issue);
            return if games.is_empty() {
                "Not available in this game mode".to_string()
            } else {
                format!("Only available on `{games}`")
            };
        }
        if issue.code.eq_ignore_ascii_case("REQ_MISSING") {
            if let Some(or_targets) = parse_or_targets_from_reason(&issue.reason) {
                let joined = or_targets
                    .into_iter()
                    .map(|target| format!("`{target}`"))
                    .collect::<Vec<_>>()
                    .join(" or ");
                return format!("Needs {joined}");
            }
            let related = format_issue_target(&issue.related_mod, issue.related_component);
            return format!("Needs `{related}`");
        }
        if issue.code.eq_ignore_ascii_case("FORBID_HIT")
            || issue.code.eq_ignore_ascii_case("RULE_HIT")
        {
            if issue.related_mod.eq_ignore_ascii_case("unknown") {
                return "Blocked by another component".to_string();
            }
            let related = format_issue_target(&issue.related_mod, issue.related_component);
            return format!("Blocked by `{related}`");
        }
        if issue.code.eq_ignore_ascii_case("INCLUDED") {
            let related = format_issue_target(&issue.related_mod, issue.related_component);
            return format!("Included by `{related}`");
        }
        if issue.code.eq_ignore_ascii_case("ORDER_BLOCK") {
            let related = format_issue_target(&issue.related_mod, issue.related_component);
            return if is_require_order_issue(issue) {
                format!("Must be after `{related}`")
            } else {
                format!("Must be before `{related}`")
            };
        }
        if issue.code.eq_ignore_ascii_case("CONDITIONAL") {
            let related = format_issue_target(&issue.related_mod, issue.related_component);
            return format!("Conditional on `{related}`");
        }
        let fallback = issue.reason.trim();
        if !fallback.is_empty() && !fallback.eq_ignore_ascii_case("unknown") {
            fallback.to_string()
        } else {
            "Compatibility issue".to_string()
        }
    }

    pub(crate) fn display_source(source: &str) -> String {
        let trimmed = source.trim();
        if let Some(idx) = trimmed.find("TP2:") {
            trimmed[idx..].to_string()
        } else {
            trimmed.to_string()
        }
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

    pub(crate) fn parse_or_targets_from_reason(reason: &str) -> Option<Vec<String>> {
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

    pub(crate) fn is_require_order_issue(issue: &CompatIssueDisplay) -> bool {
        issue.code.eq_ignore_ascii_case("ORDER_BLOCK")
            && issue
                .raw_evidence
                .as_deref()
                .map(|raw| raw.trim_start().to_ascii_uppercase().starts_with("REQUIRE"))
                .unwrap_or(false)
    }
}

pub(crate) mod compat_popup_issue_text_kind {
    pub(crate) fn human_kind(kind: &str) -> &'static str {
        match kind.to_ascii_lowercase().as_str() {
            "game_mismatch" => "Game mismatch",
            "missing_dep" => "Missing dependency",
            "conflict" | "not_compatible" => "Conflict",
            "included" => "Included",
            "conditional" => "Conditional patch",
            "order_block" => "Install order",
            "path_requirement" => "Path requirement",
            "warning" => "Warning",
            "deprecated" => "Deprecated",
            _ => "Compatibility issue",
        }
    }
}
