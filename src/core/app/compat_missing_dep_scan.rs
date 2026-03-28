// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::{Step2ComponentState, Step2ModState};

use super::compat_dependency_runtime::{
    ComponentRequirementCache, DependencyCompatHit, DependencyEvalMode, scan_dependency_hit,
};
use super::compat_rule_runtime::{active_item_order, collect_step2_active_items, normalize_mod_key};

pub(crate) fn apply_step2_scan_missing_dep(mods: &mut [Step2ModState]) {
    let active_items = collect_step2_active_items(mods);
    let mut requirement_cache = ComponentRequirementCache::new();

    for mod_state in mods {
        let current_mod_key = normalize_mod_key(&mod_state.tp_file);

        for component in &mut mod_state.components {
            if component.compat_kind.is_some() || !component.checked {
                continue;
            }

            let Some(hit) = scan_dependency_hit(
                &mod_state.tp2_path,
                &component.component_id,
                active_item_order(&active_items, &mod_state.tp_file, &component.component_id),
                &active_items,
                &mut requirement_cache,
                DependencyEvalMode::MissingOnly,
            ) else {
                continue;
            };

            if hit.kind == "missing_dep" {
                apply_missing_dependency(component, &current_mod_key, &hit);
            }
        }
    }
}

fn apply_missing_dependency(
    component: &mut Step2ComponentState,
    current_mod_key: &str,
    hit: &DependencyCompatHit,
) {
    component.disabled = false;
    component.compat_kind = Some("missing_dep".to_string());
    component.compat_source = Some(hit.source.clone());
    component.compat_related_mod = Some(hit.target_mod.clone());
    component.compat_related_component = Some(hit.target_component_id.clone());
    component.compat_graph = Some(format!(
        "{} #{} missing_dep {} #{}",
        current_mod_key,
        component.component_id.trim(),
        hit.target_mod,
        hit.target_component_id
    ));
    component.compat_evidence = Some(hit.raw_evidence.clone());
    component.disabled_reason = Some(hit.message.clone());
}
