// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Mutex, OnceLock};

use crate::ui::state::{Step1State, Step2ModState, Step3ItemState};
use crate::ui::step2::compat_types_step2::{CompatIssueDisplay, CompatIssueStatusTone};

use super::compat_conflict_runtime::{
    build_conflict_scan_context, scan_conflict_hit_with_context, ComponentConflictCache, ConflictCompatHit,
    ConflictScanContext,
};
use super::compat_dependency_runtime::{
    ComponentRequirementCache, DependencyCompatHit, DependencyEvalMode, scan_dependency_hit,
};
use super::compat_mismatch_eval::build_mismatch_context;
use super::compat_mismatch_scan::scan_predicate_guard_hit;
use super::compat_path_eval::{game_dir_for_tab, PathRequirementContext};
use super::compat_path_runtime::{ComponentPathGuardCache, PathRequirementHit, scan_path_requirement_hit};
use super::compat_rule_runtime::{
    CompatActiveItem, clear_kind_matches, collect_step3_active_items, compat_component_matches, compat_mod_matches,
    direct_rule_applies, match_kind_matches, non_empty, normalize_kind, normalize_mod_key, relation_rule_applies,
    mode_matches, single_related_target, tab_matches,
};
use super::compat_rules::{compat_rule_source_path, load_rules, rules_files_signature, CompatRule};

const STEP3_COMPAT_CACHE_LIMIT: usize = 64;

#[derive(Debug, Clone)]
pub(crate) struct Step3CompatMarker {
    pub(crate) kind: String,
    pub(crate) message: Option<String>,
    pub(crate) related_mod: Option<String>,
    pub(crate) related_component: Option<String>,
    pub(crate) source: Option<String>,
    pub(crate) raw_evidence: Option<String>,
}

pub(crate) fn collect_step3_compat_markers(
    step1: &Step1State,
    tab: &str,
    mods: &[Step2ModState],
    items: &[Step3ItemState],
) -> HashMap<String, Step3CompatMarker> {
    let cache_key = step3_compat_cache_key(step1, tab, mods, items);
    if let Some(cached) = step3_compat_cache()
        .lock()
        .expect("step3 compat cache lock poisoned")
        .get(&cache_key)
        .cloned()
    {
        return cached;
    }

    let markers = collect_step3_compat_markers_uncached(step1, tab, mods, items);
    let mut cache = step3_compat_cache()
        .lock()
        .expect("step3 compat cache lock poisoned");
    if cache.len() >= STEP3_COMPAT_CACHE_LIMIT {
        cache.clear();
    }
    cache.insert(cache_key, markers.clone());
    markers
}

fn collect_step3_compat_markers_uncached(
    step1: &Step1State,
    tab: &str,
    mods: &[Step2ModState],
    items: &[Step3ItemState],
) -> HashMap<String, Step3CompatMarker> {
    let tp2_paths = build_tp2_path_lookup(mods);
    let active_items = collect_step3_active_items(items, &tp2_paths);
    let rules = load_rules();
    let mut out = HashMap::<String, Step3CompatMarker>::new();
    let mut order = 1usize;
    let mut requirement_cache = ComponentRequirementCache::new();
    let mut conflict_cache = ComponentConflictCache::new();
    let mut path_guard_cache = ComponentPathGuardCache::new();
    let path_context = PathRequirementContext::for_tab(step1, tab);
    let conflict_context = build_conflict_scan_context(&active_items, &mut conflict_cache);
    let predicate_context =
        build_mismatch_context(step1, tab, collect_checked_components(&active_items));

    for item in items.iter().filter(|item| !item.is_parent) {
        let key = marker_key(item);
        let mut marker = scan_path_marker_for_item(item, &tp2_paths, &path_context, &mut path_guard_cache)
            .or_else(|| {
                scan_conflict_marker_for_item(
                    item,
                    order,
                    &conflict_context,
                )
            })
            .or_else(|| scan_predicate_marker_for_item(item, &tp2_paths, &predicate_context))
            .or_else(|| {
                scan_dependency_marker_for_item(
                    item,
                    order,
                    &tp2_paths,
                    &active_items,
                    &mut requirement_cache,
                )
            });
        let component_order = Some(order);
        order += 1;

        for rule in &rules {
            let current_kind = marker.as_ref().map(|value| value.kind.as_str());
            if !rule_matches(rule, step1, tab, item, current_kind) {
                continue;
            }
            clear_rule_kinds(rule, &mut marker);
            if !direct_rule_applies(rule, step1, tab) {
                continue;
            }
            apply_rule(rule, &mut marker);
        }
        for rule in &rules {
            let current_kind = marker.as_ref().map(|value| value.kind.as_str());
            if !rule_matches(rule, step1, tab, item, current_kind) {
                continue;
            }
            clear_rule_kinds(rule, &mut marker);
            if !relation_rule_applies(
                rule,
                &item.tp_file,
                &item.component_id,
                component_order,
                &active_items,
            ) {
                continue;
            }
            apply_rule(rule, &mut marker);
        }

        if let Some(marker) = marker {
            out.insert(key, marker);
        }
    }

    out
}

