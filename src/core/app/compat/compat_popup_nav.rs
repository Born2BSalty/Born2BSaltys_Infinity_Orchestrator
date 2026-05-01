// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::compat_issue::CompatIssue;
use crate::app::compat_popup_targets::issue_related_target;
use crate::app::compat_step3_rules;
use crate::app::selection_jump::{
    selected_step2_jump_target, step2_jump_to_target, step3_jump_to_target,
};
use crate::app::state::{Step2Selection, WizardState};

#[derive(Clone)]
pub(crate) struct PopupCompatTarget {
    pub(crate) game_tab: String,
    pub(crate) tp_file: String,
    pub(crate) component_id: String,
    pub(crate) component_key: String,
    pub(crate) issue_override: Option<CompatIssue>,
}

pub(crate) const COMPAT_POPUP_FILTER_OPTIONS: &[&str] = &[
    "All",
    "Conflict",
    "Order",
    "Mismatch",
    "Missing",
    "Included",
    "Path",
    "Conditional",
    "Deprecated",
    "Warning",
    "Other",
];

pub(crate) fn next_target(state: &WizardState) -> Option<PopupCompatTarget> {
    let targets = collect_popup_targets(state);
    if targets.is_empty() {
        return None;
    }
    let current = current_target_key(state);
    if let Some((game_tab, tp_file, component_id, component_key)) = current
        && let Some(index) = targets.iter().position(|target| {
            target.game_tab == game_tab
                && target.tp_file == tp_file
                && target.component_id == component_id
                && target.component_key == component_key
        })
    {
        return Some(targets[(index + 1) % targets.len()].clone());
    }
    targets.into_iter().next()
}

pub(crate) fn select_popup_target(state: &mut WizardState, target: &PopupCompatTarget) {
    state.step2.selected = Some(Step2Selection::Component {
        game_tab: target.game_tab.clone(),
        tp_file: target.tp_file.clone(),
        component_id: target.component_id.clone(),
        component_key: target.component_key.clone(),
    });
    state.step2.active_game_tab = target.game_tab.clone();
    state.step2.jump_to_selected_requested = true;
    if state.current_step == 2 {
        let _ = step3_jump_to_target(
            state,
            &target.game_tab,
            &target.tp_file,
            target.component_id.trim().parse::<u32>().ok(),
        );
    }
    state.step2.compat_popup_issue_override = target.issue_override.clone();
    state.step2.compat_popup_open = true;
}

pub(crate) fn refresh_popup_override(state: &mut WizardState) {
    if state.current_step == 2 {
        let Some(target) = current_target_override(state) else {
            state.step2.compat_popup_issue_override = None;
            state.step2.compat_popup_open = true;
            return;
        };
        state.step2.compat_popup_issue_override = target.issue_override;
        state.step2.compat_popup_open = true;
    } else {
        state.step2.compat_popup_issue_override = None;
        state.step2.compat_popup_open = true;
    }
}

pub(crate) fn selected_game_tab(state: &WizardState) -> Option<String> {
    match state.step2.selected.as_ref()? {
        Step2Selection::Mod { game_tab, .. } | Step2Selection::Component { game_tab, .. } => {
            Some(game_tab.clone())
        }
    }
}

pub(crate) fn can_jump_to_related(issue: &CompatIssue) -> bool {
    issue_related_target(issue).is_some()
}

pub(crate) fn jump_to_this(state: &mut WizardState) {
    if state.current_step == 2
        && let Some((game_tab, mod_ref, component_ref)) = selected_step2_jump_target(state)
        && step3_jump_to_target(state, &game_tab, &mod_ref, component_ref)
    {
        state.current_step = 2;
        refresh_popup_override(state);
        return;
    }

    let Some(game_tab) = selected_game_tab(state) else {
        return;
    };
    state.current_step = 1;
    state.step2.active_game_tab = game_tab;
    state.step2.jump_to_selected_requested = true;
    state.step2.compat_popup_issue_override = None;
}

pub(crate) fn jump_to_related(state: &mut WizardState, issue: &CompatIssue) {
    let Some(game_tab) = selected_game_tab(state) else {
        return;
    };
    let Some((related_mod, related_component)) = issue_related_target(issue) else {
        return;
    };
    step2_jump_to_target(state, &game_tab, &related_mod, related_component);
    state.step2.active_game_tab = game_tab.clone();
    state.step2.jump_to_selected_requested = true;
    if state.current_step == 2 {
        if step3_jump_to_target(state, &game_tab, &related_mod, related_component) {
            state.current_step = 2;
            refresh_popup_override(state);
        } else {
            state.current_step = 1;
            state.step2.compat_popup_issue_override = None;
        }
    } else {
        state.current_step = 1;
        state.step2.compat_popup_issue_override = None;
    }
}

