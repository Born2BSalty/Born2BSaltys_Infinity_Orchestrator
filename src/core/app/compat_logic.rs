// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::state::{Step1State, Step2ComponentState, Step2ModState};

use super::compat_conflict_scan::apply_step2_scan_conflict;
use super::compat_deprecated_scan::apply_step2_scan_deprecated;
use super::compat_mismatch_scan::apply_step2_scan_mismatch;
use super::compat_missing_dep_scan::apply_step2_scan_missing_dep;
use super::compat_path_scan::apply_step2_scan_path_requirement;
use super::compat_rule_runtime::{
    active_item_order, clear_kind_matches, collect_step2_active_items, compat_component_matches,
    compat_mod_matches, direct_rule_applies, kind_disables_selection, match_kind_matches,
    matched_related_target, mode_matches, non_empty, normalize_kind, normalize_mod_key,
    relation_rule_applies, single_related_target, tab_matches,
};
use super::compat_rules::{CompatRule, compat_rule_source_path, load_rules};

pub(crate) fn apply_step2_compat_rules(
    step1: &Step1State,
    bgee_mods: &mut [Step2ModState],
    bg2ee_mods: &mut [Step2ModState],
) -> Option<String> {
    clear_step2_compat_state(bgee_mods);
    clear_step2_compat_state(bg2ee_mods);

    apply_step2_scan_mismatch(step1, "BGEE", bgee_mods);
    apply_step2_scan_mismatch(step1, "BG2EE", bg2ee_mods);
    apply_step2_scan_path_requirement(step1, "BGEE", bgee_mods);
    apply_step2_scan_path_requirement(step1, "BG2EE", bg2ee_mods);
    apply_step2_scan_missing_dep(bgee_mods);
    apply_step2_scan_missing_dep(bg2ee_mods);
    apply_step2_scan_conflict(bgee_mods);
    apply_step2_scan_conflict(bg2ee_mods);
    apply_step2_scan_deprecated(bgee_mods);
    apply_step2_scan_deprecated(bg2ee_mods);

    let loaded = load_rules();
    let rules = loaded.rules;
    if !rules.is_empty() {
        apply_direct_rules_to_tab(step1, "BGEE", &rules, bgee_mods);
        apply_direct_rules_to_tab(step1, "BG2EE", &rules, bg2ee_mods);
        finalize_step2_compat_state(bgee_mods);
        finalize_step2_compat_state(bg2ee_mods);

        apply_relation_rules_to_tab(step1, "BGEE", &rules, bgee_mods);
        apply_relation_rules_to_tab(step1, "BG2EE", &rules, bg2ee_mods);
    }

    finalize_step2_compat_state(bgee_mods);
    finalize_step2_compat_state(bg2ee_mods);
    loaded.error
}

pub(crate) fn clear_step2_compat_state(mods: &mut [Step2ModState]) {
    for mod_state in mods {
        for component in &mut mod_state.components {
            component.disabled = false;
            component.compat_kind = None;
            component.compat_source = None;
            component.compat_related_mod = None;
            component.compat_related_component = None;
            component.compat_graph = None;
            component.compat_evidence = None;
            component.disabled_reason = None;
        }
        refresh_mod_checked_state(mod_state);
    }
}

fn apply_direct_rules_to_tab(
    step1: &Step1State,
    tab: &str,
    rules: &[CompatRule],
    mods: &mut [Step2ModState],
) {
    for mod_state in mods {
        let tp_file = mod_state.tp_file.clone();
        let mod_name = mod_state.name.clone();
        for component_idx in 0..mod_state.components.len() {
            for rule in rules {
                let component = &mod_state.components[component_idx];
                if !rule_selector_matches(rule, step1, tab, &tp_file, &mod_name, component) {
                    continue;
                }
                let component = &mut mod_state.components[component_idx];
                clear_rule_kinds(rule, component);
                if !direct_rule_applies(rule, step1, tab) {
                    continue;
                }
                apply_rule(rule, &tp_file, component, single_related_target(rule));
            }
        }
    }
}

fn apply_relation_rules_to_tab(
    step1: &Step1State,
    tab: &str,
    rules: &[CompatRule],
    mods: &mut [Step2ModState],
) {
    let active_items = collect_step2_active_items(mods);
    for mod_state in mods {
        let tp_file = mod_state.tp_file.clone();
        let mod_name = mod_state.name.clone();
        for component_idx in 0..mod_state.components.len() {
            for rule in rules {
                let component = &mod_state.components[component_idx];
                if !rule_selector_matches(rule, step1, tab, &tp_file, &mod_name, component) {
                    continue;
                }
                if normalize_kind(&rule.kind).eq_ignore_ascii_case("order_block") {
                    continue;
                }
                if normalize_kind(&rule.kind).eq_ignore_ascii_case("conflict") && !component.checked
                {
                    continue;
                }
                let component = &mut mod_state.components[component_idx];
                let component_id = component.component_id.clone();
                clear_rule_kinds(rule, component);
                if !relation_rule_applies(
                    rule,
                    &tp_file,
                    &component_id,
                    active_item_order(&active_items, &tp_file, &component_id),
                    &active_items,
                ) {
                    continue;
                }
                apply_rule(
                    rule,
                    &tp_file,
                    component,
                    matched_related_target(rule, &tp_file, &component_id, &active_items),
                );
            }
        }
    }
}

fn rule_selector_matches(
    rule: &CompatRule,
    step1: &Step1State,
    tab: &str,
    tp_file: &str,
    mod_name: &str,
    component: &Step2ComponentState,
) -> bool {
    mode_matches(rule, &step1.game_install)
        && tab_matches(rule, tab)
        && match_kind_matches(rule.match_kind.as_ref(), component.compat_kind.as_deref())
        && compat_mod_matches(rule, tp_file, mod_name)
        && compat_component_matches(
            rule,
            &component.component_id,
            &component.label,
            &component.raw_line,
        )
}

