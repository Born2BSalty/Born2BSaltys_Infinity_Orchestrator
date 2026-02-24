// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::state::{Step2ComponentState, Step2ModState};

pub(super) fn enforce_subcomponent_single_select(mod_state: &mut Step2ModState, changed_idx: usize) {
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

pub(super) fn enforce_subcomponent_single_select_keep_first(mod_state: &mut Step2ModState) {
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

pub(super) fn enforce_meta_mode_exclusive(mod_state: &mut Step2ModState, changed_idx: usize) {
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

pub(super) fn enforce_meta_mode_after_bulk(mod_state: &mut Step2ModState) {
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

pub(super) fn set_component_checked_state(
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

fn subcomponent_base_key(label: &str) -> Option<String> {
    let (base, _choice) = label.split_once("->")?;
    let base = base.trim();
    if base.is_empty() {
        None
    } else {
        Some(base.to_ascii_lowercase())
    }
}

pub(super) fn compat_colors(kind: Option<&str>) -> Option<(egui::Color32, egui::Color32, &'static str)> {
    match kind.unwrap_or_default() {
        "included" | "not_needed" => Some((
            egui::Color32::from_gray(150),
            egui::Color32::from_rgb(56, 72, 56),
            "Included",
        )),
        "not_compatible" | "conflict" => Some((
            egui::Color32::from_rgb(208, 96, 96),
            egui::Color32::from_rgb(88, 44, 44),
            "Conflict",
        )),
        "missing_dep" => Some((
            egui::Color32::from_rgb(120, 186, 230),
            egui::Color32::from_rgb(42, 66, 86),
            "Missing Dep",
        )),
        "game_mismatch" => Some((
            egui::Color32::from_rgb(203, 110, 188),
            egui::Color32::from_rgb(78, 44, 84),
            "Game Mismatch",
        )),
        "conditional" => Some((
            egui::Color32::from_rgb(164, 190, 208),
            egui::Color32::from_rgb(52, 66, 78),
            "Conditional",
        )),
        "warning" => Some((
            egui::Color32::from_rgb(214, 174, 84),
            egui::Color32::from_rgb(78, 62, 34),
            "Warning",
        )),
        _ => None,
    }
}

pub(super) fn parent_compat_summary(
    mod_state: &Step2ModState,
) -> Option<(egui::Color32, egui::Color32, String)> {
    let mut conflicts = 0usize;
    let mut missing_deps = 0usize;
    let mut warnings = 0usize;
    let mut game_mismatch = 0usize;
    let mut conditional = 0usize;
    let mut included = 0usize;
    for component in &mod_state.components {
        match component.compat_kind.as_deref().unwrap_or_default() {
            "not_compatible" | "conflict" => conflicts = conflicts.saturating_add(1),
            "missing_dep" => missing_deps = missing_deps.saturating_add(1),
            "warning" => warnings = warnings.saturating_add(1),
            "game_mismatch" => game_mismatch = game_mismatch.saturating_add(1),
            "conditional" => conditional = conditional.saturating_add(1),
            "included" | "not_needed" => included = included.saturating_add(1),
            _ => {}
        }
    }
    if conflicts > 0 {
        return Some((
            egui::Color32::from_rgb(220, 122, 122),
            egui::Color32::from_rgb(88, 44, 44),
            format!("{conflicts} conflict{}", if conflicts == 1 { "" } else { "s" }),
        ));
    }
    if missing_deps > 0 {
        return Some((
            egui::Color32::from_rgb(130, 196, 236),
            egui::Color32::from_rgb(42, 66, 86),
            format!("{missing_deps} missing dep{}", if missing_deps == 1 { "" } else { "s" }),
        ));
    }
    if game_mismatch > 0 {
        return Some((
            egui::Color32::from_rgb(203, 110, 188),
            egui::Color32::from_rgb(78, 44, 84),
            format!(
                "{game_mismatch} game mismatch{}",
                if game_mismatch == 1 { "" } else { "es" }
            ),
        ));
    }
    if conditional > 0 {
        return Some((
            egui::Color32::from_rgb(170, 198, 218),
            egui::Color32::from_rgb(52, 66, 78),
            format!(
                "{conditional} conditional{}",
                if conditional == 1 { "" } else { "s" }
            ),
        ));
    }
    if warnings > 0 {
        return Some((
            egui::Color32::from_rgb(222, 182, 92),
            egui::Color32::from_rgb(78, 62, 34),
            format!("{warnings} warning{}", if warnings == 1 { "" } else { "s" }),
        ));
    }
    if included > 0 {
        return Some((
            egui::Color32::from_gray(170),
            egui::Color32::from_rgb(56, 72, 56),
            format!("{included} included"),
        ));
    }
    None
}
