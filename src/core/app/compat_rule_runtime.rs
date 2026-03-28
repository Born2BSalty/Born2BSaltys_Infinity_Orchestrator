// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashMap;
use std::path::Path;

use crate::ui::state::{Step1State, Step2ModState, Step3ItemState};

use super::compat_rules::{CompatRule, StringOrMany};

#[derive(Debug, Clone)]
pub(crate) struct CompatActiveItem {
    pub(crate) tp_file: String,
    pub(crate) mod_name: String,
    pub(crate) tp2_path: String,
    pub(crate) component_id: String,
    pub(crate) order: Option<usize>,
}

pub(crate) fn collect_step2_active_items(mods: &[Step2ModState]) -> Vec<CompatActiveItem> {
    let mut out = Vec::<(usize, CompatActiveItem)>::new();
    let mut discovery_index = 0usize;
    for mod_state in mods {
        for component in mod_state
            .components
            .iter()
            .filter(|component| component.checked && !component.disabled)
        {
            out.push((discovery_index, CompatActiveItem {
                tp_file: mod_state.tp_file.clone(),
                mod_name: mod_state.name.clone(),
                tp2_path: mod_state.tp2_path.clone(),
                component_id: component.component_id.clone(),
                order: component.selected_order,
            }));
            discovery_index += 1;
        }
    }
    out.sort_by_key(|(idx, item)| (item.order.unwrap_or(usize::MAX), *idx));
    out.into_iter()
        .enumerate()
        .map(|(idx, (_discovery_index, mut item))| {
            item.order = Some(idx + 1);
            item
        })
        .collect()
}

pub(crate) fn collect_step3_active_items(
    items: &[Step3ItemState],
    tp2_paths: &HashMap<String, String>,
) -> Vec<CompatActiveItem> {
    let mut out = Vec::<CompatActiveItem>::new();
    let mut order = 1usize;
    for item in items.iter().filter(|item| !item.is_parent) {
        let tp2_path = tp2_paths
            .get(&format!(
                "{}|{}",
                item.tp_file.to_ascii_uppercase(),
                item.mod_name.to_ascii_uppercase()
            ))
            .cloned()
            .unwrap_or_default();
        out.push(CompatActiveItem {
            tp_file: item.tp_file.clone(),
            mod_name: item.mod_name.clone(),
            tp2_path,
            component_id: item.component_id.clone(),
            order: Some(order),
        });
        order += 1;
    }
    out
}

pub(crate) fn active_item_order(
    active_items: &[CompatActiveItem],
    tp_file: &str,
    component_id: &str,
) -> Option<usize> {
    active_items.iter().find_map(|item| {
        (item.tp_file.trim().eq_ignore_ascii_case(tp_file.trim())
            && item
                .component_id
                .trim()
                .eq_ignore_ascii_case(component_id.trim()))
        .then_some(item.order)
        .flatten()
    })
}

pub(crate) fn mode_matches(rule: &CompatRule, selected_mode: &str) -> bool {
    option_list_matches(rule.mode.as_ref(), selected_mode)
}

pub(crate) fn tab_matches(rule: &CompatRule, tab: &str) -> bool {
    option_list_matches(rule.tab.as_ref(), tab)
}

