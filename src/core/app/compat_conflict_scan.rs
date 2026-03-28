// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::{Step2ComponentState, Step2ModState};

use super::compat_conflict_runtime::{ComponentConflictCache, ConflictCompatHit, scan_conflict_hit};
use super::compat_rule_runtime::{active_item_order, collect_step2_active_items, normalize_mod_key};

pub(crate) fn apply_step2_scan_conflict(mods: &mut [Step2ModState]) {
    let active_items = collect_step2_active_items(mods);
    let mut conflict_cache = ComponentConflictCache::new();

    for mod_state in mods {
        let current_mod_key = normalize_mod_key(&mod_state.tp_file);

        for component in &mut mod_state.components {
            if component
                .compat_kind
                .as_deref()
                .is_some_and(|kind| !kind.eq_ignore_ascii_case("missing_dep"))
            {
                continue;
            }

            let Some(hit) = scan_conflict_hit(
                &mod_state.tp2_path,
                &mod_state.tp_file,
                &component.component_id,
                active_item_order(&active_items, &mod_state.tp_file, &component.component_id),
                &active_items,
                &mut conflict_cache,
            ) else {
                continue;
            };

            apply_conflict(component, &current_mod_key, &hit);
        }
    }
}

fn apply_conflict(
    component: &mut Step2ComponentState,
    current_mod_key: &str,
    hit: &ConflictCompatHit,
) {
    component.disabled = false;
    component.compat_kind = Some(hit.kind.to_string());
    component.compat_source = Some(hit.source.clone());
    component.compat_related_mod = Some(hit.target_mod.clone());
    component.compat_related_component = Some(hit.target_component_id.clone());
    component.compat_graph = Some(format!(
        "{} #{} {} {} #{}",
        current_mod_key,
        component.component_id.trim(),
        hit.kind,
        hit.target_mod,
        hit.target_component_id
    ));
    component.compat_evidence = Some(hit.raw_evidence.clone());
    component.disabled_reason = Some(hit.message.clone());
}
