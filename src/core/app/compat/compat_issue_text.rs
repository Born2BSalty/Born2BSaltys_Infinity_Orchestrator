// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

pub(crate) fn human_kind(kind: &str) -> &'static str {
    match kind.to_ascii_lowercase().as_str() {
        "mismatch" | "game_mismatch" => "Mismatch",
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

pub(crate) fn format_issue_target(mod_name: &str, component: Option<u32>) -> String {
    component.map_or_else(|| mod_name.to_string(), |id| format!("{mod_name} #{id}"))
}

pub(crate) fn parse_games(value: &str) -> String {
    value
        .split('|')
        .map(|entry| entry.trim().to_ascii_uppercase())
        .filter(|entry| !entry.is_empty())
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn parse_or_targets_from_reason(reason: &str) -> Option<Vec<String>> {
    let prefix = "Requires one of:";
    let body = reason.strip_prefix(prefix)?.trim();
    let parts = body
        .split(" OR ")
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if parts.len() > 1 { Some(parts) } else { None }
}

pub(crate) fn is_require_order_evidence(raw_evidence: Option<&str>) -> bool {
    raw_evidence.is_some_and(|raw| raw.trim_start().to_ascii_uppercase().starts_with("REQUIRE"))
}

pub(crate) fn display_source(source: &str) -> String {
    let trimmed = source.trim();
    trimmed
        .find("TP2:")
        .map_or_else(|| trimmed.to_string(), |idx| trimmed[idx..].to_string())
}

pub(crate) fn is_duplicate_selection_issue(
    code: &str,
    reason: &str,
    raw_evidence: Option<&str>,
) -> bool {
    code.eq_ignore_ascii_case("RULE_HIT")
        && (reason
            .to_ascii_lowercase()
            .contains("selected multiple times")
            || raw_evidence
                .unwrap_or_default()
                .eq_ignore_ascii_case("selected_set_duplicate"))
}

pub(crate) fn issue_summary(
    code: &str,
    related_mod: &str,
    related_component: Option<u32>,
    reason: &str,
    raw_evidence: Option<&str>,
    selected_mode: &str,
) -> String {
    if is_duplicate_selection_issue(code, reason, raw_evidence) {
        return "Duplicate selection".to_string();
    }
    if code.eq_ignore_ascii_case("MISMATCH") || code.eq_ignore_ascii_case("GAME_MISMATCH") {
        let games = parse_games(related_mod);
        return if games.is_empty() {
            if selected_mode.eq_ignore_ascii_case("BGEE")
                || selected_mode.eq_ignore_ascii_case("BG2EE")
            {
                "This component is not available on the current game mode.".to_string()
            } else {
                "This component is not available on the current game tab.".to_string()
            }
        } else {
            format!("Only available on `{games}`")
        };
    }
    if code.eq_ignore_ascii_case("REQ_MISSING") {
        if let Some(or_targets) = parse_or_targets_from_reason(reason) {
            let joined = or_targets
                .into_iter()
                .map(|target| format!("`{target}`"))
                .collect::<Vec<_>>()
                .join(" or ");
            return format!("Needs {joined}");
        }
        let related = format_issue_target(related_mod, related_component);
        return format!("Needs `{related}`");
    }
    if code.eq_ignore_ascii_case("FORBID_HIT") || code.eq_ignore_ascii_case("RULE_HIT") {
        if raw_evidence.is_some_and(|raw| {
            raw.trim_start()
                .to_ascii_uppercase()
                .starts_with("FORBID_COMPONENT")
        }) {
            let trimmed = reason.trim();
            if !trimmed.is_empty() && !trimmed.eq_ignore_ascii_case("unknown") {
                return trimmed.to_string();
            }
        }
        if related_mod.eq_ignore_ascii_case("unknown") {
            return "Blocked by another component".to_string();
        }
        let related = format_issue_target(related_mod, related_component);
        return format!("Blocked by `{related}`");
    }
    if code.eq_ignore_ascii_case("INCLUDED") {
        let trimmed = reason.trim();
        if !trimmed.is_empty() && !trimmed.eq_ignore_ascii_case("unknown") {
            return trimmed.to_string();
        }
        if related_mod.eq_ignore_ascii_case("unknown") {
            return "Already included elsewhere".to_string();
        }
        let related = format_issue_target(related_mod, related_component);
        return format!("Included by `{related}`");
    }
    if code.eq_ignore_ascii_case("ORDER_BLOCK") {
        let related = format_issue_target(related_mod, related_component);
        return if is_require_order_evidence(raw_evidence) {
            format!("Must be after `{related}`")
        } else {
            format!("Must be before `{related}`")
        };
    }
    if code.eq_ignore_ascii_case("CONDITIONAL") {
        let trimmed = reason.trim();
        if !trimmed.is_empty() && !trimmed.eq_ignore_ascii_case("unknown") {
            return trimmed.to_string();
        }
        let related = format_issue_target(related_mod, related_component);
        return format!("Conditional on `{related}`");
    }
    let fallback = reason.trim();
    if !fallback.is_empty() && !fallback.eq_ignore_ascii_case("unknown") {
        fallback.to_string()
    } else {
        "Compatibility issue".to_string()
    }
}
