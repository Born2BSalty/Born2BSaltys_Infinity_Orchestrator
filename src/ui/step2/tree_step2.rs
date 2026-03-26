// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

pub(crate) mod step2_tree {
pub(crate) mod render_helpers {
    use eframe::egui;
    use std::path::Path;

    use crate::compat::model::Tp2Rule;
    use crate::compat::tp2_parse::parse_tp2_rules;
    use crate::ui::state::{Step2ComponentState, Step2ModState};

pub(crate) fn enforce_subcomponent_single_select(mod_state: &mut Step2ModState, changed_idx: usize) {
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
    checked_indices.sort_by_key(|idx| mod_state.components[*idx].selected_order.unwrap_or(usize::MAX));

    for idx in checked_indices {
        if !mod_state.components.get(idx).is_some_and(|component| component.checked) {
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
    let Some(source_component_id) = mod_state.components.get(source_idx).and_then(|component| {
        component.component_id.trim().parse::<u32>().ok()
    }) else {
        return Vec::new();
    };
    let tp2_path = mod_state.tp2_path.trim();
    if tp2_path.is_empty() {
        return Vec::new();
    }

    let metadata = parse_tp2_rules(Path::new(tp2_path));
    let own_mod_key =
        crate::ui::step2::service_selection_step2::selection_normalize_mod_key(&mod_state.tp_file);
    let mut out = Vec::<String>::new();

    for (component_id, rule) in &metadata.rules {
        if *component_id != source_component_id {
            continue;
        }
        let Tp2Rule::ForbidInstalledMod {
            target_mod,
            target_component: Some(target_component),
            ..
        } = rule
        else {
            continue;
        };

        let target_mod_key =
            crate::ui::step2::service_selection_step2::selection_normalize_mod_key(target_mod);
        if target_mod_key == own_mod_key {
            out.push((*target_component).to_string());
        }
    }

    out.sort();
    out.dedup();
    out
}

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
        "game_mismatch" => Some((
            crate::ui::shared::theme_global::game_mismatch(),
            crate::ui::shared::theme_global::game_mismatch_fill(),
            "Game Mismatch",
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
    let mut missing_deps = 0usize;
    let mut order_blocks = 0usize;
    let mut warnings = 0usize;
    let mut game_mismatch = 0usize;
    let mut path_requirements = 0usize;
    let mut deprecated = 0usize;
    let mut conditional = 0usize;
    let mut included = 0usize;
    for component in &mod_state.components {
        match component.compat_kind.as_deref().unwrap_or_default() {
            "not_compatible" | "conflict" => conflicts = conflicts.saturating_add(1),
            "order_block" => order_blocks = order_blocks.saturating_add(1),
            "missing_dep" => missing_deps = missing_deps.saturating_add(1),
            "warning" => warnings = warnings.saturating_add(1),
            "game_mismatch" => game_mismatch = game_mismatch.saturating_add(1),
            "path_requirement" => path_requirements = path_requirements.saturating_add(1),
            "deprecated" => deprecated = deprecated.saturating_add(1),
            "conditional" => conditional = conditional.saturating_add(1),
            "included" | "not_needed" => included = included.saturating_add(1),
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
    if missing_deps > 0 {
        return Some((
            crate::ui::shared::theme_global::missing_dep_parent(),
            crate::ui::shared::theme_global::info_fill(),
            format!("{missing_deps} missing dep{}", if missing_deps == 1 { "" } else { "s" }),
        ));
    }
    if game_mismatch > 0 {
        return Some((
            crate::ui::shared::theme_global::game_mismatch(),
            crate::ui::shared::theme_global::game_mismatch_fill(),
            format!(
                "{game_mismatch} game mismatch{}",
                if game_mismatch == 1 { "" } else { "es" }
            ),
        ));
    }
    if path_requirements > 0 {
        return Some((
            crate::ui::shared::theme_global::missing_dep_parent(),
            crate::ui::shared::theme_global::info_fill(),
            format!(
                "{path_requirements} path requirement{}",
                if path_requirements == 1 { "" } else { "s" }
            ),
        ));
    }
    if deprecated > 0 {
        return Some((
            crate::ui::shared::theme_global::warning_parent(),
            crate::ui::shared::theme_global::warning_fill(),
            format!("{deprecated} deprecated"),
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
    if included > 0 {
        return Some((
            crate::ui::shared::theme_global::included(),
            crate::ui::shared::theme_global::included_fill(),
            format!("{included} included"),
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
        "missing_dep",
        "game_mismatch",
        "path_requirement",
        "deprecated",
        "conditional",
        "included",
        "not_needed",
        "warning",
    ];
    for kind in priority {
        let prefer_checked = kind == "conflict"
            || kind == "order_block"
            || kind == "not_compatible"
            || kind == "missing_dep";
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

}

mod render_filter {
use crate::ui::state::Step2ModState;

pub(super) fn finalize_mod_checked_state(mod_state: &mut Step2ModState) {
    let has_components = !mod_state.components.is_empty();
    mod_state.checked = has_components
        && mod_state.components.iter().filter(|c| !c.disabled).count() > 0
        && mod_state
            .components
            .iter()
            .filter(|component| !component.disabled)
            .all(|component| component.checked);
}

}

pub(crate) mod render {
    use eframe::egui;
use crate::ui::state::{Step2ModState, Step2Selection};
use crate::ui::step2::service_step2::mod_matches_filter;

use crate::ui::step2::tree_components_step2::{
    render_component_rows, ComponentRowsContext,
};
use super::render_filter::finalize_mod_checked_state;
use crate::ui::step2::tree_parent_step2::{ParentRowResult, render_parent_row};

pub struct ModTreeRenderResult {
    pub selected: Step2Selection,
    pub open_compat_for_component: Option<(String, String, String)>,
    pub open_prompt_popup: Option<(String, String)>,
}

pub struct ModTreeRenderContext<'a> {
    pub filter: &'a str,
    pub active_tab: &'a str,
    pub selected: &'a Option<Step2Selection>,
    pub next_selection_order: &'a mut usize,
    pub prompt_eval: &'a crate::ui::step2::state_step2::PromptEvalContext,
    pub collapse_epoch: u64,
    pub collapse_default_open: bool,
    pub jump_to_selected_requested: &'a mut bool,
}

pub fn render_mod_tree(
    ui: &mut egui::Ui,
    ctx: &mut ModTreeRenderContext<'_>,
    mod_state: &mut Step2ModState,
) -> Option<ModTreeRenderResult> {
    if !mod_matches_filter(mod_state, ctx.filter) {
        return None;
    }

    let header_id = egui::Id::new((
        "mod_header",
        ctx.collapse_epoch,
        &mod_state.tp_file,
        &mod_state.name,
        &mod_state.tp2_path,
    ));
    let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(
        ui.ctx(),
        header_id,
        ctx.collapse_default_open,
    );
    if *ctx.jump_to_selected_requested
        && selection_targets_mod(ctx.selected, ctx.active_tab, &mod_state.tp_file)
    {
        state.set_open(true);
    }

    let mut new_selection: Option<Step2Selection> = None;
    let mut open_compat_for_component: Option<(String, String, String)> = None;
    let mut open_prompt_popup: Option<(String, String)> = None;
    state
        .show_header(ui, |ui| {
            let ParentRowResult {
                selection,
                open_compat_for_component: parent_compat,
                open_prompt_popup: parent_prompt,
            } = render_parent_row(
                ui,
                mod_state,
                ctx.active_tab,
                ctx.selected,
                ctx.next_selection_order,
                ctx.prompt_eval,
                ctx.jump_to_selected_requested,
            );
            if selection.is_some() {
                new_selection = selection;
            }
            if parent_compat.is_some() {
                open_compat_for_component = parent_compat;
            }
            if parent_prompt.is_some() {
                open_prompt_popup = parent_prompt;
            }
        })
        .body(|ui| {
            let tp_file = mod_state.tp_file.clone();
            let mod_name = mod_state.name.clone();
            let mut row_ctx = ComponentRowsContext {
                filter: ctx.filter,
                active_tab: ctx.active_tab,
                selected: ctx.selected,
                next_selection_order: ctx.next_selection_order,
                prompt_eval: ctx.prompt_eval,
                jump_to_selected_requested: ctx.jump_to_selected_requested,
                tp_file: &tp_file,
                mod_name: &mod_name,
            };
            let row_result = render_component_rows(ui, &mut row_ctx, mod_state);
            if row_result.selection.is_some() {
                new_selection = row_result.selection;
            }
            if row_result.compat_popup.is_some() {
                open_compat_for_component = row_result.compat_popup;
            }
            if row_result.prompt_popup.is_some() {
                open_prompt_popup = row_result.prompt_popup;
            }
        });

    finalize_mod_checked_state(mod_state);
    if new_selection.is_some() || open_compat_for_component.is_some() || open_prompt_popup.is_some()
    {
        let selected = new_selection.unwrap_or_else(|| Step2Selection::Mod {
            game_tab: ctx.active_tab.to_string(),
            tp_file: mod_state.tp_file.clone(),
        });
        Some(ModTreeRenderResult {
            selected,
            open_compat_for_component,
            open_prompt_popup,
        })
    } else {
        None
    }
}

fn selection_targets_mod(
    selected: &Option<Step2Selection>,
    active_tab: &str,
    tp_file: &str,
) -> bool {
    match selected {
        Some(Step2Selection::Mod { game_tab, tp_file: selected_tp }) => {
            game_tab == active_tab && selected_tp == tp_file
        }
        Some(Step2Selection::Component {
            game_tab,
            tp_file: selected_tp,
            ..
        }) => game_tab == active_tab && selected_tp == tp_file,
        None => false,
    }
}

}

pub(crate) use render::{ModTreeRenderResult, render_mod_tree};
}
