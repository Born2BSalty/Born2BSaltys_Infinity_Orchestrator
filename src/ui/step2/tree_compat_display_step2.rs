// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::state::{Step2ComponentState, Step2ModState};

pub(crate) fn compat_colors(kind: Option<&str>) -> Option<(egui::Color32, egui::Color32, &'static str)> {
    match kind.unwrap_or_default() {
        "included" | "not_needed" => Some((
            crate::ui::shared::theme_global::included(),
            crate::ui::shared::theme_global::included_fill(),
            "Included",
        )),
        "not_compatible" | "conflict" => Some((
            crate::ui::shared::theme_global::conflict(),
            crate::ui::shared::theme_global::conflict_fill(),
            "Conflict",
        )),
        "order_block" => Some((
            crate::ui::shared::theme_global::warning_emphasis(),
            crate::ui::shared::theme_global::warning_fill(),
            "Install Order",
        )),
        "missing_dep" => Some((
            crate::ui::shared::theme_global::info(),
            crate::ui::shared::theme_global::info_fill(),
            "Missing Dep",
        )),
        "mismatch" => Some((
            crate::ui::shared::theme_global::game_mismatch(),
            crate::ui::shared::theme_global::game_mismatch_fill(),
            "Mismatch",
        )),
        "path_requirement" => Some((
            crate::ui::shared::theme_global::info(),
            crate::ui::shared::theme_global::info_fill(),
            "Path Requirement",
        )),
        "conditional" => Some((
            crate::ui::shared::theme_global::conditional(),
            crate::ui::shared::theme_global::conditional_fill(),
            "Conditional",
        )),
        "warning" => Some((
            crate::ui::shared::theme_global::warning_emphasis(),
            crate::ui::shared::theme_global::warning_fill(),
            "Warning",
        )),
        "deprecated" => Some((
            crate::ui::shared::theme_global::warning_emphasis(),
            crate::ui::shared::theme_global::warning_fill(),
            "Deprecated",
        )),
        _ => None,
    }
}

pub(crate) fn parent_compat_summary(
    mod_state: &Step2ModState,
) -> Option<(egui::Color32, egui::Color32, String)> {
    let mut conflicts = 0usize;
    let mut order_blocks = 0usize;
    let mut warnings = 0usize;
    let mut conditional = 0usize;
    for component in &mod_state.components {
        match component.compat_kind.as_deref().unwrap_or_default() {
            "not_compatible" | "conflict" => conflicts = conflicts.saturating_add(1),
            "order_block" => order_blocks = order_blocks.saturating_add(1),
            "warning" => warnings = warnings.saturating_add(1),
            "conditional" => conditional = conditional.saturating_add(1),
            _ => {}
        }
    }
    if conflicts > 0 {
        return Some((
            crate::ui::shared::theme_global::conflict_parent(),
            crate::ui::shared::theme_global::conflict_fill(),
            format!("{conflicts} conflict{}", if conflicts == 1 { "" } else { "s" }),
        ));
    }
    if order_blocks > 0 {
        return Some((
            crate::ui::shared::theme_global::warning_parent(),
            crate::ui::shared::theme_global::warning_fill(),
            format!("{order_blocks} order issue{}", if order_blocks == 1 { "" } else { "s" }),
        ));
    }
    if conditional > 0 {
        return Some((
            crate::ui::shared::theme_global::conditional_parent(),
            crate::ui::shared::theme_global::conditional_fill(),
            format!(
                "{conditional} conditional{}",
                if conditional == 1 { "" } else { "s" }
            ),
        ));
    }
    if warnings > 0 {
        return Some((
            crate::ui::shared::theme_global::warning_parent(),
            crate::ui::shared::theme_global::warning_fill(),
            format!("{warnings} warning{}", if warnings == 1 { "" } else { "s" }),
        ));
    }
    None
}

pub(crate) fn parent_compat_target(mod_state: &Step2ModState) -> Option<&Step2ComponentState> {
    let priority = [
        "conflict",
        "order_block",
        "not_compatible",
        "conditional",
        "warning",
    ];
    for kind in priority {
        let prefer_checked = kind == "conflict"
            || kind == "order_block"
            || kind == "not_compatible";
        if let Some(component) = mod_state.components.iter().find(|component| {
            component.compat_kind.as_deref() == Some(kind)
                && (!prefer_checked || component.checked)
        }) {
            return Some(component);
        }
        if let Some(component) = mod_state
            .components
            .iter()
            .find(|component| component.compat_kind.as_deref() == Some(kind))
        {
            return Some(component);
        }
    }
    None
}
