// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::Path;

use crate::app::compat_rules::CompatRule;
use crate::app::state::Step1State;

use super::{
    CompatActiveItem, matches::string_or_many_items, non_empty, normalize_kind, normalize_mod_key,
};

pub(super) fn rule_uses_related_target(rule: &CompatRule) -> bool {
    !string_or_many_items(rule.related_mod.as_ref()).is_empty()
}

pub(super) fn rule_uses_path_requirement(rule: &CompatRule) -> bool {
    non_empty(rule.path_field.as_deref()).is_some()
}

pub(crate) fn single_related_target(rule: &CompatRule) -> Option<(String, Option<String>)> {
    let mut targets = related_targets(rule).into_iter();
    let first = targets.next()?;
    if targets.next().is_none() {
        Some(first)
    } else {
        None
    }
}

pub(crate) fn matched_related_target(
    rule: &CompatRule,
    current_tp_file: &str,
    current_component_id: &str,
    active_items: &[CompatActiveItem],
) -> Option<(String, Option<String>)> {
    let targets = related_targets(rule);
    active_items.iter().find_map(|item| {
        targets.iter().find_map(|(related_mod, related_component)| {
            target_matches_one(
                item,
                current_tp_file,
                current_component_id,
                related_mod,
                related_component.as_deref(),
            )
            .then(|| (related_mod.clone(), related_component.clone()))
        })
    })
}

pub(crate) fn direct_rule_applies(rule: &CompatRule, step1: &Step1State, tab: &str) -> bool {
    if rule_uses_related_target(rule) {
        return false;
    }
    if normalize_kind(&rule.kind).eq_ignore_ascii_case("path_requirement")
        && rule_uses_path_requirement(rule)
    {
        return path_requirement_unmet(
            step1,
            rule.path_field.as_deref().unwrap_or_default(),
            rule.path_check.as_deref(),
        );
    }
    if let Some(game_file) = non_empty(rule.game_file.as_deref()) {
        return game_file_rule_applies(step1, tab, &game_file, rule.game_file_check.as_deref());
    }
    true
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
        "conflict" | "conditional" => {
            target_selected(active_items, rule, current_tp_file, current_component_id)
        }
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

pub(crate) fn game_dir_for_tab<'a>(step1: &'a Step1State, tab: &str) -> Option<&'a str> {
    let value = if tab.eq_ignore_ascii_case("BGEE") {
        if step1.game_install.eq_ignore_ascii_case("EET") {
            if step1.new_pre_eet_dir_enabled && !step1.eet_pre_dir.trim().is_empty() {
                step1.eet_pre_dir.trim()
            } else {
                step1.eet_bgee_game_folder.trim()
            }
        } else if step1.generate_directory_enabled && !step1.generate_directory.trim().is_empty() {
            step1.generate_directory.trim()
        } else {
            step1.bgee_game_folder.trim()
        }
    } else if step1.game_install.eq_ignore_ascii_case("EET") {
        if step1.new_eet_dir_enabled && !step1.eet_new_dir.trim().is_empty() {
            step1.eet_new_dir.trim()
        } else {
            step1.eet_bg2ee_game_folder.trim()
        }
    } else if step1.generate_directory_enabled && !step1.generate_directory.trim().is_empty() {
        step1.generate_directory.trim()
    } else {
        step1.bg2ee_game_folder.trim()
    };

    if value.is_empty() { None } else { Some(value) }
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
    related_targets(rule)
        .into_iter()
        .any(|(related_mod, related_component)| {
            target_matches_one(
                item,
                current_tp_file,
                current_component_id,
                &related_mod,
                related_component.as_deref(),
            )
        })
}

fn target_matches_one(
    item: &CompatActiveItem,
    current_tp_file: &str,
    current_component_id: &str,
    related_mod: &str,
    related_component: Option<&str>,
) -> bool {
    if normalize_mod_key(&item.tp_file) != normalize_mod_key(related_mod)
        && normalize_mod_key(&item.mod_name) != normalize_mod_key(related_mod)
        && !item
            .mod_name
            .trim()
            .eq_ignore_ascii_case(related_mod.trim())
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

    non_empty(related_component).is_none_or(|component_id| {
        item.component_id
            .trim()
            .eq_ignore_ascii_case(component_id.trim())
    })
}

fn related_targets(rule: &CompatRule) -> Vec<(String, Option<String>)> {
    let related_mods = string_or_many_items(rule.related_mod.as_ref());
    if related_mods.is_empty() {
        return Vec::new();
    }

    let related_components = string_or_many_items(rule.related_component.as_ref());
    if related_components.is_empty() {
        return related_mods
            .into_iter()
            .map(|related_mod| (related_mod, None))
            .collect();
    }

    if related_mods.len() == 1 {
        let related_mod = related_mods[0].clone();
        return related_components
            .into_iter()
            .map(|related_component| (related_mod.clone(), Some(related_component)))
            .collect();
    }

    if related_mods.len() == related_components.len() {
        return related_mods
            .into_iter()
            .zip(related_components)
            .map(|(related_mod, related_component)| (related_mod, Some(related_component)))
            .collect();
    }

    Vec::new()
}

fn path_requirement_unmet(step1: &Step1State, field: &str, check: Option<&str>) -> bool {
    let Some(path) = path_field_value(step1, field) else {
        return true;
    };
    if path.is_empty() {
        return true;
    }
    matches!(
        non_empty(check)
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "exists" | "dir_exists" | "file_exists"
    ) && !Path::new(path).exists()
}

fn game_file_rule_applies(
    step1: &Step1State,
    tab: &str,
    rel_path: &str,
    check: Option<&str>,
) -> bool {
    let Some(game_dir) = game_dir_for_tab(step1, tab) else {
        return false;
    };
    let exists = Path::new(game_dir)
        .join(rel_path.replace('\\', "/"))
        .exists();
    match non_empty(check)
        .unwrap_or_else(|| "exists".to_string())
        .to_ascii_lowercase()
        .as_str()
    {
        "exists" | "file_exists" => exists,
        "missing" | "not_exists" | "absent" => !exists,
        _ => false,
    }
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
