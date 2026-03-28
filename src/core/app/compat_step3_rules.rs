// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashMap;

use crate::ui::state::{Step1State, Step2ModState, Step3ItemState};
use crate::ui::step2::compat_types_step2::{CompatIssueDisplay, CompatIssueStatusTone};

use super::compat_conflict_runtime::{ComponentConflictCache, ConflictCompatHit, scan_conflict_hit};
use super::compat_dependency_runtime::{
    ComponentRequirementCache, DependencyCompatHit, DependencyEvalMode, scan_dependency_hit,
};
use super::compat_path_eval::PathRequirementContext;
use super::compat_path_runtime::{ComponentPathGuardCache, PathRequirementHit, scan_path_requirement_hit};
use super::compat_rule_runtime::{
    clear_kind_matches, collect_step3_active_items, compat_component_matches, compat_mod_matches,
    direct_rule_applies, match_kind_matches, non_empty, normalize_kind, relation_rule_applies,
    mode_matches, tab_matches,
};
use super::compat_rules::{compat_rule_source_path, load_rules, CompatRule};

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
    let tp2_paths = build_tp2_path_lookup(mods);
    let active_items = collect_step3_active_items(items, &tp2_paths);
    let rules = load_rules();
    let mut out = HashMap::<String, Step3CompatMarker>::new();
    let mut order = 1usize;
    let mut requirement_cache = ComponentRequirementCache::new();
    let mut conflict_cache = ComponentConflictCache::new();
    let mut path_guard_cache = ComponentPathGuardCache::new();
    let path_context = PathRequirementContext::for_tab(step1, tab);

    for item in items.iter().filter(|item| !item.is_parent) {
        let key = marker_key(item);
        let mut marker = scan_path_marker_for_item(item, &tp2_paths, &path_context, &mut path_guard_cache)
            .or_else(|| {
                scan_conflict_marker_for_item(
                    item,
                    order,
                    &tp2_paths,
                    &active_items,
                    &mut conflict_cache,
                )
            })
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
            if !direct_rule_applies(rule, step1) {
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
    *marker = Some(Step3CompatMarker {
        kind: kind.to_string(),
        message: non_empty(Some(rule.message.as_str())),
        related_mod: non_empty(rule.related_mod.as_deref()),
        related_component: non_empty(rule.related_component.as_deref()),
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

fn scan_conflict_marker_for_item(
    item: &Step3ItemState,
    component_order: usize,
    tp2_paths: &HashMap<String, String>,
    active_items: &[super::compat_rule_runtime::CompatActiveItem],
    conflict_cache: &mut ComponentConflictCache,
) -> Option<Step3CompatMarker> {
    let tp2_path = tp2_paths
        .get(&tp2_lookup_key(&item.tp_file, &item.mod_name))
        .filter(|value| !value.trim().is_empty())?;
    scan_conflict_hit(
        tp2_path,
        &item.tp_file,
        &item.component_id,
        Some(component_order),
        active_items,
        conflict_cache,
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
