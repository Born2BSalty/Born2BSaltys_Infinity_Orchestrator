// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::{CompatIssueDisplay, Step3ItemState, WizardState};
use crate::ui::step3::compat_modal_step3::compat_model::normalize_mod_key;

pub(crate) fn matches_issue_filter(issue: &CompatIssueDisplay, filter: &str) -> bool {
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

pub(crate) fn issue_target_exists(
    state: &WizardState,
    issue: &CompatIssueDisplay,
    affected: bool,
) -> bool {
    let (mod_name, component) = if affected {
        (&issue.affected_mod, issue.affected_component)
    } else {
        (&issue.related_mod, issue.related_component)
    };
    item_target_exists(&state.step3.bgee_items, mod_name, component)
        || item_target_exists(&state.step3.bg2ee_items, mod_name, component)
}

pub(crate) fn item_target_exists(
    items: &[Step3ItemState],
    mod_name: &str,
    component: Option<u32>,
) -> bool {
    let target_key = normalize_mod_key(mod_name);
    for item in items {
        if item.is_parent {
            continue;
        }
        let item_tp_key = normalize_mod_key(&item.tp_file);
        let item_name_key = normalize_mod_key(&item.mod_name);
        if item_tp_key != target_key && item_name_key != target_key {
            continue;
        }
        if let Some(component_id) = component {
            if item.component_id.parse::<u32>().ok() == Some(component_id) {
                return true;
            }
        } else {
            return true;
        }
    }
    false
}

pub(crate) fn format_issue_target(mod_name: &str, component: Option<u32>) -> String {
    match component {
        Some(id) => format!("{mod_name} #{id}"),
        None => mod_name.to_string(),
    }
}

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

pub(crate) fn issue_graph(issue: &CompatIssueDisplay) -> String {
    let affected = format_issue_target(&issue.affected_mod, issue.affected_component);
    let related = format_issue_target(&issue.related_mod, issue.related_component);
    if issue.code.eq_ignore_ascii_case("GAME_MISMATCH") {
        let games = parse_games(issue);
        return format!(
            "{affected} allowed on: {}",
            if games.is_empty() {
                "N/A".to_string()
            } else {
                games
            }
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

pub(crate) fn issue_verdict(issue: &CompatIssueDisplay) -> Option<String> {
    if is_duplicate_selection_issue(issue) {
        return Some("Same component appears multiple times in selection.".to_string());
    }
    if issue.code.eq_ignore_ascii_case("FORBID_HIT") || issue.code.eq_ignore_ascii_case("RULE_HIT")
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
            if games.is_empty() {
                "N/A".to_string()
            } else {
                games
            }
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
    if issue.code.eq_ignore_ascii_case("FORBID_HIT") || issue.code.eq_ignore_ascii_case("RULE_HIT")
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
            if games.is_empty() {
                "N/A".to_string()
            } else {
                games
            }
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
            if games.is_empty() {
                "N/A".to_string()
            } else {
                games
            }
        );
    }
    fallback_related.to_string()
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
    if parts.len() > 1 { Some(parts) } else { None }
}

pub(crate) fn is_duplicate_selection_issue(issue: &CompatIssueDisplay) -> bool {
    issue.code.eq_ignore_ascii_case("RULE_HIT")
        && (issue
            .reason
            .to_ascii_lowercase()
            .contains("selected multiple times")
            || issue
                .raw_evidence
                .as_deref()
                .unwrap_or_default()
                .eq_ignore_ascii_case("selected_set_duplicate"))
}
