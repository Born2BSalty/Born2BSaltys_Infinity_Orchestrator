// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashMap;

use super::compat_dependency_parse::{ComponentRequirement, load_component_requirements};
use super::compat_rule_runtime::{CompatActiveItem, normalize_mod_key};

pub(crate) type ComponentRequirementCache = HashMap<String, HashMap<String, Vec<ComponentRequirement>>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DependencyEvalMode {
    MissingOnly,
    MissingThenOrder,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DependencyCompatHit {
    pub(crate) kind: &'static str,
    pub(crate) target_mod: String,
    pub(crate) target_component_id: String,
    pub(crate) message: String,
    pub(crate) source: String,
    pub(crate) raw_evidence: String,
}

pub(crate) fn scan_dependency_hit(
    tp2_path: &str,
    component_id: &str,
    component_order: Option<usize>,
    active_items: &[CompatActiveItem],
    requirement_cache: &mut ComponentRequirementCache,
    mode: DependencyEvalMode,
) -> Option<DependencyCompatHit> {
    if tp2_path.trim().is_empty() {
        return None;
    }

    let requirements_by_component = requirement_cache
        .entry(tp2_path.to_string())
        .or_insert_with(|| load_component_requirements(tp2_path));
    let requirements = requirements_by_component.get(component_id.trim())?;

    if let Some(requirement) = requirements
        .iter()
        .find(|requirement| target_order(active_items, requirement).is_none())
    {
        let missing_message = match mode {
            DependencyEvalMode::MissingOnly => {
                "Required component is not selected in the current plan."
            }
            DependencyEvalMode::MissingThenOrder => {
                "Required component is not selected in the current order."
            }
        };
        return Some(compat_hit(
            "missing_dep",
            tp2_path,
            requirement,
            requirement
                .message
                .clone()
                .unwrap_or_else(|| missing_message.to_string()),
        ));
    }

    if mode == DependencyEvalMode::MissingThenOrder {
        let component_order = component_order?;
        if let Some(requirement) = requirements.iter().find(|requirement| {
            target_order(active_items, requirement).is_some_and(|target_order| component_order < target_order)
        }) {
            return Some(compat_hit(
                "order_block",
                tp2_path,
                requirement,
                requirement.message.clone().unwrap_or_else(|| {
                    "Required component must be installed earlier in the current order.".to_string()
                }),
            ));
        }
    }

    None
}

fn compat_hit(
    kind: &'static str,
    tp2_path: &str,
    requirement: &ComponentRequirement,
    message: String,
) -> DependencyCompatHit {
    DependencyCompatHit {
        kind,
        target_mod: requirement.target_mod.clone(),
        target_component_id: requirement.target_component_id.clone(),
        message,
        source: tp2_path.to_string(),
        raw_evidence: requirement.raw_line.clone(),
    }
}

fn target_order(
    active_items: &[CompatActiveItem],
    requirement: &ComponentRequirement,
) -> Option<usize> {
    active_items.iter().find_map(|item| {
        (item.component_id.trim() == requirement.target_component_id
            && normalize_mod_key(&item.tp_file) == requirement.target_mod)
            .then_some(item.order?)
    })
}
