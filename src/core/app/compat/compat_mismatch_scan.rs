// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::{HashMap, HashSet};

use crate::app::state::{Step1State, Step2ModState};

use super::compat_mismatch_eval::build_mismatch_context;
use super::compat_rule_runtime::{kind_disables_selection, normalize_mod_key};

#[path = "compat_mismatch_scan_classify.rs"]
mod classify;
#[path = "compat_mismatch_scan_guards.rs"]
mod guards;

pub(super) use classify::PredicateGuardHit;
use classify::preferred_guard_hit;
#[cfg(test)]
use classify::{classify_guard, preferred_failing_guard};
#[cfg(test)]
use guards::collect_requirement_guards;
use guards::load_component_guards;
pub(in crate::app) use guards::{
    RequirementGuard, guards_from_loaded_text, ordinary_guards_from_text,
};

#[cfg(test)]
pub(crate) fn apply_step2_scan_mismatch(step1: &Step1State, tab: &str, mods: &mut [Step2ModState]) {
    apply_step2_scan_mismatch_with_metadata(
        step1,
        tab,
        mods,
        &super::compat_predicate_constraints::builtin_metadata(),
    );
}

pub(in crate::app) fn apply_step2_scan_mismatch_with_metadata(
    step1: &Step1State,
    tab: &str,
    mods: &mut [Step2ModState],
    metadata: &super::compat_predicate_constraints::ConstraintMetadata,
) -> Vec<String> {
    use super::compat_predicate_constraints::{LoadedSources, covered_records, resolve_source};
    let context = build_mismatch_context(step1, tab, collect_checked_components(mods));
    let mut guard_cache = HashMap::<String, HashMap<String, Vec<RequirementGuard>>>::new();
    let mut loaded = LoadedSources::new();
    let mut covered = HashMap::new();
    let mut diagnostics = metadata.diagnostics.clone();
    for (mod_index, mod_state) in mods.iter().enumerate() {
        for (component_index, component) in mod_state.components.iter().enumerate() {
            let records = covered_records(
                metadata,
                step1,
                tab,
                &mod_state.tp_file,
                &component.component_id,
            );
            if records.is_empty() {
                continue;
            }
            let source = resolve_source(
                &records,
                &mod_state.tp_file,
                &component.component_id,
                mods,
                &mut loaded,
            );
            if let Some(error) = &source.error {
                diagnostics.push(format!(
                    "source_unverified: {} #{}: {error}",
                    source.path, component.component_id
                ));
            }
            covered.insert((mod_index, component_index), source);
        }
    }
    for mod_state in mods.iter() {
        if let Ok(path) = std::path::Path::new(&mod_state.tp2_path).canonicalize()
            && let Some(source) = loaded.get(&path)
        {
            guard_cache.insert(mod_state.tp2_path.clone(), source.guards.clone());
        }
    }

    for (mod_index, mod_state) in mods.iter_mut().enumerate() {
        let component_guards = guard_cache
            .entry(mod_state.tp2_path.clone())
            .or_insert_with(|| load_component_guards(&mod_state.tp2_path));

        for (component_index, component) in mod_state.components.iter_mut().enumerate() {
            let component_context =
                context.for_component(&mod_state.tp_file, &component.component_id);
            let hit = covered.get(&(mod_index, component_index)).map_or_else(
                || {
                    component_guards
                        .get(component.component_id.trim())
                        .and_then(|guards| preferred_guard_hit(guards, &component_context))
                },
                |source| scan_loaded_predicate_hit(source, &component_context),
            );
            let Some(hit) = hit else {
                continue;
            };
            if hit.kind.eq_ignore_ascii_case("conflict") && !component.checked {
                continue;
            }

            component.disabled = kind_disables_selection(hit.kind);
            component.compat_kind = Some(hit.kind.to_string());
            component.compat_source =
                Some(mismatch_source(&mod_state.tp2_path, &mod_state.tp_file));
            component.compat_related_mod.clone_from(&hit.related_mod);
            component
                .compat_related_component
                .clone_from(&hit.related_component);
            component.compat_graph = None;
            component.compat_evidence = Some(hit.raw_evidence);
            component.disabled_reason = Some(hit.message);
        }
    }
    diagnostics
}

pub(in crate::app) fn scan_loaded_predicate_hit(
    source: &super::compat_predicate_constraints::CoveredSource,
    context: &super::compat_mismatch_eval::MismatchContext,
) -> Option<PredicateGuardHit> {
    let guards = source
        .guards
        .iter()
        .enumerate()
        .filter(|(index, _)| !source.matched.contains(index))
        .map(|(_, guard)| guard.clone())
        .collect::<Vec<_>>();
    preferred_guard_hit(&guards, context)
}

pub(in crate::app) fn scan_predicate_guard_hit(
    tp2_path: &str,
    component_id: &str,
    context: &super::compat_mismatch_eval::MismatchContext,
) -> Option<PredicateGuardHit> {
    if tp2_path.trim().is_empty() {
        return None;
    }
    let guards_by_component = load_component_guards(tp2_path);
    let guards = guards_by_component.get(component_id.trim())?;
    preferred_guard_hit(guards, context)
}

fn collect_checked_components(mods: &[Step2ModState]) -> HashSet<(String, String)> {
    let mut checked_components = HashSet::<(String, String)>::new();
    for mod_state in mods {
        let mod_key = normalize_mod_key(&mod_state.tp_file);
        for component in &mod_state.components {
            if component.checked {
                checked_components
                    .insert((mod_key.clone(), component.component_id.trim().to_string()));
            }
        }
    }
    checked_components
}

fn mismatch_source(tp2_path: &str, tp_file: &str) -> String {
    let trimmed = tp2_path.trim();
    if trimmed.is_empty() {
        tp_file.to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
#[path = "compat_mismatch_scan_tests.rs"]
mod tests;