fn finalize_step2_compat_state(mods: &mut [Step2ModState]) {
    for mod_state in mods {
        for component in &mut mod_state.components {
            if component.disabled || component_should_disable_selection(component) {
                component.disabled = true;
                component.checked = false;
                component.selected_order = None;
            }
        }
        refresh_mod_checked_state(mod_state);
    }
}

fn component_should_disable_selection(component: &Step2ComponentState) -> bool {
    let Some(kind) = component.compat_kind.as_deref() else {
        return false;
    };
    if kind.eq_ignore_ascii_case("conflict")
        && component.compat_evidence.as_deref().is_some_and(|raw| {
            raw.trim_start()
                .to_ascii_uppercase()
                .starts_with("FORBID_COMPONENT")
        })
    {
        return false;
    }
    kind_disables_selection(kind)
}

fn refresh_mod_checked_state(mod_state: &mut Step2ModState) {
    mod_state.checked = mod_state
        .components
        .iter()
        .filter(|component| !component.disabled)
        .any(|component| component.checked)
        && mod_state
            .components
            .iter()
            .filter(|component| !component.disabled)
            .all(|component| component.checked);
}

fn apply_rule(
    rule: &CompatRule,
    tp_file: &str,
    component: &mut Step2ComponentState,
    related_target: Option<(String, Option<String>)>,
) {
    let kind = normalize_kind(&rule.kind);
    if kind.eq_ignore_ascii_case("allow") {
        clear_component_compat(component);
        return;
    }
    component.disabled = false;
    component.compat_kind = Some(kind.to_string());
    component.compat_source = Some(compat_rule_source_path(rule));
    component.compat_related_mod = related_target
        .as_ref()
        .map(|(related_mod, _)| related_mod.clone());
    component.compat_related_component =
        related_target.and_then(|(_, related_component)| related_component);
    component.compat_graph = build_graph(tp_file, component.component_id.as_str(), kind, rule);
    component.compat_evidence = Some(rule_debug_line(rule));
    component.disabled_reason = non_empty(Some(rule.message.as_str()));
    if kind.eq_ignore_ascii_case("deprecated") || kind_disables_selection(kind) {
        component.disabled = true;
    }
}

fn clear_rule_kinds(rule: &CompatRule, component: &mut Step2ComponentState) {
    if clear_kind_matches(rule.clear_kinds.as_ref(), component.compat_kind.as_deref()) {
        clear_component_compat(component);
    }
}

fn clear_component_compat(component: &mut Step2ComponentState) {
    component.disabled = false;
    component.compat_kind = None;
    component.compat_source = None;
    component.compat_related_mod = None;
    component.compat_related_component = None;
    component.compat_graph = None;
    component.compat_evidence = None;
    component.disabled_reason = None;
}

fn build_graph(tp_file: &str, component_id: &str, kind: &str, rule: &CompatRule) -> Option<String> {
    let (related_mod, related_component) = single_related_target(rule)?;
    let left = format!("{} #{}", normalize_mod_key(tp_file), component_id.trim());
    let right = if related_component.as_deref().is_none_or(str::is_empty) {
        normalize_mod_key(&related_mod)
    } else {
        format!(
            "{} #{}",
            normalize_mod_key(&related_mod),
            related_component.unwrap_or_default()
        )
    };
    Some(format!("{left} {kind} {right}"))
}

fn rule_debug_line(rule: &CompatRule) -> String {
    let mut parts = vec![format!("kind={}", rule.kind.trim())];
    let mod_items = rule.r#mod.trimmed_items();
    if !mod_items.is_empty() {
        parts.insert(0, format!("mod={}", mod_items.join(",")));
    }
    if let Some(component) = rule.component.as_ref() {
        let items = component.trimmed_items();
        if !items.is_empty() {
            parts.push(format!("component={}", items.join(",")));
        }
    }
    if let Some(component_id) = rule.component_id.as_ref() {
        let items = component_id.trimmed_items();
        if !items.is_empty() {
            parts.push(format!("component_id={}", items.join(",")));
        }
    }
    if let Some(match_kind) = rule.match_kind.as_ref() {
        parts.push(format!(
            "match_kind={}",
            match_kind.normalized_items().join(",")
        ));
    }
    if let Some(clear_kinds) = rule.clear_kinds.as_ref() {
        parts.push(format!(
            "clear_kinds={}",
            clear_kinds.normalized_items().join(",")
        ));
    }
    if let Some(position) = non_empty(rule.position.as_deref()) {
        parts.push(format!("position={position}"));
    }
    if let Some(path_field) = non_empty(rule.path_field.as_deref()) {
        parts.push(format!("path_field={path_field}"));
    }
    if let Some(game_file) = non_empty(rule.game_file.as_deref()) {
        parts.push(format!("game_file={game_file}"));
    }
    if let Some(game_file_check) = non_empty(rule.game_file_check.as_deref()) {
        parts.push(format!("game_file_check={game_file_check}"));
    }
    if let Some(related_mod) = rule.related_mod.as_ref() {
        let items = related_mod.trimmed_items();
        if !items.is_empty() {
            parts.push(format!("related_mod={}", items.join(",")));
        }
    }
    if let Some(related_component) = rule.related_component.as_ref() {
        let items = related_component.trimmed_items();
        if !items.is_empty() {
            parts.push(format!("related_component={}", items.join(",")));
        }
    }
    if let Some(message) = non_empty(Some(rule.message.as_str())) {
        parts.push(format!("message={message}"));
    }
    parts.join(" | ")
}