pub(crate) fn marker_key(item: &Step3ItemState) -> String {
    format!(
        "{}|{}|{}",
        item.tp_file.to_ascii_uppercase(),
        item.component_id,
        item.raw_line
    )
}

pub(crate) fn build_tp2_path_lookup(mods: &[Step2ModState]) -> HashMap<String, String> {
    let mut out = HashMap::<String, String>::new();
    for mod_state in mods {
        out.insert(
            tp2_lookup_key(&mod_state.tp_file, &mod_state.name),
            mod_state.tp2_path.clone(),
        );
    }
    out
}

pub(crate) fn marker_issue(marker: &Step3CompatMarker) -> CompatIssueDisplay {
    CompatIssueDisplay {
        kind: marker.kind.clone(),
        code: marker_code(marker.kind.as_str()).to_string(),
        status_label: popup_issue_status(marker.kind.as_str()).0.to_string(),
        status_tone: popup_issue_status(marker.kind.as_str()).1,
        related_mod: marker.related_mod.clone().unwrap_or_else(|| "unknown".to_string()),
        related_component: marker
            .related_component
            .as_deref()
            .and_then(|value| value.parse::<u32>().ok()),
        reason: marker.message.clone().unwrap_or_default(),
        source: marker.source.clone().unwrap_or_default(),
        raw_evidence: marker.raw_evidence.clone(),
    }
}

fn rule_matches(
    rule: &CompatRule,
    step1: &Step1State,
    tab: &str,
    item: &Step3ItemState,
    current_kind: Option<&str>,
) -> bool {
    mode_matches(rule, &step1.game_install)
        && tab_matches(rule, tab)
        && match_kind_matches(rule.match_kind.as_ref(), current_kind)
        && compat_mod_matches(rule, &item.tp_file, &item.mod_name)
        && compat_component_matches(
            rule,
            &item.component_id,
            &item.component_label,
            &item.raw_line,
        )
}

fn apply_rule(rule: &CompatRule, marker: &mut Option<Step3CompatMarker>) {
    let kind = normalize_kind(&rule.kind);
    if kind.eq_ignore_ascii_case("allow") {
        clear_marker(marker);
        return;
    }
    let related_target = single_related_target(rule);
    *marker = Some(Step3CompatMarker {
        kind: kind.to_string(),
        message: non_empty(Some(rule.message.as_str())),
        related_mod: related_target.as_ref().map(|(related_mod, _)| related_mod.clone()),
        related_component: related_target.and_then(|(_, related_component)| related_component),
        source: Some(compat_rule_source_path(rule)),
        raw_evidence: None,
    });
}

fn clear_rule_kinds(rule: &CompatRule, marker: &mut Option<Step3CompatMarker>) {
    let current_kind = marker.as_ref().map(|value| value.kind.as_str());
    if clear_kind_matches(rule.clear_kinds.as_ref(), current_kind) {
        clear_marker(marker);
    }
}

fn clear_marker(marker: &mut Option<Step3CompatMarker>) {
    *marker = None;
}

fn scan_dependency_marker_for_item(
    item: &Step3ItemState,
    component_order: usize,
    tp2_paths: &HashMap<String, String>,
    active_items: &[super::compat_rule_runtime::CompatActiveItem],
    requirement_cache: &mut ComponentRequirementCache,
) -> Option<Step3CompatMarker> {
    let tp2_path = tp2_paths
        .get(&tp2_lookup_key(&item.tp_file, &item.mod_name))
        .filter(|value| !value.trim().is_empty())?;
    scan_dependency_hit(
        tp2_path,
        &item.component_id,
        Some(component_order),
        active_items,
        requirement_cache,
        DependencyEvalMode::MissingThenOrder,
    )
    .map(requirement_marker)
}

fn requirement_marker(hit: DependencyCompatHit) -> Step3CompatMarker {
    Step3CompatMarker {
        kind: hit.kind.to_string(),
        message: Some(hit.message),
        related_mod: Some(hit.target_mod),
        related_component: Some(hit.target_component_id),
        source: Some(hit.source),
        raw_evidence: Some(hit.raw_evidence),
    }
}

fn collect_checked_components(
    active_items: &[CompatActiveItem],
) -> std::collections::HashSet<(String, String)> {
    active_items
        .iter()
        .map(|item| {
            (
                normalize_mod_key(&item.tp_file),
                item.component_id.trim().to_string(),
            )
        })
        .collect()
}

fn scan_conflict_marker_for_item(
    item: &Step3ItemState,
    component_order: usize,
    conflict_context: &ConflictScanContext,
) -> Option<Step3CompatMarker> {
    scan_conflict_hit_with_context(
        &item.tp_file,
        &item.component_id,
        Some(component_order),
        conflict_context,
    )
    .map(conflict_marker)
}

