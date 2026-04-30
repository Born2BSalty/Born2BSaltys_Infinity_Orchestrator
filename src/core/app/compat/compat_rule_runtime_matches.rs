// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::compat_rules::{CompatRule, StringOrMany};

pub(crate) fn mode_matches(rule: &CompatRule, selected_mode: &str) -> bool {
    option_list_matches(rule.mode.as_ref(), selected_mode)
}

pub(crate) fn tab_matches(rule: &CompatRule, tab: &str) -> bool {
    option_list_matches(rule.tab.as_ref(), tab)
}

pub(crate) fn compat_mod_matches(rule: &CompatRule, tp_file: &str, mod_name: &str) -> bool {
    let rule_mods = rule.r#mod.trimmed_items();
    if rule_mods.is_empty() {
        return false;
    }
    let tp_file_key = normalize_mod_key(tp_file);
    let mod_name_key = normalize_mod_key(mod_name);
    rule_mods.into_iter().any(|rule_mod| {
        let rule_key = normalize_mod_key(&rule_mod);
        !rule_key.is_empty()
            && (tp_file_key == rule_key
                || mod_name_key == rule_key
                || mod_name.trim().eq_ignore_ascii_case(rule_mod.trim()))
    })
}

pub(crate) fn compat_component_matches(
    rule: &CompatRule,
    component_id: &str,
    label: &str,
    raw_line: &str,
) -> bool {
    let component_ids = rule
        .component_id
        .as_ref()
        .map(StringOrMany::trimmed_items)
        .unwrap_or_default();
    let component_labels = rule
        .component
        .as_ref()
        .map(StringOrMany::trimmed_items)
        .unwrap_or_default();
    let has_component_selector = !component_ids.is_empty() || !component_labels.is_empty();
    if !has_component_selector {
        return true;
    }

    let id_match = if component_ids.is_empty() {
        true
    } else {
        component_ids.iter().any(|rule_id| {
            rule_id.trim() == "*" || component_id.trim().eq_ignore_ascii_case(rule_id.trim())
        })
    };

    let label_match = if component_labels.is_empty() {
        true
    } else {
        let label = label.to_ascii_lowercase();
        let raw_line = raw_line.to_ascii_lowercase();
        component_labels.iter().any(|needle| {
            let needle = needle.to_ascii_lowercase();
            label.contains(&needle) || raw_line.contains(&needle)
        })
    };

    id_match && label_match
}

pub(crate) fn match_kind_matches(
    match_kind: Option<&StringOrMany>,
    current_kind: Option<&str>,
) -> bool {
    let Some(kinds) = match_kind else {
        return true;
    };
    let Some(current_kind) = current_kind else {
        return false;
    };
    let needle = normalize_kind(current_kind).to_ascii_uppercase();
    kinds
        .normalized_items()
        .into_iter()
        .map(|item| normalize_kind(&item).to_ascii_uppercase())
        .any(|item| item == needle)
}

pub(crate) fn clear_kind_matches(
    clear_kinds: Option<&StringOrMany>,
    current_kind: Option<&str>,
) -> bool {
    let Some(kinds) = clear_kinds else {
        return false;
    };
    let Some(current_kind) = current_kind else {
        return false;
    };
    let needle = normalize_kind(current_kind).to_ascii_uppercase();
    kinds
        .normalized_items()
        .into_iter()
        .map(|item| normalize_kind(&item).to_ascii_uppercase())
        .any(|item| item == needle)
}

pub(crate) fn normalize_kind(kind: &str) -> &str {
    let normalized = kind.trim();
    if normalized.eq_ignore_ascii_case("req_missing") {
        "missing_dep"
    } else if normalized.eq_ignore_ascii_case("game_mismatch")
        || normalized.eq_ignore_ascii_case("mismatch")
    {
        "mismatch"
    } else if normalized.eq_ignore_ascii_case("rule_hit")
        || normalized.eq_ignore_ascii_case("forbid_hit")
    {
        "conflict"
    } else {
        normalized
    }
}

pub(crate) fn kind_disables_selection(kind: &str) -> bool {
    kind.eq_ignore_ascii_case("included")
        || kind.eq_ignore_ascii_case("not_needed")
        || kind.eq_ignore_ascii_case("not_compatible")
        || kind.eq_ignore_ascii_case("conditional")
        || kind.eq_ignore_ascii_case("mismatch")
        || kind.eq_ignore_ascii_case("path_requirement")
}

pub(crate) fn normalize_mod_key(value: &str) -> String {
    let lower = value.to_ascii_lowercase();
    let file = if let Some(idx) = lower.rfind(['/', '\\']) {
        &lower[idx + 1..]
    } else {
        &lower
    };
    let without_ext = file.strip_suffix(".tp2").unwrap_or(file);
    without_ext
        .strip_prefix("setup-")
        .unwrap_or(without_ext)
        .to_string()
}

pub(crate) fn non_empty(value: Option<&str>) -> Option<String> {
    let value = value?.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

pub(super) fn string_or_many_items(value: Option<&StringOrMany>) -> Vec<String> {
    value.map(StringOrMany::trimmed_items).unwrap_or_default()
}

fn option_list_matches(option: Option<&StringOrMany>, value: &str) -> bool {
    option
        .map(|items| {
            let needle = value.trim().to_ascii_uppercase();
            items
                .normalized_items()
                .into_iter()
                .any(|item| item == needle)
        })
        .unwrap_or(true)
}