pub(crate) fn compat_mod_matches(rule: &CompatRule, tp_file: &str, mod_name: &str) -> bool {
    let rule_key = normalize_mod_key(&rule.r#mod);
    if rule_key.is_empty() {
        return false;
    }
    normalize_mod_key(tp_file) == rule_key
        || normalize_mod_key(mod_name) == rule_key
        || mod_name.trim().eq_ignore_ascii_case(rule.r#mod.trim())
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
        component_ids
            .iter()
            .any(|rule_id| component_id.trim().eq_ignore_ascii_case(rule_id.trim()))
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

pub(crate) fn rule_uses_related_target(rule: &CompatRule) -> bool {
    non_empty(rule.related_mod.as_deref()).is_some()
}

pub(crate) fn rule_uses_path_requirement(rule: &CompatRule) -> bool {
    non_empty(rule.path_field.as_deref()).is_some()
}

pub(crate) fn direct_rule_applies(rule: &CompatRule, step1: &Step1State) -> bool {
    if normalize_kind(&rule.kind).eq_ignore_ascii_case("path_requirement")
        && rule_uses_path_requirement(rule)
    {
        return path_requirement_unmet(
            step1,
            rule.path_field.as_deref().unwrap_or_default(),
            rule.path_check.as_deref(),
        );
    }
    !rule_uses_related_target(rule)
}

pub(crate) fn relation_rule_applies(
    rule: &CompatRule,
    current_tp_file: &str,
    current_component_id: &str,
    component_order: Option<usize>,
    active_items: &[CompatActiveItem],
) -> bool {
    if !rule_uses_related_target(rule) {
        return false;
    }

    match normalize_kind(&rule.kind) {
        "conflict" => target_selected(active_items, rule, current_tp_file, current_component_id),
        "missing_dep" => {
            !target_selected(active_items, rule, current_tp_file, current_component_id)
        }
        "order_block" => {
            let Some(component_order) = component_order else {
                return false;
            };
            let Some(target_order) =
                target_order(active_items, rule, current_tp_file, current_component_id)
            else {
                return false;
            };
            match non_empty(rule.position.as_deref())
                .unwrap_or_default()
                .to_ascii_lowercase()
                .as_str()
            {
                "before" => component_order > target_order,
                "after" => component_order < target_order,
                _ => false,
            }
        }
        _ => false,
    }
}

fn option_list_matches(option: Option<&StringOrMany>, value: &str) -> bool {
    option
        .map(|items| {
            let needle = value.trim().to_ascii_uppercase();
            items.normalized_items().into_iter().any(|item| item == needle)
        })
        .unwrap_or(true)
}

fn target_selected(
    active_items: &[CompatActiveItem],
    rule: &CompatRule,
    current_tp_file: &str,
    current_component_id: &str,
) -> bool {
    active_items
        .iter()
        .any(|item| target_matches(rule, item, current_tp_file, current_component_id))
}

fn target_order(
    active_items: &[CompatActiveItem],
    rule: &CompatRule,
    current_tp_file: &str,
    current_component_id: &str,
) -> Option<usize> {
    active_items
        .iter()
        .find(|item| target_matches(rule, item, current_tp_file, current_component_id))
        .and_then(|item| item.order)
}

fn target_matches(
    rule: &CompatRule,
    item: &CompatActiveItem,
    current_tp_file: &str,
    current_component_id: &str,
) -> bool {
    let related_mod = non_empty(rule.related_mod.as_deref()).unwrap_or_default();
    if related_mod.is_empty() {
        return false;
    }
    if normalize_mod_key(&item.tp_file) != normalize_mod_key(&related_mod)
        && normalize_mod_key(&item.mod_name) != normalize_mod_key(&related_mod)
        && !item.mod_name.trim().eq_ignore_ascii_case(related_mod.trim())
    {
        return false;
    }
    if normalize_mod_key(&item.tp_file) == normalize_mod_key(current_tp_file)
        && item
            .component_id
            .trim()
            .eq_ignore_ascii_case(current_component_id.trim())
    {
        return false;
    }

    if let Some(component_id) = non_empty(rule.related_component.as_deref()) {
        item.component_id.trim().eq_ignore_ascii_case(component_id.trim())
    } else {
        true
    }
}

fn path_requirement_unmet(step1: &Step1State, field: &str, check: Option<&str>) -> bool {
    let Some(path) = path_field_value(step1, field) else {
        return true;
    };
    if path.is_empty() {
        return true;
    }
    matches!(
        non_empty(check).unwrap_or_default().to_ascii_lowercase().as_str(),
        "exists" | "dir_exists" | "file_exists"
    ) && !Path::new(path).exists()
}

fn path_field_value<'a>(step1: &'a Step1State, field: &str) -> Option<&'a str> {
    let value = match field.trim().to_ascii_lowercase().as_str() {
        "weidu_log_folder" => step1.weidu_log_folder.trim(),
        "mod_installer_binary" => step1.mod_installer_binary.trim(),
        "bgee_game_folder" => step1.bgee_game_folder.trim(),
        "bgee_log_folder" => step1.bgee_log_folder.trim(),
        "bgee_log_file" => step1.bgee_log_file.trim(),
        "bg2ee_game_folder" => step1.bg2ee_game_folder.trim(),
        "bg2ee_log_folder" => step1.bg2ee_log_folder.trim(),
        "bg2ee_log_file" => step1.bg2ee_log_file.trim(),
        "eet_bgee_game_folder" => step1.eet_bgee_game_folder.trim(),
        "eet_bgee_log_folder" => step1.eet_bgee_log_folder.trim(),
        "eet_bg2ee_game_folder" => step1.eet_bg2ee_game_folder.trim(),
        "eet_bg2ee_log_folder" => step1.eet_bg2ee_log_folder.trim(),
        "eet_pre_dir" => step1.eet_pre_dir.trim(),
        "eet_new_dir" => step1.eet_new_dir.trim(),
        "game" => step1.game.trim(),
        "log_file" => step1.log_file.trim(),
        "generate_directory" => step1.generate_directory.trim(),
        "mods_folder" => step1.mods_folder.trim(),
        "weidu_binary" => step1.weidu_binary.trim(),
        _ => return None,
    };
    Some(value)
}