fn conflict_marker(hit: ConflictCompatHit) -> Step3CompatMarker {
    Step3CompatMarker {
        kind: hit.kind.to_string(),
        message: Some(hit.message),
        related_mod: Some(hit.target_mod),
        related_component: Some(hit.target_component_id),
        source: Some(hit.source),
        raw_evidence: Some(hit.raw_evidence),
    }
}

fn scan_predicate_marker_for_item(
    item: &Step3ItemState,
    tp2_paths: &HashMap<String, String>,
    predicate_context: &super::compat_mismatch_eval::MismatchContext,
) -> Option<Step3CompatMarker> {
    let tp2_path = tp2_paths
        .get(&tp2_lookup_key(&item.tp_file, &item.mod_name))
        .filter(|value| !value.trim().is_empty())?;
    let hit = scan_predicate_guard_hit(tp2_path, &item.component_id, predicate_context)?;
    if !hit.kind.eq_ignore_ascii_case("conflict") {
        return None;
    }
    Some(Step3CompatMarker {
        kind: hit.kind.to_string(),
        message: Some(hit.message),
        related_mod: hit.related_mod,
        related_component: hit.related_component,
        source: Some(tp2_path.to_string()),
        raw_evidence: Some(hit.raw_evidence),
    })
}

fn scan_path_marker_for_item(
    item: &Step3ItemState,
    tp2_paths: &HashMap<String, String>,
    context: &PathRequirementContext,
    path_guard_cache: &mut ComponentPathGuardCache,
) -> Option<Step3CompatMarker> {
    let tp2_path = tp2_paths
        .get(&tp2_lookup_key(&item.tp_file, &item.mod_name))
        .filter(|value| !value.trim().is_empty())?;
    scan_path_requirement_hit(tp2_path, &item.component_id, context, path_guard_cache)
        .map(path_requirement_marker)
}

fn path_requirement_marker(hit: PathRequirementHit) -> Step3CompatMarker {
    Step3CompatMarker {
        kind: hit.kind.to_string(),
        message: Some(hit.message),
        related_mod: hit.related_target,
        related_component: None,
        source: Some(hit.source),
        raw_evidence: Some(hit.raw_evidence),
    }
}

fn tp2_lookup_key(tp_file: &str, mod_name: &str) -> String {
    format!(
        "{}|{}",
        tp_file.to_ascii_uppercase(),
        mod_name.to_ascii_uppercase()
    )
}

fn step3_compat_cache() -> &'static Mutex<HashMap<u64, HashMap<String, Step3CompatMarker>>> {
    static CACHE: OnceLock<Mutex<HashMap<u64, HashMap<String, Step3CompatMarker>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn step3_compat_cache_key(
    step1: &Step1State,
    tab: &str,
    mods: &[Step2ModState],
    items: &[Step3ItemState],
) -> u64 {
    let mut hasher = DefaultHasher::new();
    let tp2_paths = build_tp2_path_lookup(mods);

    "step3-compat-v1".hash(&mut hasher);
    step1.game_install.hash(&mut hasher);
    tab.to_ascii_uppercase().hash(&mut hasher);
    game_dir_for_tab(step1, tab).unwrap_or("").hash(&mut hasher);
    rules_files_signature().hash(&mut hasher);

    for item in items.iter().filter(|item| !item.is_parent) {
        item.tp_file.to_ascii_uppercase().hash(&mut hasher);
        item.mod_name.to_ascii_uppercase().hash(&mut hasher);
        item.component_id.hash(&mut hasher);
        item.component_label.hash(&mut hasher);
        item.raw_line.hash(&mut hasher);
        item.selected_order.hash(&mut hasher);
        tp2_paths
            .get(&tp2_lookup_key(&item.tp_file, &item.mod_name))
            .cloned()
            .unwrap_or_default()
            .hash(&mut hasher);
    }

    hasher.finish()
}

fn marker_code(kind: &str) -> &'static str {
    match kind {
        "mismatch" => "MISMATCH",
        "missing_dep" => "REQ_MISSING",
        "conflict" | "not_compatible" => "RULE_HIT",
        "included" => "INCLUDED",
        "order_block" => "ORDER_BLOCK",
        "conditional" => "CONDITIONAL",
        "path_requirement" => "PATH_REQUIREMENT",
        "deprecated" => "DEPRECATED",
        _ => "RULE_HIT",
    }
}

fn popup_issue_status(kind: &str) -> (&'static str, CompatIssueStatusTone) {
    match kind {
        "included" => ("Already included", CompatIssueStatusTone::Neutral),
        "not_needed" => ("Not needed", CompatIssueStatusTone::Neutral),
        "missing_dep" | "order_block" | "warning" | "deprecated" | "conditional" => {
            ("Warning only", CompatIssueStatusTone::Warning)
        }
        _ => ("Resolve before continuing", CompatIssueStatusTone::Blocking),
    }
}
