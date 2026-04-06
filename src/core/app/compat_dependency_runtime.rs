// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashMap;

use super::compat_dependency_parse::{
    ComponentRequirement, ComponentRequirementTarget, load_component_requirements,
};
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
        .find(|requirement| selected_targets(active_items, requirement).is_empty())
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
            requirement.targets.first()?,
            missing_dependency_message(requirement, missing_message),
        ));
    }

    if mode == DependencyEvalMode::MissingThenOrder {
        let component_order = component_order?;
        if let Some((requirement, target)) = requirements.iter().find_map(|requirement| {
            let selected = selected_targets(active_items, requirement);
            if selected.is_empty() || selected.iter().any(|(_, target_order)| component_order >= *target_order)
            {
                return None;
            }
            selected
                .into_iter()
                .min_by_key(|(_, target_order)| *target_order)
                .map(|(target, _)| (requirement, target))
        }) {
            return Some(compat_hit(
                "order_block",
                tp2_path,
                requirement,
                target,
                order_block_message(requirement),
            ));
        }
    }

    None
}

fn compat_hit(
    kind: &'static str,
    tp2_path: &str,
    requirement: &ComponentRequirement,
    target: &ComponentRequirementTarget,
    message: String,
) -> DependencyCompatHit {
    DependencyCompatHit {
        kind,
        target_mod: target.target_mod.clone(),
        target_component_id: target.target_component_id.clone(),
        message,
        source: tp2_path.to_string(),
        raw_evidence: requirement.raw_line.clone(),
    }
}

fn selected_targets<'a>(
    active_items: &[CompatActiveItem],
    requirement: &'a ComponentRequirement,
) -> Vec<(&'a ComponentRequirementTarget, usize)> {
    requirement
        .targets
        .iter()
        .filter_map(|target| {
            active_items.iter().find_map(|item| {
                if item.component_id.trim() == target.target_component_id
                    && normalize_mod_key(&item.tp_file) == target.target_mod
                {
                    item.order.map(|order| (target, order))
                } else {
                    None
                }
            })
        })
        .collect()
}

fn missing_dependency_message(requirement: &ComponentRequirement, fallback: &str) -> String {
    if requirement.targets.len() > 1 {
        return format!("Requires one of: {}", joined_targets(requirement));
    }
    requirement
        .message
        .clone()
        .unwrap_or_else(|| fallback.to_string())
}

fn order_block_message(requirement: &ComponentRequirement) -> String {
    if requirement.targets.len() > 1 {
        return "Selected prerequisite must be installed earlier in the current order.".to_string();
    }
    requirement.message.clone().unwrap_or_else(|| {
        "Required component must be installed earlier in the current order.".to_string()
    })
}

fn joined_targets(requirement: &ComponentRequirement) -> String {
    requirement
        .targets
        .iter()
        .map(target_label)
        .collect::<Vec<_>>()
        .join(" OR ")
}

fn target_label(target: &ComponentRequirementTarget) -> String {
    format!("{} #{}", target.target_mod, target.target_component_id)
}

#[cfg(test)]
#[path = "compat_dependency_runtime_tests.rs"]
mod tests;
