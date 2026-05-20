// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::state::{Step1State, Step2ComponentState, Step2ModState};

use super::compat_path_eval::PathRequirementContext;
use super::compat_path_runtime::{
    ComponentPathGuardCache, PathRequirementHit, scan_path_requirement_hit,
};
use super::compat_rule_runtime::normalize_mod_key;

pub(crate) fn apply_step2_scan_path_requirement(
    step1: &Step1State,
    tab: &str,
    mods: &mut [Step2ModState],
) {
    let context = PathRequirementContext::for_tab(step1, tab);
    let mut path_guard_cache = ComponentPathGuardCache::new();

    for mod_state in mods {
        let current_mod_key = normalize_mod_key(&mod_state.tp_file);
        for component in &mut mod_state.components {
            if component.compat_kind.is_some() {
                continue;
            }

            let Some(hit) = scan_path_requirement_hit(
                &mod_state.tp2_path,
                &component.component_id,
                &context,
                &mut path_guard_cache,
            ) else {
                continue;
            };
            if hit.kind.eq_ignore_ascii_case("missing_dep") && !component.checked {
                continue;
            }

            apply_path_requirement(component, &current_mod_key, &hit);
        }
    }
}

fn apply_path_requirement(
    component: &mut Step2ComponentState,
    current_mod_key: &str,
    hit: &PathRequirementHit,
) {
    component.disabled = hit.kind.eq_ignore_ascii_case("path_requirement");
    component.compat_kind = Some(hit.kind.to_string());
    component.compat_source = Some(hit.source.clone());
    component.compat_related_mod.clone_from(&hit.related_target);
    component.compat_related_component = None;
    component.compat_graph = Some(format!(
        "{} #{} {}",
        current_mod_key,
        component.component_id.trim(),
        hit.kind
    ));
    component.compat_evidence = Some(hit.raw_evidence.clone());
    component.disabled_reason = Some(hit.message.clone());
}
