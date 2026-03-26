// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::{CompatIssueDisplay, Step3ItemState, WizardState};
use crate::ui::step3::compat_modal_step3::compat_model::normalize_mod_key;

pub(crate) fn matches_issue_filter(issue: &CompatIssueDisplay, filter: &str) -> bool {
    match filter.to_ascii_lowercase().as_str() {
        "conflicts" => {
            issue.code.eq_ignore_ascii_case("FORBID_HIT")
                || issue.code.eq_ignore_ascii_case("ORDER_BLOCK")
                || issue.code.eq_ignore_ascii_case("RULE_HIT")
                || issue.reason.to_ascii_lowercase().contains("incompatible")
                || issue.reason.to_ascii_lowercase().contains("conflict")
        }
        "dependencies" => issue.code.eq_ignore_ascii_case("REQ_MISSING"),
        "conditionals" => issue.code.eq_ignore_ascii_case("CONDITIONAL"),
        "warnings" => !issue.is_blocking,
        _ => true,
    }
}

pub(crate) fn issue_target_exists(state: &WizardState, issue: &CompatIssueDisplay, affected: bool) -> bool {
    let (mod_name, component) = if affected {
        (&issue.affected_mod, issue.affected_component)
    } else {
        (&issue.related_mod, issue.related_component)
    };
    item_target_exists(&state.step3.bgee_items, mod_name, component)
        || item_target_exists(&state.step3.bg2ee_items, mod_name, component)
}

pub(crate) fn item_target_exists(items: &[Step3ItemState], mod_name: &str, component: Option<u32>) -> bool {
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

pub(crate) fn is_require_order_issue(issue: &CompatIssueDisplay) -> bool {
    issue.code.eq_ignore_ascii_case("ORDER_BLOCK")
        && issue
            .raw_evidence
            .as_deref()
            .map(|raw| raw.trim_start().to_ascii_uppercase().starts_with("REQUIRE"))
            .unwrap_or(false)
}

pub(crate) fn human_kind(code: &str) -> &'static str {
    if code.eq_ignore_ascii_case("GAME_MISMATCH") {
        "Game mismatch"
    } else if code.eq_ignore_ascii_case("REQ_MISSING") {
        "Missing dependency"
    } else if code.eq_ignore_ascii_case("INCLUDED") {
        "Included"
    } else if code.eq_ignore_ascii_case("CONDITIONAL") {
        "Conditional patch"
    } else if code.eq_ignore_ascii_case("FORBID_HIT") || code.eq_ignore_ascii_case("RULE_HIT") {
        "Conflict"
    } else if code.eq_ignore_ascii_case("ORDER_BLOCK") {
        "Install order"
    } else {
        "Compatibility issue"
    }
}

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
    if issue.code.eq_ignore_ascii_case("INCLUDED") {
        return format!("{affected} is included by {related}");
    }
    if issue.code.eq_ignore_ascii_case("ORDER_BLOCK") {
        return if is_require_order_issue(issue) {
            format!("{affected} must be installed after {related}")
        } else {
            format!("{affected} must be installed before {related}")
        };
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
    format!("{affected} -> {related}")
}

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

pub(crate) fn is_duplicate_selection_issue(issue: &CompatIssueDisplay) -> bool {
    issue.code.eq_ignore_ascii_case("RULE_HIT")
        && (issue.reason.to_ascii_lowercase().contains("selected multiple times")
            || issue
                .raw_evidence
                .as_deref()
                .unwrap_or_default()
                .eq_ignore_ascii_case("selected_set_duplicate"))
}
