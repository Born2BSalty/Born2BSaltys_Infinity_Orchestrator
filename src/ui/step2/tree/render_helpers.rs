// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::state::{Step2ComponentState, Step2ModState};

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
