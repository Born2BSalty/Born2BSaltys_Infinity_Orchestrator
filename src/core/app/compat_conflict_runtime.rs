// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashMap;
use std::collections::HashSet;

use super::compat_conflict_parse::{ComponentConflict, load_component_conflicts};
use super::compat_rule_runtime::{CompatActiveItem, normalize_mod_key};

pub(crate) type ComponentConflictCache = HashMap<String, HashMap<String, Vec<ComponentConflict>>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConflictCompatHit {
    pub(crate) kind: &'static str,
    pub(crate) target_mod: String,
    pub(crate) target_component_id: String,
    pub(crate) message: String,
    pub(crate) source: String,
    pub(crate) raw_evidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ConflictEdge {
    target_key: String,
    target_mod: String,
    target_component_id: String,
    message: Option<String>,
    source: String,
    raw_evidence: String,
}

pub(crate) fn scan_conflict_hit(
    tp2_path: &str,
    current_tp_file: &str,
    current_component_id: &str,
    current_component_order: Option<usize>,
    active_items: &[CompatActiveItem],
    conflict_cache: &mut ComponentConflictCache,
) -> Option<ConflictCompatHit> {
    if tp2_path.trim().is_empty() {
        return None;
    }

    let conflicts_by_component = conflict_cache
        .entry(tp2_path.to_string())
        .or_insert_with(|| load_component_conflicts(tp2_path));
    if conflicts_by_component.is_empty() {
        return None;
    }

    let current_key = active_item_key(current_tp_file, current_component_id);
    let active_orders = active_order_map(active_items);
    let adjacency = build_selected_conflict_graph(active_items, conflict_cache);
    let edges = adjacency.get(&current_key)?;

    detect_cycle_hit(edges, &current_key, &adjacency)
        .or_else(|| detect_order_hit(edges, &active_orders, current_component_order))
}

fn detect_order_hit(
    edges: &[ConflictEdge],
    active_orders: &HashMap<String, usize>,
    current_component_order: Option<usize>,
) -> Option<ConflictCompatHit> {
    let current_component_order = current_component_order?;
    for edge in edges {
        let Some(target_component_order) = active_orders.get(&edge.target_key) else {
            continue;
        };
        if current_component_order > *target_component_order {
            return Some(ConflictCompatHit {
                kind: "order_block",
                target_mod: edge.target_mod.clone(),
                target_component_id: edge.target_component_id.clone(),
                message: edge.message.clone().unwrap_or_else(|| {
                    "This component must be installed before the related component.".to_string()
                }),
                source: edge.source.clone(),
                raw_evidence: edge.raw_evidence.clone(),
            });
        }
    }
    None
}

fn detect_cycle_hit(
    edges: &[ConflictEdge],
    current_key: &str,
    adjacency: &HashMap<String, Vec<ConflictEdge>>,
) -> Option<ConflictCompatHit> {
    for edge in edges {
        if is_direct_reciprocal(edge, current_key, adjacency) {
            return Some(ConflictCompatHit {
                kind: "conflict",
                target_mod: edge.target_mod.clone(),
                target_component_id: edge.target_component_id.clone(),
                message: format!(
                    "Mutually exclusive with {} #{}; these components forbid each other and cannot be selected together.",
                    edge.target_mod, edge.target_component_id
                ),
                source: edge.source.clone(),
                raw_evidence: edge.raw_evidence.clone(),
            });
        }
        let mut visited = HashSet::<String>::new();
        if path_reaches(&edge.target_key, current_key, adjacency, &mut visited) {
            return Some(ConflictCompatHit {
                kind: "conflict",
                target_mod: edge.target_mod.clone(),
                target_component_id: edge.target_component_id.clone(),
                message:
                    "Part of an impossible FORBID_COMPONENT cycle; remove one of the linked components."
                        .to_string(),
                source: edge.source.clone(),
                raw_evidence: edge.raw_evidence.clone(),
            });
        }
    }
    None
}

fn is_direct_reciprocal(
    edge: &ConflictEdge,
    current_key: &str,
    adjacency: &HashMap<String, Vec<ConflictEdge>>,
) -> bool {
    adjacency.get(&edge.target_key).is_some_and(|reverse_edges| {
        reverse_edges
            .iter()
            .any(|reverse| reverse.target_key == current_key)
    })
}

fn path_reaches(
    start: &str,
    target: &str,
    adjacency: &HashMap<String, Vec<ConflictEdge>>,
    visited: &mut HashSet<String>,
) -> bool {
    if start == target {
        return true;
    }
    if !visited.insert(start.to_string()) {
        return false;
    }
    adjacency.get(start).is_some_and(|edges| {
        edges.iter().any(|edge| path_reaches(&edge.target_key, target, adjacency, visited))
    })
}

fn build_selected_conflict_graph(
    active_items: &[CompatActiveItem],
    conflict_cache: &mut ComponentConflictCache,
) -> HashMap<String, Vec<ConflictEdge>> {
    let active_keys = active_key_set(active_items);
    let mut adjacency = HashMap::<String, Vec<ConflictEdge>>::new();

    for item in active_items {
        if item.tp2_path.trim().is_empty() {
            continue;
        }
        let conflicts_by_component = conflict_cache
            .entry(item.tp2_path.clone())
            .or_insert_with(|| load_component_conflicts(&item.tp2_path));
        let Some(conflicts) = conflicts_by_component.get(item.component_id.trim()) else {
            continue;
        };
        let from_key = active_item_key(&item.tp_file, &item.component_id);
        for conflict in conflicts {
            let target_key = conflict_target_key(conflict);
            if !active_keys.contains(&target_key) || target_key == from_key {
                continue;
            }
            adjacency
                .entry(from_key.clone())
                .or_default()
                .push(ConflictEdge {
                    target_key,
                    target_mod: conflict.target_mod.clone(),
                    target_component_id: conflict.target_component_id.clone(),
                    message: conflict.message.clone(),
                    source: item.tp2_path.clone(),
                    raw_evidence: conflict.raw_line.clone(),
                });
        }
    }

    adjacency
}

fn active_order_map(active_items: &[CompatActiveItem]) -> HashMap<String, usize> {
    let mut out = HashMap::<String, usize>::new();
    for item in active_items {
        if let Some(order) = item.order {
            out.insert(active_item_key(&item.tp_file, &item.component_id), order);
        }
    }
    out
}

fn active_key_set(active_items: &[CompatActiveItem]) -> HashSet<String> {
    active_items
        .iter()
        .map(|item| active_item_key(&item.tp_file, &item.component_id))
        .collect()
}

fn active_item_key(tp_file: &str, component_id: &str) -> String {
    format!("{}|{}", normalize_mod_key(tp_file), component_id.trim())
}

fn conflict_target_key(conflict: &ComponentConflict) -> String {
    format!(
        "{}|{}",
        conflict.target_mod,
        conflict.target_component_id.trim()
    )
}
