// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde_json::json;

use crate::app::compat_logic::apply_step2_compat_rules;
use crate::app::compat_rule_runtime::CompatActiveItem;
use crate::app::compat_rule_runtime::{
    active_item_order, collect_step2_active_items, compat_component_matches, compat_mod_matches,
    direct_rule_applies, match_kind_matches, mode_matches, relation_rule_applies, tab_matches,
};
use crate::app::compat_rules::{
    CompatRule, compat_rule_source_bucket, compat_rule_source_path, load_rules,
};
use crate::app::state::{Step2ComponentState, Step2ModState, WizardState};

pub(super) fn write_compat_rule_trace_json(
    run_dir: &Path,
    state: &WizardState,
    timestamp_unix_secs: u64,
) -> Result<PathBuf> {
    let out_path = run_dir.join("compat_rule_trace.json");
    let loaded = load_rules();
    let load_error = loaded.error.clone();
    let rules = loaded.rules;

    let mut bgee_recomputed = state.step2.bgee_mods.clone();
    let mut bg2ee_recomputed = state.step2.bg2ee_mods.clone();
    let _ = apply_step2_compat_rules(&state.step1, &mut bgee_recomputed, &mut bg2ee_recomputed);

    let rows = vec![
        trace_tab(
            "BGEE",
            state,
            &state.step2.bgee_mods,
            &bgee_recomputed,
            &rules,
        ),
        trace_tab(
            "BG2EE",
            state,
            &state.step2.bg2ee_mods,
            &bg2ee_recomputed,
            &rules,
        ),
    ];

    let payload = json!({
        "schema_version": 1,
        "generated_at_unix": timestamp_unix_secs,
        "rule_count": rules.len(),
        "load_error": load_error,
        "tabs": rows,
    });

    fs::write(&out_path, serde_json::to_string_pretty(&payload)?)?;
    Ok(out_path)
}

fn trace_tab(
    tab: &str,
    state: &WizardState,
    actual_mods: &[Step2ModState],
    recomputed_mods: &[Step2ModState],
    rules: &[CompatRule],
) -> serde_json::Value {
    let active_items = collect_step2_active_items(actual_mods);
    let mut components = Vec::<serde_json::Value>::new();

    for (actual_mod, recomputed_mod) in actual_mods.iter().zip(recomputed_mods.iter()) {
        for (actual_component, recomputed_component) in actual_mod
            .components
            .iter()
            .zip(recomputed_mod.components.iter())
        {
            let active_order = active_item_order(
                &active_items,
                &actual_mod.tp_file,
                &actual_component.component_id,
            );
            let rule_matches = build_rule_matches(
                state,
                tab,
                actual_mod,
                actual_component,
                rules,
                &active_items,
                active_order,
            );
            let actual_kind = actual_component.compat_kind.as_deref().unwrap_or_default();
            let recomputed_kind = recomputed_component
                .compat_kind
                .as_deref()
                .unwrap_or_default();
            let has_difference = actual_kind != recomputed_kind
                || actual_component.disabled != recomputed_component.disabled
                || actual_component.disabled_reason != recomputed_component.disabled_reason;
            let has_rule_activity = !rule_matches.is_empty();

            if !has_difference && !has_rule_activity {
                continue;
            }

            components.push(json!({
                "mod_name": actual_mod.name,
                "tp_file": actual_mod.tp_file,
                "component_id": actual_component.component_id,
                "label": actual_component.label,
                "selected_order_raw": actual_component.selected_order,
                "selected_order_active": active_order,
                "actual": {
                    "compat_kind": actual_component.compat_kind,
                    "disabled": actual_component.disabled,
                    "disabled_reason": actual_component.disabled_reason,
                    "compat_source": actual_component.compat_source,
                },
                "recomputed": {
                    "compat_kind": recomputed_component.compat_kind,
                    "disabled": recomputed_component.disabled,
                    "disabled_reason": recomputed_component.disabled_reason,
                    "compat_source": recomputed_component.compat_source,
                },
                "rule_matches": rule_matches,
            }));
        }
    }

    json!({
        "tab": tab,
        "component_count": components.len(),
        "components": components,
    })
}

fn build_rule_matches(
    state: &WizardState,
    tab: &str,
    mod_state: &Step2ModState,
    component: &Step2ComponentState,
    rules: &[CompatRule],
    active_items: &[CompatActiveItem],
    active_order: Option<usize>,
) -> Vec<serde_json::Value> {
    let mut out = Vec::<serde_json::Value>::new();

    for (rule_index, rule) in rules.iter().enumerate() {
        let mode_match = mode_matches(rule, &state.step1.game_install);
        let tab_match = tab_matches(rule, tab);
        let kind_match =
            match_kind_matches(rule.match_kind.as_ref(), component.compat_kind.as_deref());
        let mod_match = compat_mod_matches(rule, &mod_state.tp_file, &mod_state.name);
        let component_match = compat_component_matches(
            rule,
            &component.component_id,
            &component.label,
            &component.raw_line,
        );
        let selector_match = mode_match && tab_match && kind_match && mod_match && component_match;
        let direct_match = selector_match && direct_rule_applies(rule, &state.step1, tab);
        let relation_match = selector_match
            && relation_rule_applies(
                rule,
                &mod_state.tp_file,
                &component.component_id,
                active_order,
                active_items,
            );

        if !(mod_match || direct_match || relation_match) {
            continue;
        }

        out.push(json!({
            "rule_index": rule_index,
            "kind": rule.kind,
            "message": rule.message,
            "source_bucket": compat_rule_source_bucket(rule),
            "source_path": compat_rule_source_path(rule),
            "component": rule.component.as_ref().map(|value| value.trimmed_items()),
            "component_id": rule.component_id.as_ref().map(|value| value.trimmed_items()),
            "mode_match": mode_match,
            "tab_match": tab_match,
            "match_kind_match": kind_match,
            "mod_match": mod_match,
            "component_match": component_match,
            "selector_match": selector_match,
            "direct_match": direct_match,
            "relation_match": relation_match,
        }));
    }

    out
}
