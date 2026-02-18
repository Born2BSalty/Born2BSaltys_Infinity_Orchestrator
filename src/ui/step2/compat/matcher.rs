// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::Path;

use crate::ui::state::{Step1State, Step2ComponentState};

use super::model::Step2CompatRule;

pub fn match_rule(
    rule: &Step2CompatRule,
    step1: &Step1State,
    tab: &str,
    mod_name: &str,
    tp_file: &str,
    component: &Step2ComponentState,
) -> bool {
    if !rule_matches_mode(rule, step1) {
        return false;
    }
    if !rule_matches_tab(rule, tab) {
        return false;
    }
    if !rule_matches_mod(rule, mod_name, tp_file) {
        return false;
    }
    if !rule_matches_component_id(rule, component) {
        return false;
    }
    if !rule_matches_component_text(rule, component) {
        return false;
    }
    true
}

pub fn rule_disables_component(rule: &Step2CompatRule) -> bool {
    matches!(
        rule.kind.trim().to_ascii_lowercase().as_str(),
        "included" | "not_needed" | "not_compatible"
    )
}

fn rule_matches_mode(rule: &Step2CompatRule, step1: &Step1State) -> bool {
    let Some(mode) = &rule.mode else {
        return true;
    };
    let wanted = step1.game_install.to_ascii_uppercase();
    mode.normalized_items().iter().any(|m| m == &wanted)
}

fn rule_matches_tab(rule: &Step2CompatRule, tab: &str) -> bool {
    let Some(tab_value) = &rule.tab else {
        return true;
    };
    let wanted = tab.to_ascii_uppercase();
    tab_value.normalized_items().iter().any(|t| t == &wanted)
}

fn rule_matches_mod(rule: &Step2CompatRule, mod_name: &str, tp_file: &str) -> bool {
    let expected = normalize_key(rule.r#mod.as_str());
    let mod_name_norm = normalize_key(mod_name);
    let tp_file_norm = normalize_key(tp_file);
    let tp_stem = normalize_tp2_stem(tp_file);
    expected == mod_name_norm || expected == tp_file_norm || expected == tp_stem
}

fn rule_matches_component_id(rule: &Step2CompatRule, component: &Step2ComponentState) -> bool {
    let Some(component_id) = &rule.component_id else {
        return true;
    };
    component.component_id.trim() == component_id.trim()
}

fn rule_matches_component_text(rule: &Step2CompatRule, component: &Step2ComponentState) -> bool {
    let Some(component_text) = &rule.component else {
        return true;
    };
    component
        .label
        .to_ascii_lowercase()
        .contains(&component_text.to_ascii_lowercase())
}

fn normalize_key(value: &str) -> String {
    value
        .to_ascii_lowercase()
        .replace('\\', "/")
        .trim()
        .to_string()
}

fn normalize_tp2_stem(value: &str) -> String {
    let filename = Path::new(value)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(value)
        .to_ascii_lowercase();
    let stem = filename.strip_suffix(".tp2").unwrap_or(&filename);
    stem.strip_prefix("setup-")
        .unwrap_or(stem)
        .to_string()
}
