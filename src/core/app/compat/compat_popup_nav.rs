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
    pub(crate) kind: Option<String>,
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
    state.step2.active_game_tab.clone_from(&target.game_tab);
    state.step2.jump_to_selected_requested = true;
    if state.current_step == 2 {
        let _ = step3_jump_to_target(
            state,
            &target.game_tab,
            &target.tp_file,
            target.component_id.trim().parse::<u32>().ok(),
        );
    }
    state
        .step2
        .compat_popup_issue_override
        .clone_from(&target.issue_override);
    state.step2.compat_popup_open = true;
}

pub(crate) fn available_popup_filters(state: &WizardState) -> Vec<(&'static str, usize)> {
    let candidates = collect_popup_targets_with_filter(state, "All");
    let mut out = vec![("All", candidates.len())];
    for option in COMPAT_POPUP_FILTER_OPTIONS {
        if option.eq_ignore_ascii_case("All") {
            continue;
        }
        let count = candidates
            .iter()
            .filter(|target| compat_filter_matches(option, target.kind.as_deref()))
            .count();
        if count > 0 {
            out.push((*option, count));
        }
    }
    out
}

pub(crate) fn normalize_popup_filter(state: &mut WizardState, options: &[(&'static str, usize)]) {
    let is_available = options
        .iter()
        .any(|(name, _)| state.step2.compat_popup_filter.eq_ignore_ascii_case(name));
    if !is_available {
        state.step2.compat_popup_filter = "All".to_string();
    }
}

pub(crate) fn select_first_matching_target(state: &mut WizardState) {
    let targets = collect_popup_targets(state);
    if let Some(current) = current_target_key(state)
        && targets.iter().any(|target| {
            target.game_tab == current.0
                && target.tp_file == current.1
                && target.component_id == current.2
                && target.component_key == current.3
        })
    {
        return;
    }
    let Some(first) = targets.into_iter().next() else {
        return;
    };
    select_popup_target(state, &first);
}

pub(crate) fn refresh_popup_override(state: &mut WizardState) {
    if state.current_step == 2 {
        state.step2.compat_popup_issue_override =
            current_target_override(state).and_then(|target| target.issue_override);
    } else {
        state.step2.compat_popup_issue_override = None;
    }
    state.step2.compat_popup_open = true;
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
    state.step2.active_game_tab.clone_from(&game_tab);
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
    collect_popup_targets_with_filter(state, &state.step2.compat_popup_filter)
}

fn collect_popup_targets_with_filter(state: &WizardState, filter: &str) -> Vec<PopupCompatTarget> {
    let Some(game_tab) = selected_game_tab(state) else {
        return Vec::new();
    };
    if state.current_step == 2 {
        collect_step3_targets(state, &game_tab, filter)
    } else {
        collect_step2_targets(state, &game_tab, filter)
    }
}

fn collect_step2_targets(
    state: &WizardState,
    game_tab: &str,
    filter: &str,
) -> Vec<PopupCompatTarget> {
    let mods = if game_tab.eq_ignore_ascii_case("BGEE") {
        &state.step2.bgee_mods
    } else {
        &state.step2.bg2ee_mods
    };
    let mut out = Vec::<PopupCompatTarget>::new();
    for mod_state in mods {
        for component in &mod_state.components {
            if !component.checked {
                continue;
            }
            if component.compat_kind.is_none() && component.disabled_reason.is_none() {
                continue;
            }
            if !compat_filter_matches(filter, component.compat_kind.as_deref()) {
                continue;
            }
            out.push(PopupCompatTarget {
                game_tab: game_tab.to_string(),
                tp_file: mod_state.tp_file.clone(),
                component_id: component.component_id.clone(),
                component_key: component.raw_line.clone(),
                kind: component.compat_kind.clone(),
                issue_override: None,
            });
        }
    }
    out
}

fn collect_step3_targets(
    state: &WizardState,
    game_tab: &str,
    filter: &str,
) -> Vec<PopupCompatTarget> {
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
        if !compat_filter_matches(filter, Some(marker.kind.as_str())) {
            continue;
        }
        out.push(PopupCompatTarget {
            game_tab: game_tab.to_string(),
            tp_file: item.tp_file.clone(),
            component_id: item.component_id.clone(),
            component_key: item.raw_line.clone(),
            kind: Some(marker.kind.clone()),
            issue_override: Some(compat_step3_rules::marker_issue(marker)),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::{Step2ComponentState, Step2ModState};

    fn component(id: &str, checked: bool, kind: &str) -> Step2ComponentState {
        Step2ComponentState {
            component_id: id.to_string(),
            label: id.to_string(),
            weidu_group: None,
            collapsible_group: None,
            collapsible_group_is_umbrella: false,
            collapsible_group_combinable: false,
            raw_line: format!("~MOD.TP2~ #0 #{id}"),
            prompt_summary: None,
            prompt_events: Vec::new(),
            is_meta_mode_component: false,
            disabled: false,
            compat_kind: Some(kind.to_string()),
            compat_source: None,
            compat_related_mod: None,
            compat_related_component: None,
            compat_graph: None,
            compat_evidence: None,
            disabled_reason: None,
            checked,
            selected_order: None,
        }
    }

    fn state_with(components: Vec<Step2ComponentState>) -> WizardState {
        let mut state = WizardState {
            current_step: 1,
            ..WizardState::default()
        };
        state.step2.active_game_tab = "BGEE".to_string();
        state.step2.compat_popup_filter = "All".to_string();
        state.step2.bgee_mods = vec![Step2ModState {
            name: "Mod".to_string(),
            tp_file: "mod.tp2".to_string(),
            tp2_path: String::new(),
            readme_path: None,
            ini_path: None,
            web_url: None,
            package_marker: None,
            latest_checked_version: None,
            update_locked: false,
            mod_prompt_summary: None,
            mod_prompt_events: Vec::new(),
            checked: false,
            hidden_components: Vec::new(),
            components,
        }];
        state.step2.selected = Some(Step2Selection::Mod {
            game_tab: "BGEE".to_string(),
            tp_file: "mod.tp2".to_string(),
        });
        state
    }

    #[test]
    fn step2_targets_skip_unticked_components() {
        let state = state_with(vec![
            component("1", true, "conflict"),
            component("2", false, "conflict"),
        ]);
        let targets = collect_popup_targets(&state);
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].component_id, "1");
    }

    #[test]
    fn available_filters_lead_with_all_and_list_present_categories_only() {
        let state = state_with(vec![
            component("1", true, "conflict"),
            component("2", true, "not_compatible"),
            component("3", true, "warning"),
            component("4", false, "mismatch"),
        ]);
        let filters = available_popup_filters(&state);
        assert_eq!(filters[0], ("All", 3));
        assert!(filters.contains(&("Conflict", 2)));
        assert!(filters.contains(&("Warning", 1)));
        assert!(!filters.iter().any(|(name, _)| *name == "Mismatch"));
    }

    #[test]
    fn absent_stored_filter_falls_back_to_all() {
        let mut state = state_with(vec![component("1", true, "conflict")]);
        state.step2.compat_popup_filter = "Mismatch".to_string();
        let options = available_popup_filters(&state);
        normalize_popup_filter(&mut state, &options);
        assert_eq!(state.step2.compat_popup_filter, "All");

        state.step2.compat_popup_filter = "Conflict".to_string();
        let options = available_popup_filters(&state);
        normalize_popup_filter(&mut state, &options);
        assert_eq!(state.step2.compat_popup_filter, "Conflict");
    }

    #[test]
    fn single_pass_filter_counts_match_per_filter_collection() {
        let state = state_with(vec![
            component("1", true, "conflict"),
            component("2", true, "not_compatible"),
            component("3", true, "order_block"),
            component("4", true, "warning"),
            component("5", true, "some_unmapped_kind"),
            component("6", false, "conflict"),
        ]);
        for (name, count) in available_popup_filters(&state) {
            assert_eq!(
                count,
                collect_popup_targets_with_filter(&state, name).len(),
                "count mismatch for filter {name}"
            );
        }
    }

    #[test]
    fn selecting_a_filter_lands_on_its_first_target() {
        let mut state = state_with(vec![
            component("1", true, "conflict"),
            component("2", true, "warning"),
        ]);
        state.step2.compat_popup_filter = "Warning".to_string();
        select_first_matching_target(&mut state);
        match state.step2.selected.as_ref() {
            Some(Step2Selection::Component { component_id, .. }) => {
                assert_eq!(component_id, "2");
            }
            _ => panic!("expected a component selection"),
        }
    }
}