pub(crate) fn compat_filter_matches(filter: &str, kind: Option<&str>) -> bool {
    let Some(kind) = kind.map(|value| value.trim().to_ascii_lowercase()) else {
        return filter.eq_ignore_ascii_case("All");
    };
    if filter.eq_ignore_ascii_case("All") {
        return true;
    }
    match filter.trim().to_ascii_lowercase().as_str() {
        "conflict" => matches!(kind.as_str(), "conflict" | "not_compatible"),
        "order" => kind == "order_block",
        "mismatch" => matches!(kind.as_str(), "mismatch" | "game_mismatch"),
        "missing" => kind == "missing_dep",
        "included" => matches!(kind.as_str(), "included" | "not_needed"),
        "path" => kind == "path_requirement",
        "conditional" => kind == "conditional",
        "deprecated" => kind == "deprecated",
        "warning" => kind == "warning",
        "other" => !matches!(
            kind.as_str(),
            "conflict"
                | "not_compatible"
                | "order_block"
                | "mismatch"
                | "game_mismatch"
                | "missing_dep"
                | "included"
                | "not_needed"
                | "path_requirement"
                | "conditional"
                | "deprecated"
                | "warning"
        ),
        _ => true,
    }
}

fn current_target_override(state: &WizardState) -> Option<PopupCompatTarget> {
    let current = current_target_key(state)?;
    collect_popup_targets(state).into_iter().find(|target| {
        target.game_tab == current.0
            && target.tp_file == current.1
            && target.component_id == current.2
            && target.component_key == current.3
    })
}

fn current_target_key(state: &WizardState) -> Option<(String, String, String, String)> {
    match state.step2.selected.as_ref()? {
        Step2Selection::Mod { .. } => None,
        Step2Selection::Component {
            game_tab,
            tp_file,
            component_id,
            component_key,
        } => Some((
            game_tab.clone(),
            tp_file.clone(),
            component_id.clone(),
            component_key.clone(),
        )),
    }
}

fn collect_popup_targets(state: &WizardState) -> Vec<PopupCompatTarget> {
    let Some(game_tab) = selected_game_tab(state) else {
        return Vec::new();
    };
    if state.current_step == 2 {
        collect_step3_targets(state, &game_tab)
    } else {
        collect_step2_targets(state, &game_tab)
    }
}

fn collect_step2_targets(state: &WizardState, game_tab: &str) -> Vec<PopupCompatTarget> {
    let mods = if game_tab.eq_ignore_ascii_case("BGEE") {
        &state.step2.bgee_mods
    } else {
        &state.step2.bg2ee_mods
    };
    let mut out = Vec::<PopupCompatTarget>::new();
    for mod_state in mods {
        for component in &mod_state.components {
            if component.compat_kind.is_none() && component.disabled_reason.is_none() {
                continue;
            }
            if !compat_filter_matches(
                &state.step2.compat_popup_filter,
                component.compat_kind.as_deref(),
            ) {
                continue;
            }
            out.push(PopupCompatTarget {
                game_tab: game_tab.to_string(),
                tp_file: mod_state.tp_file.clone(),
                component_id: component.component_id.clone(),
                component_key: component.raw_line.clone(),
                issue_override: None,
            });
        }
    }
    out
}

fn collect_step3_targets(state: &WizardState, game_tab: &str) -> Vec<PopupCompatTarget> {
    let (mods, items) = if game_tab.eq_ignore_ascii_case("BGEE") {
        (&state.step2.bgee_mods, &state.step3.bgee_items)
    } else {
        (&state.step2.bg2ee_mods, &state.step3.bg2ee_items)
    };
    let markers =
        compat_step3_rules::collect_step3_compat_markers(&state.step1, game_tab, mods, items);
    let mut out = Vec::<PopupCompatTarget>::new();
    for item in items.iter().filter(|item| !item.is_parent) {
        let key = compat_step3_rules::marker_key(item);
        let Some(marker) = markers.get(&key) else {
            continue;
        };
        if !compat_filter_matches(&state.step2.compat_popup_filter, Some(marker.kind.as_str())) {
            continue;
        }
        out.push(PopupCompatTarget {
            game_tab: game_tab.to_string(),
            tp_file: item.tp_file.clone(),
            component_id: item.component_id.clone(),
            component_key: item.raw_line.clone(),
            issue_override: Some(compat_step3_rules::marker_issue(marker)),
        });
    }
    out
}
