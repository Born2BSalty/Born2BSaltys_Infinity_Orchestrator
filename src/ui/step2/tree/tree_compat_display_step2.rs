// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::{Step2ComponentState, Step2ModState};
use crate::ui::shared::redesign_tokens::{
    ThemePalette, redesign_compat_conditional, redesign_compat_conditional_fill,
    redesign_compat_conflict, redesign_compat_conflict_fill, redesign_compat_included,
    redesign_compat_included_fill, redesign_compat_info, redesign_compat_info_fill,
    redesign_compat_mismatch, redesign_compat_mismatch_fill, redesign_compat_warning,
    redesign_compat_warning_fill, redesign_warning_soft,
};

pub(crate) fn compat_colors(
    kind: Option<&str>,
    palette: ThemePalette,
) -> Option<(egui::Color32, egui::Color32, &'static str)> {
    match kind.unwrap_or_default() {
        "included" | "not_needed" => Some((
            redesign_compat_included(palette),
            redesign_compat_included_fill(palette),
            "Included",
        )),
        "not_compatible" | "conflict" => Some((
            redesign_compat_conflict(palette),
            redesign_compat_conflict_fill(palette),
            "Conflict",
        )),
        "order_block" => Some((
            redesign_compat_warning(palette),
            redesign_compat_warning_fill(palette),
            "Install Order",
        )),
        "missing_dep" => Some((
            redesign_compat_info(palette),
            redesign_compat_info_fill(palette),
            "Missing Dep",
        )),
        "mismatch" => Some((
            redesign_compat_mismatch(palette),
            redesign_compat_mismatch_fill(palette),
            "Mismatch",
        )),
        "path_requirement" => Some((
            redesign_compat_info(palette),
            redesign_compat_info_fill(palette),
            "Path Requirement",
        )),
        "conditional" => Some((
            redesign_compat_conditional(palette),
            redesign_compat_conditional_fill(palette),
            "Conditional",
        )),
        "warning" => Some((
            redesign_compat_warning(palette),
            redesign_compat_warning_fill(palette),
            "Warning",
        )),
        "deprecated" => Some((
            redesign_compat_warning(palette),
            redesign_compat_warning_fill(palette),
            "Deprecated",
        )),
        _ => None,
    }
}

pub(crate) fn parent_compat_summary(
    mod_state: &Step2ModState,
    palette: ThemePalette,
) -> Option<(egui::Color32, egui::Color32, String)> {
    let mut conflicts = 0usize;
    let mut order_blocks = 0usize;
    let mut warnings = 0usize;
    for component in &mod_state.components {
        match component.compat_kind.as_deref().unwrap_or_default() {
            "not_compatible" | "conflict" => conflicts = conflicts.saturating_add(1),
            "order_block" => order_blocks = order_blocks.saturating_add(1),
            "warning" => warnings = warnings.saturating_add(1),
            _ => {}
        }
    }
    if conflicts > 0 {
        return Some((
            redesign_compat_conflict(palette),
            redesign_compat_conflict_fill(palette),
            format!(
                "{conflicts} conflict{}",
                if conflicts == 1 { "" } else { "s" }
            ),
        ));
    }
    if order_blocks > 0 {
        return Some((
            redesign_warning_soft(palette),
            redesign_compat_warning_fill(palette),
            format!(
                "{order_blocks} order issue{}",
                if order_blocks == 1 { "" } else { "s" }
            ),
        ));
    }
    if warnings > 0 {
        return Some((
            redesign_warning_soft(palette),
            redesign_compat_warning_fill(palette),
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
        let prefer_checked =
            kind == "conflict" || kind == "order_block" || kind == "not_compatible";
        if let Some(component) = mod_state.components.iter().find(|component| {
            component.compat_kind.as_deref() == Some(kind) && (!prefer_checked || component.checked)
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
