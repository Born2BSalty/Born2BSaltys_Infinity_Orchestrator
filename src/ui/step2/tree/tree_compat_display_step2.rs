// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::{Step2ComponentState, Step2ModState};
use crate::ui::shared::redesign_tokens::{
    ThemePalette, redesign_conditional, redesign_conditional_fill, redesign_conflict,
    redesign_conflict_fill, redesign_conflict_parent, redesign_game_mismatch,
    redesign_game_mismatch_fill, redesign_included, redesign_included_fill, redesign_info,
    redesign_info_fill, redesign_warning_emphasis, redesign_warning_fill, redesign_warning_parent,
};

pub(crate) fn compat_colors(
    kind: Option<&str>,
) -> Option<(egui::Color32, egui::Color32, &'static str)> {
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

pub(crate) fn compat_colors_redesign(
    kind: Option<&str>,
    palette: ThemePalette,
) -> Option<(egui::Color32, egui::Color32, &'static str)> {
    match kind.unwrap_or_default() {
        "included" | "not_needed" => Some((
            redesign_included(palette),
            redesign_included_fill(palette),
            "Included",
        )),
        "not_compatible" | "conflict" => Some((
            redesign_conflict(palette),
            redesign_conflict_fill(palette),
            "Conflict",
        )),
        "order_block" => Some((
            redesign_warning_emphasis(palette),
            redesign_warning_fill(palette),
            "Install Order",
        )),
        "missing_dep" => Some((
            redesign_info(palette),
            redesign_info_fill(palette),
            "Missing Dep",
        )),
        "mismatch" => Some((
            redesign_game_mismatch(palette),
            redesign_game_mismatch_fill(palette),
            "Mismatch",
        )),
        "path_requirement" => Some((
            redesign_info(palette),
            redesign_info_fill(palette),
            "Path Requirement",
        )),
        "conditional" => Some((
            redesign_conditional(palette),
            redesign_conditional_fill(palette),
            "Conditional",
        )),
        "warning" => Some((
            redesign_warning_emphasis(palette),
            redesign_warning_fill(palette),
            "Warning",
        )),
        "deprecated" => Some((
            redesign_warning_emphasis(palette),
            redesign_warning_fill(palette),
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
            redesign_conflict_parent(palette),
            redesign_conflict_fill(palette),
            format!(
                "{conflicts} conflict{}",
                if conflicts == 1 { "" } else { "s" }
            ),
        ));
    }
    if order_blocks > 0 {
        return Some((
            redesign_warning_parent(palette),
            redesign_warning_fill(palette),
            format!(
                "{order_blocks} order issue{}",
                if order_blocks == 1 { "" } else { "s" }
            ),
        ));
    }
    if warnings > 0 {
        return Some((
            redesign_warning_parent(palette),
            redesign_warning_fill(palette),
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
