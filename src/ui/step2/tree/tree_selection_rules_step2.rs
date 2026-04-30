// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::state::{Step2ComponentState, Step2ModState};

pub(crate) fn enforce_subcomponent_single_select(
    mod_state: &mut Step2ModState,
    changed_idx: usize,
) {
    let Some(base_key) = subcomponent_base_key(&mod_state.components[changed_idx].label) else {
        return;
    };
    for (idx, comp) in mod_state.components.iter_mut().enumerate() {
        if idx == changed_idx {
            continue;
        }
        let Some(other_key) = subcomponent_base_key(&comp.label) else {
            continue;
        };
        if other_key.eq_ignore_ascii_case(&base_key) {
            comp.checked = false;
            comp.selected_order = None;
        }
    }
}

pub(crate) fn enforce_subcomponent_single_select_keep_first(mod_state: &mut Step2ModState) {
    let mut seen = std::collections::HashSet::<String>::new();
    for comp in &mut mod_state.components {
        if !comp.checked {
            continue;
        }
        let Some(base_key) = subcomponent_base_key(&comp.label) else {
            continue;
        };
        if !seen.insert(base_key) {
            comp.checked = false;
            comp.selected_order = None;
        }
    }
}

pub(crate) fn enforce_collapsible_group_umbrella_rules(
    mod_state: &mut Step2ModState,
    changed_idx: usize,
) {
    let Some(group) = mod_state.components[changed_idx].collapsible_group.clone() else {
        return;
    };

    let changed_is_umbrella = mod_state.components[changed_idx].collapsible_group_is_umbrella;
    for (idx, comp) in mod_state.components.iter_mut().enumerate() {
        if idx == changed_idx {
            continue;
        }
        if !comp
            .collapsible_group
            .as_deref()
            .is_some_and(|other| other.eq_ignore_ascii_case(&group))
        {
            continue;
        }
        if changed_is_umbrella || comp.collapsible_group_is_umbrella {
            comp.checked = false;
            comp.selected_order = None;
        }
    }
}

pub(crate) fn enforce_collapsible_group_umbrella_after_bulk(mod_state: &mut Step2ModState) {
    let mut groups_with_specific_choices = std::collections::HashSet::<String>::new();
    for comp in &mod_state.components {
        if comp.checked
            && !comp.collapsible_group_is_umbrella
            && let Some(group) = comp.collapsible_group.as_deref()
        {
            groups_with_specific_choices.insert(group.to_ascii_lowercase());
        }
    }

    if groups_with_specific_choices.is_empty() {
        return;
    }

    for comp in &mut mod_state.components {
        if !comp.checked || !comp.collapsible_group_is_umbrella {
            continue;
        }
        let Some(group) = comp.collapsible_group.as_deref() else {
            continue;
        };
        if groups_with_specific_choices.contains(&group.to_ascii_lowercase()) {
            comp.checked = false;
            comp.selected_order = None;
        }
    }
}

pub(crate) fn enforce_meta_mode_exclusive(mod_state: &mut Step2ModState, changed_idx: usize) {
    let changed_is_meta = mod_state
        .components
        .get(changed_idx)
        .map(|c| c.is_meta_mode_component)
        .unwrap_or(false);
    if changed_is_meta {
        for (idx, comp) in mod_state.components.iter_mut().enumerate() {
            if idx != changed_idx && !comp.disabled {
                comp.checked = false;
                comp.selected_order = None;
            }
        }
    } else {
        for comp in &mut mod_state.components {
            if comp.is_meta_mode_component {
                comp.checked = false;
                comp.selected_order = None;
            }
        }
    }
}

pub(crate) fn enforce_meta_mode_after_bulk(mod_state: &mut Step2ModState) {
    let any_normal_checked = mod_state
        .components
        .iter()
        .any(|c| c.checked && !c.disabled && !c.is_meta_mode_component);
    if any_normal_checked {
        for comp in &mut mod_state.components {
            if comp.is_meta_mode_component {
                comp.checked = false;
                comp.selected_order = None;
            }
        }
        return;
    }

    let mut first_meta_seen = false;
    for comp in &mut mod_state.components {
        if !comp.checked || comp.disabled || !comp.is_meta_mode_component {
            continue;
        }
        if !first_meta_seen {
            first_meta_seen = true;
        } else {
            comp.checked = false;
            comp.selected_order = None;
        }
    }
}

pub(crate) fn enforce_tp2_same_mod_exclusive_on_select(
    mod_state: &mut Step2ModState,
    changed_idx: usize,
) {
    if !mod_state
        .components
        .get(changed_idx)
        .is_some_and(|component| component.checked)
    {
        return;
    }

    let target_ids = same_mod_exclusive_targets(mod_state, changed_idx);
    if target_ids.is_empty() {
        return;
    }

    for (idx, comp) in mod_state.components.iter_mut().enumerate() {
        if idx == changed_idx {
            continue;
        }
        if target_ids.iter().any(|id| id == comp.component_id.trim()) {
            comp.checked = false;
            comp.selected_order = None;
        }
    }
}

pub(crate) fn enforce_tp2_same_mod_exclusive_after_bulk(mod_state: &mut Step2ModState) {
    let mut checked_indices: Vec<usize> = mod_state
        .components
        .iter()
        .enumerate()
        .filter_map(|(idx, comp)| comp.checked.then_some(idx))
        .collect();
    checked_indices.sort_by_key(|idx| {
        mod_state.components[*idx]
            .selected_order
            .unwrap_or(usize::MAX)
    });

    for idx in checked_indices {
        if !mod_state
            .components
            .get(idx)
            .is_some_and(|component| component.checked)
        {
            continue;
        }
        enforce_tp2_same_mod_exclusive_on_select(mod_state, idx);
    }
}

pub(crate) fn set_component_checked_state(
    component: &mut Step2ComponentState,
    next_selection_order: &mut usize,
) {
    if component.checked {
        if component.selected_order.is_none() {
            component.selected_order = Some(*next_selection_order);
            *next_selection_order += 1;
        }
    } else {
        component.selected_order = None;
    }
}

pub(crate) fn split_subcomponent_label(label: &str) -> Option<(String, String)> {
    let (base, choice) = label.split_once("->")?;
    let base = base.trim();
    let choice = choice.trim();
    if base.is_empty() || choice.is_empty() {
        None
    } else {
        Some((base.to_string(), choice.to_string()))
    }
}

fn subcomponent_base_key(label: &str) -> Option<String> {
    let (base, _choice) = split_subcomponent_label(label)?;
    Some(base.to_ascii_lowercase())
}

fn same_mod_exclusive_targets(mod_state: &Step2ModState, source_idx: usize) -> Vec<String> {
    let _ = (mod_state, source_idx);
    Vec::new()
}
